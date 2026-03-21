AZTIBASE VALIDATOR NODE
======================

Join the Aztibase testnet as a validator.

Quick Start (Windows):
  1. Double-click start.bat
  2. Wait for "Validator is RUNNING"
  3. Install the Aztibase Wallet Chrome extension
  4. Connect wallet to http://127.0.0.1:9944
  5. Click "Faucet" to get testnet AZTB
  6. Click "Become Validator"
  7. Done! You'll join at the next epoch boundary.

Quick Start (Linux/Mac):
  chmod +x aztibase
  ./aztibase --config node.toml --genesis genesis.toml

What Happens:
  - First run generates your validator key in keys/
  - Node connects to the Aztibase VPS boot node
  - Syncs the full chain from genesis
  - After you register, you produce blocks alongside other validators

Files:
  aztibase.exe   - Node binary (Windows)
  aztibase       - Node binary (Linux)
  node.toml      - Configuration (boot node pre-configured)
  genesis.toml   - Genesis block (must match the network)
  start.bat      - Windows launcher (double-click)
  stop.bat       - Stop the node
  keys/          - Your validator keypair (auto-generated)
  data/          - Chain data (created on first run)
  node.log       - Log output

Endpoints (after starting):
  RPC:       http://127.0.0.1:9944
  Health:    http://127.0.0.1:9944/health
  Metrics:   http://127.0.0.1:9944/metrics/json

Requirements:
  - Windows 10/11 x64 or Linux x64
  - Internet connection
  - Port 30333 open (for P2P)

Stop:
  Close the terminal window, or double-click stop.bat

Staking:
  - Minimum stake: 10,000 AZTB (faucet gives 1,000,000)
  - Rewards: 70% of epoch emission to validators
  - No claim step - rewards auto-credit each epoch

Need Help?
  Contact the network coordinator.
