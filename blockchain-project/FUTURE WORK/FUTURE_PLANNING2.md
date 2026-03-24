# Aztibase Network — Future Planning: AI Sentinel & Infrastructure

**Date:** 2026-03-11
**Status:** SCOPED — Pending testnet launch

---

## 1. AI Sentinel — Chain Self-Monitoring

### Overview

Aztibase is uniquely positioned to be the **first L1 blockchain with protocol-embedded AI self-monitoring**. No production chain currently does this. The existing infrastructure (TractRuntime, AgentExecute, AnomalyScorer, WebSocket subscriptions) provides ~90% of the plumbing.

The AI Sentinel watches chain health metrics in real-time, scores anomalies using a lightweight ONNX model running in the node's own tract runtime, and surfaces alerts to operators and the block explorer dashboard.

### Existing Infrastructure (Already Built)

| Component | Status | Location |
|-----------|--------|----------|
| TractRuntime (ONNX inference) | PROD, 13 tests | `aztibase-runtime/src/tract_runtime.rs` |
| AnomalyScorer (heuristic) | PROD | `aztibase-runtime/src/anomaly.rs` |
| AgentExecute tx + policy limits | PROD, 10 tests | `aztibase-execution/src/agent.rs` |
| ComputeCommitment registry | PROD | `aztibase-consensus/src/pouw.rs` |
| TaskPool + assignment + settlement | PROD | `aztibase-node/src/task_pool.rs` |
| 15 Prometheus metrics | PROD | `aztibase-rpc/src/metrics.rs` |
| WebSocket subscriptions (newHeads, finality) | PROD | `aztibase-rpc/src/server.rs` |
| ModelRegistry (on-chain) | PROD | `aztibase-execution/src/model_registry.rs` |

### The Model — Dense Autoencoder

```
Input [15 metrics] → Dense(15,8) → ReLU → Dense(8,4) → ReLU → Dense(4,8) → ReLU → Dense(8,15)
```

| Spec | Value |
|------|-------|
| Parameters | 371 |
| ONNX size | ~5 KB |
| Inference latency | ~20 microseconds (CPU) |
| Training | Offline in Python on testnet data, export to ONNX |
| Anomaly score | Reconstruction error (MSE) — high = anomaly |
| tract compatible | 100% |

The `AnomalyScorer` in `anomaly.rs` already does heuristic scoring — upgrading to ONNX model inference is a straightforward swap.

### Input Features (15)

**Safe — consensus-derived, verifiable by all validators:**

| # | Feature | Source |
|---|---------|--------|
| 1 | Block height delta (blocks per window) | Block headers |
| 2 | Commit latency (ms, rolling avg) | Block timestamps |
| 3 | Commit latency stddev | Block timestamps |
| 4 | TPS (rolling window) | Block tx counts |
| 5 | TPS acceleration (delta) | Derived from #4 |
| 6 | Base fee | Execution state |
| 7 | Base fee delta | Derived from #6 |
| 8 | Equivocation count (window) | Consensus proofs |
| 9 | Validator set size | Staking store |
| 10 | Total gas used (window) | Block receipts |
| 11 | Block fullness ratio | Block gas / limit |
| 12 | Empty block ratio (window) | Block headers |
| 13 | Time since last finality | Finality certificates |

**Unsafe — locally observed, Tier 1 dashboard only (not for consensus):**

| # | Feature | Risk |
|---|---------|------|
| 14 | Peer count | Byzantine nodes can fake |
| 15 | Mempool size | Node-local, not verifiable |

### Three-Tier Rollout

#### Tier 1: Observer (Low Risk) — Target: Sprint 057-058

Each node runs the model locally every N blocks (~100). Scores are exposed via RPC and WebSocket. No on-chain actions.

```
Every N blocks:
  Metrics collector → Normalize 15 features → tract ONNX inference
                                                      │
                                                 anomaly_score
                                                      │
                            ┌─────────────────────────┼──────────────────┐
                            ▼                         ▼                  ▼
                   aztb_getAnomalyScore     WebSocket push         Node logs
                   (new RPC endpoint)      (new "anomaly" topic)   (tracing)
                            │
                            ▼
                  Explorer dashboard panel
                  (real-time health bar)
```

