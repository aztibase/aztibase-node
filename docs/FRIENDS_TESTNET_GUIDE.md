# Friends Testnet Guide

Run a private Aztibase testnet with friends. Each person generates their own validator keys, one person (the coordinator) assembles the genesis config, and everyone joins the same network.

## Prerequisites

Every participant needs:
- Rust toolchain (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- Git (`sudo apt install git` or equivalent)
- A machine with a public IP or port forwarding on ports 30333 (P2P) and 9944 (RPC)

Build the binary:
```bash
git clone https://github.com/user/aztibase.git
cd aztibase
cargo build --release
```

## Step 1: Each Friend Generates Validator Keys

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

**Share with the coordinator:**
- `Address`
- `Public key`
- `BLS public key`

**NEVER share:** The keyfile itself contains your secret keys. Only share the 3 values printed to the console.

## Step 2: Coordinator Creates the Genesis

The coordinator collects everyone's public info and builds the genesis config.

### Initialize scaffold
```bash
./target/release/aztibase genesis init --output friends-testnet
```

### Add each validator
For each friend (using the info they shared):
```bash
./target/release/aztibase genesis add-validator \
  --genesis friends-testnet/genesis.toml \
  --name alice \
  --address 3d2ffd...288e \
  --public-key ffe4c3...ffde \
  --bls-public-key 8d0689...5b0f \
  --stake 1000000
```

If the coordinator has the keyfile locally (their own node):
```bash
./target/release/aztibase genesis add-validator \
  --genesis friends-testnet/genesis.toml \
  --name coordinator \
  --key my-validator.json \
  --stake 1000000
```

### Add a faucet account (optional)
```bash
./target/release/aztibase genesis add-account \
  --genesis friends-testnet/genesis.toml \
  --address <coordinator-address> \
  --balance 10000000
```

### Validate
```bash
./target/release/aztibase genesis validate --genesis friends-testnet/genesis.toml
```

### Review
```bash
./target/release/aztibase genesis show --genesis friends-testnet/genesis.toml
```

## Step 3: Distribute the Genesis File

Send `friends-testnet/genesis.toml` to every participant. Use any method: email, Discord, shared drive.

Everyone must have the **exact same file**. Verify by checking the genesis hash:
```bash
./target/release/aztibase genesis show --genesis genesis.toml
```
All participants should see the same `Genesis hash:` value.

## Step 4: Pick a Seed Node

One participant (usually the coordinator) runs the first node. Their IP becomes the seed node that others connect to.

The seed node's multiaddr format:
```
/ip4/<PUBLIC-IP>/tcp/30333
```

Example: `/ip4/203.0.113.42/tcp/30333`

## Step 5: Start Nodes

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

### Using the setup script (VPS)
```bash
sudo bash setup-validator.sh \
  --network custom \
  --genesis genesis.toml \
  --boot-nodes "/ip4/<SEED-IP>/tcp/30333"
```

## Step 6: Verify

Check that nodes see each other:
```bash
curl -s http://localhost:9944 -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"aztb_nodeInfo","params":[],"id":1}' | jq
```

Check block production:
```bash
curl -s http://localhost:9944 -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"aztb_blockHeight","params":[],"id":1}' | jq
```

Faucet drip (if configured):
```bash
curl -s http://localhost:9944 -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"aztb_faucetDrip","params":["0x<address>"],"id":1}' | jq
```

## Troubleshooting

### Nodes can't find each other
- Ensure port 30333 is open: `sudo ufw allow 30333/tcp`
- Check the seed node IP is correct and publicly reachable
- Verify all nodes use the same genesis.toml (compare genesis hashes)

### "Genesis hash mismatch" errors
- Someone has a different genesis.toml. Re-distribute the file and restart.

### No blocks being produced
- Need at least 2 validators for consensus (3 recommended for BFT)
- Check logs: `journalctl -u aztibase-validator -f` (if using systemd)

### Boot node unreachable
- The seed node must be running before others start
- Check firewall rules on the seed node
- If behind NAT, set up port forwarding for 30333

## Security Reminders

- **Never share** your `my-validator.json` file or mnemonic phrase
- Only share: address, public key, BLS public key
- Back up your keyfile and mnemonic in a secure location
- This is a testnet -- do not use real funds or production keys
