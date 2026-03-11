# Aztibase Network — JSON-RPC API Reference

**Transport:** HTTP POST to `http://<node>:9944/` or WebSocket `ws://<node>:9944/ws`
**Protocol:** JSON-RPC 2.0
**Chain ID:** `0xa27b` (41595)

---

## Request Format

```json
{
  "jsonrpc": "2.0",
  "method": "aztb_<methodName>",
  "params": [...],
  "id": 1
}
```

All addresses and hashes are 32-byte hex strings prefixed with `0x`.

---

## Account Methods

### aztb_getBalance

Returns the balance of an account.

**Parameters:** `[address]`
- `address` (string) — `0x` + 64 hex chars

**Returns:** `"0x{balance}"` — hex-encoded u128

```bash
curl -X POST http://localhost:9944 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"aztb_getBalance","params":["0x0102..."],"id":1}'
```

### aztb_getNonce

Returns the transaction nonce for an account.

**Parameters:** `[address]`

**Returns:** `nonce` (u64)

### aztb_getCode

Returns deployed contract bytecode.

**Parameters:** `[address]`

**Returns:** `"0x{code}"` or `null` if no code deployed

### aztb_getAccountType

Returns the account classification.

**Parameters:** `[address]`

**Returns:** `"EOA"` | `"Contract"` | `"Agent"`

---

## Transaction Methods

### aztb_sendTransaction

Submit a signed transaction to the mempool.

**Parameters:** `[tx_hex]`
- `tx_hex` (string) — `0x` + hex-encoded signed transaction bytes

**Returns:** `"0x{tx_hash}"` — 32-byte transaction hash

### aztb_getTransactionReceipt

Get the execution receipt for a transaction.

**Parameters:** `[tx_hash]`

**Returns:** Receipt object or `null`

```json
{
  "txHash": "0x...",
  "success": true,
  "gasUsed": "0x5208",
  "contractAddress": "0x...",
  "error": null,
  "inferenceHash": null
}
```

### aztb_getTransactionByHash

Retrieve a stored transaction by hash.

**Parameters:** `[tx_hash]`

**Returns:** Transaction object or `null`

```json
{
  "hash": "0x...",
  "raw": "0x...",
  "receipt": { ... }
}
```

---

## Block Methods

### aztb_blockNumber

Returns the current batch/block height.

**Parameters:** none

**Returns:** `"0x{height}"` — hex-encoded u64

### aztb_getBlockByNumber

Retrieve a block by its number.

**Parameters:** `[number]` — u64 or hex string

**Returns:** Block object or `null`

```json
{
  "number": "0x...",
  "hash": "0x...",
  "transactions": ["0x...", ...],
  "stateRoot": "0x..."
}
```

### aztb_getBlockByHash

Retrieve a block by its hash.

**Parameters:** `[hash]`

**Returns:** Block object or `null`

### aztb_getBlockRange

Retrieve a range of blocks (max 100).

**Parameters:** `[from, to]` — u64 values

**Returns:** Array of `{ "number": "0x...", "hash": "0x..." }`

### aztb_getBlockTransactionCount

Returns the number of transactions in a block.

**Parameters:** `[number]` — u64 block number

**Returns:** Integer tx count, or `null` if block not found

```bash
curl -X POST http://localhost:9944 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"aztb_getBlockTransactionCount","params":[42],"id":1}'
```

### aztb_sendRawTransaction

Submit a signed transaction envelope (alias for `aztb_sendTransaction`).

**Parameters:** `[hex_encoded_envelope]` — hex string of the signed tx envelope

**Returns:** `"ok"` on success

```bash
curl -X POST http://localhost:9944 -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"aztb_sendRawTransaction","params":["aa..."],"id":1}'
```

### aztb_getBatchRoot

Get the state root for a specific batch.

**Parameters:** `[batch_hash]`

**Returns:** `"0x{root}"` or `null`

### aztb_getTransactionsByBatch

List all transaction hashes in a batch.

**Parameters:** `[batch_hash]`

**Returns:** Array of tx hash strings or `null`

### aztb_getReceiptsByBatch

List all receipts in a batch.

**Parameters:** `[batch_hash]`

**Returns:** Array of receipt objects or `null`

---

## Chain Methods

### aztb_chainId

Returns the chain identifier.

**Parameters:** none

**Returns:** `"0xa27b"`

### aztb_genesisHash

Returns the genesis block hash.

