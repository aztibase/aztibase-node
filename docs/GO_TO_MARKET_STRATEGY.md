# Aztibase Network — Go-To-Market Strategy

**Date:** 2026-03-13
**Status:** DRAFT — For team discussion

---

## Table of Contents

1. [Token Listing Strategy](#1-token-listing-strategy)
2. [Ecosystem Bootstrap](#2-ecosystem-bootstrap)
3. [Marketing Strategy](#3-marketing-strategy)
4. [Validator Hardware & Sales](#4-validator-hardware--sales)
5. [Presale & Funding](#5-presale--funding)
6. [Legal Considerations (South Africa)](#6-legal-considerations-south-africa)
7. [Budget Tiers](#7-budget-tiers)
8. [Timeline & Roadmap](#8-timeline--roadmap)

---

## 1. Token Listing Strategy

### The Bridge Problem

Aztibase uses Ed25519/BLAKE3 — it's NOT EVM-compatible. To access existing DEX liquidity, we need a wrapped token (wAZTB) on Ethereum, BSC, or Solana.

#### Bridge Options (pick one)

| Option | How | Cost | Trust Model | Timeline |
|--------|-----|------|-------------|----------|
| **A: Manual wrap (fastest)** | Deploy ERC-20 wAZTB, team acts as custodian | ~$500 gas | Centralized (like early WBTC) | 1-2 weeks |
| **B: Custom bridge** | Lock-and-mint bridge with multisig validators | Dev time only | Semi-decentralized | 4-8 weeks |
| **C: Wormhole integration** | Join Wormhole guardian network | Partnership required | Decentralized | Unlikely at launch |

**Recommendation:** Start with Option A (manual wrap) to get listed fast, then build Option B for decentralization.

### DEX Listing Path

DEXs are permissionless — no listing fee, just seed liquidity.

| DEX | Chain | Why | Min Liquidity Needed |
|-----|-------|-----|---------------------|
| Uniswap v3 | Ethereum | Largest DEX, most visibility | $50K-$100K |
| PancakeSwap | BSC | Lower fees, large retail base | $20K-$50K |
| Raydium/Jupiter | Solana | Fast-growing, low fees | $20K-$50K |

**Note:** CoinMarketCap/CoinGecko require ~$400K-$500K in DEX liquidity to approve a listing.

### CEX Listing Path

| Tier | Exchanges | Listing Cost (all-in) | Requirements |
|------|-----------|----------------------|--------------|
| **Tier 3 (entry)** | MEXC, Bitget, LBank | $40K-$150K | Audit, legal entity, basic community |
| **Tier 2** | Gate.io, KuCoin, Bybit | $150K-$300K | $50K+ daily volume, 1K+ holders, audit |
| **Tier 1** | Binance, Coinbase, OKX | $300K-$1M+ | $500K+ daily volume, 10K+ holders |

**What CEXs look for:** Daily volume, unique holders, market cap, security audit (CertiK/Hacken/PeckShield), legal entity with KYB, whitepaper, market maker partnership.

**Realistic path:** DEX first → MEXC (lowest barrier Tier 3) → KuCoin/Gate with traction data → Binance/Coinbase is a long-term goal.

### Long-Term: Native DEX

Since Aztibase has a dual VM (WASM + EVM), we can build a native DEX on our own chain, eliminating bridge dependency entirely (like Hyperliquid did).

---

## 2. Ecosystem Bootstrap

### How Validators Earn (No Apps Needed)

The tokenomics are already implemented in the codebase:

- **Hard cap:** 1 billion AZTB
- **Emission pool:** 600M tokens over ~10 years (halving every 2 years)
- **Year 1-2:** 120M AZTB/year minted automatically

Every epoch (~7 minutes), emission is split:

| Pool | Share | Purpose |
|------|-------|---------|
| Validators | 70% | Block rewards (proportional to stake + participation) |
| PoUW | 15% | AI compute task rewards |
| Treasury | 10% | Protocol development |
| Insurance | 5% | Staking insurance / slashing fund |

**Year 1 math:** 120M × 70% = 84M AZTB to validators. With 10 validators = ~8.4M each. With 100 = ~840K each.

### Value Accrual — What Makes AZTB Worth Something

1. **Gas fees** — anyone using the chain needs AZTB
2. **Staking lockup** — validators lock AZTB, reducing circulating supply
3. **AI compute market (PoUW)** — pay AZTB for decentralized AI inference (unique differentiator)
4. **dApps/L2s** — apps drive transaction volume
5. **Scarcity** — hard cap + halving schedule (like Bitcoin)

### Developer Attraction Strategy

| Priority | Action | Cost |
|----------|--------|------|
| 1 | "Build on Aztibase" tutorial (30-min completion) | Free |
| 2 | Docs site via mdBook (Rust-native) | Free |
| 3 | Target Rust developers (underserved by most L1s) | Free |
| 4 | Target AI/ML developers (tract, ONNX, candle users) | Free |
| 5 | Grant program in AZTB tokens (milestone-based) | Token allocation only |
| 6 | Virtual hackathon on DoraHacks | $300-$500 in prizes |

### Grant Program (Token-Based, $0 Cash)

| Tier | Amount | For |
|------|--------|-----|
| Micro | 5,000 AZTB | Bug bounties, docs, small tools |
| Builder | 25,000 AZTB | dApps, SDKs, integrations |
| Pioneer | 100,000 AZTB | Major infra (bridges, explorers, wallets) |

Milestone-based: 30% start, 40% milestone, 30% completion. Reserve 5-10% of token supply.

---

## 3. Marketing Strategy

### What One Person Can Manage

| Channel | Time/Week | Priority |
|---------|-----------|----------|
| X/Twitter (@aztibase) | 3-4 hrs | CRITICAL |
| Discord moderation | 1-2 hrs | HIGH |
| Blog/Mirror.xyz posts | 2-3 hrs (biweekly) | HIGH |
| Reddit engagement | 30 min | LOW |
| YouTube | Skip until team grows | DEFER |
| Telegram | 30 min (mirror Discord) | LOW |

**Total: ~6-8 hours/week. Sustainable for a solo dev.**

### X/Twitter Strategy

- **Build in public:** Weekly dev threads with real code, test results, architecture decisions
- **Thread format:** 5-8 tweet threads, each tweet standalone with value
- **Topics:** PoUW explainers, DAG vs linear chain, why Rust, AI inference verification
- **Engage 30 min/day:** Reply to L1 projects, AI-crypto discussions, Rust devs
- **X Spaces:** Host/join spaces on AI + crypto topics
- **Get Premium:** $8/mo for blue check, longer posts, analytics

### The AI Angle (Differentiator)

Competitors: Bittensor ($3B+), Render ($3B+), Akash ($1B+). All bolt compute onto existing chains.

**Aztibase difference:** AI compute is integrated INTO consensus (PoUW). Validators earn by doing useful AI inference, not wasting energy.

**How to market it (without being scammy):**
- Show real inference running on testnet (demo video)
- Publish benchmarks: "1,000 ONNX inferences in X seconds at Y cost"
- Never say "AI blockchain" — say "blockchain where mining does useful AI work"
- Position for AI agents making micropayments (major 2026 trend)

### Community Building Pre-Launch

#### Testnet Incentive Program

Multi-phase with tracked participation:

1. Phase 1: Run a validator node (10 pts/day uptime)
2. Phase 2: Submit transactions, use faucet (1 pt each)
3. Phase 3: Deploy a smart contract (25 pts)
4. Phase 4: Stress test / break things (50 pts per confirmed bug)

Points convert to mainnet AZTB airdrop. Anti-sybil: GitHub linking, Discord verification, minimum activity thresholds.

**Platform:** Zealy free tier (5,000 verifications/mo, ~30 active users). Paid at $150/mo when >100 members.

#### Ambassador Program

- Start with 5-10 ambassadors from earliest followers
- Roles: content creation, translations, community support
- Reward: early validator slot, AZTB allocation
- SA advantage: recruit from local tech communities

### South Africa Advantage

- SA formally classified crypto as financial products, 240+ licensed CASPs
- Regulated jurisdiction = trust signal (unlike offshore-only projects)
- African crypto adoption growing fast — position as African-founded L1
- Aztibase (Pty) Ltd gives legitimacy many projects lack

---

## 4. Validator Hardware & Sales

### Minimum Specs for Aztibase Validators

| Tier | CPU | RAM | Storage | Network | Use Case |
|------|-----|-----|---------|---------|----------|
| Minimum | 4 cores, 3+ GHz | 16 GB | 500 GB NVMe | 100 Mbps | Testnet |
| Recommended | 8 cores, 3.5+ GHz | 32 GB | 1 TB NVMe | 500 Mbps | Mainnet |
| Full (+ PoUW AI) | 8-16 cores, 3.5+ GHz | 64 GB | 2 TB NVMe | 1 Gbps | Validator + AI inference |

### Off-the-Shelf Hardware Options

#### Budget ($200-$400) — Testnet

| Device | CPU | RAM | Storage | Price |
|--------|-----|-----|---------|-------|
| Beelink SER5 5560U | Ryzen 5 6C/12T | 16 GB | 500 GB | ~$230 |
| Beelink EQ12 | Intel N100 4C/4T | 16 GB | 500 GB | ~$180 |
| GMKtec NucBox K2 | i5-12450H 8C/12T | 16 GB | 512 GB | ~$300 |

#### Mid ($400-$800) — Mainnet (sweet spot)

| Device | CPU | RAM | Storage | Price |
|--------|-----|-----|---------|-------|
| Beelink SER7 7840HS | Ryzen 7 8C/16T | 32 GB | 1 TB | ~$500 |
| MINISFORUM UM780 XTX | Ryzen 7 8C/16T | 32 GB | 1 TB | ~$550 |
| Intel NUC 13 Pro | i7-1360P 12C/16T | 32 GB | 1 TB | ~$650 |

#### High-End ($800-$2000) — Validator + AI

| Device | CPU | RAM | Storage | Price |
|--------|-----|-----|---------|-------|
| MINISFORUM MS-A1 | Ryzen 9 16C/32T | 64 GB | 2 TB | ~$1,100 |
| Supermicro E302 | Xeon D-1700 8-10C | 64 GB ECC | 2 TB | ~$1,500 |
| Custom AM5 ITX build | Ryzen 9 7950X 16C/32T | 64 GB | 2 TB | ~$1,200 |

### Cloud/VPS Alternative

| Provider | vCPUs | RAM | Storage | Monthly Cost |
|----------|-------|-----|---------|-------------|
| **Hetzner CAX31 (ARM)** | 8 | 16 GB | 160 GB | ~€11 |
| **Hetzner CX43** | 8 | 16 GB | 160 GB | ~€14 |
| **Contabo Cloud VPS L** | 8 | 30 GB | 400 GB | ~€17 |
| DigitalOcean CPU-Opt 8 | 8 | 16 GB | 100 GB | ~$96 |

**Best value:** Hetzner CAX31 (ARM) at €11/mo. Aztibase is pure Rust, compiles natively on ARM.

**Warning:** Hetzner's ToS may restrict blockchain nodes. Contabo/OVH/Vultr have no restrictions.

### Validator-as-a-Product (Tiered Model)

| Tier | Product | Price | What's Included |
|------|---------|-------|-----------------|
| 1 | **Lite Node** (software only) | Free | Download, run on any hardware, earn minimal rewards |
| 2 | **Validator Box** | $599-$799 | Pre-configured mini PC + Ubuntu + Aztibase + 10K-50K AZTB pre-staked |
| 3 | **AI Node** | $1,200-$1,800 | High-end hardware + PoUW AI capability + 100K AZTB pre-staked |
| 4 | **Validator NFT** (digital) | $200-$1,000 | NFT granting mainnet validator slot, limited supply (first 1,000) |

#### How to Start (White-Label Approach)

1. Buy 10-50 Beelink SER7 units (~$500 each)
2. Pre-flash with Ubuntu + Aztibase node software
3. Add logo sticker + setup guide
4. Sell as "Aztibase Validator Node" at $699 (margin: ~$200/unit)
5. Include pre-staked AZTB tokens in the bundle

**Custom hardware (full OEM):** MOQ 50-100 units for branding, $5-15/unit extra. Full custom chassis: $5K-$20K NRE. Not worth it until you've sold 100+ white-label units.

#### Revenue Projections (Year 1, Conservative)

| Product | Units | Price | Revenue |
|---------|-------|-------|---------|
| Validator Box (Tier 2) | 50-100 | $699 | $35K-$70K |
| AI Node (Tier 3) | 10-20 | $1,500 | $15K-$30K |
| Validator NFT (Tier 4) | 200-500 | $500 | $100K-$250K |
| Token Sale (IDO/LBP) | 1 | — | $100K-$500K |
| **Total** | | | **$250K-$850K** |

---

## 5. Presale & Funding

### Funding Path (Most Capital-Efficient)

1. **Validator NFTs** → early funding ($100K-$250K)
2. **Fjord Foundry LBP** → broader token distribution ($100K-$300K)
3. **MEXC listing** → exchange liquidity
4. **Hardware sales** → recurring revenue stream

### Token Sale Options

| Type | Raise | Platform | Requirements |
|------|-------|----------|-------------|
| Friends & Family | $5K-$50K | Direct | Working testnet |
| Seed / Angels | $50K-$250K | Direct / SAFT | 3+ months commits, roadmap |
| IDO (public) | $100K-$500K | Fjord Foundry, DAOMaker | Testnet validators, audit, community 500+ |
| IEO | $500K-$5M | MEXC, Gate.io | Mainnet, exchange partnership, 5K+ community |

### Launchpad Comparison

| Platform | Track Record | Avg Raise | Best For |
|----------|-------------|-----------|---------|
| **Fjord Foundry** | $1B+ raised, 100+ projects | Variable (LBP) | Fair price discovery, permissionless |
| **DAOMaker** | $90M+, 200+ projects | $46K-$4M | Broader reach, SHO model |
| **Polkastarter** | $50M+, 100+ IDOs | $200K-$500K | POLS token holder audience |

**Recommendation:** Fjord Foundry LBP — permissionless, fair price discovery, no upfront cost, community-driven.

### What Investors Want to See

- Working code (9 crates, 940+ tests — strong signal)
- Unique value prop (AI-native L1 + PoUW — differentiated)
- Legal entity (Aztibase (Pty) Ltd — established)
- Community traction (Discord, X followers, testnet participants)
- Security audit (at least one)
- Clear tokenomics (vesting, supply cap, inflation)
- **Red flag to address:** Solo developer. Consider 2-3 advisors or a co-founder.

---

## 6. Legal Considerations (South Africa)

### FSCA CASP License

SA requires crypto asset service providers to register with the FSCA. Key questions:

- Does issuing AZTB tokens constitute a "crypto asset service"?
- Is AZTB a utility token (gas/staking/compute) or a security token (investment)?
- If purely utility: may not trigger securities regulation
- If presale promising returns: likely triggers FSCA oversight

### Recommended Legal Structure

| Entity | Role | Jurisdiction |
|--------|------|-------------|
| **Aztibase (Pty) Ltd** | Development company, IP holder | South Africa |
| **Aztibase Foundation** (new) | Token issuance, treasury, grants | BVI / Cayman / Singapore / Switzerland |

This is standard practice — development company in home jurisdiction, token entity offshore.

### Action Items

- Engage SA crypto attorney (ENSafrica, Werksmans, Bowmans, or Fasken)
- Budget: R50,000-R150,000 (~$2,700-$8,100) for legal opinion + CASP application
- FICA/FIC compliance (AML/KYC) is mandatory for any entity handling crypto in SA

---

## 7. Budget Tiers

### $0 Budget (Time Only)

| Activity | Cost | Expected Outcome |
|----------|------|-----------------|
| X/Twitter organic posting | Free | 500-2,000 followers in 6 months |
| Discord server + free bots | Free | Community home base |
| Mirror.xyz blog posts | Free | Crypto-native credibility + SEO |
| Dev.to technical articles | Free | Rust developer awareness |
| GitHub presence | Free | Developer trust signal |
| Testnet with on-chain tracking | Free | 50-200 testnet participants |
| Zealy free tier | Free | Basic quest system |
| DoraHacks hackathon listing | Free | Developer attention |

### $1,000 Budget

Everything above, PLUS:

| Activity | Cost |
|----------|------|
| aztibase.com hosting + domain renewal | ~$100/yr |
| Zealy paid (3 months) | $450 |
| X Premium for @aztibase | $96/yr |
| Virtual hackathon prizes | ~$300 |
| Logo/banner design (Fiverr) | ~$50-100 |

### $10,000 Budget

Everything above, PLUS:

| Activity | Cost |
|----------|------|
| 2-3 micro-influencer partnerships | $1,000-$2,000 |
| Crypto PR article (1 placement) | $1,000-$2,000 |
| Hackathon prizes (larger) | $2,000 |
| Conference attendance (1 event) | $1,500-$2,500 |
| Part-time community manager (3 months) | $1,500-$3,000 |
| Security audit (1 crate) | $1,000-$2,000 |

### Minimum Viable Launch Budget

| Item | Cost |
|------|------|
| Security audit (1 mid-tier firm) | $15K-$30K |
| Initial DEX liquidity (wAZTB/ETH pool) | $25K-$50K |
| Ethereum gas (bridge + token deploy) | $500-$1K |
| MEXC listing (all-in) | $100K-$150K |
| Market maker (basic) | $20K-$50K |
| Legal (SA attorney + entity setup) | $3K-$10K |
| Community incentives (year 1) | $10K-$20K |
| **Total** | **$175K-$300K** |

---

## 8. Timeline & Roadmap

### Month 1-2: Foundation

- [ ] Set up @aztibase X/Twitter with Premium
- [ ] Create Discord server (announcements, dev-updates, validators, general)
- [ ] Publish 2 Mirror.xyz articles (PoUW explainer, architecture overview)
- [ ] Start weekly X threads (build in public)
- [ ] Finalize tokenomics (vesting, allocation, utility)
- [ ] Engage SA crypto attorney
- [ ] Order 5 prototype Validator Boxes (Beelink SER7)

### Month 3-4: Public Testnet

- [ ] Launch public testnet with faucet
- [ ] Set up Zealy quest system
- [ ] Announce testnet incentive program (points → future airdrop)
- [ ] Recruit first 5 ambassadors
- [ ] Write "Build on Aztibase" tutorial
- [ ] Apply to Fjord Foundry for LBP

### Month 5-6: Developer Attraction

- [ ] Host first virtual hackathon on DoraHacks ($300-$500 prizes)
- [ ] Publish grant program structure
- [ ] First conference or meetup attendance
- [ ] Security audit (Hacken or Halborn — more affordable than CertiK)
- [ ] Seek 2-3 strategic advisors

### Month 7-8: Token Launch

- [ ] Mainnet genesis ceremony
- [ ] Deploy wAZTB bridge (manual wrap → Uniswap/PancakeSwap)
- [ ] Fjord Foundry LBP ($100K-$300K target)
- [ ] Testnet airdrop to participants
- [ ] CoinMarketCap / CoinGecko applications

### Month 9-12: Scale

- [ ] Apply to MEXC with DEX traction data
- [ ] Sell first batch of Validator Boxes
- [ ] Validator NFT mint (limited supply)
- [ ] Second hackathon (larger scope)
- [ ] Partnership discussions with complementary projects

### Month 12-18: Growth

- [ ] Tier 2 CEX listing (KuCoin/Gate.io)
- [ ] Scale hardware sales to 100+ units
- [ ] Native DEX on Aztibase chain
- [ ] Full bridge (decentralized)
- [ ] Ecosystem grants program at scale

---

## Key Takeaways

1. **Validators earn from day one** via block rewards (70% of emission) — no apps needed
2. **Value comes from demand** — the AI compute market (PoUW) is the unique selling point
3. **Start listing on DEXs** (permissionless, $50K liquidity) then work up to CEXs
4. **Sell white-label Validator Boxes** at $699 (margin ~$200) — start with 10-50 units
5. **Validator NFTs** ($200-$1,000) are the fastest path to early funding
6. **Fjord Foundry LBP** is the best launchpad for a solo-dev project
7. **Total minimum viable launch: ~$175K-$300K** — can be partially bootstrapped via NFT sales
8. **Legal first:** Get SA crypto attorney opinion before any token sale
9. **The solo-dev red flag:** Consider 2-3 advisors to strengthen credibility
10. **AI narrative is hot in 2026** — lean into PoUW as differentiator, show real demos

---

## Sources

Research conducted 2026-03-13. Key references:
- MEXC/Gate.io/KuCoin listing requirements and costs (listing.help)
- Fjord Foundry, DAOMaker launchpad data
- Berachain, Monad, Hyperliquid launch strategies
- SA FSCA crypto regulations (cryptoforinnovation.org, bitcoinke.io)
- Hardware pricing from Beelink, MINISFORUM, Hetzner, Contabo
- Helium, DIMO, Deeper Network hardware-as-product models
- Crypto marketing strategies (blockchain-ads.com, solus.agency)
- AI + crypto convergence (Fortune, Entrepreneur, 2026)
