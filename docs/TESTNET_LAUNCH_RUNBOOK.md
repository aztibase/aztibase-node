# Aztibase Testnet Launch Runbook

**Version:** 1.0
**Date:** 2026-03-11
**Network:** Aztibase Testnet (Chain ID: 0xA27B / 41595)

---

## Pre-Launch Checklist

### Code Readiness

| Check | Status | How to Verify |
|-------|--------|---------------|
| All tests pass | 931 tests | `CARGO_INCREMENTAL=0 cargo test --workspace` |
| Clippy clean | 0 warnings | `cargo clippy --workspace` |
| Fmt clean | Pass | `cargo fmt --check --all` |
| Snapshot export works | Phase 2 done | `aztibase snapshot export --output /tmp/test.snap` |
| Mainnet genesis audited | 400M AZTB | `cargo test mainnet_genesis_valid` |
| Security flags | 0 ELEVATED | All 9 flags resolved (Sprint 048) |

### Infrastructure Readiness

| Component | Domain | Hosting | Status |
|-----------|--------|---------|--------|
| Seed Node 1 | testnet1.aztibase.com | Cloud VM (Ubuntu 22.04+) | PENDING |
| Seed Node 2 | testnet2.aztibase.com | Cloud VM (Ubuntu 22.04+) | PENDING |
| Seed Node 3 | testnet3.aztibase.com | Cloud VM (Ubuntu 22.04+) | PENDING |
| Block Explorer | explorer.aztibase.com | Vercel (static) | PENDING |
| Faucet UI | faucet.aztibase.com | Vercel (static) | PENDING |
| Testnet Landing | testnet.aztibase.com | Vercel (static) | PENDING |
| DNS Records | aztibase.com | Cloudflare / registrar | PENDING |

---

## Step 1: Provision Cloud VMs (3 seed nodes)

### Requirements per node
- **CPU**: 4+ cores (2 minimum)
- **RAM**: 8 GB (4 minimum)
- **Disk**: 100 GB NVMe SSD (40 minimum)
- **Network**: 1 Gbps (100 Mbps minimum)
- **OS**: Ubuntu 22.04 LTS
- **Ports**: TCP 30333 (P2P), TCP 9944 (RPC), TCP 443 (HTTPS via reverse proxy)

### Recommended providers
- Hetzner (CPX41 or AX41-NVMe) — best price/performance in EU
- DigitalOcean (CPU-Optimized 4vCPU)
- Vultr (High Frequency 4 CPU)
- AWS (c6a.xlarge) — if you need multi-region

### Provision script (run on each VM)
```bash
# SSH into each VM
ssh root@<VM_IP>

# Download and run bootstrap
curl -sSf https://raw.githubusercontent.com/user/aztibase/dev/deploy/bootstrap.sh | bash

# Or manual setup:
apt update && apt install -y build-essential pkg-config libssl-dev git
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env
git clone https://github.com/user/aztibase.git /opt/aztibase
cd /opt/aztibase
CARGO_INCREMENTAL=0 cargo build --release -p aztibase-node
```

---

## Step 2: Generate Testnet Keys

On your **local machine** (not the VMs):

```bash
# Build locally
cargo build --release -p aztibase-node

# Generate 3 validator keypairs
for i in 1 2 3; do
  ./target/release/aztibase wallet generate \
    --mnemonic --passphrase "testnet-validator-${i}" \
    --output "keys/validator-${i}.json"
done

# Generate faucet key
./target/release/aztibase wallet generate \
  --mnemonic --passphrase "testnet-faucet" \
  --output "keys/faucet.json"
```

**Back up all key files and mnemonic phrases securely.** Use a password manager or encrypted volume.

Alternatively, use the pre-generated keys from `testnet/genesis/keys/` (for testing only — generate fresh keys for public testnet).

---

## Step 3: Deploy Node Configs

Copy the seed node configs to each VM:

```bash
# Node 1
scp deploy/seed-nodes/seed-1.toml root@<VM1_IP>:/opt/aztibase/config.toml
scp keys/validator-1.json root@<VM1_IP>:/opt/aztibase/validator.json

# Node 2
scp deploy/seed-nodes/seed-2.toml root@<VM2_IP>:/opt/aztibase/config.toml
scp keys/validator-2.json root@<VM2_IP>:/opt/aztibase/validator.json

# Node 3
scp deploy/seed-nodes/seed-3.toml root@<VM3_IP>:/opt/aztibase/config.toml
scp keys/validator-3.json root@<VM3_IP>:/opt/aztibase/validator.json
```