**Parameters:** none

**Returns:** `"0x{hash}"`

### aztb_getStateRoot

Returns the current state root.

**Parameters:** none

**Returns:** `"0x{root}"`

### aztb_gasPrice

Returns the current base fee.

**Parameters:** none

**Returns:** `"0x{fee}"` — hex-encoded u128

### aztb_estimateGas

Estimate gas for a transaction type.

**Parameters:** `[prefix]` — `0x` + 1-byte tx type prefix

**Returns:** `"0x{estimate}"` — hex-encoded gas estimate

**Tx Type Prefixes:**
- `0x01` — Transfer (21,000 gas)
- `0x02` — Deploy
- `0x03` — Call
- `0x04` — EVM Deploy
- `0x05` — EVM Call
- `0x06` — Stake
- `0x07` — AI Inference

### aztb_nodeInfo

Returns node metadata.

**Parameters:** none

**Returns:**

```json
{
  "version": "0.1.0",
  "chainId": "0xa27b",
  "blockHeight": 1234,
  "protocolVersion": "aztb/1"
}
```

---

## AI Compute Methods

### aztb_getModelInfo

Get metadata for a registered AI model.

**Parameters:** `[model_id]` — string

**Returns:** Model object or `null`

```json
{
  "modelId": "sentiment_v1",
  "owner": "0x...",
  "fingerprint": "0x...",
  "computeCost": 1000,
  "minStake": 50000,
  "registeredRound": 42,
  "active": true
}
```

### aztb_listModels

List all registered models.

**Parameters:** none

**Returns:** Array of model objects

### aztb_getTaskStatus

Get the status of an AI compute task.

**Parameters:** `[task_id]` — 32-byte hex

**Returns:** Task object or `null`

```json
{
  "taskId": "0x...",
  "modelId": "sentiment_v1",
  "inputHash": "0x...",
  "requester": "0x...",
  "reward": "0x...",
  "deadlineRound": 100,
  "status": "pending",
  "assignedValidator": "0x..."
}
```

### aztb_pendingTaskCount

Count of tasks awaiting execution.

**Parameters:** none

**Returns:** `count` (u64)

### aztb_getComputeCommitment

Get a validator's compute commitment.

**Parameters:** `[validator_id]` — 32-byte hex

**Returns:** Commitment object or `null`

```json
{
  "validatorId": "0x...",
  "supportedModels": ["sentiment_v1", ...],
  "committedStake": 100000,
  "registeredRound": 10,
  "active": true
}
```

### aztb_listComputeProviders

List validators supporting a specific model.

**Parameters:** `[model_id]` — string

**Returns:** Array of validator address strings

---

## Governance Methods

### aztb_getProposal

Get a governance proposal by ID.

**Parameters:** `[proposal_id]` — 32-byte hex

**Returns:** Proposal object or `null`

```json
{
  "id": "0x...",
  "proposer": "0x...",
  "description": "Increase epoch length",
  "paramKey": "epoch_length",
  "paramValue": 2000,
  "startRound": 500,
  "endRound": 1500,
  "status": "Active",
  "approveWeight": "0x...",
  "rejectWeight": "0x...",
  "voterCount": 3
}
```

### aztb_listProposals

List governance proposals, optionally filtered by status.

**Parameters:** `[status]` (optional) — `"Active"` | `"Passed"` | `"Rejected"` | `"Executed"`

**Returns:** Array of proposal objects

### aztb_getChainParam

Get a governance-controlled chain parameter.

**Parameters:** `[key]` — string (e.g., `"base_fee_floor"`, `"epoch_length"`)

**Returns:** Parameter object or `null`

```json
{
  "key": "base_fee_floor",
  "value": "1000",
  "type": "U128",
  "description": "Minimum base fee"
}
```

### aztb_listChainParams

List all chain parameters.

**Parameters:** none

**Returns:** Array of parameter objects

---

## Tokenomics Methods

### aztb_getEmissionInfo

Get current token emission state.

**Parameters:** none

**Returns:**

```json
{
  "current_epoch": 5,
  "total_emitted": "15000000",
  "total_supply_in_existence": "415000000",
  "remaining_emission": "585000000",
  "hard_cap": "1000000000",
  "genesis_mint": "400000000",
  "epoch_length": 1000,
  "treasury_balance": "...",
  "insurance_balance": "..."
}
```

### aztb_getVestingStatus

Get vesting schedule for a genesis allocation category.

