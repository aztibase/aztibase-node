# Validator Go-to-Market Options — Aztibase Network

**Date:** 2026-03-13
**Status:** Research / Planning

---

## Hardware Requirements (from VPS_GUIDE.md)

| | Testnet | Mainnet |
|--|---------|---------|
| CPU | 2 vCPU | 4+ vCPU |
| RAM | 4 GB | 8-16 GB |
| Disk | 50 GB SSD | 200 GB NVMe |
| Network | 100 Mbps | 1 Gbps |
| Storage growth | ~1 GB/month | ~1 GB/month |

Aztibase is lightweight compared to Ethereum/Solana — a major selling point.

---

## Option A: Recommended Off-the-Shelf Hardware + Software Image

**Lowest risk, fastest to market.**

| Hardware | Specs | Price | Notes |
|----------|-------|-------|-------|
| Raspberry Pi 5 (16GB) kit | 4-core ARM, 16GB, NVMe hat + 512GB SSD | ~$200 | 10W power, proven for Ethereum (200 validators on one Pi 5) |
| Beelink SER5 | Ryzen 5 6C/12T, 16GB DDR4, 500GB NVMe | ~$260-300 | Sweet spot — x86, no compilation issues |
| Beelink SER5 Max | Ryzen 7 8C/16T, 24GB DDR5, 500GB NVMe | ~$299 | Best value for performance |
| Beelink SER8/SER9 | Ryzen 7/9, 32GB DDR5, 1TB NVMe | $500-700 | Overkill but future-proof |

**You provide:** Downloadable OS image (Ubuntu + Aztibase pre-configured) or setup-validator.sh script.

**Pros:** Zero inventory risk, immediate, users can upgrade hardware independently.
**Cons:** No hardware margin, no brand differentiation.

---

## Option B: Custom-Branded "Aztibase Node" Hardware

**Higher margin, stronger brand, but requires capital.**

Shenzhen OEM manufacturers (Jinghong, OAI PC, INCTEL) offer custom-branded mini PCs:

| Volume | Per-Unit Cost | Sell Price | Margin | Upfront Investment |
|--------|--------------|-----------|--------|-------------------|
| 100 units | $200-350 | $500-600 | ~$200/unit | $20K-35K |
| 500 units | $150-250 | $450-500 | ~$250/unit | $75K-125K |
| 1,000 units | $100-200 | $400-450 | ~$250/unit | $100K-200K |

Customization: branded enclosure, logo, BIOS splash, pre-installed Aztibase image, custom packaging. Lead time: 45-90 days.

**OEM Manufacturers:**
- Jinghong (odmminipc.com) — low MOQ, custom branding
- OAI PC (oaipc.com) — Shenzhen, custom BIOS/OS/logo
- Shenzhen JIALAIBAO Technology — accepts orders as low as 100 units
- INCTEL (inctelpc.com) — industrial and mini PC manufacturer

**Enclosure costs:**
- CNC machined: $20-50/unit, low tooling ($500-2,000). Good for <500 units.
- Injection molded: $2-5/unit, high tooling ($5,000-15,000). Only viable at 1,000+ units.

**Pros:** Hardware margin ($200-250/unit), brand identity, controlled experience.
**Cons:** $20K+ upfront minimum, inventory risk, warranty/support burden.

---

## Option C: Presale / Crowdfunding Campaign

**Proven models from DePIN projects:**

| Project | Platform | Raised | Hardware Price | Model |
|---------|----------|--------|---------------|-------|
| Deeper Network | Kickstarter → Indiegogo | $2.3M | $200-400 | VPN router + token mining |
| Helium | Third-party manufacturers | $1B+ ecosystem | $200-500 | Hotspot earns HNT |
| XNET | Own website (IDO) | 40M tokens across 6 rounds | Separate hardware | Token presale + hardware deploy |

### Recommended Presale Structure

1. **Hardware-only presale** (legally safest — Helium SEC case dismissed April 2025 confirms this):
   - Sell "Aztibase Validator Node" at $400-500
   - Buyer earns AZTB through running the validator (staking rewards)
   - No tokens bundled with hardware — tokens are *earned*, not *sold*
   - This avoids securities classification under Howey test

2. **Presale tiers:**
   - Early Bird (first 50): $350 + priority genesis slot
   - Standard (next 200): $450
   - Late (remaining): $500
   - All include: hardware, pre-loaded image, genesis validator slot, setup support

3. **Platform options:**
   - Own website (cheapest, full control, but requires trust/audience)
   - Kickstarter/Indiegogo (built-in audience, credibility, but fees ~5-8%)
   - Crypto-native: could accept AZTB/ETH/USDC payments

