# Validator Onboarding Guide

Welcome to the Aztibase Network testnet! This guide gets you running as a validator in under 5 minutes.

## What You Need

- A PC (Windows 10/11 x64) or Linux x64 machine
- Internet connection

## Quick Start (Windows)

1. Download `aztibase-validator-v0.1.4-windows-x64.zip` from GitHub:
   https://github.com/aztibase/aztibase-node/releases

2. Extract the zip to a folder

3. Double-click `start.bat`

4. Wait for "Validator is RUNNING"

5. Install the Aztibase Wallet Chrome extension

6. Connect wallet to `http://127.0.0.1:9944`

7. Click "Faucet" to get testnet AZTB

8. Click "Become Validator"

9. Done! You join the active validator set at the next epoch boundary.

## Quick Start (Linux)

```bash
# Download and extract
wget https://github.com/aztibase/aztibase-node/releases/download/v0.1.4/aztibase-validator-v0.1.4-linux-x64.tar.gz
tar xzf aztibase-validator-v0.1.4-linux-x64.tar.gz
cd aztibase-validator

# Generate keys and start
chmod +x aztibase
./aztibase wallet generate --validator --output keys/validator.json
./aztibase --config node.toml --genesis genesis.toml
```

Then use the wallet CLI to register:
```bash
./aztibase wallet faucet --rpc http://127.0.0.1:9944
./aztibase wallet register-validator --rpc http://127.0.0.1:9944
```

## What Happens After Starting

1. Your node connects to the Aztibase VPS boot node (102.209.21.247)
2. Syncs the full chain from genesis block 1
3. Once synced, you see blocks being produced in the logs
4. After you register as validator, you start producing blocks at the next epoch

## Check Your Node

```bash
curl -s http://127.0.0.1:9944/health
curl -s http://127.0.0.1:9944 -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"aztb_blockHeight","params":[],"id":1}'
curl -s http://127.0.0.1:9944 -X POST -H "Content-Type: application/json" -d '{"jsonrpc":"2.0","method":"aztb_nodeInfo","params":[],"id":1}'
```

## Network Info

| Item | Value |
|------|-------|
| Chain ID | 0xA27B |
| Token | AZTB |
| Boot Node | /ip4/102.209.21.247/tcp/30333 |
| RPC (VPS) | http://102.209.21.247:9944 |
| Minimum Stake | 10,000 AZTB |
| Faucet Drip | 1,000,000 AZTB |

## Staking & Rewards

- Faucet gives 1,000,000 AZTB (enough to register)
- Minimum validator stake: 10,000 AZTB
- Epoch rewards: 70% of emission goes to validators
- Rewards auto-credit each epoch (no claim step)
- Commission: 10% (configurable via governance)

## Troubleshooting

### "Connection refused" to boot node
- Check that 102.209.21.247 is reachable: `ping 102.209.21.247`
- The VPS must be running (it should be 24/7)

### No peers found
- Ensure port 30333 is not blocked by your firewall
- On Windows: allow aztibase.exe through Windows Firewall when prompted

### Node syncing slowly
- This is normal on first start. The node catches up from genesis.
- Watch block height increase in logs or via health endpoint.

### Genesis hash mismatch
- You have a different genesis.toml than the network
- Re-download the package and use the included genesis.toml

## Security

- This is a **testnet** -- tokens have no real value
- Never reuse mainnet keys on a testnet
- Back up your `keys/validator.json` file
- Never share your key file with anyone

## Stop Your Node

- Windows: close the terminal window or double-click `stop.bat`
- Linux: Ctrl+C or `kill $(pgrep aztibase)`