**Dashboard visualization:**
```
┌─────────────────────────────────────────────┐
│  Chain Health         score: 0.03  [NORMAL] │
│  ████████████████████░░░░  3% anomaly       │
│                                             │
│  Consensus: OK    Execution: OK             │
│  Network: OK      Validators: OK            │
└─────────────────────────────────────────────┘
```

When anomaly score crosses threshold → bar turns yellow/red, WebSocket pushes alert to all connected explorer clients.

**Deliverables:**
- Sentinel service in node binary (metrics collection + tract inference loop)
- `aztb_getAnomalyScore` RPC endpoint
- `anomaly` WebSocket subscription topic
- Explorer health panel (JS)
- Pre-trained ONNX model (from testnet data)

#### Tier 2: Advisor (Medium Risk) — Post-Testnet Validation

AI agent submits governance proposals via `AgentExecute` when anomaly persists for N consecutive windows. Validators still vote — the AI suggests, humans decide.

**Requirements before Tier 2:**
- Minimum 2 weeks of testnet anomaly data with known-good baseline
- False positive rate measured and acceptable (< 1 per day)
- Determinism validated across node platforms (or model quantized to int8)
- AgentPolicy configured with conservative spending limits

#### Tier 3: Autonomous (High Risk) — Post-Mainnet, If Ever

Auto-adjusts chain parameters within tight bounds (e.g., base fee ±5%) without full governance vote. Requires cooldowns, rollback capability, and years of operational trust.

**Do not implement Tier 3 without:**
- 6+ months of Tier 2 operational data
- Formal security audit of the autonomous action paths
- Community governance vote approving autonomous parameter ranges

### Challenges & Mitigations

#### 1. Determinism (CRITICAL for Tier 2+)

**Problem:** `f32` math can produce different results across CPU architectures (x86 vs ARM). If anomaly scores influence on-chain state, validators must agree.

**Mitigation:**
- Tier 1: Not an issue (scores are local-only)
- Tier 2+: Quantize model to `int8`, use only consensus-derived metrics, require supermajority agreement on anomaly state before any proposal

#### 2. Adversarial Input Poisoning

**Problem:** Attacker spams transactions to inflate TPS, then stops — creating a false "TPS crash" anomaly.

**Mitigation:**
- Long rolling windows (100+ blocks)
- Require anomaly persistence across multiple consecutive windows
- Use consensus-derived metrics only (not mempool/peer count)

#### 3. Feedback Loops (Tier 3)

**Problem:** Model detects high latency → proposes reducing block size → smaller blocks cause congestion → model sees congestion as anomaly → cascade.

**Mitigation:**
- Cooldown period: minimum 1000 blocks between AI-triggered proposals
- Maximum adjustment magnitude per epoch
- Rollback capability
- Human-readable proposal descriptions

#### 4. Cold Start

**Problem:** New network has no baseline for "normal."

**Mitigation:**
- Pre-train on testnet data before mainnet launch
- Explicit burn-in period: first 10K blocks = learning mode, no scoring
- Ship with conservative default thresholds

#### 5. Model Transparency

**Problem:** On-chain models must be public for verifiability. Attackers can predict trigger conditions.

**Mitigation:**
- Accept it — this is a feature, not a bug (transparency builds trust)
- Design conservative thresholds
- The model is a tool for validators, not a secret weapon

### Effort Estimate

| Phase | Work | Sprints |
|-------|------|---------|
| Collect testnet metrics (24-48 hrs) | Ops, after testnet launch | 0 (passive) |
| Train autoencoder, export ONNX | Python, offline | 1 session |
| Sentinel service in node binary | Rust: metrics collection + tract loop | 1 sprint |
| RPC endpoint + WS topic | RPC server addition | 0.5 sprint |
| Explorer health panel | JS dashboard update | 0.5 sprint |
| **Total for Tier 1** | | **~2 sprints** |

