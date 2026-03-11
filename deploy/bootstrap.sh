#!/usr/bin/env bash
set -euo pipefail

# Aztibase Testnet Node Bootstrap Script
# Target: Ubuntu 22.04+ (amd64)
# Usage: curl -sSf https://raw.githubusercontent.com/user/aztibase/dev/deploy/bootstrap.sh | bash

AZTIBASE_USER="aztibase"
AZTIBASE_HOME="/var/lib/aztibase"
AZTIBASE_CONFIG="/etc/aztibase"
AZTIBASE_BIN="/usr/local/bin/aztibase"
REPO_URL="https://github.com/user/aztibase.git"
BRANCH="dev"

echo "==> Aztibase Testnet Node Bootstrap"
echo "    Target: $(uname -m) $(lsb_release -ds 2>/dev/null || echo 'Linux')"

# ── 1. Install system dependencies ──────────────────────────────────
echo "==> Installing system dependencies..."
sudo apt-get update -qq
sudo apt-get install -y -qq build-essential pkg-config libssl-dev git curl

# ── 2. Install Rust (if not present) ───────────────────────────────
if ! command -v cargo &>/dev/null; then
    echo "==> Installing Rust toolchain..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    source "$HOME/.cargo/env"
else
    echo "==> Rust already installed: $(rustc --version)"
fi

# ── 3. Clone and build ─────────────────────────────────────────────
echo "==> Cloning Aztibase repository..."
WORK_DIR=$(mktemp -d)
git clone --depth 1 --branch "$BRANCH" "$REPO_URL" "$WORK_DIR/aztibase"
cd "$WORK_DIR/aztibase"

echo "==> Building release binary (this may take 5-10 minutes)..."
CARGO_INCREMENTAL=0 cargo build --release -p aztibase-node

echo "==> Installing binary to $AZTIBASE_BIN"
sudo cp target/release/aztibase "$AZTIBASE_BIN"
sudo chmod +x "$AZTIBASE_BIN"

# ── 4. Create user and directories ─────────────────────────────────
echo "==> Setting up aztibase user and directories..."
if ! id "$AZTIBASE_USER" &>/dev/null; then
    sudo useradd --system --home-dir "$AZTIBASE_HOME" --shell /usr/sbin/nologin "$AZTIBASE_USER"
fi
sudo mkdir -p "$AZTIBASE_HOME" "$AZTIBASE_CONFIG/keys"
sudo chown -R "$AZTIBASE_USER:$AZTIBASE_USER" "$AZTIBASE_HOME"

# ── 5. Generate validator key ───────────────────────────────────────
if [ ! -f "$AZTIBASE_CONFIG/keys/validator.json" ]; then
    echo "==> Generating validator keypair..."
    "$AZTIBASE_BIN" wallet generate --output "$AZTIBASE_CONFIG/keys/validator.json"
    sudo chown "$AZTIBASE_USER:$AZTIBASE_USER" "$AZTIBASE_CONFIG/keys/validator.json"
    sudo chmod 600 "$AZTIBASE_CONFIG/keys/validator.json"
    echo "    Key saved to $AZTIBASE_CONFIG/keys/validator.json"
    echo "    IMPORTANT: Back up this file! It contains your validator identity."
fi

# ── 6. Install systemd service ──────────────────────────────────────
echo "==> Installing systemd service..."
sudo cp "$WORK_DIR/aztibase/deploy/systemd/aztibase.service" /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable aztibase

# ── 7. Firewall ─────────────────────────────────────────────────────
echo "==> Configuring firewall (ufw)..."
if command -v ufw &>/dev/null; then
    sudo ufw allow 30333/tcp comment "Aztibase P2P TCP"
    sudo ufw allow 30333/udp comment "Aztibase P2P QUIC"
    sudo ufw allow 9944/tcp comment "Aztibase RPC"
fi

# ── 8. Clean up ─────────────────────────────────────────────────────
echo "==> Cleaning up build artifacts..."
rm -rf "$WORK_DIR"

echo ""
echo "==> Bootstrap complete!"
echo ""
echo "    Start the node:     sudo systemctl start aztibase"
echo "    View logs:          sudo journalctl -u aztibase -f"
echo "    Check status:       sudo systemctl status aztibase"
echo "    RPC endpoint:       http://localhost:9944"
echo "    Metrics:            http://localhost:9944/metrics"
echo ""
echo "    Your validator address:"
if [ -f "$AZTIBASE_CONFIG/keys/validator.json" ]; then
    python3 -c "import json; print('    ' + json.load(open('$AZTIBASE_CONFIG/keys/validator.json'))['address'])" 2>/dev/null || echo "    (check $AZTIBASE_CONFIG/keys/validator.json)"
fi
echo ""
echo "    To join as a validator, stake at least 10,000 AZTB to your address."
echo "    Get testnet tokens from the faucet: https://faucet.aztibase.com"
