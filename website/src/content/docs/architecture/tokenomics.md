---
title: Tokenomics
description: AZTB token economics, emission schedule, and staking model
---

The AZTB token powers the Aztibase Network — used for gas fees, staking, governance, and AI compute payments.

## Supply

| Parameter | Value |
|-----------|-------|
| Total supply | 1,000,000,000 AZTB |
| Genesis mint | 400,000,000 (40%) |
| Emission pool | 600,000,000 (60%) |

## Emission Schedule

Emissions follow a halving schedule over ~10 years:

| Period | Annual Emission |
|--------|----------------|
| Years 1-2 | 120,000,000 AZTB |
| Years 3-4 | 60,000,000 AZTB |
| Years 5-6 | 30,000,000 AZTB |
| Years 7-8 | 15,000,000 AZTB |
| Years 9-10 | 7,500,000 AZTB |
| Year 11+ | 3,750,000 AZTB/year (tail emission) |

Tail emission ensures perpetual validator incentives without inflating beyond the emission pool.

## Emission Distribution

| Recipient | Share | Purpose |
|-----------|-------|---------|
| Validators | 70% | Block production and consensus participation |
| PoUW (AI compute) | 15% | Proof of Useful Work AI inference providers |
| Treasury | 10% | Protocol development and grants |
| Insurance | 5% | Slashing insurance and emergency fund |

## Gas & Fee Model

Aztibase uses an EIP-1559-style dynamic fee mechanism:

| Parameter | Value |
|-----------|-------|
| Min base fee | 1 |
| Max base fee | 1,000,000,000 |
| Target gas per batch | 15,000,000 |
| Max gas per batch | 30,000,000 |
| Base fee change denominator | 8 |

**Fee formula:** `total_cost = gas_limit × gas_price + value`

Unused gas is refunded after execution. Validator fees are collected from consumed gas.

## Transaction Gas Costs

| Transaction Type | Gas Limit |
|-----------------|-----------|
| Transfer | 21,000 |
| FaucetDrip | 30,000 |
| CastVote | 40,000 |
| PostTask | 42,000 |
| ContractCall / EvmCall | 50,000 |
| CreateAgent | 53,000 |
| Stake / Unstake / Delegate | 60,000 |
| CommitCompute | 75,000 |
| AgentExecute | 80,000 |
| AnchorL2State | 80,000 |
| ContractDeploy / EvmDeploy | 100,000 |
| RegisterModel / CreateProposal | 100,000 |

## Staking

| Parameter | Value |
|-----------|-------|
| Minimum stake | 50,000 AZTB |
| Minimum stake floor (governance) | 10,000 AZTB |
| Maximum stake | 50,000,000 AZTB (5% of supply) |
| Unbonding period | ~21 days |
| Default commission | 10% |
| APY range | 3-12% |

### Slashing

| Violation | Penalty |
|-----------|---------|
| Equivocation (double-voting) | 10% of stake |
| Downtime (1,000+ rounds) | 0.5% of stake |

## Governance

AZTB holders can participate in on-chain governance:

| Parameter | Value |
|-----------|-------|
| Min voting period | 10 rounds |
| Max voting period | 10,000 rounds |
| Min voters for quorum | 2 |
| Max active proposals | 64 |

Governance can adjust staking parameters within defined bounds (e.g., minimum stake between 10,000 and 500,000 AZTB).
