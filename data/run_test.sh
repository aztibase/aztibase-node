#!/bin/bash
export PATH="/c/Users/Lenovo/.cargo/bin:$PATH"
cd "c:/Users/Lenovo/Documents/Project Genises"

echo "Starting Node 1..."
./target/debug/aztibase.exe --config ./data/node1.toml --data-dir ./data/node1 --validator-index 1 --validator-count 2 > ./data/node1.log 2>&1 &

sleep 3

echo "Starting Node 2..."
./target/debug/aztibase.exe --config ./data/node2.toml --data-dir ./data/node2 --validator-index 2 --validator-count 2 > ./data/node2.log 2>&1 &

echo "Running for 15 seconds..."
sleep 15

echo "Killing nodes..."
taskkill //F //IM aztibase.exe 2>/dev/null
sleep 1

echo "DONE"