---

## Option D: VPS / Cloud Validator Path (Parallel Offering)

**Lowest barrier — for people who don't want hardware.**

| Provider | Monthly Cost | Specs |
|----------|-------------|-------|
| Hetzner CX22 | €3.99/mo (~$4.30) | 2 vCPU, 4GB, 40GB (testnet) |
| Hetzner CX32 | €7.59/mo (~$8.20) | 4 vCPU, 8GB, 80GB (mainnet) |
| Contabo VPS M | €10.49/mo | 6 vCPU, 16GB, 200GB |

Existing setup-validator.sh handles this. A "Deploy to Hetzner" button or DigitalOcean Marketplace image would complete this path.

---

## Option E: Validator-as-a-Service (VaaS)

**For people who just want to stake, not operate.**

- You or a partner runs the infrastructure
- Users delegate AZTB to your validator
- Commission: 5-15% of staking rewards
- Providers like Allnodes charge $5-50/month flat fee per validator

Works well alongside hardware sales — captures the passive investor segment.

**Major VaaS providers for reference:**
- Everstake: 80+ chains, $10B+ staked, 99.99% uptime
- Allnodes: 80+ chains, $5-50/month flat fee, beginner-friendly
- Validation Cloud: enterprise-grade, non-custodial
- P2P.org: 40+ chains, institutional focus

---

## Existing Prebuilt Validator Hardware Market

| Company | Product | Price | Notes |
|---------|---------|-------|-------|
| DappNode Home 32/2 | Intel i7, 32GB, 2TB NVMe | ~€1,655 | Market leader, DappNode OS |
| DappNode Home 64/8 | Intel i7, 64GB, 8TB NVMe | ~€1,825 | Multi-chain flagship |
| DappNode Basic | Intel NUC i3, 8GB, 512GB | ~€600-800 | Entry-level |
| Avado (defunct) | NUC-based | Was $600-1,600 | Company wound down |

The market is small and niche. Most products are Intel NUC derivatives with custom OS images at $600-$2,000. No dominant leader — opportunity exists.

---

## Recommended Phased Approach

### Phase 1 — Now (Friends Testnet)
- Recommend Beelink SER5 ($260-300) or Pi 5 ($200) + existing setup script
- Validate the workflow with friends, zero financial risk

### Phase 2 — Post-Testnet Validation
- Launch presale on aztibase.com for 100 custom-branded units
- Price: $450-500, accept crypto + fiat
- Use presale funds to cover OEM order ($20-35K)
- Each unit = genesis validator slot + pre-configured hardware

### Phase 3 — Post-Mainnet
- Add VaaS / delegation for passive participants
- Scale hardware orders based on demand (500+ units at better margins)
- Consider Kickstarter for broader reach

---

## Legal Considerations

The Helium SEC dismissal (April 2025, with prejudice) is the strongest precedent for DePIN hardware sales. Key requirements:

1. Hardware sold separately from tokens
2. AZTB earned through genuine network contribution (running a validator)
3. No profit guarantees made
4. Proper corporate entity (Aztibase (Pty) Ltd — exists)
5. Hardware+token bundles = HIGH risk (Howey test: "expectation of profits from efforts of others")
6. FIT21 Act (pending) favorable for DePIN utility tokens

**Bottom line:** Sell hardware, let operators earn tokens through work. Do not pre-load or bundle tokens.

---

## AI Compute & PoUW: Where Does the Work Come From?

### Current AI Infrastructure (as built)

- **Runtime:** `tract` (pure Rust, CPU-only ONNX inference)
- **Max model size:** 64 MiB per ONNX model
- **Inference timeout:** 30 seconds
- **Verification:** Deterministic re-execution (BLAKE3 hash comparison)
- **Quorum:** ≥2 validators must produce matching result_hash

### The PoUW Pipeline (fully implemented, 13+ e2e tests)

```
1. MODEL PROVIDER → TxKind::RegisterModel { name, hash, uri, royalty_bps }
   └─ Registers ONNX model on-chain in ModelRegistry

2. COMPUTE VALIDATOR → TxKind::CommitCompute { model_id, stake_bond }
   └─ Opts in to run inference, bonds stake as commitment

3. TASK REQUESTER → TxKind::PostTask { model_id, input_hash, reward, expiry }
   └─ Posts inference job, pays AZTB reward. Enters TaskPool (max 1,024 pending)

4. TASK ASSIGNER → Auto-assigns to best validator
   └─ Ranked by PoUW score: 40% accuracy + 30% latency + 30% availability

5. ASSIGNED VALIDATOR → Runs inference via tract, submits attestation
   └─ TxKind::SubmitAttestation { task_id, result_hash, signature }

6. QUORUM CHECK → AttestationAggregator (≥2 matching results)

7. SETTLEMENT → Reward split among attesting validators
   └─ Model provider gets 5% royalty, validators split the rest
```

