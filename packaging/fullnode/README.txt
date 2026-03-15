AZTIBASE FULL NODE
==================

Run a full node that follows the Aztibase testnet.
Full nodes sync from genesis via block sync catch-up, then follow
live via gossipsub. No staking required.

Quick Start:
  1. Run: bash start.sh
  2. Check: curl http://127.0.0.1:9947/health

The node connects to public testnet boot nodes automatically
and syncs to the chain tip within minutes.

Files:
  aztibase.exe   - The node binary
  genesis.toml   - Chain genesis config
  node.toml      - Node config (boot nodes pre-configured)
  start.sh       - Start/stop script

Endpoints:
  RPC:     http://127.0.0.1:9947
  Health:  http://127.0.0.1:9947/health
  Metrics: http://127.0.0.1:9947/metrics/json

To become a validator:
  1. Start your node (syncs as full node first)
  2. Get AZTB from faucet or another account
  3. Send a stake transaction (minimum 50M AZTB)
  4. At the next epoch boundary, you join the active validator set

Stop: bash start.sh stop
Logs: node.log
