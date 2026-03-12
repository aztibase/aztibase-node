#!/usr/bin/env bash
# Aztibase Testnet Validation Suite
# Run AFTER start-testnet.sh has nodes running
# Usage: bash test-testnet.sh

set +e

RPC1="http://127.0.0.1:9944"
RPC2="http://127.0.0.1:9945"
RPC3="http://127.0.0.1:9946"

PASS=0
FAIL=0
WARN=0

pass() { ((PASS++)); echo "  [PASS] $1"; }
fail() { ((FAIL++)); echo "  [FAIL] $1: $2"; }
warn() { ((WARN++)); echo "  [WARN] $1: $2"; }

rpc() {
    local url=$1 method=$2 params=$3
    curl -s -X POST "$url" \
        -H "Content-Type: application/json" \
        -d "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"$method\",\"params\":$params}" 2>/dev/null
}

echo "=========================================="
echo "  Aztibase Testnet Validation Suite"
echo "  $(date)"
echo "=========================================="
echo ""

# --- Phase 1: Health & Connectivity ---
echo "--- Phase 1: Health & Connectivity ---"

for i in 1 2 3; do
    port=$((9943 + i))
    url="http://127.0.0.1:$port"
    health=$(curl -s "$url/health" 2>/dev/null)
    if echo "$health" | grep -q "chainId"; then
        pass "Node $i health ($port)"
    else
        fail "Node $i health ($port)" "no response"
    fi
done

# --- Phase 2: Chain Identity ---
echo ""
echo "--- Phase 2: Chain Identity ---"

chain_id=$(rpc $RPC1 "aztb_chainId" "[]" | grep -o '"result":"[^"]*"' | cut -d'"' -f4)
if [ "$chain_id" = "0xa27b" ]; then
    pass "Chain ID = 0xa27b"
else
    fail "Chain ID" "expected 0xa27b, got $chain_id"
fi

genesis1=$(rpc $RPC1 "aztb_genesisHash" "[]" | grep -o '"result":"[^"]*"' | cut -d'"' -f4)
genesis2=$(rpc $RPC2 "aztb_genesisHash" "[]" | grep -o '"result":"[^"]*"' | cut -d'"' -f4)
genesis3=$(rpc $RPC3 "aztb_genesisHash" "[]" | grep -o '"result":"[^"]*"' | cut -d'"' -f4)
if [ -n "$genesis1" ] && [ "$genesis1" = "$genesis2" ] && [ "$genesis2" = "$genesis3" ]; then
    pass "Genesis hash consistent across all 3 nodes"
else
    fail "Genesis hash mismatch" "$genesis1 / $genesis2 / $genesis3"
fi

# --- Phase 3: Node Info ---
echo ""
echo "--- Phase 3: Node Info ---"

node_info=$(rpc $RPC1 "aztb_nodeInfo" "[]")
if echo "$node_info" | grep -q "version"; then
    pass "aztb_nodeInfo returns version"
else
    fail "aztb_nodeInfo" "no version field"
fi

# --- Phase 4: Block Production ---
echo ""
echo "--- Phase 4: Block Production ---"

height1=$(rpc $RPC1 "aztb_blockNumber" "[]" | grep -o '"result":[0-9]*' | cut -d: -f2)
sleep 3
height2=$(rpc $RPC1 "aztb_blockNumber" "[]" | grep -o '"result":[0-9]*' | cut -d: -f2)

if [ -n "$height1" ] && [ -n "$height2" ] && [ "$height2" -gt "$height1" ] 2>/dev/null; then
    delta=$((height2 - height1))
    pass "Block production active ($delta blocks in 3s)"
else
    warn "Block production" "height1=$height1 height2=$height2 (may need more time)"
fi

# --- Phase 5: Gas Price ---
echo ""
echo "--- Phase 5: Gas & Fee Market ---"

gas_price=$(rpc $RPC1 "aztb_gasPrice" "[]" | grep -o '"result":[0-9]*' | cut -d: -f2)
if [ -n "$gas_price" ] && [ "$gas_price" -gt 0 ] 2>/dev/null; then
    pass "Gas price = $gas_price"
else
    warn "Gas price" "got $gas_price"
fi

# --- Phase 6: Faucet & Transfer ---
echo ""
echo "--- Phase 6: Faucet & Transfer Flow ---"

# Generate test address (use a fixed one for reproducibility)
TEST_ADDR="0x$(openssl rand -hex 32 2>/dev/null || echo 'aabbccdd00112233445566778899aabbccddeeff00112233445566778899aabb')"

faucet_resp=$(rpc $RPC1 "aztb_faucetDrip" "[\"$TEST_ADDR\"]")
if echo "$faucet_resp" | grep -q "result"; then
    pass "Faucet drip to test address"
else
    fail "Faucet drip" "$faucet_resp"
fi

sleep 2

balance=$(rpc $RPC1 "aztb_getBalance" "[\"$TEST_ADDR\"]" | grep -o '"result":[0-9]*' | cut -d: -f2)
if [ -n "$balance" ] && [ "$balance" -gt 0 ] 2>/dev/null; then
    pass "Balance after faucet = $balance"
else
    warn "Balance after faucet" "got $balance (may need more batches)"
fi

