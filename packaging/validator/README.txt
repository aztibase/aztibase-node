AZTIBASE VALIDATOR NODE
======================

Run a validator node on the Aztibase testnet.

Prerequisites:
  - Windows x64
  - Node.js installed (for the dashboard server)
  - AZTB tokens for staking (minimum 10,000 AZTB)

Quick Start:
  1. Run: bash start.sh
     (This auto-generates your validator keys on first run)

  2. Install the Aztibase Wallet extension (Chrome)
     Connect to http://127.0.0.1:9944

  3. Hit the Faucet button to get testnet AZTB

  4. Go to Staking tab → hit "Become Validator"
     That's it — you're a validator at the next epoch.

  5. Open the dashboard:
     http://127.0.0.1:8080

How It Works:
  - The --testnet flag uses the built-in genesis config and connects to
    public testnet boot nodes automatically.
  - node.toml has boot_nodes pre-configured for the public testnet.
  - Your validator key is generated on first run and saved to keys/.
  - After staking, your node joins the active validator set at the next
    epoch boundary and starts earning rewards automatically.

Tokenomics:
  - Genesis supply: 400,000,000 AZTB
  - Validator allocation: 20,000,000 AZTB (5% of genesis)
  - Emission: 600,000,000 AZTB over ~10 years (halving every 2 years)
  - Rewards: 70% of epoch emission goes to validators, proportional to stake
  - No claim step — rewards are auto-credited to your balance each epoch

Endpoints:
  RPC:        http://127.0.0.1:9944
  Dashboard:  http://127.0.0.1:8080
  Health:     http://127.0.0.1:9944/health
  Metrics:    http://127.0.0.1:9944/metrics/json

Files:
  aztibase.exe   - The node binary
  node.toml      - Node configuration (boot nodes pre-configured)
  keys/          - Your validator keypair (auto-generated on first run)
  explorer/      - Validator Dashboard (web UI)
  start.sh       - Start/stop script

Stop:  bash start.sh stop
Logs:  node.log
