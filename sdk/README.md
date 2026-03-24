# @aztibase/sdk

TypeScript SDK for the Aztibase Network.

## Install

```bash
npm install @aztibase/sdk
```

## Quick Start

```typescript
import { AztibaseClient } from '@aztibase/sdk';

const client = new AztibaseClient('http://102.209.21.247:9944');

// Get block height
const height = await client.getBlockNumber();

// Get balance
const balance = await client.getBalance('0x...');

// Get a block
const block = await client.getBlockByNumber(1000);

// Wait for a transaction
const receipt = await client.waitForTransaction('0x...');
```

## WebSocket Subscriptions

```typescript
import { AztibaseWs } from '@aztibase/sdk';

const ws = new AztibaseWs('ws://102.209.21.247:9944/ws');
await ws.connect();

// Subscribe to new blocks
const unsub = await ws.subscribe('newBlocks', (block) => {
  console.log('New block:', block.number);
});

// Unsubscribe
await unsub();

// Close
ws.close();
```

## API

### AztibaseClient

| Method | Returns |
|--------|---------|
| `getBalance(address)` | `bigint` |
| `getNonce(address)` | `number` |
| `getCode(address)` | `Hex \| null` |
| `getAccountType(address)` | `string` |
| `sendTransaction(signedTxHex)` | `Hash` |
| `getTransactionReceipt(txHash)` | `TransactionReceipt \| null` |
| `getTransactionByHash(txHash)` | `Transaction \| null` |
| `estimateGas(params)` | `number` |
| `getBlockNumber()` | `number` |
| `getBlockByNumber(n)` | `Block \| null` |
| `getBlockByHash(hash)` | `Block \| null` |
| `getBlockRange(start, end)` | `Block[]` |
| `getConsensusState()` | `ConsensusState` |
| `getValidators()` | `Validator[]` |
| `getActiveValidators()` | `Validator[]` |
| `getNodeInfo()` | `NodeInfo` |
| `getPeerCount()` | `number` |
| `getChainId()` | `Hex` |
| `getGasPrice()` | `number` |
| `getChainHealth()` | `ChainHealth \| null` |
| `getSentinelActions(limit)` | `SentinelAction[]` |
| `getEpochSummaries(limit)` | `EpochSummary[]` |
| `faucetDrip(address)` | `Hash` |
| `getMetrics()` | `Record<string, unknown>` |
| `waitForTransaction(txHash, opts)` | `TransactionReceipt` |

### AztibaseWs

| Method | Returns |
|--------|---------|
| `connect()` | `void` |
| `subscribe(topic, callback)` | `() => Promise<void>` (unsubscribe fn) |
| `close()` | `void` |

Topics: `newBlocks`, `newTransactions`, `consensusUpdates`, `chainHealth`

## License

MIT
