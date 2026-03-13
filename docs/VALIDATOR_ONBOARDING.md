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

## Option B: Run a Validator Node (Full Participation)

### Step 1: Install Tailscale

Tailscale creates a secure private network between you and the testnet.

1. Go to https://tailscale.com/download
2. Install for your OS
3. Sign in with Google/Microsoft/GitHub
4. Ask the coordinator for the **Tailscale network invite link**
5. Join the network -- you'll get an IP like `100.x.x.x`

### Step 2: Install Rust and Build

**Linux/Mac:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

**Windows:**
Download and run https://win.rustup.rs

Then clone and build:
```bash
git clone https://github.com/user/aztibase.git
cd aztibase
cargo build --release
```

### Step 3: Generate Validator Keys

```bash
./target/release/aztibase wallet generate --validator --output my-validator.json
```

Send the coordinator these 3 values (printed to console):
- Address
- Public key
- BLS public key

**NEVER share the keyfile itself.**

Back up `my-validator.json` somewhere safe.

### Step 4: Receive Genesis File

The coordinator will send you `genesis.toml` after adding your validator. Save it in your project directory.

Verify the genesis hash matches:
```bash
./target/release/aztibase genesis show --genesis genesis.toml
```

Everyone should see the same hash.

### Step 5: Start Your Node

Get the coordinator's Tailscale IP (e.g., `100.104.71.94`).

```bash
./target/release/aztibase \
  --genesis genesis.toml \
  --validator-key my-validator.json \
  --rpc-addr 0.0.0.0:9944 \
  --listen /ip4/0.0.0.0/tcp/30333 \
  --listen /ip4/0.0.0.0/udp/30333/quic-v1 \
  --boot-node /ip4/100.104.71.94/tcp/30333 \
  --metrics
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
- Consensus needs at least 2 validators online
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
