# Aztibase Network — Node

Run a full node or validator on the Aztibase Network testnet.

## Quick Start (Windows x64)

1. Download the latest release from [Releases](https://github.com/Gerrit740/aztibase-node/releases)
2. Extract the archive
3. Install [Tailscale](https://tailscale.com) and join the network
4. Edit `node.toml` — replace `SEED_NODE_IP` with the seed node's Tailscale IP
5. Run: `bash start.sh`
6. Verify: `curl http://127.0.0.1:9944/health`

## Files

| File | Description |
|------|-------------|
| `aztibase.exe` | Node binary |
| `genesis.toml` | Chain genesis config (shared by all nodes) |
| `node.toml` | Your node config (edit boot_nodes IP here) |
| `start.sh` | Start/stop script |

## Become a Validator

1. Start your node (syncs as full node first)
2. Get AZTB from the faucet or another account
3. Send a stake transaction (minimum 50M AZTB)
4. At the next epoch boundary you become a validator and earn rewards

## Commands

```bash
bash start.sh        # Start
bash start.sh stop   # Stop
curl http://127.0.0.1:9944/health  # Health check
cat node.log         # Logs
```

## Links

- Website: [aztibase.com](https://aztibase.com)
- Explorer: [explorer.aztibase.com](https://explorer.aztibase.com)
- RPC: `https://rpc.aztibase.com`

## License

MIT / Apache-2.0
