#!/usr/bin/env bash
set -euo pipefail

VALIDATOR_COUNT="${VALIDATOR_COUNT:-3}"
FUNDED_ACCOUNTS="${FUNDED_ACCOUNTS:-2}"
LOG_LEVEL="${LOG_LEVEL:-info}"
DATA_ROOT="${DATA_ROOT:-$(mktemp -d)}"
GENESIS_DIR="$DATA_ROOT/genesis"

echo "=== Aztibase Local Testnet ==="
echo "Validators: $VALIDATOR_COUNT"
echo "Funded:     $FUNDED_ACCOUNTS"
echo "Data root:  $DATA_ROOT"
echo "Log level:  $LOG_LEVEL"
echo ""

cargo build --release -p aztibase-node 2>&1 | tail -1
BINARY="./target/release/aztibase"

if [ ! -f "$BINARY" ] && [ -f "./target/release/aztibase.exe" ]; then
    BINARY="./target/release/aztibase.exe"
fi

# Generate genesis configuration + per-node configs
echo "Generating genesis..."
$BINARY genesis \
    --validators "$VALIDATOR_COUNT" \
    --funded "$FUNDED_ACCOUNTS" \
    --output "$GENESIS_DIR"
echo "Genesis written to $GENESIS_DIR"
echo ""

PIDS=()

cleanup() {
    echo ""
    echo "Shutting down all nodes..."
    for pid in "${PIDS[@]}"; do
        kill "$pid" 2>/dev/null || true
    done
    wait 2>/dev/null
    echo "All nodes stopped."
}

trap cleanup EXIT INT TERM

for i in $(seq 1 "$VALIDATOR_COUNT"); do
    NODE_CONFIG="$GENESIS_DIR/node-$i.toml"

    echo "Starting node $i (config=$NODE_CONFIG)"

    $BINARY \
        --config "$NODE_CONFIG" \
        --log-level "$LOG_LEVEL" &

    PIDS+=($!)
done

echo ""
echo "All $VALIDATOR_COUNT nodes started. Press Ctrl+C to stop."
echo ""
echo "RPC endpoints:"
for i in $(seq 1 "$VALIDATOR_COUNT"); do
    RPC_PORT=$((9943 + i))
    echo "  Node $i: http://127.0.0.1:$RPC_PORT"
done
echo ""
echo "Genesis dir: $GENESIS_DIR"
echo "Key files:   $GENESIS_DIR/keys/"
echo ""

wait