### The Demand Gap (two-sided marketplace problem)

The protocol pipeline is complete and tested, but **no one is currently posting PostTask transactions.** There are no:
- dApps that submit inference requests
- Inference marketplace UI
- External API to submit tasks
- Reference models deployed on the network

Validators won't commit compute without tasks. Task requesters won't come without validators.

### Hardware Tiers for AI Capability

| Tier | Role | Hardware | AI Capability | Price |
|------|------|----------|---------------|-------|
| **Consensus Validator** | Block production, voting, staking only | Pi 5 or Beelink SER5 | None (PassthroughRuntime) | $200-300 |
| **AI Compute Validator** | Consensus + inference + PoUW rewards | Beelink SER5 Max or better | 64 MiB ONNX models, ~10-50ms inference | $300-500+ |

Not every validator needs AI capability. `PassthroughRuntime` lets consensus-only nodes skip inference entirely.

### What 64 MiB ONNX Models Can Do

- Image classification (MobileNet ~14MB, EfficientNet-Lite ~20MB)
- Object detection (YOLOv5-nano ~4MB)
- Sentiment analysis, text classification
- Anomaly detection, time series forecasting
- Small NLP (DistilBERT is ~67MB — just over limit)

These are NOT LLMs. No GPT, no Llama, no diffusion. That's by design — deterministic, verifiable, lightweight inference.

### Bootstrapping Demand: How Other Projects Solve It

| Approach | Example | How It Works |
|----------|---------|--------------|
| **Protocol-generated tasks** | Helium (proof of coverage) | Network itself generates work to prove validator capability |
| **Seed with own dApps** | Render Network | Build apps that consume inference, creating baseline task flow |
| **Bounty/subsidy program** | Akash Network | Subsidize early task requesters |
| **Synthetic benchmarks** | Many PoW chains | Validators run standard benchmark models to prove capability |

### Recommended Demand Bootstrap Strategy

**Short-term (no code changes):**
- Ship 2-3 reference ONNX models with validator image (MobileNet classifier, sentiment model, anomaly detector)
- Create a simple "inference demo" dApp that posts tasks via RPC
- Use faucet AZTB to fund test inference tasks on friends testnet

