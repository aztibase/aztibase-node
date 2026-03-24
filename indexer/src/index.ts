import path from 'path';
import { IndexerDb } from './db.js';
import { Syncer } from './sync.js';
import { createApi } from './api.js';

const RPC_URL = process.env.AZTB_RPC || 'http://127.0.0.1:9944';
const DB_PATH = process.env.AZTB_DB || path.resolve('indexer.db');
const API_PORT = parseInt(process.env.AZTB_API_PORT || '3001', 10);
const POLL_INTERVAL = parseInt(process.env.AZTB_POLL_MS || '5000', 10);

let rpcId = 0;
let lastCallTime = 0;
const MIN_CALL_GAP_MS = 15;

async function rpcCall<T>(method: string, params: unknown[] = []): Promise<T> {
  const now = Date.now();
  const wait = MIN_CALL_GAP_MS - (now - lastCallTime);
  if (wait > 0) await new Promise(r => setTimeout(r, wait));
  lastCallTime = Date.now();

  for (let attempt = 0; attempt < 3; attempt++) {
    rpcId++;
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 10_000);

    try {
      const res = await fetch(RPC_URL, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ jsonrpc: '2.0', method, params, id: rpcId }),
        signal: controller.signal,
      });
      const json = await res.json() as { result?: T; error?: { message: string } };
      if (json.error) {
        if (json.error.message.includes('rate limit') && attempt < 2) {
          await new Promise(r => setTimeout(r, 1000 * (attempt + 1)));
          continue;
        }
        throw new Error(json.error.message);
      }
      return json.result as T;
    } catch (e) {
      if (attempt === 2) throw e;
      await new Promise(r => setTimeout(r, 500 * (attempt + 1)));
    } finally {
      clearTimeout(timer);
    }
  }
  throw new Error('RPC call failed after retries');
}

const rpc = { call: rpcCall };

async function main() {
  console.log(`[indexer] Aztibase Indexer v0.1.0`);
  console.log(`[indexer] RPC: ${RPC_URL}`);
  console.log(`[indexer] DB: ${DB_PATH}`);
  console.log(`[indexer] API: http://0.0.0.0:${API_PORT}`);

  const db = new IndexerDb(DB_PATH);
  const syncer = new Syncer(rpc, db);
  const api = createApi(db, API_PORT);

  await api.start();

  console.log('[indexer] Starting backfill...');
  await syncer.backfill((indexed, tip) => {
    if (indexed % 1000 === 0 || indexed === tip) {
      const pct = tip > 0 ? ((indexed / tip) * 100).toFixed(1) : '0';
      console.log(`[indexer] Backfill: ${indexed.toLocaleString()} / ${tip.toLocaleString()} (${pct}%)`);
    }
  });

  console.log('[indexer] Starting live follower...');
  syncer.startLiveFollow(POLL_INTERVAL);

  const shutdown = () => {
    console.log('\n[indexer] Shutting down...');
    syncer.stop();
    api.stop();
    db.close();
    process.exit(0);
  };

  process.on('SIGINT', shutdown);
  process.on('SIGTERM', shutdown);
}

main().catch((e) => {
  console.error('[indexer] Fatal:', e);
  process.exit(1);
});
