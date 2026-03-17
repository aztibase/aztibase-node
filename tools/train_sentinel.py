#!/usr/bin/env python3
"""
Aztibase Sentinel Tier 2 — ONNX Autoencoder Training

Trains an autoencoder on healthy chain telemetry from sentinel_features.csv,
exports to ONNX for use by TractRuntime in aztibase-node.

Usage:
    pip install torch numpy onnx
    python tools/train_sentinel.py --csv data/node1/sentinel_features.csv

Output:
    models/sentinel_v1.onnx
"""

import argparse
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
    "block_height_delta", "commit_latency_ms", "commit_latency_stddev",
    "tps", "tps_acceleration", "base_fee", "base_fee_delta",
    "equivocation_count", "validator_set_size", "total_gas_used",
    "block_fullness", "empty_block_ratio", "time_since_finality_ms",
    "peer_count", "mempool_size",
]

NUM_FEATURES = len(FEATURE_NAMES)
HEALTHY_SCORE_THRESHOLD = 0.1


# ── Autoencoder ───────────────────────────────────────────────────

class Autoencoder(nn.Module):
    def __init__(self, input_dim=15, hidden_dim=8, latent_dim=4):
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
    """Load sentinel CSV, return features array and scores."""
    data = np.genfromtxt(csv_path, delimiter=",", skip_header=1)
    if data.ndim == 1:
        data = data.reshape(1, -1)

    if data.shape[0] == 0:
        print(f"ERROR: No data rows in {csv_path}")
        sys.exit(1)

    # Columns: timestamp_ms, batch_height, score, [15 features]
    scores = data[:, 2]
    features = data[:, 3:3 + NUM_FEATURES]

    if features.shape[1] != NUM_FEATURES:
        print(f"ERROR: Expected {NUM_FEATURES} features, got {features.shape[1]}")
        sys.exit(1)

    return features, scores


def normalize(features: np.ndarray):
    """Min-max normalize to [0, 1]. Returns normalized data + params for inference."""
    mins = features.min(axis=0)
    maxs = features.max(axis=0)
    ranges = maxs - mins
    ranges[ranges == 0] = 1.0  # avoid div by zero for constant features
    normalized = (features - mins) / ranges
    return normalized, mins, ranges


# ── Training ──────────────────────────────────────────────────────

def train_model(features: np.ndarray, epochs: int = 200, lr: float = 0.001, batch_size: int = 64):
    model = Autoencoder(NUM_FEATURES, 8, 4)
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

        if (epoch + 1) % 20 == 0 or epoch == 0:
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

    threshold = np.percentile(errors, percentile)
    print(f"  Reconstruction errors: min={errors.min():.6f}  max={errors.max():.6f}  mean={errors.mean():.6f}")
    print(f"  Anomaly threshold (p{percentile:.0f}): {threshold:.6f}")
    return threshold


# ── Export ────────────────────────────────────────────────────────

def export_onnx(model: nn.Module, output_path: str):
    model.eval()
    dummy = torch.randn(1, NUM_FEATURES)
    torch.onnx.export(
        model,
        dummy,
        output_path,
        input_names=["features"],
        output_names=["reconstructed"],
        dynamic_axes={"features": {0: "batch"}, "reconstructed": {0: "batch"}},
        opset_version=13,
    )
    size_kb = Path(output_path).stat().st_size / 1024
    print(f"  Exported: {output_path} ({size_kb:.1f} KB)")


def save_normalization_params(mins: np.ndarray, ranges: np.ndarray, threshold: float, output_path: str):
    """Save normalization params so Rust can normalize input before inference."""
    np.savez(
        output_path,
        mins=mins.astype(np.float32),
        ranges=ranges.astype(np.float32),
        threshold=np.float32(threshold),
        feature_names=FEATURE_NAMES,
    )
    print(f"  Normalization params: {output_path}")


# ── Main ──────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Train Sentinel Tier 2 autoencoder")
    parser.add_argument("--csv", required=True, help="Path to sentinel_features.csv")
    parser.add_argument("--output-dir", default="models", help="Output directory for ONNX model")
    parser.add_argument("--epochs", type=int, default=200, help="Training epochs")
    parser.add_argument("--min-rows", type=int, default=500, help="Minimum rows required")
    args = parser.parse_args()

    print(f"\n=== Aztibase Sentinel Tier 2 Training ===\n")

    # Load data
    print(f"Loading: {args.csv}")
    features, scores = load_csv(args.csv)
    print(f"  Total rows: {len(features)}")

    # Filter healthy rows only
    healthy_mask = scores <= HEALTHY_SCORE_THRESHOLD
    healthy = features[healthy_mask]
    anomalous = features[~healthy_mask]
    print(f"  Healthy rows: {len(healthy)}  (score <= {HEALTHY_SCORE_THRESHOLD})")
    print(f"  Anomalous rows: {len(anomalous)}  (for validation)")

    if len(healthy) < args.min_rows:
        print(f"\n  NOT ENOUGH DATA. Need at least {args.min_rows} healthy rows.")
        print(f"  Keep the testnet running with --sentinel-export.")
        print(f"  At ~1 row per 25s, {args.min_rows} rows takes ~{args.min_rows * 25 // 3600} hours.\n")
        sys.exit(1)

    # Normalize
    print("\nNormalizing features...")
    normalized, mins, ranges = normalize(healthy)

    # Train
    print(f"\nTraining autoencoder ({NUM_FEATURES}→8→4→8→{NUM_FEATURES})...")
    model = train_model(normalized, epochs=args.epochs)

    # Compute threshold
    print("\nComputing anomaly threshold...")
    threshold = compute_threshold(model, normalized)

    # Validate against anomalous rows (if any)
    if len(anomalous) > 0:
        print(f"\nValidating against {len(anomalous)} anomalous rows...")
        model.eval()
        with torch.no_grad():
            norm_anom = (anomalous - mins) / ranges
            tensor_anom = torch.FloatTensor(norm_anom)
            output_anom = model(tensor_anom)
            anom_errors = torch.mean((output_anom - tensor_anom) ** 2, dim=1).numpy()
        detected = (anom_errors > threshold).sum()
        print(f"  Detected {detected}/{len(anomalous)} anomalies ({detected / len(anomalous) * 100:.1f}%)")

    # Export
    output_dir = Path(args.output_dir)
    output_dir.mkdir(exist_ok=True)

    print(f"\nExporting model...")
    export_onnx(model, str(output_dir / "sentinel_v1.onnx"))
    save_normalization_params(mins, ranges, threshold, str(output_dir / "sentinel_v1_params.npz"))

    print(f"\n=== Done ===")
    print(f"Model: {output_dir / 'sentinel_v1.onnx'}")
    print(f"Params: {output_dir / 'sentinel_v1_params.npz'}")
    print(f"\nNext: wire up Tier 2 in sentinel.rs to load this model.\n")


if __name__ == "__main__":
    main()
