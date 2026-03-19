#!/usr/bin/env python3
"""
Aztibase Tx Anomaly Scorer — ONNX Autoencoder Training

Trains an autoencoder on normal transaction features from tx_features.csv,
exports to ONNX + JSON params for use by AnomalyScorer in aztibase-runtime.

Usage:
    pip install torch numpy onnx
    py -X utf8 tools/train_tx_anomaly.py --csv data/node1/tx_features.csv

Collect training data:
    aztibase --tx-anomaly-export   (adds per-tx rows to data/<node>/tx_features.csv)

Output:
    models/tx_anomaly_v1.onnx          — autoencoder model
    models/tx_anomaly_v1_params.json   — normalization params + threshold
"""

import argparse
import json
import sys
from pathlib import Path

import numpy as np

try:
    import torch
    import torch.nn as nn
    from torch.utils.data import DataLoader, TensorDataset
except ImportError:
    print("Install PyTorch: pip install torch")
    sys.exit(1)

# ── Feature config ────────────────────────────────────────────────

FEATURE_NAMES = [
    "value", "gas_price", "gas_limit", "payload_size",
    "is_contract_deploy", "is_ai_infer",
]

NUM_FEATURES = len(FEATURE_NAMES)
HEALTHY_SCORE_THRESHOLD = 0.15


# ── Autoencoder ───────────────────────────────────────────────────

class TxAutoencoder(nn.Module):
    def __init__(self, input_dim=6, hidden_dim=4, latent_dim=2):
        super().__init__()
        self.encoder = nn.Sequential(
            nn.Linear(input_dim, hidden_dim),
            nn.ReLU(),
            nn.Linear(hidden_dim, latent_dim),
            nn.ReLU(),
        )
        self.decoder = nn.Sequential(
            nn.Linear(latent_dim, hidden_dim),
            nn.ReLU(),
            nn.Linear(hidden_dim, input_dim),
        )

    def forward(self, x):
        return self.decoder(self.encoder(x))


# ── Data loading ──────────────────────────────────────────────────

def load_csv(csv_path: str):
    """Load tx features CSV, return features array and scores."""
    data = np.genfromtxt(csv_path, delimiter=",", skip_header=1)
    if data.ndim == 1:
        data = data.reshape(1, -1)

    if data.shape[0] == 0:
        print(f"ERROR: No data rows in {csv_path}")
        sys.exit(1)

    # Columns: batch_height, score, value, gas_price, gas_limit,
    #          payload_size, is_contract_deploy, is_ai_infer
    scores = data[:, 1]
    features = data[:, 2:2 + NUM_FEATURES]

    if features.shape[1] != NUM_FEATURES:
        print(f"ERROR: Expected {NUM_FEATURES} feature columns, got {features.shape[1]}")
        sys.exit(1)

    return features, scores


def merge_csvs(csv_paths):
    """Merge multiple CSV files (e.g., from different nodes)."""
    all_features = []
    all_scores = []
    for p in csv_paths:
        f, s = load_csv(p)
        all_features.append(f)
        all_scores.append(s)
        print(f"  {p}: {len(f)} rows")
    return np.vstack(all_features), np.concatenate(all_scores)


def normalize(features: np.ndarray):
    """Min-max normalize to [0, 1]. Returns normalized data + params."""
    mins = features.min(axis=0)
    maxs = features.max(axis=0)
    ranges = maxs - mins
    ranges[ranges == 0] = 1.0
    normalized = (features - mins) / ranges
    return normalized, mins, ranges


# ── Training ──────────────────────────────────────────────────────

def train_model(features: np.ndarray, epochs: int = 300, lr: float = 0.001, batch_size: int = 64):
    model = TxAutoencoder(NUM_FEATURES, 4, 2)
    optimizer = torch.optim.Adam(model.parameters(), lr=lr)
    criterion = nn.MSELoss()

    tensor_data = torch.FloatTensor(features)
    dataset = TensorDataset(tensor_data, tensor_data)
    loader = DataLoader(dataset, batch_size=batch_size, shuffle=True)

    model.train()
    for epoch in range(epochs):
        total_loss = 0.0
        count = 0
        for batch_x, _ in loader:
            output = model(batch_x)
            loss = criterion(output, batch_x)
            optimizer.zero_grad()
            loss.backward()
            optimizer.step()
            total_loss += loss.item()
            count += 1

        if (epoch + 1) % 50 == 0 or epoch == 0:
            avg = total_loss / max(count, 1)
            print(f"  Epoch {epoch + 1:>4d}/{epochs}  loss={avg:.6f}")

    return model


def compute_threshold(model: nn.Module, features: np.ndarray, percentile: float = 99.0):
    """Compute anomaly threshold from training data reconstruction errors."""
    model.eval()
    with torch.no_grad():
        tensor_data = torch.FloatTensor(features)
        output = model(tensor_data)
        errors = torch.mean((output - tensor_data) ** 2, dim=1).numpy()

    threshold = float(np.percentile(errors, percentile))
    print(f"  Reconstruction errors: min={errors.min():.6f}  max={errors.max():.6f}  mean={errors.mean():.6f}")
    print(f"  Anomaly threshold (p{percentile:.0f}): {threshold:.6f}")
    return threshold


