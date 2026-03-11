# Aztibase Validator — VPS & Hardware Guide

Everything you need to choose the right server for running an Aztibase validator.

---

## Can I Run a Validator on My Local Machine?

### Quick Hardware Assessment

| Component | Your Machine | Minimum | Recommended | Verdict |
|-----------|-------------|---------|-------------|---------|
| CPU | i7 7th gen (4C/8T) | 4 cores | 8+ cores | Borderline |
| RAM | 8 GB | 4 GB | 16 GB | Too low for production |
| Disk | 250 GB SSD | 50 GB SSD | 200+ GB NVMe | OK for now |
| Network | Home broadband | 10 Mbps | 100+ Mbps | Variable |
| Uptime | Power outages, restarts | 99%+ | 99.9%+ | Unreliable |

**Verdict: Not recommended for production validators.** Your i7 7th gen can technically run testnet, but 8 GB RAM is tight when compiling and running simultaneously, and home internet has no uptime guarantee. Validators that go offline get slashed.

**Use it for:** local testnet development and testing.

**For production:** rent a VPS. Costs start at ~$4/month.

---

## Minimum Hardware Requirements

| Component | Minimum (Testnet) | Recommended (Mainnet) |
|-----------|-------------------|----------------------|
| CPU | 2 vCPU | 4+ vCPU (dedicated preferred) |
| RAM | 4 GB | 8-16 GB |
| Disk | 50 GB SSD | 200 GB NVMe |
| Network | 100 Mbps | 1 Gbps unmetered |
| OS | Ubuntu 22.04 / Debian 12 | Ubuntu 24.04 LTS |

**Storage note:** Aztibase uses redb (embedded database). Disk usage grows ~1 GB/month on testnet. For archive nodes, plan for 500 GB+.

---

## VPS Provider Comparison

Prices as of March 2026. All plans include IPv4, SSD/NVMe storage, and unmetered or high-limit bandwidth.

### Budget Tier (Testnet / Early Mainnet)

| Provider | Plan | CPU | RAM | Disk | Bandwidth | Price/mo | Notes |
|----------|------|-----|-----|------|-----------|----------|-------|
| **Hetzner** | CX22 | 2 vCPU | 4 GB | 40 GB | 20 TB | **€3.99** | Best value. EU/US DCs. |
| **Contabo** | VPS S | 4 vCPU | 8 GB | 50 GB | 32 TB | **€6.99** | Generous specs. EU DCs. |
| **OVH** | Starter | 2 vCPU | 4 GB | 40 GB | Unmetered | **€5.50** | EU/CA DCs. Good network. |
| **Vultr** | Cloud Compute | 2 vCPU | 4 GB | 80 GB | 3 TB | **$12** | 32 locations worldwide. |
| **DigitalOcean** | Basic | 2 vCPU | 4 GB | 80 GB | 4 TB | **$24** | Great UI, more expensive. |
| **Linode** | Nanode 4GB | 2 vCPU | 4 GB | 80 GB | 4 TB | **$24** | Akamai-backed. |

### Production Tier (Mainnet Validators)

| Provider | Plan | CPU | RAM | Disk | Bandwidth | Price/mo | Notes |
|----------|------|-----|-----|------|-----------|----------|-------|
| **Hetzner** | CX32 | 4 vCPU | 8 GB | 80 GB | 20 TB | **€7.59** | Best price/performance. |
| **Contabo** | VPS M | 6 vCPU | 16 GB | 100 GB | 32 TB | **€10.49** | Most RAM for the price. |
| **OVH** | Essential | 4 vCPU | 8 GB | 80 GB | Unmetered | **€13** | Unmetered bandwidth. |
| **Hetzner** | CCX23 (ARM) | 4 vCPU | 8 GB | 80 GB | 20 TB | **€7.19** | ARM64, great perf/watt. |

### Dedicated Servers (High-Performance Validators)

| Provider | Plan | CPU | RAM | Disk | Price/mo | Notes |
|----------|------|-----|-----|------|----------|-------|
| **Hetzner** | AX42 | Ryzen 5 3600 | 64 GB | 2× 512 GB NVMe | **€44** | Bare metal, no overhead. |
| **OVH** | Rise-1 | Intel E-2386G | 32 GB | 2× 480 GB SSD | **€54** | DDoS protection included. |

---

## Recommendation: Start with Hetzner CX22

