#!/usr/bin/env bash
# Single-node local test — mirrors VPS configuration.
# One validator, no boot nodes, no AI, no metrics overhead.
# Usage: bash test-singlenode.sh [stop]

set -euo pipefail

BINARY="./target/release/aztibase"
CONFIG="./data/singlenode/node.toml"
DATA_DIR="./data/singlenode"
LOG_FILE="$DATA_DIR/singlenode.log"
PID_FILE="$DATA_DIR/singlenode.pid"

if [[ "${1:-}" == "stop" ]]; then
    if [[ -f "$PID_FILE" ]]; then
        PID=$(cat "$PID_FILE")
        kill "$PID" 2>/dev/null && echo "Stopped (PID $PID)" || echo "Already stopped"
        rm -f "$PID_FILE"
    else
        echo "No PID file — node not running"
    fi
    exit 0
fi

# Wipe previous state so chain starts fresh
rm -rf "$DATA_DIR/db" "$DATA_DIR/execution_db" \
       "$DATA_DIR/peer_store.redb" "$DATA_DIR/peer_reputation.redb"

echo "Starting single-node (mirrors VPS config)..."
echo "Log: $LOG_FILE"
echo "RPC: http://127.0.0.1:9950"

nohup "$BINARY" --config "$CONFIG" > "$LOG_FILE" 2>&1 &
echo $! > "$PID_FILE"
echo "PID $(cat $PID_FILE) — watching blocks..."

# Watch for block production
sleep 3
tail -f "$LOG_FILE" | grep --line-buffered -E 'Proposed vertex|Block committed|block_height|WARN|ERROR|stall|panic'
