#!/usr/bin/env bash
# Aztibase Local Testnet Launcher
# Run: bash start-testnet.sh
# Stop: bash start-testnet.sh stop

cd "$(dirname "$0")"

if [ "$1" = "stop" ]; then
    taskkill //F //IM aztibase.exe 2>/dev/null
    taskkill //F //IM node.exe 2>/dev/null
    echo "Testnet stopped."
    exit 0
fi

echo "=== Aztibase Local Testnet ==="

# Kill any running instances
taskkill //F //IM aztibase.exe 2>/dev/null

# Reset old data
for node in data/node1 data/node2 data/node3; do
    rm -f "$node/db" "$node/execution_db" "$node/peer_store.redb" "$node/peer_reputation.redb" "$node/batch_archive.redb" "$node/"*.log
done
echo "[OK] Old data cleared"

# Start 3 validators (staggered to avoid boot_node race conditions)
./target/release/aztibase.exe --config data/node1/node1-local.toml --sentinel-export > data/node1/node1.log 2>&1 &
sleep 2
./target/release/aztibase.exe --config data/node2/node2-local.toml --sentinel-export > data/node2/node2.log 2>&1 &
sleep 2
./target/release/aztibase.exe --config data/node3/node3-local.toml --sentinel-export > data/node3/node3.log 2>&1 &
echo "[OK] 3 validator nodes started"

# Start explorer web server
cd explorer && npx -y serve -l 8080 -s . > /dev/null 2>&1 &
cd ..
echo "[OK] Explorer server started"

# Wait for nodes to boot
echo "Waiting for nodes..."
sleep 5

# Health check
curl -s http://127.0.0.1:9944/health
echo ""

echo ""
echo "=== Testnet Running ==="
echo "  Node 1 RPC:  http://127.0.0.1:9944"
echo "  Node 2 RPC:  http://127.0.0.1:9945"
echo "  Node 3 RPC:  http://127.0.0.1:9946"
echo "  Explorer:    http://127.0.0.1:8080"
echo "  Health:      http://127.0.0.1:9944/health"
echo "  Metrics:     http://127.0.0.1:9944/metrics"
echo ""
echo "  Stop: bash start-testnet.sh stop"