**Parameters:** `[category]` — string (e.g., `"core_team"`, `"ecosystem_dev"`, `"liquidity"`)

**Returns:** Vesting object or error

```json
{
  "category": "core_team",
  "total": "120000000",
  "vested": "0",
  "locked": "120000000",
  "cliff_end_round": 26280000,
  "end_round": 105120000,
  "description": "Core team allocation"
}
```

---

## Staking Methods

### aztb_getValidatorStake

Get staking info for a validator.

**Parameters:** `[validator_id]` — 32-byte hex

**Returns:** Validator stake object

```json
{
  "validator_id": "0x...",
  "self_stake": 100000,
  "total_delegated": 50000,
  "effective_stake": 150000,
  "active": true,
  "registered_round": 10,
  "slash_history": []
}
```

### aztb_getDelegation

Get delegation info for a delegator.

**Parameters:** `[delegator_address]` — 32-byte hex

**Returns:** Delegation object or `null`

```json
{
  "validator_id": "0x...",
  "amount": 25000,
  "round_delegated": 50
}
```

### aztb_getActiveValidators

List all active validators sorted by stake.

**Parameters:** none

**Returns:** Array of validator objects

### aztb_getUnbondingStatus

Check unbonding entries for an address.

**Parameters:** `[address]` — 32-byte hex

**Returns:** Array of unbonding entries

```json
[
  { "amount": 10000, "available_round": 2000 }
]
```

---

## Agent Methods

### aztb_getAgentPolicy

Get the spending policy for an AI agent account.

**Parameters:** `[agent_address]` — 32-byte hex

**Returns:** Policy object or `null`

```json
{
  "owner": "0x...",
  "per_tx_limit": "100000000",
  "per_epoch_limit": "10000000000",
  "allowed_tx_kinds": ["0x01", "0x07"],
  "expiry_epoch": 100
}
```

---

## Checkpoint Methods

### aztb_getCheckpoint

Get checkpoint data at a specific batch index.

**Parameters:** `[batch_index]` — u64

**Returns:** Checkpoint object or `null`

```json
{
  "batch_index": 1000,
  "state_root": "0x...",
  "has_finality_cert": true,
  "timestamp": 1710072000
}
```

### aztb_latestCheckpoint

Get the most recent checkpoint.

**Parameters:** none

**Returns:** Checkpoint object or `null`

---

## Faucet

### aztb_faucetDrip

Request testnet tokens (testnet only, rate-limited).

**Parameters:** `[address]` — 32-byte hex

**Returns:**

```json
{
  "address": "0x...",
  "amount": 1000000,
  "balance": 1000000
}
```

---

## WebSocket Subscriptions

Connect via `ws://<node>:9944/ws`

### aztb_subscribe

Subscribe to real-time events.

**Parameters:** `[topic]` — `"newHeads"` | `"finality"`

**Returns:** `"0x{subscription_id}"`

**Notification format:**

```json
{
  "jsonrpc": "2.0",
  "method": "aztb_subscription",
  "params": {
    "subscription": "0x1",
    "result": { ... }
  }
}
```

### aztb_unsubscribe

Unsubscribe from events.

**Parameters:** `[subscription_id]`

**Returns:** `true` if removed, `false` if not found

---

## L2 Bridge

### aztb_getL2State

Get the latest anchored L2 state for a registered L2 chain.

**Parameters:** `[l2_chain_id]` — 32-byte hex chain ID

**Returns:** `{ state_root, block_range, sequencer, batch_index, finalized }` or `null` if L2 not registered / no anchors

---

### aztb_listL2s

List all registered L2 chains.

**Parameters:** none

**Returns:** `[ { l2_chain_id, name, sequencer_set, bridge_address } ]`

---

### aztb_getBridgeBalance

Get locked AZTB balance in bridge escrow for an account on a specific L2.

**Parameters:** `[l2_chain_id, account]` — both 32-byte hex

**Returns:** `{ locked: string }` (u128 as decimal string)

---

### aztb_getBridgeProofStatus

Check whether a withdrawal proof hash has already been used.

**Parameters:** `[proof_hash]` — 32-byte hex

**Returns:** `{ used: bool }`

---

## Error Codes

| Code | Meaning |
|------|---------|
| -32600 | Invalid request |
| -32601 | Method not found |
| -32602 | Invalid params |
| -32603 | Internal error |
| -32000 | Server error (custom) |