For most validators joining testnet or early mainnet:

- **Hetzner CX22** at **€3.99/month** (~$4.30 USD)
- 2 vCPU, 4 GB RAM, 40 GB SSD, 20 TB bandwidth
- EU (Falkenstein, Nuremberg, Helsinki) or US (Ashburn) datacenters
- Upgrade to CX32 (€7.59) when mainnet traffic increases

Why Hetzner:
1. Cheapest reliable VPS in the market
2. Excellent network (low latency between EU/US nodes)
3. No bandwidth surprises (20 TB is more than enough)
4. Simple API and UI
5. Many blockchain projects use Hetzner (Ethereum, Solana, etc.)

---

## Step-by-Step: Rent a VPS and Start Validating

### 1. Create an Account

Go to [https://accounts.hetzner.com/signUp](https://accounts.hetzner.com/signUp) (or your chosen provider).

### 2. Create a Server

1. Click **Add Server**
2. Location: choose closest to you or to other validators
3. Image: **Ubuntu 24.04**
4. Type: **CX22** (shared vCPU, 4 GB RAM)
5. SSH key: add your public key (recommended) or use a password
6. Click **Create & Buy Now**

Cost: ~€3.99/month, billed hourly.

### 3. Connect via SSH

```bash
ssh root@YOUR_SERVER_IP
```

### 4. Run the One-Click Setup

```bash
curl -sSf https://raw.githubusercontent.com/user/aztibase/dev/deploy/setup-validator.sh -o setup.sh
sudo bash setup.sh
```

The script will:
- Install all dependencies (Rust, build tools)
- Compile the Aztibase binary (~10-15 minutes on CX22)
- Generate your validator keypair
- Configure systemd auto-restart
- Open firewall ports
- Optionally set up Grafana Cloud monitoring

### 5. Verify Your Node

```bash
# Check node is running
sudo systemctl status aztibase-validator

# Watch logs
sudo journalctl -u aztibase-validator -f

# Health check
curl -s http://localhost:9944/health | jq .

# Check block height
curl -s -X POST http://localhost:9944 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"aztb_blockNumber","params":[],"id":1}' | jq .
```

### 6. Fund and Stake

1. Get your validator address from the setup output
2. **Testnet:** Get tokens from [https://faucet.aztibase.com](https://faucet.aztibase.com)
3. **Mainnet:** Send AZTB to your validator address
4. Submit a Stake transaction for ≥10,000 AZTB

---

## Geographic Distribution

For network health, validators should be geographically diverse. Current seed nodes:

| Node | Location | Provider |
|------|----------|----------|
| testnet1.aztibase.com | EU (Germany) | Hetzner |
| testnet2.aztibase.com | EU (Finland) | Hetzner |
| testnet3.aztibase.com | US (Virginia) | Hetzner |

Choose a location that adds diversity. Good choices for new validators:
- Asia: Tokyo, Singapore (Vultr, DigitalOcean)
- South America: São Paulo (Vultr, DigitalOcean)
- Oceania: Sydney (Vultr, Linode)
- Africa: Johannesburg (limited options — self-host or AWS Cape Town)

---

## Monitoring Your Costs

| What | Cost | Notes |
|------|------|-------|
| VPS (Hetzner CX22) | €3.99/mo | Validator node |
| Grafana Cloud | Free | 10k metric series |
| Domain (optional) | €10/year | For your validator identity |
| **Total minimum** | **~€4/month** | |

---

## FAQ

**Q: Can I run multiple validators on one VPS?**
A: Not recommended. Each validator needs its own keypair and P2P port. Running multiple increases risk — if the VPS goes down, all validators get slashed.

**Q: Do I need a static IP?**
A: No. Aztibase uses DNS-based boot nodes and libp2p peer discovery. Your node will be found automatically.

**Q: Can I use AWS/GCP/Azure?**
A: Yes, but they cost 3-5x more than the VPS providers listed above for equivalent specs. Only use major clouds if you need specific compliance or geographic requirements.

**Q: What about ARM servers?**
A: Aztibase compiles and runs on ARM64 (aarch64). Hetzner's Ampere ARM servers (CAX series) offer excellent performance per euro. The setup script auto-detects ARM.

**Q: How much bandwidth does a validator use?**
A: ~5-20 GB/day depending on network activity. The 20 TB/month included with most VPS plans is far more than needed.