# ── Export ────────────────────────────────────────────────────────

def export_onnx(model: nn.Module, output_path: str):
    model.eval()
    dummy = torch.randn(1, NUM_FEATURES)
    torch.onnx.export(
        model,
        (dummy,),
        output_path,
        input_names=["features"],
        output_names=["reconstructed"],
        opset_version=13,
        dynamo=False,
    )
    size_kb = Path(output_path).stat().st_size / 1024
    print(f"  Exported: {output_path} ({size_kb:.1f} KB)")


def save_params_json(mins: np.ndarray, ranges: np.ndarray, threshold: float, output_path: str):
    """Save normalization params as JSON for Rust AnomalyScorer to load."""
    params = {
        "mins": mins.astype(float).tolist(),
        "ranges": ranges.astype(float).tolist(),
        "threshold": float(threshold),
        "feature_names": FEATURE_NAMES,
    }
    with open(output_path, "w") as f:
        json.dump(params, f, indent=2)
    print(f"  Params: {output_path}")


# ── Main ──────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Train tx anomaly autoencoder")
    parser.add_argument("--csv", required=True, nargs="+", help="Path(s) to tx_features.csv (can merge multiple)")
    parser.add_argument("--output-dir", default="models", help="Output directory for ONNX model")
    parser.add_argument("--epochs", type=int, default=300, help="Training epochs")
    parser.add_argument("--min-rows", type=int, default=200, help="Minimum rows required")
    parser.add_argument("--threshold-percentile", type=float, default=99.0, help="Anomaly threshold percentile")
    args = parser.parse_args()

    print(f"\n=== Aztibase Tx Anomaly Scorer Training ===\n")

    # Load data
    print("Loading CSV(s):")
    if len(args.csv) == 1:
        features, scores = load_csv(args.csv[0])
        print(f"  {args.csv[0]}: {len(features)} rows")
    else:
        features, scores = merge_csvs(args.csv)

    print(f"  Total rows: {len(features)}")

    # Filter healthy rows only (low heuristic score = normal transactions)
    healthy_mask = scores <= HEALTHY_SCORE_THRESHOLD
    healthy = features[healthy_mask]
    anomalous = features[~healthy_mask]
    print(f"  Normal txs: {len(healthy)}  (score <= {HEALTHY_SCORE_THRESHOLD})")
    print(f"  Anomalous txs: {len(anomalous)}  (for validation)")

    if len(healthy) < args.min_rows:
        print(f"\n  NOT ENOUGH DATA. Need at least {args.min_rows} normal tx rows.")
        print(f"  Run the testnet with --tx-anomaly-export and send transactions.")
        print(f"  Transfers, contract calls, and faucet drips all generate training data.\n")
        sys.exit(1)

    # Normalize
    print("\nNormalizing features...")
    normalized, mins, ranges = normalize(healthy)
    for i, name in enumerate(FEATURE_NAMES):
        print(f"  {name}: min={mins[i]:.2f}  range={ranges[i]:.2f}")

    # Train
    print(f"\nTraining autoencoder ({NUM_FEATURES}->4->2->4->{NUM_FEATURES})...")
    model = train_model(normalized, epochs=args.epochs)

    # Compute threshold
    print("\nComputing anomaly threshold...")
    threshold = compute_threshold(model, normalized, args.threshold_percentile)

    # Validate against anomalous rows (if any)
    if len(anomalous) > 0:
        print(f"\nValidating against {len(anomalous)} anomalous txs...")
        model.eval()
        with torch.no_grad():
            norm_anom = (anomalous - mins) / ranges
            tensor_anom = torch.FloatTensor(norm_anom.astype(np.float32))
            output_anom = model(tensor_anom)
            anom_errors = torch.mean((output_anom - tensor_anom) ** 2, dim=1).numpy()
        detected = (anom_errors > threshold).sum()
        print(f"  Detected {detected}/{len(anomalous)} anomalies ({detected / len(anomalous) * 100:.1f}%)")

    # Export
    output_dir = Path(args.output_dir)
    output_dir.mkdir(exist_ok=True)

    print(f"\nExporting model...")
    onnx_path = str(output_dir / "tx_anomaly_v1.onnx")
    params_path = str(output_dir / "tx_anomaly_v1_params.json")

    export_onnx(model, onnx_path)
    save_params_json(mins, ranges, threshold, params_path)

    print(f"\n=== Done ===")
    print(f"Model:  {onnx_path}")
    print(f"Params: {params_path}")
    print(f"\nCopy both files to models/ or data/<node>/models/ and restart the node.")
    print(f"The AnomalyScorer will auto-load the ONNX model on startup.\n")


if __name__ == "__main__":
    main()
