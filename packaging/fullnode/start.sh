#!/usr/bin/env bash
# Aztibase Full Node
# Usage: bash start.sh [stop]

cd "$(dirname "$0")"

if [ "$1" = "stop" ]; then
    taskkill //F //IM aztibase.exe 2>/dev/null
    echo "Node stopped."
    exit 0
fi

echo "=== Aztibase Full Node ==="

# Kill any running instance
taskkill //F //IM aztibase.exe 2>/dev/null

# Create data directory
mkdir -p data

# Start the node
echo "Starting full node..."
./aztibase.exe --config node.toml > node.log 2>&1 &

echo "Waiting for node to boot..."
sleep 5

# Health check
HEALTH=$(curl -s http://127.0.0.1:9947/health 2>/dev/null)
if [ -n "$HEALTH" ]; then
    echo "$HEALTH"
    echo ""
    echo "=== Full Node Running ==="
    echo "  RPC:     http://127.0.0.1:9947"
    echo "  Health:  http://127.0.0.1:9947/health"
    echo "  Metrics: http://127.0.0.1:9947/metrics/json"
    echo ""
    echo "  Stop: bash start.sh stop"
else
    echo "[WARN] Node may still be starting — check node.log"
fi
