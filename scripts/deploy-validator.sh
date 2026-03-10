#!/usr/bin/env bash
set -euo pipefail

BINARY_URL="${BINARY_URL:-}"
GENESIS_URL="${GENESIS_URL:-}"
NODE_NAME="${NODE_NAME:-validator1}"
DATA_DIR="${DATA_DIR:-/opt/aztibase}"
RPC_ADDR="${RPC_ADDR:-0.0.0.0:9944}"
P2P_PORT="${P2P_PORT:-30333}"
LOG_LEVEL="${LOG_LEVEL:-info}"
SERVICE_USER="${SERVICE_USER:-aztibase}"

usage() {
    echo "Usage: deploy-validator.sh [OPTIONS]"
    echo ""
    echo "Deploy an Aztibase validator node on a VPS/cloud server."
    echo ""
    echo "Environment variables:"
    echo "  BINARY_URL    URL to download the aztibase binary (required unless --testnet)"
    echo "  GENESIS_URL   URL to download genesis.toml (optional if using --testnet)"
    echo "  NODE_NAME     Node directory name (default: validator1)"
    echo "  DATA_DIR      Data directory (default: /opt/aztibase)"
    echo "  RPC_ADDR      RPC listen address (default: 0.0.0.0:9944)"
    echo "  P2P_PORT      P2P listen port (default: 30333)"
    echo "  LOG_LEVEL     Log level (default: info)"
    echo "  SERVICE_USER  System user to run the node (default: aztibase)"
    echo ""
    echo "Options:"
    echo "  --testnet     Use built-in testnet genesis and boot nodes"
    echo "  --help        Show this help"
    exit 0
}

TESTNET_FLAG=""
for arg in "$@"; do
    case "$arg" in
        --testnet) TESTNET_FLAG="--testnet" ;;
        --help) usage ;;
    esac
done

if [ -z "$BINARY_URL" ] && [ ! -f "$DATA_DIR/bin/aztibase" ]; then
    echo "Error: BINARY_URL not set and no existing binary at $DATA_DIR/bin/aztibase"
    echo "Set BINARY_URL to download, or place the binary manually."
    exit 1
fi

echo "=== Aztibase Validator Deployment ==="
echo "Node: $NODE_NAME"
echo "Data: $DATA_DIR"
echo "RPC:  $RPC_ADDR"
echo "P2P:  $P2P_PORT"

# Create system user
if ! id "$SERVICE_USER" &>/dev/null; then
    echo "Creating user $SERVICE_USER..."
    useradd --system --no-create-home --shell /usr/sbin/nologin "$SERVICE_USER"
fi

# Create directories
mkdir -p "$DATA_DIR/bin" "$DATA_DIR/$NODE_NAME"

# Download binary
if [ -n "$BINARY_URL" ]; then
    echo "Downloading binary..."
    curl -fsSL "$BINARY_URL" -o "$DATA_DIR/bin/aztibase"
    chmod +x "$DATA_DIR/bin/aztibase"
fi

# Download genesis (unless using --testnet)
if [ -n "$GENESIS_URL" ] && [ -z "$TESTNET_FLAG" ]; then
    echo "Downloading genesis config..."
    curl -fsSL "$GENESIS_URL" -o "$DATA_DIR/$NODE_NAME/genesis.toml"
fi

# Generate validator key if not present
KEY_DIR="$DATA_DIR/$NODE_NAME/keys"
mkdir -p "$KEY_DIR"
if [ -z "$(ls -A "$KEY_DIR" 2>/dev/null)" ]; then
    echo "Generating validator keypair..."
    "$DATA_DIR/bin/aztibase" wallet generate \
        --output "$KEY_DIR/$NODE_NAME.json" \
        --mnemonic --passphrase ""
    echo "WARNING: Key generated without passphrase. Secure it for production."
fi

# Build CLI args
ARGS="--data-dir $DATA_DIR/$NODE_NAME"
ARGS="$ARGS --rpc-addr $RPC_ADDR"
ARGS="$ARGS --listen /ip4/0.0.0.0/tcp/$P2P_PORT"
ARGS="$ARGS --listen /ip4/0.0.0.0/udp/$P2P_PORT/quic-v1"
ARGS="$ARGS --log-level $LOG_LEVEL"
ARGS="$ARGS --metrics"

if [ -n "$TESTNET_FLAG" ]; then
    ARGS="$ARGS --testnet"
elif [ -f "$DATA_DIR/$NODE_NAME/genesis.toml" ]; then
    ARGS="$ARGS --genesis $DATA_DIR/$NODE_NAME/genesis.toml"
fi

KEY_FILE=$(ls "$KEY_DIR"/*.json 2>/dev/null | head -1)
if [ -n "$KEY_FILE" ]; then
    ARGS="$ARGS --validator-key $KEY_FILE"
fi

chown -R "$SERVICE_USER:$SERVICE_USER" "$DATA_DIR"

# Create systemd service
cat > /etc/systemd/system/aztibase-validator.service << UNIT
[Unit]
Description=Aztibase Network Validator ($NODE_NAME)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$SERVICE_USER
ExecStart=$DATA_DIR/bin/aztibase $ARGS
Restart=on-failure
RestartSec=10
LimitNOFILE=65535

Environment=RUST_LOG=$LOG_LEVEL
WorkingDirectory=$DATA_DIR/$NODE_NAME

[Install]
WantedBy=multi-user.target
UNIT

systemctl daemon-reload
systemctl enable aztibase-validator.service

echo ""
echo "=== Deployment Complete ==="
echo "Binary:  $DATA_DIR/bin/aztibase"
echo "Data:    $DATA_DIR/$NODE_NAME"
echo "Service: aztibase-validator.service"
echo ""
echo "Commands:"
echo "  systemctl start aztibase-validator   # Start the node"
echo "  systemctl status aztibase-validator  # Check status"
echo "  journalctl -u aztibase-validator -f  # Follow logs"
echo "  curl http://localhost:9944/health     # Health check"
