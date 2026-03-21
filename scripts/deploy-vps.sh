#!/bin/bash
# Aztibase VPS Deploy Script
# Run from WSL: bash /mnt/c/Users/Lenovo/Documents/Project\ Genises/scripts/deploy-vps.sh

set -e

VPS="root@102.209.21.247"
BINARY="$HOME/aztibase/target/release/aztibase"

echo "============================================"
echo " Aztibase VPS Deploy"
echo "============================================"
echo ""

# Check binary exists
if [ ! -f "$BINARY" ]; then
    echo "[ERROR] Binary not found at $BINARY"
    echo "Run: cd ~/aztibase && cargo build --release"
    exit 1
fi

echo "[1/6] Stopping aztibase service..."
ssh "$VPS" "systemctl stop aztibase && echo 'stopped' || echo 'was not running'"

echo ""
echo "[2/6] Uploading new binary (50 MB)..."
scp "$BINARY" "$VPS":/usr/local/bin/aztibase
ssh "$VPS" "chmod +x /usr/local/bin/aztibase"

echo ""
echo "[3/6] Wiping chain data..."
ssh "$VPS" "rm -rf /var/lib/aztibase/*"

echo ""
echo "[4/6] Updating genesis config (stake_gated registration)..."
ssh "$VPS" "grep -q 'registration_mode' /etc/aztibase/genesis.toml && \
    sed -i 's/registration_mode.*/registration_mode = \"stake_gated\"/' /etc/aztibase/genesis.toml || \
    sed -i '/^timestamp/a registration_mode = \"stake_gated\"' /etc/aztibase/genesis.toml"
ssh "$VPS" "echo '--- genesis.toml ---' && cat /etc/aztibase/genesis.toml"

echo ""
echo "[5/6] Enabling AI sentinel..."
ssh "$VPS" "sed -i 's/enabled = false/enabled = true/' /etc/aztibase/node.toml"
ssh "$VPS" "echo '--- node.toml [ai] ---' && grep -A2 '\[ai\]' /etc/aztibase/node.toml"

echo ""
echo "[6/6] Starting aztibase service..."
ssh "$VPS" "systemctl start aztibase"
sleep 3

echo ""
echo "============================================"
echo " Verifying..."
echo "============================================"
echo ""

# Health check
HEALTH=$(ssh "$VPS" "curl -s http://127.0.0.1:9944/health 2>/dev/null")
if echo "$HEALTH" | grep -q '"status":"ok"'; then
    echo "[OK] Node is running"
    echo "     $HEALTH"
else
    echo "[WARN] Health check failed. Check logs:"
    echo "       ssh $VPS journalctl -u aztibase -n 30"
fi

echo ""
echo "Done. Monitor with:"
echo "  ssh $VPS journalctl -u aztibase -f"
echo "  curl http://102.209.21.247:9944/health"
