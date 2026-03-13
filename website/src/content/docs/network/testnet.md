---
title: Testnet Guide
description: Join the Aztibase friends testnet
---

Run a private Aztibase testnet with friends. Each participant generates validator keys, one coordinator assembles the genesis config, and everyone joins the same network.

## Prerequisites

Every participant needs:
- **Rust toolchain**: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Git**: `sudo apt install git` or equivalent
- A machine with a public IP or port forwarding on ports **30333** (P2P) and **9944** (RPC)

Build the binary:
```bash
git clone https://github.com/aztibase/aztibase.git
cd aztibase
cargo build --release
```

## Step 1: Generate Validator Keys

Every participant runs:
```bash
./target/release/aztibase wallet generate --validator --output my-validator.json
```

Output:
```
Address: 3d2ffd...288e
Public key: ffe4c3...ffde
BLS public key: 8d0689...5b0f
Key file: my-validator.json
```

**Share with the coordinator:** Address, Public key, BLS public key

**NEVER share:** The keyfile itself — it contains your secret keys.

## Step 2: Coordinator Creates the Genesis

### Initialize scaffold
```bash
./target/release/aztibase genesis init --output friends-testnet
```

### Add each validator
```bash
./target/release/aztibase genesis add-validator \
  --genesis friends-testnet/genesis.toml \
  --name alice \
  --address 3d2ffd...288e \
  --public-key ffe4c3...ffde \
  --bls-public-key 8d0689...5b0f \
  --stake 1000000
```

### Add a faucet account (optional)
```bash
./target/release/aztibase genesis add-account \
  --genesis friends-testnet/genesis.toml \
  --address <coordinator-address> \
  --balance 10000000
```

### Validate and review
```bash
./target/release/aztibase genesis validate --genesis friends-testnet/genesis.toml
./target/release/aztibase genesis show --genesis friends-testnet/genesis.toml
```

## Step 3: Distribute the Genesis File

Send `friends-testnet/genesis.toml` to every participant. Everyone must have the **exact same file**.

Verify by comparing genesis hashes:
```bash
./target/release/aztibase genesis show --genesis genesis.toml
```

All participants should see the same `Genesis hash:` value.

## Step 4: Start Nodes

### Seed node (coordinator)
```bash
./target/release/aztibase \
  --genesis genesis.toml \
  --validator-key my-validator.json \
  --rpc-addr 0.0.0.0:9944 \
  --listen /ip4/0.0.0.0/tcp/30333 \
  --listen /ip4/0.0.0.0/udp/30333/quic-v1 \
  --metrics
```

### Other participants
```bash
./target/release/aztibase \
  --genesis genesis.toml \
  --validator-key my-validator.json \
  --rpc-addr 0.0.0.0:9944 \
  --listen /ip4/0.0.0.0/tcp/30333 \
  --listen /ip4/0.0.0.0/udp/30333/quic-v1 \
  --boot-node /ip4/<SEED-IP>/tcp/30333 \
  --metrics
```

## Step 5: Verify

```bash
# Check peers
curl -s http://localhost:9944 -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"aztb_nodeInfo","params":[],"id":1}' | jq

# Check blocks
curl -s http://localhost:9944 -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"aztb_blockHeight","params":[],"id":1}' | jq

# Faucet drip
curl -s http://localhost:9944 -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"aztb_faucetDrip","params":["0x<address>"],"id":1}' | jq
```

## Troubleshooting

**Nodes can't find each other:**
- Ensure port 30333 is open: `sudo ufw allow 30333/tcp`
- Check the seed node IP is publicly reachable
- Verify all nodes use the same genesis.toml

**"Genesis hash mismatch" errors:**
- Someone has a different genesis.toml. Re-distribute and restart.

**No blocks being produced:**
- Need at least 2 validators (3 recommended for BFT)
- Check logs: `journalctl -u aztibase-validator -f`

**Boot node unreachable:**
- Seed node must be running before others start
- Check firewall rules and NAT port forwarding for 30333
