---
title: Transaction Types
description: All transaction types supported by the Aztibase Network
---

Aztibase supports 27 transaction types, each identified by a single byte prefix.

## Core Transactions

| Byte | Kind | Gas Limit | Description |
|------|------|-----------|-------------|
| 0x01 | Transfer | 21,000 | Send AZTB between accounts |
| 0x1B | FaucetDrip | 30,000 | Request testnet tokens (testnet only) |

## Smart Contract Transactions

| Byte | Kind | Gas Limit | Description |
|------|------|-----------|-------------|
| 0x02 | ContractDeploy | 100,000 | Deploy a WASM smart contract |
| 0x03 | ContractCall | 50,000 | Call a WASM smart contract |
| 0x04 | EvmDeploy | 100,000 | Deploy an EVM smart contract |
| 0x05 | EvmCall | 50,000 | Call an EVM smart contract |

## AI Transactions

| Byte | Kind | Gas Limit | Description |
|------|------|-----------|-------------|
| 0x06 | AiInfer | 100,000 | Submit AI inference result |
| 0x07 | CreateAgent | 53,000 | Deploy an AI agent |
| 0x08 | RegisterModel | 100,000 | Register an AI model on-chain |
| 0x09 | PostTask | 42,000 | Submit AI inference task |
| 0x0A | SubmitAttestation | 50,000 | Attest to computation result |
| 0x0B | CommitCompute | 75,000 | Commit to process AI task |
| 0x0C | DeregisterCompute | 50,000 | Exit compute market |
| 0x0D | DeregisterModel | 60,000 | Remove model from registry |
| 0x14 | SetAgentPolicy | 60,000 | Configure agent permissions |
| 0x15 | AgentExecute | 80,000 | Agent-initiated execution |

## Governance Transactions

| Byte | Kind | Gas Limit | Description |
|------|------|-----------|-------------|
| 0x0E | CreateProposal | 100,000 | Submit governance proposal |
| 0x0F | CastVote | 40,000 | Vote on active proposal |

## Staking Transactions

| Byte | Kind | Gas Limit | Description |
|------|------|-----------|-------------|
| 0x10 | Stake | 60,000 | Stake AZTB as a validator |
| 0x11 | Unstake | 60,000 | Begin unstaking (21-day unbond) |
| 0x12 | Delegate | 60,000 | Delegate stake to a validator |
| 0x13 | Undelegate | 60,000 | Remove delegation |
| 0x1A | RotateValidatorKey | 60,000 | Rotate validator signing keys |

## Bridge Transactions

| Byte | Kind | Gas Limit | Description |
|------|------|-----------|-------------|
| 0x16 | AnchorL2State | 80,000 | Anchor L2 state root on L1 |
| 0x17 | BridgeDeposit | 50,000 | Deposit to L2 bridge |
| 0x18 | BridgeWithdraw | 70,000 | Withdraw from L2 bridge |
| 0x19 | RegisterL2 | 100,000 | Register new L2 chain |

## Transaction Envelope

All transactions are signed using Ed25519 and wrapped in a standard envelope:

```
SignedTransaction {
    tx_type: u8,          // Type byte from table above
    from: [u8; 32],       // Sender address
    to: [u8; 32],         // Recipient (or zero for deploys)
    value: u128,          // AZTB amount
    nonce: u64,           // Sender's current nonce
    gas_limit: u64,       // Max gas units
    gas_price: u64,       // Price per gas unit
    data: Vec<u8>,        // Type-specific payload
    signature: [u8; 64],  // Ed25519 signature
}
```

## Fee Calculation

```
total_cost = gas_limit × gas_price + value
```

- Gas is escrowed before execution
- Unused gas is refunded after execution
- Base fee adjusts dynamically (EIP-1559 style)
- Target: 15M gas per batch, max: 30M gas per batch
