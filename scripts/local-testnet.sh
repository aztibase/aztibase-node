#!/usr/bin/env bash
set -euo pipefail

VALIDATOR_COUNT=3
BASE_TCP_PORT=30333
BASE_RPC_PORT=9944
LOG_LEVEL="${LOG_LEVEL:-info}"
DATA_ROOT="${DATA_ROOT:-$(mktemp -d)}"

echo "=== Aztibase Local Testnet ==="
echo "Validators: $VALIDATOR_COUNT"
echo "Data root:  $DATA_ROOT"
echo "Log level:  $LOG_LEVEL"
echo ""

cargo build --release -p aztibase-node 2>&1 | tail -1
BINARY="./target/release/aztibase"

if [ ! -f "$BINARY" ] && [ -f "./target/release/aztibase.exe" ]; then
    BINARY="./target/release/aztibase.exe"
fi

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
    TCP_PORT=$((BASE_TCP_PORT + i - 1))
    RPC_PORT=$((BASE_RPC_PORT + i - 1))
    NODE_DIR="$DATA_ROOT/node-$i"
    mkdir -p "$NODE_DIR"

    echo "Starting node $i (validator=$i, tcp=$TCP_PORT, rpc=$RPC_PORT)"

    $BINARY \
        --data-dir "$NODE_DIR" \
        --validator-index "$i" \
        --validator-count "$VALIDATOR_COUNT" \
        --listen "/ip4/127.0.0.1/tcp/$TCP_PORT" \
        --rpc-addr "127.0.0.1:$RPC_PORT" \
        --log-level "$LOG_LEVEL" &

    PIDS+=($!)
done

echo ""
echo "All $VALIDATOR_COUNT nodes started. Press Ctrl+C to stop."
echo ""
echo "RPC endpoints:"
for i in $(seq 1 "$VALIDATOR_COUNT"); do
    RPC_PORT=$((BASE_RPC_PORT + i - 1))
    echo "  Node $i: http://127.0.0.1:$RPC_PORT"
done
echo ""

wait
