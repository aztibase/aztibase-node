import { RpcClient } from './rpc.js';
import type {
  Address,
  Hash,
  Hex,
  Block,
  Transaction,
  TransactionReceipt,
  Validator,
  ConsensusState,
  NodeInfo,
  ChainHealth,
  SentinelAction,
  EpochSummary,
  ClientOptions,
} from './types.js';

function parseHexInt(value: string | number): number {
  if (typeof value === 'number') return value;
  if (typeof value === 'string' && value.startsWith('0x')) return parseInt(value, 16);
  return Number(value);
}

function parseHexBigInt(value: string | number | bigint): bigint {
  if (typeof value === 'bigint') return value;
  if (typeof value === 'string' && value.startsWith('0x')) return BigInt(value);
  return BigInt(value);
}

function normalizeBlock(raw: Record<string, unknown>): Block {
  return {
    number: parseHexInt(raw.number as string | number),
    hash: raw.hash as Hash,
    parentHashes: (raw.parentHashes ?? raw.parent_hashes ?? []) as Hash[],
    stateRoot: (raw.stateRoot ?? raw.state_root) as Hash,
    timestamp: parseHexInt(raw.timestamp as string | number),
    transactions: (raw.transactions ?? []) as Hash[],
    txCount: (raw.tx_count ?? raw.txCount ?? (raw.transactions as unknown[])?.length ?? 0) as number,
    gasUsed: parseHexInt((raw.gasUsed ?? raw.gas_used ?? 0) as string | number),
    gasLimit: parseHexInt((raw.gasLimit ?? raw.gas_limit ?? 0) as string | number),
    proposer: (raw.proposer ?? '0x') as Address,
  };
}

export class AztibaseClient {
  private rpc: RpcClient;

  constructor(options: ClientOptions | string) {
    const opts = typeof options === 'string' ? { url: options } : options;
    this.rpc = new RpcClient(opts.url, opts.timeout);
  }

  // ── Account Methods ──────────────────────────────────────────────

  async getBalance(address: Address): Promise<bigint> {
    const result = await this.rpc.call<string>('aztb_getBalance', [address]);
    return parseHexBigInt(result);
  }

  async getNonce(address: Address): Promise<number> {
    const result = await this.rpc.call<string | number>('aztb_getNonce', [address]);
    return parseHexInt(result);
  }

  async getCode(address: Address): Promise<Hex | null> {
    return this.rpc.call<Hex | null>('aztb_getCode', [address]);
  }

  async getAccountType(address: Address): Promise<string> {
    return this.rpc.call<string>('aztb_getAccountType', [address]);
  }

  // ── Transaction Methods ──────────────────────────────────────────

  async sendTransaction(signedTxHex: Hex): Promise<Hash> {
    return this.rpc.call<Hash>('aztb_sendTransaction', [signedTxHex]);
  }

  async getTransactionReceipt(txHash: Hash): Promise<TransactionReceipt | null> {
    return this.rpc.call<TransactionReceipt | null>('aztb_getTransactionReceipt', [txHash]);
  }

  async getTransactionByHash(txHash: Hash): Promise<Transaction | null> {
    return this.rpc.call<Transaction | null>('aztb_getTransactionByHash', [txHash]);
  }

  async estimateGas(params: {
    from: Address;
    to?: Address;
    data?: Hex;
    value?: bigint;
    gas?: number;
  }): Promise<number> {
    const callParams: Record<string, string> = { from: params.from };
    if (params.to) callParams.to = params.to;
    if (params.data) callParams.data = params.data;
    if (params.value != null) callParams.value = '0x' + params.value.toString(16);
    if (params.gas != null) callParams.gas = '0x' + params.gas.toString(16);

    const result = await this.rpc.call<string>('aztb_estimateGas', [callParams]);
    return parseHexInt(result);
  }

  // ── Block Methods ────────────────────────────────────────────────

  async getBlockNumber(): Promise<number> {
    const result = await this.rpc.call<string>('aztb_blockNumber');
    return parseHexInt(result);
  }

  async getBlockByNumber(blockNumber: number): Promise<Block | null> {
    const raw = await this.rpc.call<Record<string, unknown> | null>(
      'aztb_getBlockByNumber',
      [blockNumber]
    );
    return raw ? normalizeBlock(raw) : null;
  }

  async getBlockByHash(hash: Hash): Promise<Block | null> {
    const raw = await this.rpc.call<Record<string, unknown> | null>(
      'aztb_getBlockByHash',
      [hash]
    );
    return raw ? normalizeBlock(raw) : null;
  }

  async getBlockRange(start: number, end: number): Promise<Block[]> {
    const raw = await this.rpc.call<Record<string, unknown>[]>(
      'aztb_getBlockRange',
      [start, end]
    );
    return (raw ?? []).map(normalizeBlock);
  }

  // ── Consensus Methods ────────────────────────────────────────────

  async getConsensusState(): Promise<ConsensusState> {
    return this.rpc.call<ConsensusState>('aztb_consensusState');
  }

  async getValidators(): Promise<Validator[]> {
    return this.rpc.call<Validator[]>('aztb_getValidators');
  }

  async getActiveValidators(): Promise<Validator[]> {
    return this.rpc.call<Validator[]>('aztb_getActiveValidators');
  }

  async getValidatorStake(address: Address): Promise<unknown> {
    return this.rpc.call('aztb_getValidatorStake', [address]);
  }

  // ── Network Methods ──────────────────────────────────────────────

  async getNodeInfo(): Promise<NodeInfo> {
    return this.rpc.call<NodeInfo>('aztb_nodeInfo');
  }

  async getPeerCount(): Promise<number> {
    return this.rpc.call<number>('aztb_peerCount');
  }

  async getChainId(): Promise<Hex> {
    return this.rpc.call<Hex>('aztb_chainId');
  }

  async getGasPrice(): Promise<number> {
    const result = await this.rpc.call<string | number>('aztb_gasPrice');
    return parseHexInt(result);
  }

  // ── Sentinel / Health Methods ────────────────────────────────────

  async getChainHealth(): Promise<ChainHealth | null> {
    return this.rpc.call<ChainHealth | null>('aztb_getChainHealth');
  }

  async getSentinelActions(limit: number = 5): Promise<SentinelAction[]> {
    return this.rpc.call<SentinelAction[]>('aztb_getSentinelActions', [limit]);
  }

  async getEpochSummaries(limit: number = 5): Promise<EpochSummary[]> {
    return this.rpc.call<EpochSummary[]>('aztb_getEpochSummaries', [limit]);
  }

  // ── Faucet (Testnet) ─────────────────────────────────────────────

  async faucetDrip(address: Address): Promise<Hash> {
    return this.rpc.call<Hash>('aztb_faucetDrip', [address]);
  }

  // ── Metrics ──────────────────────────────────────────────────────

  async getMetrics(): Promise<Record<string, unknown>> {
    return this.rpc.fetchMetrics();
  }

  // ── Utility ──────────────────────────────────────────────────────

  async waitForTransaction(
    txHash: Hash,
    options: { timeout?: number; interval?: number } = {}
  ): Promise<TransactionReceipt> {
    const timeout = options.timeout ?? 30_000;
    const interval = options.interval ?? 1_000;
    const start = Date.now();

    while (Date.now() - start < timeout) {
      const receipt = await this.getTransactionReceipt(txHash);
      if (receipt) return receipt;
      await new Promise(resolve => setTimeout(resolve, interval));
    }

    throw new Error(`Transaction ${txHash} not confirmed within ${timeout}ms`);
  }
}
