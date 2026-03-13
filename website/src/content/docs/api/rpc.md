---
title: JSON-RPC API
description: Complete API reference for Aztibase nodes
---

Aztibase nodes expose a JSON-RPC 2.0 API over HTTP and WebSocket.

## Connection

| Transport | URL |
|-----------|-----|
| HTTP | `http://<node>:9944/` |
| WebSocket | `ws://<node>:9944/ws` |

**Rate limit:** 100 requests/second per IP

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

**Parameters:** `[address]` — `0x` + 64 hex chars

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
**Returns:** `"0x{code}"` or `null`

### aztb_getAccountType

Returns the account classification.

**Parameters:** `[address]`
**Returns:** `"EOA"` | `"Contract"` | `"Agent"`

---

## Transaction Methods

### aztb_sendTransaction

Submit a signed transaction to the mempool.

**Parameters:** `[tx_hex]` — `0x` + hex-encoded signed transaction bytes
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
  "transactions": ["0x...", "..."],
  "stateRoot": "0x..."
}
```

### aztb_getBlockByHash

Retrieve a block by its hash.

**Parameters:** `[hash]`
**Returns:** Block object or `null`

### aztb_getBlockRange

Retrieve multiple blocks by range.

**Parameters:** `[start, end]` — max range of 100

---

## Consensus Methods

### aztb_consensusState

Returns current consensus status.

**Returns:**
```json
{
  "round": 42,
  "wave": 10,
  "validators": 3,
  "committed_batches": 500
}
```

### aztb_getValidators

Returns the current validator set.

### aztb_getValidatorStake

Returns stake information for a specific validator.

**Parameters:** `[validator_address]`

---

## Network Methods

### aztb_nodeInfo

Returns node information including peer count and version.

### aztb_peerCount

Returns the number of connected peers.

### aztb_chainId

Returns the chain ID.
**Returns:** `"0xa27b"` (41595)

### aztb_blockHeight

Alias for `aztb_blockNumber`.

---

## Faucet Methods (Testnet Only)

### aztb_faucetDrip

Request testnet tokens.

**Parameters:** `[address]`
**Returns:** Transaction hash

**Drip amount:** 1,000,000 AZTB
**Cooldown:** 60 seconds

---

## Subscription Methods (WebSocket)

### aztb_subscribe

Subscribe to real-time events.

**Topics:**
- `newBlocks` — New committed blocks
- `newTransactions` — New transactions in mempool
- `consensusUpdates` — Consensus state changes

```json
{
  "jsonrpc": "2.0",
  "method": "aztb_subscribe",
  "params": ["newBlocks"],
  "id": 1
}
```

### aztb_unsubscribe

Unsubscribe from an event stream.

**Parameters:** `[subscription_id]`

---

## WebSocket Limits

| Parameter | Value |
|-----------|-------|
| Max connections | 256 |
| Max per IP | 8 |
| Max subscriptions per client | 16 |

## Error Codes

| Code | Meaning |
|------|---------|
| -32700 | Parse error |
| -32600 | Invalid request |
| -32601 | Method not found |
| -32602 | Invalid params |
| -32603 | Internal error |
