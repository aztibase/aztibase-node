#!/usr/bin/env bash
set -euo pipefail

DATA_DIR="${DATA_DIR:-./data}"

echo "=== Aztibase Testnet Reset ==="

if [ ! -d "$DATA_DIR/genesis" ]; then
    echo "No testnet data found at $DATA_DIR. Nothing to reset."
    exit 0
fi

echo "Wiping node databases (preserving keys + genesis)..."

for node_dir in "$DATA_DIR"/node*/; do
    if [ -d "$node_dir/db" ]; then
        rm -rf "$node_dir/db"
        echo "  Removed $node_dir/db"
    fi
    if [ -d "$node_dir/execution_db" ]; then
        rm -rf "$node_dir/execution_db"
        echo "  Removed $node_dir/execution_db"
    fi
done

echo ""
echo "Reset complete. Genesis and keys preserved."
echo "Run 'docker compose up --build' to restart with fresh state."
