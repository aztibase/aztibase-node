import { describe, it, expect, vi, beforeEach } from 'vitest';
import { AztibaseClient } from '../src/client.js';
import { AztibaseRpcError } from '../src/rpc.js';
import { TxKind } from '../src/types.js';

const mockFetch = vi.fn();
global.fetch = mockFetch;

function mockRpcResponse(result: unknown) {
  mockFetch.mockResolvedValueOnce({
    ok: true,
    json: async () => ({ jsonrpc: '2.0', result, id: 1 }),
  });
}

function mockRpcError(code: number, message: string) {
  mockFetch.mockResolvedValueOnce({
    ok: true,
    json: async () => ({ jsonrpc: '2.0', error: { code, message }, id: 1 }),
  });
}

describe('AztibaseClient', () => {
  let client: AztibaseClient;

  beforeEach(() => {
    mockFetch.mockReset();
    client = new AztibaseClient('http://localhost:9944');
  });

  it('creates from string URL', () => {
    const c = new AztibaseClient('http://localhost:9944');
    expect(c).toBeInstanceOf(AztibaseClient);
  });

  it('creates from options object', () => {
    const c = new AztibaseClient({ url: 'http://localhost:9944', timeout: 5000 });
    expect(c).toBeInstanceOf(AztibaseClient);
  });

  describe('getBalance', () => {
    it('parses hex balance to bigint', async () => {
      mockRpcResponse('0xf4240');
      const balance = await client.getBalance('0x' + '00'.repeat(32));
      expect(balance).toBe(1_000_000n);
    });

    it('handles zero balance', async () => {
      mockRpcResponse('0x0');
      const balance = await client.getBalance('0x' + '00'.repeat(32));
      expect(balance).toBe(0n);
    });
  });

  describe('getNonce', () => {
    it('parses hex nonce', async () => {
      mockRpcResponse('0xa');
      const nonce = await client.getNonce('0x' + '00'.repeat(32));
      expect(nonce).toBe(10);
    });

    it('handles numeric nonce', async () => {
      mockRpcResponse(5);
      const nonce = await client.getNonce('0x' + '00'.repeat(32));
      expect(nonce).toBe(5);
    });
  });

  describe('getBlockNumber', () => {
    it('parses hex height', async () => {
      mockRpcResponse('0x434b6');
      const height = await client.getBlockNumber();
      expect(height).toBe(275_638);
    });
  });

  describe('getBlockByNumber', () => {
    it('normalizes block fields', async () => {
      mockRpcResponse({
        number: '0x1',
        hash: '0xabc',
        stateRoot: '0xdef',
        timestamp: 1000,
        transactions: ['0xtx1', '0xtx2'],
        gasUsed: '0x5208',
        proposer: '0xval1',
      });
      const block = await client.getBlockByNumber(1);
      expect(block).not.toBeNull();
      expect(block!.number).toBe(1);
      expect(block!.hash).toBe('0xabc');
      expect(block!.txCount).toBe(2);
      expect(block!.gasUsed).toBe(21000);
    });

    it('returns null for missing block', async () => {
      mockRpcResponse(null);
      const block = await client.getBlockByNumber(999999);
      expect(block).toBeNull();
    });
  });

  describe('getBlockRange', () => {
    it('returns normalized blocks', async () => {
      mockRpcResponse([
        { number: 1, hash: '0xa', stateRoot: '0xb', timestamp: 100, transactions: [] },
        { number: 2, hash: '0xc', stateRoot: '0xd', timestamp: 200, transactions: ['0xtx'] },
      ]);
      const blocks = await client.getBlockRange(1, 2);
      expect(blocks).toHaveLength(2);
      expect(blocks[0].number).toBe(1);
      expect(blocks[1].txCount).toBe(1);
    });

    it('handles empty range', async () => {
      mockRpcResponse([]);
      const blocks = await client.getBlockRange(100, 100);
      expect(blocks).toHaveLength(0);
    });
  });

  describe('sendTransaction', () => {
    it('returns tx hash', async () => {
      mockRpcResponse('0x' + 'ab'.repeat(32));
      const hash = await client.sendTransaction('0xsignedtx');
      expect(hash).toBe('0x' + 'ab'.repeat(32));
    });
  });

  describe('getTransactionReceipt', () => {
    it('returns receipt', async () => {
      mockRpcResponse({
        txHash: '0xabc',
        success: true,
        gasUsed: 21000,
        contractAddress: null,
        error: null,
        inferenceHash: null,
      });
      const receipt = await client.getTransactionReceipt('0xabc');
      expect(receipt).not.toBeNull();
      expect(receipt!.success).toBe(true);
    });

    it('returns null for pending tx', async () => {
      mockRpcResponse(null);
      const receipt = await client.getTransactionReceipt('0xabc');
      expect(receipt).toBeNull();
    });
  });

  describe('getChainHealth', () => {
    it('returns health data', async () => {
      mockRpcResponse({ score: 95, level: 'normal', batchHeight: 1000 });
      const health = await client.getChainHealth();
      expect(health).not.toBeNull();
      expect(health!.level).toBe('normal');
    });
  });

  describe('faucetDrip', () => {
    it('returns tx hash', async () => {
      mockRpcResponse('0xdriphash');
      const hash = await client.faucetDrip('0x' + '11'.repeat(32));
      expect(hash).toBe('0xdriphash');
    });
  });

  describe('waitForTransaction', () => {
    it('resolves when receipt appears', async () => {
      mockRpcResponse(null);
      mockRpcResponse({ txHash: '0xabc', success: true, gasUsed: 21000, contractAddress: null, error: null, inferenceHash: null });

      const receipt = await client.waitForTransaction('0xabc', { interval: 50, timeout: 5000 });
      expect(receipt.success).toBe(true);
    });

    it('rejects on timeout', async () => {
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => ({ jsonrpc: '2.0', result: null, id: 1 }),
      });

      await expect(
        client.waitForTransaction('0xabc', { interval: 50, timeout: 200 })
      ).rejects.toThrow('not confirmed');
    });
  });

  describe('error handling', () => {
    it('throws AztibaseRpcError on RPC error', async () => {
      mockRpcError(-32601, 'Method not found');
      await expect(client.getBlockNumber()).rejects.toThrow(AztibaseRpcError);

      mockRpcError(-32601, 'Method not found');
      await expect(client.getBlockNumber()).rejects.toThrow('Method not found');
    });

    it('throws on HTTP error', async () => {
      mockFetch.mockResolvedValueOnce({ ok: false, status: 500, statusText: 'Internal Server Error' });
      await expect(client.getBlockNumber()).rejects.toThrow('HTTP 500');
    });
  });
});

describe('TxKind', () => {
  it('has correct byte values', () => {
    expect(TxKind.Transfer).toBe(0x01);
    expect(TxKind.EvmDeploy).toBe(0x04);
    expect(TxKind.Stake).toBe(0x10);
    expect(TxKind.BridgeDeposit).toBe(0x17);
    expect(TxKind.FaucetDrip).toBe(0x1b);
  });
});