### Unique Selling Point

> "Aztibase is the first L1 that monitors itself using its own AI layer."

Every other chain uses external Grafana/PagerDuty. Aztibase dog-foods its own AI inference runtime for chain health monitoring. This is a genuine differentiator for marketing, developer mindshare, and validator experience.

---

## 2. Cloud & Infrastructure Planning

### Local Machine Limitations

Current dev machine: i7 7th gen, 8GB RAM, Radeon graphics, 250GB SSD.

**Assessment:** Insufficient for running 3 validator nodes simultaneously. Suitable for single-node development only. The AI Sentinel model (5 KB, 20μs inference) runs fine on this hardware — the bottleneck is disk space and memory for multiple nodes.

### Cloud Recommendations

#### Free Tier Options

| Provider | Plan | Specs | Cost | Notes |
|----------|------|-------|------|-------|
| Oracle Cloud | ARM Ampere A1 | 4 OCPU, 24 GB RAM, 200 GB | $0 forever | Best free option. Availability can be limited. |
| Google Cloud | e2-micro | 0.25 vCPU, 1 GB | $0 (always free) | Too small for validator, OK for monitoring |
| AWS | t3.micro | 2 vCPU, 1 GB | $0 (12 months) | Too small for validator |

#### Budget Options (Recommended)

| Provider | Plan | Specs | Monthly Cost | Region |
|----------|------|-------|-------------|--------|
| Hetzner | CAX21 (ARM) | 4 vCPU, 8 GB, 80 GB | ~€6 (~$7) | EU (Falkenstein, Helsinki) |
| Hetzner | CPX31 (x86) | 4 vCPU, 8 GB, 160 GB | ~€13 (~$15) | EU |
| Contabo | VPS S | 4 vCPU, 8 GB, 200 GB | ~€7 (~$8) | EU, US |
| OVHcloud | B2-15 | 4 vCPU, 15 GB, 100 GB | ~€13 (~$15) | EU, US, APAC |
| Vultr | High Frequency | 4 vCPU, 8 GB, 128 GB | ~$24 | Global |
| DigitalOcean | CPU-Optimized | 4 vCPU, 8 GB, 50 GB | ~$42 | Global |

**Recommendation for testnet (3 seed nodes):**
- 3x Hetzner CAX21 (ARM) = ~$21/month total
- Or 1x Oracle free tier + 2x Hetzner = ~$14/month total
- Rust cross-compiles cleanly to ARM (`aarch64-unknown-linux-gnu`)

### Monitoring — Free Options

#### Grafana Cloud Free Tier (Recommended)

| Spec | Limit |
|------|-------|
| Active metric series | 10,000 |
| Retention | 14 days |
| Users | 3 |
| Dashboards | Unlimited |
| Alerting | Included |
| Cost | $0 forever |

**How it works:** Run Grafana Alloy (lightweight agent) on each node. It scrapes `/metrics` and pushes to Grafana Cloud's managed Prometheus (Mimir) backend. You configure dashboards in the web UI.

**10K series is enough for 3 nodes with 15 custom metrics each** (~500-700 series per node with node_exporter).

**Setup time:** ~15 minutes per node.

#### Alternative: Self-Hosted (VictoriaMetrics + Grafana OSS)

On a $5-7/month VPS:
- VictoriaMetrics (lighter than Prometheus, single binary)
- Grafana OSS
- Unlimited series, months of retention
- You maintain it

#### Zero-Infra Option: UptimeRobot

- Free tier: 50 monitors, 5-minute check interval
- Add HTTP monitors for `https://testnet{1,2,3}.aztibase.com/health`
- Email/Slack alerts on failure
- Setup: 2 minutes

### Recommended Stack

```
Testnet Infrastructure (~$21/month):
├── 3x Hetzner CAX21 (ARM, €6 each) — validator seed nodes
├── Grafana Cloud Free — dashboards + alerting
├── UptimeRobot Free — uptime monitoring
├── Vercel Free — explorer, faucet, landing page
└── Cloudflare Free — DNS
```

