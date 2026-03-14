#!/usr/bin/env bash
# Aztibase Full Node / Validator
# Usage: bash start.sh [stop]

cd "$(dirname "$0")"

if [ "$1" = "stop" ]; then
    taskkill //F //IM aztibase.exe 2>/dev/null
    echo "Node stopped."
    exit 0
fi

echo "=== Aztibase Node ==="

# Kill any running instance
taskkill //F //IM aztibase.exe 2>/dev/null

# Check boot node IP is configured
if grep -q "SEED_NODE_IP" node.toml; then
    echo ""
    echo "ERROR: You need to set the seed node IP in node.toml"
    echo "  Open node.toml and replace SEED_NODE_IP with the Tailscale IP"
    echo "  of an existing node (e.g. 100.104.71.94)"
    echo ""
    exit 1
fi

# Create data directory
mkdir -p data

# Start the node
echo "Starting node..."
./aztibase.exe --config node.toml > node.log 2>&1 &

echo "Waiting for node to boot..."
sleep 5

# Health check
HEALTH=$(curl -s http://127.0.0.1:9944/health 2>/dev/null)
if [ -n "$HEALTH" ]; then
    echo "$HEALTH"
    echo ""
    echo "=== Node Running ==="
    echo "  RPC:     http://127.0.0.1:9944"
    echo "  Health:  http://127.0.0.1:9944/health"
    echo "  Metrics: http://127.0.0.1:9944/metrics/json"
    echo ""
    echo "  Stop: bash start.sh stop"
else
    echo "[WARN] Node may still be starting — check node.log"
fi
