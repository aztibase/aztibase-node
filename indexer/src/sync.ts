import type { IndexerDb, IndexedBlock, IndexedTx } from './db.js';

const TX_TYPE_MAP: Record<number, string> = {
  0x01: 'Transfer', 0x02: 'ContractDeploy', 0x03: 'ContractCall',
  0x04: 'EvmDeploy', 0x05: 'EvmCall', 0x06: 'AiInfer',
  0x07: 'CreateAgent', 0x08: 'RegisterModel', 0x09: 'PostTask',
  0x0a: 'SubmitAttestation', 0x0b: 'CommitCompute', 0x0c: 'DeregisterCompute',
  0x0d: 'DeregisterModel', 0x0e: 'CreateProposal', 0x0f: 'CastVote',
  0x10: 'Stake', 0x11: 'Unstake', 0x12: 'Delegate', 0x13: 'Undelegate',
  0x14: 'SetAgentPolicy', 0x15: 'AgentExecute', 0x16: 'AnchorL2State',
  0x17: 'BridgeDeposit', 0x18: 'BridgeWithdraw', 0x19: 'RegisterL2',
  0x1a: 'RotateValidatorKey', 0x1b: 'FaucetDrip',
};

function parseTxType(raw: unknown): string {
  if (typeof raw === 'string' && TX_TYPE_MAP[parseInt(raw, 16)]) {
    return TX_TYPE_MAP[parseInt(raw, 16)];
  }
  if (typeof raw === 'number' && TX_TYPE_MAP[raw]) {
    return TX_TYPE_MAP[raw];
  }
  if (typeof raw === 'string' && raw.length > 2) return raw;
  return 'Unknown';
}

function parseHexInt(v: string | number): number {
  if (typeof v === 'number') return v;
  if (typeof v === 'string' && v.startsWith('0x')) return parseInt(v, 16);
  return Number(v) || 0;
}

interface RpcCaller {
  call<T>(method: string, params?: unknown[]): Promise<T>;
}

export class Syncer {
  private rpc: RpcCaller;
  private db: IndexerDb;
  private running = false;

  constructor(rpc: RpcCaller, db: IndexerDb) {
    this.rpc = rpc;
    this.db = db;
  }

  async backfill(onProgress?: (indexed: number, tip: number) => void): Promise<void> {
    const tipHex = await this.rpc.call<string>('aztb_blockNumber');
    const tip = parseHexInt(tipHex);
    let cursor = this.db.getLastIndexedBlock();

    if (cursor >= tip) {
      onProgress?.(cursor, tip);
      return;
    }

    console.log(`[sync] Backfilling from block ${cursor} to ${tip} (${tip - cursor} blocks)`);

    while (cursor < tip) {
      const from = cursor + 1;
      const to = Math.min(cursor + 50, tip);
      const rawBlocks = await this.rpc.call<Record<string, unknown>[]>('aztb_getBlockRange', [from, to]);

      if (!rawBlocks || rawBlocks.length === 0) {
        cursor = to;
        continue;
      }

      const blocks: IndexedBlock[] = [];
      const txs: IndexedTx[] = [];

      for (const raw of rawBlocks) {
        const num = parseHexInt(raw.number as string | number);
        const txHashes = (raw.transactions || []) as string[];

        blocks.push({
          number: num,
          hash: (raw.hash as string) || '',
          parent_hashes: JSON.stringify(raw.parentHashes || raw.parent_hashes || []),
          state_root: (raw.stateRoot || raw.state_root || '') as string,
          tx_count: txHashes.length,
          gas_used: parseHexInt((raw.gasUsed || raw.gas_used || 0) as string | number),
          timestamp: parseHexInt((raw.timestamp || 0) as string | number),
          proposer: (raw.proposer || '') as string,
        });

        for (let i = 0; i < txHashes.length; i++) {
          const txHash = txHashes[i];
          try {
            const txData = await this.rpc.call<Record<string, unknown>>('aztb_getTransactionByHash', [txHash]);
            if (!txData) continue;
            const receipt = (txData.receipt || {}) as Record<string, unknown>;
            txs.push({
              hash: txHash,
              block_number: num,
              tx_index: i,
              tx_type: parseTxType(txData.type || txData.tx_type || txData.kind),
              from_addr: (txData.from || txData.sender || '') as string,
              to_addr: (txData.to || txData.recipient || '') as string,
              value: String(txData.value || 0),
              gas_used: String(receipt.gasUsed || txData.gas_used || txData.gas_limit || 0),
              status: receipt.success != null ? (receipt.success ? 1 : 0) : -1,
              timestamp: parseHexInt((raw.timestamp || 0) as string | number),
            });
          } catch {
            // skip failed tx fetches
          }
        }
      }

      this.db.insertBatch(blocks, txs);
      cursor = to;
      this.db.setLastIndexedBlock(cursor);
      onProgress?.(cursor, tip);
    }

    console.log(`[sync] Backfill complete at block ${cursor}`);
  }