**Edit each config** to replace placeholder addresses with the actual VM IPs/domains:
- `listen_addresses` — bind to `0.0.0.0` (already set in seed configs)
- `boot_nodes` — replace `testnet{1,2,3}.aztibase.com` with actual IPs if DNS isn't ready yet
- `validator_key` — path to the validator key file

---

## Step 4: Configure DNS

Add these DNS records in your domain registrar (Cloudflare recommended):

| Type | Name | Value | Proxy |
|------|------|-------|-------|
| A | testnet1 | `<VM1_IP>` | DNS only (no proxy) |
| A | testnet2 | `<VM2_IP>` | DNS only (no proxy) |
| A | testnet3 | `<VM3_IP>` | DNS only (no proxy) |
| CNAME | explorer | `cname.vercel-dns.com` | DNS only |
| CNAME | faucet | `cname.vercel-dns.com` | DNS only |
| CNAME | testnet | `cname.vercel-dns.com` | DNS only |

**Important**: Seed nodes must NOT be behind a CDN proxy (Cloudflare orange cloud). P2P and RPC need direct TCP access.

---

## Step 5: TLS (HTTPS) for RPC

Validators connect over raw TCP (port 30333). But the RPC endpoint (port 9944) should be behind HTTPS for browser access.

### Option A: Caddy reverse proxy (recommended — auto TLS)
```bash
apt install -y caddy

# /etc/caddy/Caddyfile
cat > /etc/caddy/Caddyfile << 'EOF'
testnet1.aztibase.com {
    reverse_proxy localhost:9944
}
EOF

systemctl restart caddy
```

### Option B: nginx + certbot
```bash
apt install -y nginx certbot python3-certbot-nginx
certbot --nginx -d testnet1.aztibase.com
# nginx reverse_proxy to localhost:9944
```

---

## Step 6: Start the Nodes

Start in order: Node 1 first (as seed), then 2 and 3.

```bash
# On each VM
cd /opt/aztibase

# Using systemd (recommended)
cp deploy/systemd/aztibase.service /etc/systemd/system/
systemctl daemon-reload
systemctl enable aztibase
systemctl start aztibase

# Check logs
journalctl -u aztibase -f

# Or run directly
./target/release/aztibase \
  --testnet \
  --config config.toml \
  --validator-key validator.json \
  --data-dir ./data \
  --metrics
```

### Verify each node
```bash
# Health check
curl http://localhost:9944/health

# Node info
curl -s -X POST http://localhost:9944 \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"aztb_nodeInfo","params":[],"id":1}' | python3 -m json.tool

# Block height
curl -s -X POST http://localhost:9944 \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"aztb_blockNumber","params":[],"id":1}'

# Peer count (should be 2 once all nodes are up)
curl http://localhost:9944/metrics 2>/dev/null | grep peer_count
```

### What to expect
- Node 1 starts alone, begins producing blocks (solo validator mode)
- Nodes 2 and 3 connect, peer count rises to 2 on each
- Block production should stabilize at ~400ms per block
- Genesis hash must match across all 3 nodes

---

## Step 7: Deploy Static Sites

### Block Explorer → explorer.aztibase.com
```bash
# In the explorer/ directory
# Set the default RPC endpoint
# The explorer uses ?rpc= query param, default is localhost:9944
# For production, users visit: explorer.aztibase.com?rpc=https://testnet1.aztibase.com:9944

# Deploy to Vercel
cd explorer
npx vercel --prod
# Set custom domain: explorer.aztibase.com in Vercel dashboard
```

**Tip**: Update the default RPC_URL in `explorer/index.html` line 234 before deploying:
```javascript
const RPC_URL = params.get('rpc') || 'https://testnet1.aztibase.com';
```

### Faucet UI → faucet.aztibase.com
```bash
cd faucet
npx vercel --prod
# Set custom domain: faucet.aztibase.com
```

Same tip — update the default RPC URL in the faucet page.

