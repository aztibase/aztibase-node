AZTIBASE NODE - Quick Start
============================

1. Install Tailscale (tailscale.com) and join the network
2. Open node.toml and replace SEED_NODE_IP with the seed node's Tailscale IP
3. Run: bash start.sh
4. Check: curl http://127.0.0.1:9944/health

Files:
  aztibase.exe   - The node binary
  genesis.toml   - Chain genesis config (same for all nodes)
  node.toml      - Your node config (edit boot_nodes IP here)
  start.sh       - Start/stop script

To become a validator:
  1. Start your node (syncs as full node first)
  2. Get AZTB from faucet or another account
  3. Send a stake transaction (minimum 50M AZTB)
  4. At the next epoch boundary, you become a validator and earn rewards

Stop: bash start.sh stop
Logs: node.log
