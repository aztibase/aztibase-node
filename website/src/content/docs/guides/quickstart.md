---
title: Quick Start
description: Build Aztibase from source and run a local node
---

Get a local Aztibase node running in under 5 minutes.

## Prerequisites

- **Rust** 1.75+ (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- **Git** (`sudo apt install git` or equivalent)
- 4 GB RAM, 10 GB free disk space

## Build from Source

```bash
git clone https://github.com/aztibase/aztibase.git
cd aztibase
cargo build --release
```

The binary will be at `./target/release/aztibase`.

## Run a Single Node

```bash
./target/release/aztibase \
  --rpc-addr 127.0.0.1:9944
```

This starts a node with default genesis configuration. The node will:
- Listen for P2P connections on port 30333
- Serve JSON-RPC on port 9944
- Store data in `./data/`

## Verify It Works

```bash
# Check node health
curl -s http://localhost:9944/health | jq .

# Get block height
curl -s -X POST http://localhost:9944 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"aztb_blockNumber","params":[],"id":1}' | jq .

# Get node info
curl -s -X POST http://localhost:9944 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"aztb_nodeInfo","params":[],"id":1}' | jq .
```

## Run a Local Testnet (3 Nodes)

For a multi-node setup with consensus:

```bash
bash start-testnet.sh
```

This starts 3 validator nodes with staggered startup:
- **Node 1**: P2P 30333, RPC 9944
- **Node 2**: P2P 30334, RPC 9945
- **Node 3**: P2P 30335, RPC 9946

Stop the testnet:
```bash
bash start-testnet.sh stop
```

## Generate a Wallet

```bash
# Standard wallet
./target/release/aztibase wallet generate

# Validator wallet (includes BLS keys)
./target/release/aztibase wallet generate --validator --output my-keys.json
```

## Get Testnet Tokens

Request tokens from the faucet:

```bash
curl -s -X POST http://localhost:9944 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"aztb_faucetDrip","params":["0xYOUR_ADDRESS"],"id":1}' | jq .
```

Each drip sends 1,000,000 AZTB with a 60-second cooldown.

## Next Steps

- [Run a Validator](/guides/run-validator/) — Join the public testnet
- [API Reference](/api/rpc/) — Full JSON-RPC documentation
- [Testnet Guide](/network/testnet/) — Join the friends testnet
