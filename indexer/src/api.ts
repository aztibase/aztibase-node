import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { readFileSync } from 'fs';
import Fastify from 'fastify';
import type { IndexerDb } from './db.js';

export function createApi(db: IndexerDb, port: number = 3001) {
  const app = Fastify({ logger: false });

  app.addHook('onRequest', async (request, reply) => {
    reply.header('Access-Control-Allow-Origin', '*');
    reply.header('Access-Control-Allow-Methods', 'GET, OPTIONS');
    reply.header('Access-Control-Allow-Headers', 'Content-Type');
    if (request.method === 'OPTIONS') {
      reply.status(204).send();
    }
  });

  app.get('/', async (_request, reply) => {
    const htmlPath = join(dirname(fileURLToPath(import.meta.url)), '..', 'public', 'index.html');
    const html = readFileSync(htmlPath, 'utf-8');
    reply.type('text/html').send(html);
  });

  app.get('/api', async () => ({
    service: 'aztibase-indexer',
    version: '0.1.0',
    endpoints: ['/stats','/blocks/recent','/block/:number','/txs/recent','/tx/:hash','/address/:addr/txs','/addresses/top','/search/:query'],
  }));

  app.get('/health', async () => ({ status: 'ok', service: 'aztibase-indexer' }));

  app.get('/stats', async () => db.getStats());

  app.get<{ Params: { hash: string } }>('/tx/:hash', async (request, reply) => {
    const tx = db.getTx(request.params.hash);
    if (!tx) return reply.status(404).send({ error: 'Transaction not found' });
    return tx;
  });

  app.get<{ Params: { number: string } }>('/block/:number', async (request, reply) => {
    const num = parseInt(request.params.number, 10);
    if (isNaN(num)) return reply.status(400).send({ error: 'Invalid block number' });
    const block = db.getBlock(num);
    if (!block) return reply.status(404).send({ error: 'Block not found' });
    const txs = db.getTxsByBlock(num);
    return { ...block, transactions: txs };
  });

  app.get<{ Params: { hash: string } }>('/block/hash/:hash', async (request, reply) => {
    const block = db.getBlockByHash(request.params.hash);
    if (!block) return reply.status(404).send({ error: 'Block not found' });
    const txs = db.getTxsByBlock(block.number);
    return { ...block, transactions: txs };
  });

  app.get('/blocks/recent', async (request) => {
    const limit = Math.min(parseInt((request.query as any).limit || '20', 10), 100);
    return db.getRecentBlocks(limit);
  });

  app.get<{ Params: { address: string } }>('/address/:address/txs', async (request) => {
    const { address } = request.params;
    const query = request.query as any;
    const limit = Math.min(parseInt(query.limit || '50', 10), 200);
    const offset = parseInt(query.offset || '0', 10);
    const txs = db.getTxsByAddress(address, limit, offset);
    const total = db.getTxCountByAddress(address);
    return { address, total, limit, offset, transactions: txs };
  });

  app.get('/txs/recent', async (request) => {
    const limit = Math.min(parseInt((request.query as any).limit || '50', 10), 200);
    return db.getRecentTxs(limit);
  });

  app.get('/addresses/top', async (request) => {
    const limit = Math.min(parseInt((request.query as any).limit || '20', 10), 100);
    return db.getTopAddresses(limit);
  });

  app.get<{ Params: { query: string } }>('/search/:query', async (request, reply) => {
    const q = request.params.query;

    if (/^\d+$/.test(q)) {
      const block = db.getBlock(parseInt(q, 10));
      if (block) return { type: 'block', data: block };
    }

    const tx = db.getTx(q);
    if (tx) return { type: 'transaction', data: tx };

    const block = db.getBlockByHash(q);
    if (block) return { type: 'block', data: block };

    if (q.length === 66 || q.length === 64) {
      const addr = q.startsWith('0x') ? q : '0x' + q;
      const txCount = db.getTxCountByAddress(addr);
      if (txCount > 0) return { type: 'address', data: { address: addr, tx_count: txCount } };
    }

    return reply.status(404).send({ error: 'Not found' });
  });

  return {
    start: async () => {
      await app.listen({ port, host: '0.0.0.0' });
      console.log(`[api] Indexer API listening on http://0.0.0.0:${port}`);
    },
    stop: async () => {
      await app.close();
    },
  };
}
