# Validator Onboarding Guide

Welcome to the Aztibase Friends Testnet! This guide gets you connected in under 10 minutes.

## What You Need

- A PC (Windows, Mac, or Linux)
- Internet connection

## Option A: Connect as a Light Client (Easiest)

No software to install. Use the public RPC endpoints directly.

### Check the network

```bash
curl -s https://rpc.aztibase.com -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"aztb_blockHeight","params":[],"id":1}'
```

### Get testnet tokens

```bash
curl -s https://rpc.aztibase.com -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"aztb_faucetDrip","params":["0xYOUR_ADDRESS"],"id":1}'
```

### Check balance

```bash
curl -s https://rpc.aztibase.com -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"aztb_getBalance","params":["0xYOUR_ADDRESS"],"id":1}'
```

## Option B: Run a Validator Node (Pre-built Binary)

### Step 1: Install Tailscale

Tailscale creates a secure private network between you and the testnet.

1. Go to https://tailscale.com/download
2. Install for your OS
3. Sign in with the same account as the coordinator
4. Once connected, you'll get an IP like `100.x.x.x`
5. Verify: `ping 100.104.71.94` (coordinator's node)

### Step 2: Download the Node Package

Download from GitHub releases:

https://github.com/aztibase/aztibase-node/releases

Choose your package:
- **aztibase-testnet-v0.1.0-windows-x64.tar.gz** — Generic full node (configure your own keys)
- **aztibase-validator4-v0.1.0-windows-x64.tar.gz** — Pre-configured validator-4 (ready to run)

Extract the archive. You need Git Bash or WSL to run the start script.

### Step 3: Configure (if using generic package)

Edit `node.toml` — replace `SEED_NODE_IP` with the coordinator's Tailscale IP:
```toml
boot_nodes = [
    "/ip4/100.104.71.94/tcp/30333",
]
```

If you have a validator key, place it in the same folder and add to `node.toml`:
```toml
validator_key = "./keys/your-validator.json"
```

### Step 4: Start Your Node

```bash
bash start.sh
```

### Step 6: Verify Connection

Check that your node sees peers:
```bash
curl -s http://localhost:9944 -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"aztb_nodeInfo","params":[],"id":1}'
```

Check block production:
```bash
curl -s http://localhost:9944 -X POST \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"aztb_blockHeight","params":[],"id":1}'
```

## Option C: Build from Source (Advanced)

If you have access to the private source repo:

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone <private-repo-url>
cd aztibase
cargo build --release

# Generate validator keys
cargo run --package aztibase-core --example keygen > my-validator.json
```

Send the coordinator your **address**, **public_key**, and **bls_public_key** (never the secret keys).

## Network Info

| Endpoint | URL |
|----------|-----|
| RPC (Node 1) | https://rpc.aztibase.com |
| RPC (Node 2) | https://rpc2.aztibase.com |
| RPC (Node 3) | https://rpc3.aztibase.com |
| Chain ID | 0xA27B |
| Token | AZTB |

## Troubleshooting

### "Connection refused" to boot node
- Make sure Tailscale is running (green icon in system tray)
- Ping the coordinator: `ping 100.104.71.94`
- The coordinator's testnet must be running

### No blocks produced
- Consensus needs all genesis validators online (currently 4)
- Check your logs for errors

### Genesis hash mismatch
- You have a different genesis.toml than the coordinator
- Re-download the file and restart

### Node won't compile
- Ensure Rust is up to date: `rustup update`
- On Windows: install Visual Studio Build Tools with C++ workload

## Security

- This is a **testnet** -- tokens have no real value
- Never reuse mainnet keys on a testnet
- Never share your `my-validator.json` file
- Back up your keys offline
