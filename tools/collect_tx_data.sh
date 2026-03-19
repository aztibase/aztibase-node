#!/usr/bin/env bash
# Sends transactions to the local testnet to collect tx anomaly training data.
# Usage: bash tools/collect_tx_data.sh [count]
# Default: 300 transactions (faucet drips + transfers mix)

set -e
cd "$(dirname "$0")/.."

RPC1="http://127.0.0.1:9944"
RPC2="http://127.0.0.1:9945"
RPC3="http://127.0.0.1:9946"
COUNT=${1:-300}

rpc_call() {
    local url=$1 method=$2 params=$3
    curl -s -X POST "$url" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"method\":\"$method\",\"params\":$params,\"id\":1}" \
        2>/dev/null | python3 -c "import sys,json; r=json.load(sys.stdin); print(r.get('result','err'))" 2>/dev/null || echo "err"
}

echo "=== Collecting tx anomaly training data ==="
echo "Target: $COUNT transactions across 3 nodes"
echo ""

# Faucet drips (3 per node, cheap way to get some tx data)
echo "Phase 1: Faucet drips..."
for rpc in $RPC1 $RPC2 $RPC3; do
    rpc_call "$rpc" "aztb_faucetDrip" "[]" > /dev/null
done
echo "  Sent 3 faucet drips"

# Transfers via signed wallet CLI
echo "Phase 2: Transfers via wallet..."
SENT=3

# Use node1's key to send transfers
KEY1="data/node1/validator.key"
KEY2="data/node2/validator.key"
KEY3="data/node3/validator.key"

# Get addresses
ADDR1=$(./target/release/aztibase.exe wallet show --key "$KEY1" 2>/dev/null | grep -o '0x[0-9a-f]*' | head -1)
ADDR2=$(./target/release/aztibase.exe wallet show --key "$KEY2" 2>/dev/null | grep -o '0x[0-9a-f]*' | head -1)
ADDR3=$(./target/release/aztibase.exe wallet show --key "$KEY3" 2>/dev/null | grep -o '0x[0-9a-f]*' | head -1)

if [ -z "$ADDR1" ] || [ -z "$ADDR2" ]; then
    echo "  Could not read validator keys. Falling back to RPC-only collection."
    echo "  Sending repeated faucet drips..."
    while [ $SENT -lt $COUNT ]; do
        for rpc in $RPC1 $RPC2 $RPC3; do
            rpc_call "$rpc" "aztb_faucetDrip" "[]" > /dev/null
            SENT=$((SENT + 1))
            if [ $((SENT % 50)) -eq 0 ]; then
                echo "  Progress: $SENT / $COUNT"
            fi
            if [ $SENT -ge $COUNT ]; then
                break 2
            fi
        done
        sleep 0.5
    done
else
    echo "  Validator 1: $ADDR1"
    echo "  Validator 2: $ADDR2"
    echo "  Validator 3: $ADDR3"

    # Alternate transfers between validators with varying amounts
    NONCE1=1
    NONCE2=1
    NONCE3=1

    while [ $SENT -lt $COUNT ]; do
        # v1 -> v2 (small transfer)
        AMT=$((100 + RANDOM % 5000))
        ./target/release/aztibase.exe wallet transfer \
            --key "$KEY1" --to "$ADDR2" --value $AMT --nonce $NONCE1 --gas-price 1 --rpc $RPC1 2>/dev/null && NONCE1=$((NONCE1 + 1)) && SENT=$((SENT + 1))

        # v2 -> v3 (medium transfer)
        AMT=$((500 + RANDOM % 10000))
        ./target/release/aztibase.exe wallet transfer \
            --key "$KEY2" --to "$ADDR3" --value $AMT --nonce $NONCE2 --gas-price 1 --rpc $RPC2 2>/dev/null && NONCE2=$((NONCE2 + 1)) && SENT=$((SENT + 1))

        # v3 -> v1 (large transfer)
        AMT=$((1000 + RANDOM % 50000))
        ./target/release/aztibase.exe wallet transfer \
            --key "$KEY3" --to "$ADDR1" --value $AMT --nonce $NONCE3 --gas-price 1 --rpc $RPC3 2>/dev/null && NONCE3=$((NONCE3 + 1)) && SENT=$((SENT + 1))

        if [ $((SENT % 30)) -eq 0 ]; then
            echo "  Progress: $SENT / $COUNT"
        fi

        sleep 0.2
    done
fi

echo ""
echo "=== Data collection complete: $SENT transactions sent ==="
echo ""

# Check CSV sizes
for node in data/node1 data/node2 data/node3; do
    if [ -f "$node/tx_features.csv" ]; then
        ROWS=$(wc -l < "$node/tx_features.csv")
        echo "  $node/tx_features.csv: $((ROWS - 1)) data rows"
    else
        echo "  $node/tx_features.csv: NOT FOUND"
    fi
done

echo ""
echo "Next: py -X utf8 tools/train_tx_anomaly.py --csv data/node1/tx_features.csv data/node2/tx_features.csv data/node3/tx_features.csv"
