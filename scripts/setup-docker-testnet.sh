#!/usr/bin/env bash
set -euo pipefail

VALIDATOR_COUNT="${VALIDATOR_COUNT:-3}"
FUNDED_ACCOUNTS="${FUNDED_ACCOUNTS:-2}"
DATA_DIR="${DATA_DIR:-./data}"

echo "=== Aztibase Docker Testnet Setup ==="
echo "Validators:     $VALIDATOR_COUNT"
echo "Funded accounts: $FUNDED_ACCOUNTS"
echo "Data dir:       $DATA_DIR"
echo ""

cargo build --release -p aztibase-node 2>&1 | tail -1
BINARY="./target/release/aztibase"

if [ ! -f "$BINARY" ] && [ -f "./target/release/aztibase.exe" ]; then
    BINARY="./target/release/aztibase.exe"
fi

if [ -d "$DATA_DIR/genesis" ]; then
    echo "Existing testnet data found at $DATA_DIR."
    echo "Run 'make reset' first to clear state, or remove $DATA_DIR manually."
    exit 1
fi

echo "Generating Docker testnet layout..."
$BINARY genesis \
    --validators "$VALIDATOR_COUNT" \
    --funded "$FUNDED_ACCOUNTS" \
    --output "$DATA_DIR" \
    --docker

echo ""
echo "Setup complete. Run 'docker compose up --build' to start the testnet."
echo ""
echo "Services:"
echo "  Validator 1:  http://localhost:9944 (RPC)"
echo "  Validator 2:  http://localhost:9945 (RPC)"
echo "  Validator 3:  http://localhost:9946 (RPC)"
echo "  Prometheus:   http://localhost:9090"
echo "  Grafana:      http://localhost:3000 (admin/aztibase)"