  processBlock(raw: Record<string, unknown>, txDataList: Record<string, unknown>[]): void {
    const num = parseHexInt(raw.number as string | number);
    const txHashes = (raw.transactions || []) as string[];

    const block: IndexedBlock = {
      number: num,
      hash: (raw.hash as string) || '',
      parent_hashes: JSON.stringify(raw.parentHashes || raw.parent_hashes || []),
      state_root: (raw.stateRoot || raw.state_root || '') as string,
      tx_count: txHashes.length,
      gas_used: parseHexInt((raw.gasUsed || raw.gas_used || 0) as string | number),
      timestamp: parseHexInt((raw.timestamp || 0) as string | number),
      proposer: (raw.proposer || '') as string,
    };

    const txs: IndexedTx[] = txDataList.map((txData, i) => {
      const receipt = (txData.receipt || {}) as Record<string, unknown>;
      return {
        hash: (txData.hash || txHashes[i] || '') as string,
        block_number: num,
        tx_index: i,
        tx_type: parseTxType(txData.type || txData.tx_type || txData.kind),
        from_addr: (txData.from || txData.sender || '') as string,
        to_addr: (txData.to || txData.recipient || '') as string,
        value: String(txData.value || 0),
        gas_used: String(receipt.gasUsed || txData.gas_used || txData.gas_limit || 0),
        status: receipt.success != null ? (receipt.success ? 1 : 0) : -1,
        timestamp: block.timestamp,
      };
    });

    this.db.insertBatch([block], txs);
    this.db.setLastIndexedBlock(num);
  }

  async startLiveFollow(pollIntervalMs: number = 5000): Promise<void> {
    this.running = true;
    console.log('[sync] Live follower started');

    while (this.running) {
      try {
        const tipHex = await this.rpc.call<string>('aztb_blockNumber');
        const tip = parseHexInt(tipHex);
        const last = this.db.getLastIndexedBlock();

        if (tip > last) {
          const from = last + 1;
          const to = Math.min(last + 50, tip);
          const rawBlocks = await this.rpc.call<Record<string, unknown>[]>('aztb_getBlockRange', [from, to]);

          if (rawBlocks && rawBlocks.length > 0) {
            const blocks: IndexedBlock[] = [];
            const allTxs: IndexedTx[] = [];

            for (const raw of rawBlocks) {
              const num = parseHexInt(raw.number as string | number);
              const txHashes = (raw.transactions || []) as string[];

              blocks.push({
                number: num,
                hash: (raw.hash as string) || '',
                parent_hashes: JSON.stringify(raw.parentHashes || raw.parent_hashes || []),
                state_root: (raw.stateRoot || raw.state_root || '') as string,
                tx_count: txHashes.length,
                gas_used: parseHexInt((raw.gasUsed || raw.gas_used || 0) as string | number),
                timestamp: parseHexInt((raw.timestamp || 0) as string | number),
                proposer: (raw.proposer || '') as string,
              });

              const BATCH = 10;
              for (let i = 0; i < txHashes.length; i += BATCH) {
                const batch = txHashes.slice(i, i + BATCH);
                const results = await Promise.all(
                  batch.map(h => this.rpc.call<Record<string, unknown>>('aztb_getTransactionByHash', [h]).catch(() => null))
                );
                for (let j = 0; j < results.length; j++) {
                  const txData = results[j];
                  if (!txData) continue;
                  const receipt = (txData.receipt || {}) as Record<string, unknown>;
                  allTxs.push({
                    hash: batch[j],
                    block_number: num,
                    tx_index: i + j,
                    tx_type: parseTxType(txData.type || txData.tx_type || txData.kind),
                    from_addr: (txData.from || txData.sender || '') as string,
                    to_addr: (txData.to || txData.recipient || '') as string,
                    value: String(txData.value || 0),
                    gas_used: String(receipt.gasUsed || txData.gas_used || txData.gas_limit || 0),
                    status: receipt.success != null ? (receipt.success ? 1 : 0) : -1,
                    timestamp: parseHexInt((raw.timestamp || 0) as string | number),
                  });
                }
              }
            }

            this.db.insertBatch(blocks, allTxs);
            this.db.setLastIndexedBlock(to);
            if (allTxs.length > 0) {
              console.log(`[sync] Indexed blocks ${from}-${to} (${allTxs.length} txs)`);
            }
          }
        }
      } catch (e) {
        console.error('[sync] Poll error:', (e as Error).message);
      }

      await new Promise(r => setTimeout(r, pollIntervalMs));
    }
  }

  stop() {
    this.running = false;
  }
}
