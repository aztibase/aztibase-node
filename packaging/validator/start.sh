#!/usr/bin/env bash
# Aztibase Validator Node
# Usage: bash start.sh [stop]

cd "$(dirname "$0")"

if [ "$1" = "stop" ]; then
    taskkill //F //IM aztibase.exe 2>/dev/null
    taskkill //F //IM node.exe 2>/dev/null
    echo "Node and dashboard stopped."
    exit 0
fi

echo "=== Aztibase Validator ==="

# Check for keys
if [ ! -f "keys/validator.json" ]; then
    echo ""
    echo "No validator key found. Generating one now..."
    mkdir -p keys
    ./aztibase.exe wallet generate --validator --output keys/validator.json
    echo ""
    echo "Key saved to keys/validator.json"
    echo "IMPORTANT: Back up this file. If you lose it, you lose your validator identity."
    echo ""
fi

# Kill any running instance
taskkill //F //IM aztibase.exe 2>/dev/null

# Create data directory
mkdir -p data

# Start the node (--testnet uses built-in genesis config and boot nodes)
echo "Starting validator..."
./aztibase.exe --config node.toml --testnet > node.log 2>&1 &

# Start dashboard on port 8080
if [ -d "explorer" ]; then
    echo "Starting dashboard..."
    cd explorer && npx -y serve -l 8080 -s . > /dev/null 2>&1 &
    cd ..
fi

echo "Waiting for node to boot..."
sleep 5

# Health check
HEALTH=$(curl -s http://127.0.0.1:9944/health 2>/dev/null)
if [ -n "$HEALTH" ]; then
    echo "$HEALTH"
    echo ""
    echo "=== Validator Running ==="
    echo "  RPC:        http://127.0.0.1:9944"
    echo "  Dashboard:  http://127.0.0.1:8080"
    echo "  Health:     http://127.0.0.1:9944/health"
    echo "  Metrics:    http://127.0.0.1:9944/metrics/json"
    echo ""
    echo "  Stop: bash start.sh stop"
else
    echo "[WARN] Node may still be starting — check node.log"
fi
