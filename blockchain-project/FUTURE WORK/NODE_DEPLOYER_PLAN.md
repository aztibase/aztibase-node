# Aztibase Network — Node Deployer & VaaS Plan

> Multi-provider validator deployment platform with decentralization algorithm.
> Research completed 2026-03-23.

---

## Vision

A mobile/web app where anyone deploys Aztibase validator nodes to global VPS providers. User owns their provider account. Aztibase connects them and earns affiliate revenue. Algorithmic decentralization incentives prevent concentration.

---

## Phased Build Plan

### Phase 0 — Telegram Bot (1-2 weeks)
- Multi-user monitoring bot on VPS
- Per-user node isolation (scoped by Telegram chat_id)
- Commands: /status, /health, /alert, /nodes, /add, /remove
- Tech: Python + python-telegram-bot + aiosqlite
- Alerts: node down, block stale, peer loss, validator inactive

### Phase 1 — CLI Tool (3-4 weeks)
- `aztibase-deploy create --provider hetzner --region nbg1 --plan cx22`
- Terraform/cloud-init under the hood
- Hetzner + DigitalOcean adapters
- Distributed in GitHub release package

### Phase 2 — Web Dashboard (4-6 weeks)
- Rust Axum backend + HTMX frontend
- Deploy wizard (region, plan, provider)
- Grafana embed for monitoring
- Decentralization map (node distribution)

### Phase 3 — Mobile App (8-12 weeks, only if web validates demand)
- React Native + Expo
- Full OAuth flow for DigitalOcean
- Secure key generation in device enclave
- Push notifications via Expo
- Monitoring dashboard

---

## Provider Account Model

Users create their own accounts. Aztibase connects via OAuth/API token.

| Provider | Auth Model | Revenue |
|----------|-----------|---------|
| DigitalOcean | Full OAuth 2.0 | 10% recurring commission for 12 months (CJ affiliate) |
| Hetzner | Bearer token (user pastes) | Referral credits only |
| Vultr | Bearer token | Partner tiers (credit-based) |
| OVHcloud | OAuth2 client credentials | Formal VAR/MSP program |

---

## Key Security — Three Tiers

### Tier 1 (MVP — Ship First)
- Key generated on phone (Ed25519 from BIP-39 mnemonic)
- Stored in iOS Secure Enclave / Android Keystore
- Encrypted keystore uploaded to VPS (AES-256-GCM)
- Phone sends decrypt passphrase at each VPS boot over TLS
- VPS disk compromise = useless encrypted blob
- VPS RAM compromise = accepted risk at this tier

### Tier 2 (Post-Mainnet — Remote Signer)
- Node binary gets `--remote-signer <url>` flag
- Signing service runs at different provider
- Compromising one provider gets nothing

### Tier 3 (Endgame — Threshold Signatures)
- FROST threshold Ed25519
- Key sharded 2-of-3 across providers via DKG
- Full key never exists anywhere
- ~50 Cosmos validators run this pattern (Horcrux)

---

## Decentralization Algorithm

```
Max 15% of total validators per provider
Max 10% per single datacenter building
Max 25% per country
Max 40% per continent

Reward multiplier:
  Underserved regions: 1.0-1.25x
  Saturated regions: 0.75-1.0x
```

App nudges users toward decentralization through economics, not restrictions.

---

## Infrastructure Research Summary

### Equinix
- World's largest DC company ($9.2B revenue, 260+ DCs, 71 markets)
- Equinix Metal (bare metal API) shutting down June 30, 2026
- Equinix Fabric (private interconnect) survives, sub-1ms between cages
- Enterprise pricing: ~$2,500-4,000/month minimum. No free tier.
- Relevant post-mainnet for institutional validators

### Free Infrastructure Stack
| Tool | What | Free Tier |
|------|------|-----------|
| Oracle Cloud A1 | ARM compute | 4 OCPU, 24 GB RAM, forever free |
| GitHub Actions | CI/CD | Unlimited on public repos |
| Cloudflare | CDN + Tunnel + Workers | Unlimited bandwidth, 100K req/day |
| Tailscale | P2P mesh | 100 devices |
| Grafana Cloud | Remote monitoring | 10K series, 14-day retention |
| Uptime Kuma | Endpoint monitoring | Self-hosted, unlimited |
| Arweave | Permanent storage | ~$7/GB one-time |

### Budget Bare Metal for Validators
| Provider | Spec | Price |
|----------|------|-------|
| Hetzner CX22 | 2 vCPU, 4 GB, 40 GB NVMe | ~$4/month |
| Hetzner AX41-NVMe | 6C/12T, 64 GB RAM, 2x NVMe | ~$39/month |
| Fluence Network | 2 vCPU, 4 GB, decentralized | ~$11/month |
| Latitude.sh | AMD EPYC, 100 Gbps | ~$291/month |

### Centralization Warning
- Hetzner banned all Solana nodes in Nov 2022 (1000+ validators offline)
- AWS outage Oct 2025 ($2.8B losses, Base L2 spiked 5x)
- Any L1 where single provider holds >20% of stake has existential risk

---

## References

- Equinix Metal EOL June 30, 2026 (The Register, DCD)
- DigitalOcean OAuth API docs
- Hetzner API token documentation
- Solana decentralization facts (Helius)
- Horcrux threshold signing (Strangelove Ventures)
- ether.fi key management architecture
- Stereum Ethereum node launcher (SSH + Ansible pattern)
- eth-docker, Sedge deployment tools