**Medium-term (some code needed):**
- Add **heartbeat tasks** — protocol-generated synthetic inference at regular intervals to test validator liveness and capability (like Helium's proof of coverage)
- Build an inference API gateway (REST → PostTask tx) so web2 devs can consume AI without knowing blockchain
- Create inference marketplace page on aztibase.com

**Long-term:**
- Partner with AI model providers to list models in registry
- Build SDKs (Python, JS) that abstract the blockchain: `client.infer(model="sentiment", input=text)`
- This is where real revenue and network value comes from

### Hardware Presale Angle

Two SKUs naturally emerge:
1. **Aztibase Standard Node** ($350-450) — consensus only, lower spec
2. **Aztibase Compute Node** ($500-650) — consensus + AI inference, higher spec with better CPU

The Compute Node commands a premium because it earns both staking rewards AND PoUW inference rewards. This is a strong selling point for the presale — "earn more by doing AI work for the network."

---

## Business in a Box: Non-Technical User Experience

### The Problem — Current Setup Is Too Technical

```
1. Install Rust toolchain          ← "what's a toolchain?"
2. Git clone + cargo build         ← 10-15 min compile, errors on low RAM
3. Generate validator keys (CLI)   ← terminal commands, hex strings
4. Receive genesis.toml            ← "what's TOML?"
5. Start node with 6+ CLI flags    ← terrifying
6. Verify via curl + jq            ← no chance
7. Monitor via Prometheus/Grafana  ← DevOps skills required
```

### The Goal — Plug and Play

```
1. Plug in box, connect ethernet/WiFi
2. Open browser → http://my-aztibase.local (mDNS)
3. See setup wizard: "Welcome to Aztibase"
4. Create account (generates keys behind the scenes)
5. Enter network invite code OR scan QR
6. Box joins network automatically
7. Dashboard shows: status, balance, earnings, peers
```

### Layer 1: Pre-Flashed OS Image (eliminates compile/install)

For prebuilt hardware, ship with a pre-flashed NVMe/SD card:
- Ubuntu Server minimal + Aztibase binary pre-installed
- Systemd service auto-starts on boot
- mDNS (avahi) so the box is reachable at `aztibase.local` from any browser on the LAN
- No terminal, no SSH, no Rust toolchain

### Layer 2: Web Setup Wizard (eliminates CLI key generation and config)

A local web UI served by the node on port 80. This is the biggest missing piece.

| Screen | What the User Sees | What Happens Technically |
|--------|--------------------|--------------------------|
| **Welcome** | "Name your node" text field | Sets `NODE_NAME` in config |
| **Network** | "Enter invite code" (e.g. `AZTB-7K3M-QXPW`) | Decodes to genesis hash + boot node address, downloads genesis.toml |
| **Security** | "Set a PIN" + optional "write down these 24 words" | Generates Ed25519+BLS validator keys, encrypts keyfile with PIN |
| **Connecting** | Progress bar: "Finding peers... Syncing..." | Starts node process, discovers peers via boot nodes |
| **Dashboard** | Live status page | Redirects to dashboard (Layer 3) |

### Layer 3: Built-In Dashboard (eliminates curl/Grafana)

Served from the same web UI. Shows everything a non-technical operator cares about.

| Panel | Data Source (RPC) |
|-------|-------------------|
| Node Status (online / syncing / error) | `aztb_nodeInfo` |
| Block Height | `aztb_blockHeight` |
| Peer Count | `aztb_nodeInfo` → peer_count |
| Your Balance | `aztb_getBalance` |
| Earnings (staking rewards over time) | Track balance deltas locally |
| Network Health | `aztb_getActiveValidators` |
| Uptime | Local timestamp tracking |

### Layer 4: Auto-Updates (eliminates maintenance)

- Systemd timer checks for new releases daily
- Downloads binary, verifies Ed25519 signature, swaps binary, restarts service
- Operator sees "Update available" banner on dashboard, or auto-applies
- No SSH required, ever

### The Invite Code Concept

Instead of sharing genesis.toml files and multiaddr strings, the coordinator generates a short invite code:

```
Invite code: AZTB-7K3M-QXPW
```

Encodes (base32 or similar):
- Genesis hash (to verify correct network)
- Boot node IP:port (or DNS name)
- Optional: network name

The new validator enters this in the web wizard. The box resolves everything automatically.

### Implementation Approach

| Approach | Description | Pros | Cons |
|----------|-------------|------|------|
| **Rust + htmx (recommended)** | Serve HTML from existing axum RPC server | Same binary, no extra deps, ships with node | Limited UI polish |
| **Static HTML + vanilla JS** | Embed minimal HTML/JS in binary | Ultra-lightweight, zero dependencies | Basic styling |
| **React/Vue SPA** | Separate frontend build bundled into binary | Polished UI, responsive | Extra build step |
| **Tauri desktop app** | Native app with system tray | Desktop feel | Not for headless boxes |

**Recommendation:** Rust + htmx or static HTML/JS. The node already runs an axum HTTP server — add a few HTML routes and you have a zero-dependency setup wizard + dashboard. No extra processes, no Docker, no nginx.

### Development Estimate

| Component | Effort | Ship With |
|-----------|--------|-----------|
| Pre-flash OS image (Ubuntu + binary + systemd + mDNS) | 1-2 days | Hardware v1 |
| Web setup wizard (5 screens) | 3-5 days | Hardware v1 |
| Dashboard (status + balance + peers + earnings) | 2-3 days | Hardware v1 |
| Invite code system (encode/decode + resolution) | 1 day | Hardware v1 |
| Auto-updater (signed binary swap) | 1-2 days | Hardware v1.1 |
| Mobile companion app (status monitoring) | 2-4 weeks | Phase 2 |

**Total for v1 "plug and play": ~1-2 weeks of development.**

### What This Enables for Presale Marketing

- "Plug in. Connect WiFi. Enter code. You're earning."
- "No programming. No terminal. No cloud accounts."
- "Monitor your validator from any phone or laptop on your network."
- "Automatic updates — your node stays current without you lifting a finger."

This is the DappNode model but lighter, cheaper, and AI-native.

---

## Sources

- DappNode: dappnode.com
- Beelink: bee-link.com
- Web3 Pi (200 validators on Pi 5): web3pi.io
- Deeper Network Kickstarter: $2.3M raised
- Helium SEC dismissal: April 2025
- XNET IDO: 6 rounds, 40M tokens
- Shenzhen OEM: odmminipc.com, oaipc.com, inctelpc.com
- VaaS: everstake.one, allnodes.com, validationcloud.io
