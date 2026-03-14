AZTIBASE VALIDATOR NODE
======================

Run a validator node on the Aztibase testnet.

Prerequisites:
  - Windows x64
  - Node.js installed (for the dashboard server)
  - Network access to at least one existing validator

Setup:
  1. Generate your validator keys:
       ./aztibase.exe wallet generate --validator --output keys/validator.json

  2. Edit node.toml:
       - Add boot_nodes (IP addresses of existing validators)

  3. Register as a validator (requires AZTB tokens):
       ./aztibase.exe wallet stake --amount <AMOUNT> --validator-key keys/validator.json

  4. Start the node:
       bash start.sh

  5. Open the dashboard:
       http://127.0.0.1:8080

Endpoints:
  RPC:        http://127.0.0.1:9944
  Dashboard:  http://127.0.0.1:8080
  Health:     http://127.0.0.1:9944/health
  Metrics:    http://127.0.0.1:9944/metrics/json

Files:
  aztibase.exe   - The node binary
  genesis.toml   - Chain genesis config (must match all nodes)
  node.toml      - Node configuration (edit boot_nodes before starting)
  keys/          - Your validator keypair (generated during setup)
  explorer/      - Validator Dashboard (web UI)
  start.sh       - Start/stop script

Stop:  bash start.sh stop
Logs:  node.log
