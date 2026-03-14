AZTIBASE VALIDATOR 4 - Remote Node
====================================

This is a validator node package for the Aztibase testnet.
It connects to the seed nodes on the main PC via Tailscale.

Prerequisites:
  - Tailscale installed and connected to the same tailnet
  - Main PC testnet running (bash start-testnet-public.sh)

Quick Start:
  1. Run: bash start.sh
  2. Check: curl http://127.0.0.1:9944/health

Files:
  aztibase.exe   - The node binary
  genesis.toml   - Chain genesis config (must match all nodes)
  node.toml      - Node config (points to main PC via Tailscale)
  keys/          - Validator keypair (DO NOT share the secret key)
  start.sh       - Start/stop script

Stop: bash start.sh stop
Logs: node.log