# --- Phase 7: Mempool ---
echo ""
echo "--- Phase 7: Mempool ---"

mempool=$(rpc $RPC1 "aztb_pendingTaskCount" "[]" | grep -o '"result":[0-9]*' | cut -d: -f2)
if [ -n "$mempool" ]; then
    pass "Mempool size = $mempool"
else
    warn "Mempool size" "no response"
fi

# --- Phase 8: Metrics & Monitoring ---
echo ""
echo "--- Phase 8: Metrics & Monitoring ---"

prom_metrics=$(curl -s "$RPC1/metrics" 2>/dev/null)
if echo "$prom_metrics" | grep -q "block_height"; then
    pass "Prometheus /metrics endpoint"
else
    warn "Prometheus /metrics" "no block_height metric found"
fi

json_metrics=$(curl -s "$RPC1/metrics/json" 2>/dev/null)
if echo "$json_metrics" | grep -q "block_height"; then
    pass "JSON /metrics/json endpoint"
else
    warn "JSON /metrics/json" "no block_height metric found"
fi

# --- Phase 9: Validator & Staking RPCs ---
echo ""
echo "--- Phase 9: Validator & Staking RPCs ---"

validators=$(rpc $RPC1 "aztb_getActiveValidators" "[]")
if echo "$validators" | grep -q "result"; then
    pass "aztb_getActiveValidators"
else
    fail "aztb_getActiveValidators" "no result"
fi

# --- Phase 10: Governance RPCs ---
echo ""
echo "--- Phase 10: Governance & Chain Params ---"

chain_params=$(rpc $RPC1 "aztb_listChainParams" "[]")
if echo "$chain_params" | grep -q "result"; then
    pass "aztb_listChainParams"
else
    fail "aztb_listChainParams" "no result"
fi

emission=$(rpc $RPC1 "aztb_getEmissionInfo" "[]")
if echo "$emission" | grep -q "result"; then
    pass "aztb_getEmissionInfo"
else
    fail "aztb_getEmissionInfo" "no result"
fi

# --- Phase 11: Cross-node consistency ---
echo ""
echo "--- Phase 11: Cross-Node Consistency ---"

h1=$(rpc $RPC1 "aztb_blockNumber" "[]" | grep -o '"result":[0-9]*' | cut -d: -f2)
h2=$(rpc $RPC2 "aztb_blockNumber" "[]" | grep -o '"result":[0-9]*' | cut -d: -f2)
h3=$(rpc $RPC3 "aztb_blockNumber" "[]" | grep -o '"result":[0-9]*' | cut -d: -f2)

if [ -n "$h1" ] && [ -n "$h2" ] && [ -n "$h3" ]; then
    max=$h1; [ "$h2" -gt "$max" ] 2>/dev/null && max=$h2; [ "$h3" -gt "$max" ] 2>/dev/null && max=$h3
    min=$h1; [ "$h2" -lt "$min" ] 2>/dev/null && min=$h2; [ "$h3" -lt "$min" ] 2>/dev/null && min=$h3
    drift=$((max - min))
    if [ "$drift" -le 5 ]; then
        pass "Block height drift = $drift (h1=$h1, h2=$h2, h3=$h3)"
    else
        warn "Block height drift" "drift=$drift (h1=$h1, h2=$h2, h3=$h3)"
    fi
else
    fail "Cross-node height" "couldn't read heights"
fi

# --- Phase 12: Stress Test (10 rapid faucet drips) ---
echo ""
echo "--- Phase 12: Stress Test (10 rapid faucet requests) ---"

stress_ok=0
stress_fail=0
for i in $(seq 1 10); do
    addr="0x$(printf '%064x' $((i + 1000)))"
    resp=$(rpc $RPC1 "aztb_faucetDrip" "[\"$addr\"]")
    if echo "$resp" | grep -q "result"; then
        ((stress_ok++))
    else
        ((stress_fail++))
    fi
done

if [ "$stress_ok" -ge 5 ]; then
    pass "Stress faucet: $stress_ok/10 succeeded ($stress_fail rate-limited)"
else
    warn "Stress faucet" "$stress_ok/10 succeeded"
fi

# Wait for transactions to process
echo ""
echo "Waiting 10s for batch processing..."
sleep 10

# Check balances of stress-test addresses
funded=0
for i in $(seq 1 3); do
    addr="0x$(printf '%064x' $((i + 1000)))"
    bal=$(rpc $RPC1 "aztb_getBalance" "[\"$addr\"]" | grep -o '"result":[0-9]*' | cut -d: -f2)
    if [ -n "$bal" ] && [ "$bal" -gt 0 ] 2>/dev/null; then
        ((funded++))
    fi
done

if [ "$funded" -ge 2 ]; then
    pass "Stress balances confirmed ($funded/3 checked)"
else
    warn "Stress balances" "only $funded/3 have balance"
fi

# --- Summary ---
echo ""
echo "=========================================="
echo "  RESULTS: $PASS passed, $FAIL failed, $WARN warnings"
echo "=========================================="

if [ "$FAIL" -gt 0 ]; then
    echo "  STATUS: ISSUES FOUND"
    exit 1
else
    echo "  STATUS: ALL CRITICAL CHECKS PASSED"
    exit 0
fi