### Testnet Landing → testnet.aztibase.com
```bash
cd testnet
npx vercel --prod
# Set custom domain: testnet.aztibase.com
```

---

## Step 8: Smoke Test

Run these checks after all nodes are up and sites are deployed:

```bash
# 1. Chain ID matches
curl -s -X POST https://testnet1.aztibase.com \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"aztb_chainId","params":[],"id":1}'
# Expected: {"result":"0xa27b",...}

# 2. Genesis hash matches across all nodes
for i in 1 2 3; do
  echo -n "Node $i: "
  curl -s -X POST https://testnet${i}.aztibase.com \
    -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","method":"aztb_genesisHash","params":[],"id":1}' | python3 -c "import sys,json; print(json.load(sys.stdin)['result'])"
done

# 3. Block production is advancing
curl -s -X POST https://testnet1.aztibase.com \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"aztb_blockNumber","params":[],"id":1}'
# Wait 5 seconds, call again — height should increase

# 4. Faucet drip works
curl -s -X POST https://testnet1.aztibase.com \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"aztb_faucetDrip","params":["<YOUR_ADDRESS>"],"id":1}'

# 5. Balance check
curl -s -X POST https://testnet1.aztibase.com \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"aztb_getBalance","params":["<YOUR_ADDRESS>"],"id":1}'

# 6. Explorer loads blocks (open in browser)
# https://explorer.aztibase.com?rpc=https://testnet1.aztibase.com

# 7. Faucet UI works (open in browser)
# https://faucet.aztibase.com?rpc=https://testnet1.aztibase.com

# 8. Metrics endpoint returns Prometheus data
curl -s https://testnet1.aztibase.com/metrics | head -20
```

---

## Chain Monitoring

### Built-in Metrics (Prometheus-compatible)

Every node with `--metrics` exposes `GET /metrics` (Prometheus text format) and `GET /metrics/json`.

**Available metrics:**

| Metric | Type | Description |
|--------|------|-------------|
| `aztibase_vertices_proposed` | Counter | DAG vertices proposed by this node |
| `aztibase_vertices_received` | Counter | DAG vertices received from peers |
| `aztibase_commits` | Counter | Consensus commits (finalized batches) |
| `aztibase_rounds_advanced` | Counter | Consensus rounds advanced |
| `aztibase_equivocations` | Counter | Equivocation events detected |
| `aztibase_last_commit_latency_us` | Gauge | Last commit latency in microseconds |
| `aztibase_txs_processed` | Counter | Total transactions processed |
| `aztibase_block_height` | Gauge | Current block height |
| `aztibase_base_fee` | Gauge | Current base fee |
| `aztibase_mempool_size` | Gauge | Pending transactions in mempool |
| `aztibase_pending_tasks` | Gauge | Pending AI compute tasks |
| `aztibase_peer_count` | Gauge | Connected P2P peers |
| `aztibase_active_validators` | Gauge | Active validator count |
| `aztibase_total_staked` | Gauge | Total AZTB staked |
| `aztibase_slashes_applied` | Counter | Slash events applied |

### Option 1: Grafana + Prometheus (recommended for production)

```bash
# On a monitoring VM (or same VMs, separate Docker)
# docker-compose.yml
version: '3'
services:
  prometheus:
    image: prom/prometheus:latest
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
    ports:
      - "9090:9090"

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=changeme
```

```yaml
# prometheus.yml
global:
  scrape_interval: 5s

scrape_configs:
  - job_name: 'aztibase-testnet'
    static_configs:
      - targets:
          - 'testnet1.aztibase.com:9944'
          - 'testnet2.aztibase.com:9944'
          - 'testnet3.aztibase.com:9944'
    metrics_path: '/metrics'
    scheme: 'https'
```

**Key Grafana panels to create:**
1. **Block Height** — time series, all 3 nodes overlaid (detect sync issues)
2. **Commits/sec** — rate of consensus commits
3. **Peer Count** — per node (alert if drops below 2)
4. **Mempool Size** — pending tx queue depth
5. **Commit Latency** — last_commit_latency_us (should stay < 1s)
6. **TPS** — rate(txs_processed) across nodes
7. **Equivocations** — alert if > 0
8. **Active Validators** — should stay at 3

### Option 2: Simple cron monitoring (low-cost alternative)