---

## 3. Business-in-a-Box Validator Setup

### Vision

A single script that takes a non-technical person from zero to running validator in under 10 minutes. Works on both local machines and fresh cloud VMs.

### Target Command

```bash
curl -sSf https://get.aztibase.com/validator | bash
```

Or for manual:

```bash
git clone https://github.com/aztibase/aztibase.git
cd aztibase
./setup-validator.sh
```

### What the Script Does

1. **Detects environment** (Ubuntu/Debian/macOS/Windows WSL, ARM/x86)
2. **Installs dependencies** (Rust toolchain if missing)
3. **Downloads or builds binary** (pre-built binaries preferred for speed)
4. **Generates validator keys** (with mnemonic backup prompt)
5. **Configures node** (interactive: testnet/mainnet, data directory, RPC port)
6. **Sets up systemd service** (Linux) or launchd (macOS)
7. **Starts the node** and shows health check
8. **Prints summary** with validator address, backup instructions, and dashboard URL

### Pre-Built Binaries (Key for Non-Technical Users)

Building from source takes 5-15 minutes and requires Rust toolchain. For "business in a box" we need pre-built binaries:

| Platform | Target | Priority |
|----------|--------|----------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | HIGH |
| Linux ARM64 | `aarch64-unknown-linux-gnu` | HIGH (Hetzner ARM, Oracle ARM) |
| macOS ARM | `aarch64-apple-darwin` | MEDIUM |
| macOS Intel | `x86_64-apple-darwin` | LOW |
| Windows | `x86_64-pc-windows-msvc` | LOW (WSL preferred) |

**Distribution:** GitHub Releases with checksums. CI/CD pipeline (GitHub Actions) builds all targets on tag push.

### Sprint Scope

The business-in-a-box setup script is a Sprint 059+ deliverable, after:
1. Testnet launch and stabilization (Sprint 057)
2. AI Sentinel Tier 1 (Sprint 058)
3. Pre-built binary CI pipeline

---

## 4. Sequence of Future Work

```
Current: Sprint 056 (state snapshots + mainnet genesis) — COMPLETE, uncommitted

  ┌─ Sprint 057: Testnet launch + cloud deployment
  │    ├── Provision 3 Hetzner CAX21 nodes
  │    ├── Deploy seed node configs
  │    ├── Deploy explorer/faucet to Vercel
  │    ├── Grafana Cloud setup
  │    ├── UptimeRobot monitors
  │    └── Smoke test + public announcement
  │
  ├─ Sprint 058: AI Sentinel Tier 1
  │    ├── Collect 24-48 hrs testnet metrics
  │    ├── Train autoencoder in Python, export ONNX
  │    ├── Sentinel service in node binary
  │    ├── aztb_getAnomalyScore RPC + WS topic
  │    └── Explorer health panel
  │
  ├─ Sprint 059: Business-in-a-Box
  │    ├── Pre-built binary CI pipeline
  │    ├── setup-validator.sh one-click script
  │    ├── Validator onboarding docs
  │    └── External validator testing
  │
  └─ Sprint 060+: AI Sentinel Tier 2 (governance proposals)
       ├── False positive analysis from Tier 1 data
       ├── int8 model quantization for determinism
       ├── AgentPolicy configuration for Sentinel
       └── Governance proposal integration
```

---

## References

- [tract ONNX runtime](https://github.com/sonos/tract) — Pure Rust, used in `aztibase-runtime`
- [OutlierNets paper](https://pmc.ncbi.nlm.nih.gov/articles/PMC8309714/) — Compact autoencoders for on-device anomaly detection (686 params, microsecond latency)
- [Grafana Cloud Free Tier](https://grafana.com/pricing/) — 10K series, 14-day retention, $0
- [Hetzner Cloud](https://www.hetzner.com/cloud/) — ARM VPS from €4/month
- Aztibase existing code: `anomaly.rs`, `tract_runtime.rs`, `agent.rs`, `metrics.rs`, `server.rs`
