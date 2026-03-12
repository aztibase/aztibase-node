#!/usr/bin/env bash
set -euo pipefail

# Aztibase Friends Testnet — Genesis Ceremony Coordinator Script
# Walks you through creating a shared genesis config for a private testnet.

AZTIBASE_BIN="${AZTIBASE_BIN:-./target/release/aztibase}"
OUTPUT_DIR="${1:-friends-testnet}"
DEFAULT_STAKE="1000000"

GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
BOLD='\033[1m'
NC='\033[0m'

if [ ! -f "$AZTIBASE_BIN" ]; then
    echo "Binary not found at $AZTIBASE_BIN"
    echo "Build first: cargo build --release"
    echo "Or set AZTIBASE_BIN=/path/to/aztibase"
    exit 1
fi

echo -e "${CYAN}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}  ${BOLD}Aztibase Friends Testnet — Genesis Ceremony${NC}             ${CYAN}║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""

echo -e "${BOLD}Step 1: Initialize genesis scaffold${NC}"
"$AZTIBASE_BIN" genesis init --output "$OUTPUT_DIR"
echo ""

GENESIS_FILE="$OUTPUT_DIR/genesis.toml"

echo -e "${BOLD}Step 2: Add validators${NC}"
echo "For each friend, enter their name and public info."
echo "They get this by running: aztibase wallet generate --validator"
echo ""

while true; do
    echo -e "${YELLOW}Add a validator? (y/n)${NC}"
    read -r ADD_VAL
    if [ "$ADD_VAL" != "y" ] && [ "$ADD_VAL" != "Y" ]; then
        break
    fi

    echo -n "  Name: "
    read -r VAL_NAME

    echo -n "  Has local keyfile? (path or 'n'): "
    read -r KF_PATH

    if [ "$KF_PATH" != "n" ] && [ "$KF_PATH" != "N" ] && [ -f "$KF_PATH" ]; then
        echo -n "  Stake [$DEFAULT_STAKE]: "
        read -r STAKE
        STAKE="${STAKE:-$DEFAULT_STAKE}"

        "$AZTIBASE_BIN" genesis add-validator \
            --genesis "$GENESIS_FILE" \
            --name "$VAL_NAME" \
            --key "$KF_PATH" \
            --stake "$STAKE"
    else
        echo -n "  Address (hex): "
        read -r ADDR
        echo -n "  Public key (hex): "
        read -r PK
        echo -n "  BLS public key (hex): "
        read -r BLS
        echo -n "  Stake [$DEFAULT_STAKE]: "
        read -r STAKE
        STAKE="${STAKE:-$DEFAULT_STAKE}"

        "$AZTIBASE_BIN" genesis add-validator \
            --genesis "$GENESIS_FILE" \
            --name "$VAL_NAME" \
            --address "$ADDR" \
            --public-key "$PK" \
            --bls-public-key "$BLS" \
            --stake "$STAKE"
    fi
    echo ""
done

echo ""
echo -e "${BOLD}Step 3: Add funded accounts (optional)${NC}"
while true; do
    echo -e "${YELLOW}Add a funded account? (y/n)${NC}"
    read -r ADD_ACCT
    if [ "$ADD_ACCT" != "y" ] && [ "$ADD_ACCT" != "Y" ]; then
        break
    fi

    echo -n "  Address (hex): "
    read -r ACCT_ADDR
    echo -n "  Balance [10000000]: "
    read -r BALANCE
    BALANCE="${BALANCE:-10000000}"

    "$AZTIBASE_BIN" genesis add-account \
        --genesis "$GENESIS_FILE" \
        --address "$ACCT_ADDR" \
        --balance "$BALANCE"
    echo ""
done

echo ""
echo -e "${BOLD}Step 4: Validate & show${NC}"
"$AZTIBASE_BIN" genesis validate --genesis "$GENESIS_FILE"
echo ""
"$AZTIBASE_BIN" genesis show --genesis "$GENESIS_FILE"

echo ""
echo -e "${GREEN}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║  Genesis ceremony complete!                              ║${NC}"
echo -e "${GREEN}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""
echo "Distribute this file to all participants:"
echo "  $GENESIS_FILE"
echo ""
echo "Each participant starts their node with:"
echo "  aztibase --genesis genesis.toml --validator-key my-validator.json \\"
echo "    --boot-node /ip4/<SEED-IP>/tcp/30333 \\"
echo "    --rpc-addr 0.0.0.0:9944 --listen /ip4/0.0.0.0/tcp/30333 --metrics"
echo ""
echo "Or use the setup script on a VPS:"
echo "  bash setup-validator.sh --network custom --genesis genesis.toml \\"
echo "    --boot-nodes /ip4/<SEED-IP>/tcp/30333"