Create a monitoring script that runs every 60 seconds:

```bash
#!/bin/bash
# monitor.sh — run via cron every minute
NODES=("https://testnet1.aztibase.com" "https://testnet2.aztibase.com" "https://testnet3.aztibase.com")
LOG="/var/log/aztibase-monitor.log"
ALERT_WEBHOOK="https://hooks.slack.com/services/YOUR/WEBHOOK/URL"

for node in "${NODES[@]}"; do
  # Health check
  status=$(curl -s -o /dev/null -w "%{http_code}" "$node/health" --max-time 5)
  if [ "$status" != "200" ]; then
    msg="ALERT: $node health check failed (HTTP $status)"
    echo "$(date -u) $msg" >> "$LOG"
    curl -s -X POST "$ALERT_WEBHOOK" -d "{\"text\":\"$msg\"}" > /dev/null
    continue
  fi

  # Block height
  height=$(curl -s -X POST "$node" \
    -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","method":"aztb_blockNumber","params":[],"id":1}' \
    --max-time 5 | python3 -c "import sys,json; print(json.load(sys.stdin).get('result','?'))" 2>/dev/null)

  # Peer count
  peers=$(curl -s "$node/metrics/json" --max-time 5 | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('network',{}).get('peer_count','?'))" 2>/dev/null)

  echo "$(date -u) $node height=$height peers=$peers" >> "$LOG"
done
```

```bash
# crontab -e
* * * * * /opt/aztibase/monitor.sh
```

### Option 3: UptimeRobot / BetterStack (zero-infra)

- Add HTTP monitors for each `https://testnet{1,2,3}.aztibase.com/health`
- Set check interval: 60 seconds
- Alert via email/Slack/Discord on failure
- Free tier covers 3 monitors easily

### Alerting Rules (regardless of monitoring stack)

| Condition | Severity | Action |
|-----------|----------|--------|
| /health returns non-200 | CRITICAL | Restart node, check logs |
| Block height stale > 60s | CRITICAL | Check consensus, peer connections |
| Peer count < 2 | WARNING | Check network, firewall rules |
| Equivocations > 0 | WARNING | Investigate validator key compromise |
| Mempool > 10,000 | WARNING | Check for spam, adjust rate limits |
| Commit latency > 2s | WARNING | Check network latency between nodes |
| Disk usage > 80% | WARNING | Plan disk expansion or enable pruning |

---

## Operational Procedures

### Restart a node
```bash
systemctl restart aztibase
journalctl -u aztibase -f  # watch logs
```

### Check for stuck consensus
```bash
# Compare block heights across nodes
for i in 1 2 3; do
  echo -n "Node $i: "
  curl -s -X POST https://testnet${i}.aztibase.com \
    -H 'Content-Type: application/json' \
    -d '{"jsonrpc":"2.0","method":"aztb_blockNumber","params":[],"id":1}'
  echo
done
```

### Export state snapshot (for new validators)
```bash
ssh root@testnet1.aztibase.com
cd /opt/aztibase
./target/release/aztibase snapshot export --output /tmp/testnet-snapshot.snap
# Transfer to new validator, start with: aztibase --snapshot /tmp/testnet-snapshot.snap --testnet
```

### Reset a node (nuclear option)
```bash
systemctl stop aztibase
rm -rf /opt/aztibase/data/db /opt/aztibase/data/execution_db /opt/aztibase/data/*.redb
systemctl start aztibase
# Node will re-sync from peers (or use --snapshot to bootstrap faster)
```

---

## Rollback Plan

If critical bugs are found after launch:

1. **Stop all nodes**: `systemctl stop aztibase` on all 3 VMs
2. **Fix the bug** in code, push to `dev`, build
3. **Export snapshot** from the healthiest node before stopping
4. **Deploy fixed binary** to all VMs
5. **Restart** — nodes resume from persisted state
6. If state is corrupted: **import snapshot** and restart

---

## Post-Launch Monitoring Cadence

| Timeframe | Action |
|-----------|--------|
| First 1 hour | Watch logs continuously on all 3 nodes |
| First 24 hours | Check every 2 hours: heights, peers, latency |
| First week | Daily check: disk usage, error logs, metrics trends |
| Ongoing | Automated alerts handle it; weekly manual review |
