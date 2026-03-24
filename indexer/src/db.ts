import Database from 'better-sqlite3';
import path from 'path';

export interface IndexedTx {
  hash: string;
  block_number: number;
  tx_index: number;
  tx_type: string;
  from_addr: string;
  to_addr: string;
  value: string;
  gas_used: string;
  status: number;
  timestamp: number;
}

export interface IndexedBlock {
  number: number;
  hash: string;
  parent_hashes: string;
  state_root: string;
  tx_count: number;
  gas_used: number;
  timestamp: number;
  proposer: string;
}

export interface AddressActivity {
  address: string;
  tx_count: number;
  first_seen: number;
  last_seen: number;
}

export class IndexerDb {
  private db: Database.Database;

  constructor(dbPath: string) {
    this.db = new Database(dbPath);
    this.db.pragma('journal_mode = WAL');
    this.db.pragma('synchronous = NORMAL');
    this.migrate();
  }

  private migrate() {
    this.db.exec(`
      CREATE TABLE IF NOT EXISTS blocks (
        number INTEGER PRIMARY KEY,
        hash TEXT NOT NULL UNIQUE,
        parent_hashes TEXT NOT NULL DEFAULT '[]',
        state_root TEXT NOT NULL,
        tx_count INTEGER NOT NULL DEFAULT 0,
        gas_used INTEGER NOT NULL DEFAULT 0,
        timestamp INTEGER NOT NULL DEFAULT 0,
        proposer TEXT NOT NULL DEFAULT ''
      );

      CREATE TABLE IF NOT EXISTS transactions (
        hash TEXT PRIMARY KEY,
        block_number INTEGER NOT NULL,
        tx_index INTEGER NOT NULL DEFAULT 0,
        tx_type TEXT NOT NULL DEFAULT 'Transfer',
        from_addr TEXT NOT NULL,
        to_addr TEXT NOT NULL DEFAULT '',
        value TEXT NOT NULL DEFAULT '0',
        gas_used TEXT NOT NULL DEFAULT '0',
        status INTEGER NOT NULL DEFAULT -1,
        timestamp INTEGER NOT NULL DEFAULT 0
      );

      CREATE TABLE IF NOT EXISTS address_activity (
        address TEXT PRIMARY KEY,
        tx_count INTEGER NOT NULL DEFAULT 0,
        first_seen INTEGER NOT NULL,
        last_seen INTEGER NOT NULL
      );

      CREATE TABLE IF NOT EXISTS meta (
        key TEXT PRIMARY KEY,
        value TEXT NOT NULL
      );

      CREATE INDEX IF NOT EXISTS idx_tx_block ON transactions(block_number);
      CREATE INDEX IF NOT EXISTS idx_tx_from ON transactions(from_addr);
      CREATE INDEX IF NOT EXISTS idx_tx_to ON transactions(to_addr);
      CREATE INDEX IF NOT EXISTS idx_tx_timestamp ON transactions(timestamp DESC);
      CREATE INDEX IF NOT EXISTS idx_blocks_timestamp ON blocks(timestamp DESC);
    `);
  }

  insertBlock(block: IndexedBlock) {
    const stmt = this.db.prepare(`
      INSERT OR REPLACE INTO blocks (number, hash, parent_hashes, state_root, tx_count, gas_used, timestamp, proposer)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)
    `);
    stmt.run(
      block.number, block.hash, block.parent_hashes, block.state_root,
      block.tx_count, block.gas_used, block.timestamp, block.proposer
    );
  }

  insertTx(tx: IndexedTx) {
    const stmt = this.db.prepare(`
      INSERT OR REPLACE INTO transactions (hash, block_number, tx_index, tx_type, from_addr, to_addr, value, gas_used, status, timestamp)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);
    stmt.run(
      tx.hash, tx.block_number, tx.tx_index, tx.tx_type,
      tx.from_addr, tx.to_addr, tx.value, tx.gas_used, tx.status, tx.timestamp
    );

    this.upsertAddress(tx.from_addr, tx.timestamp);
    if (tx.to_addr) this.upsertAddress(tx.to_addr, tx.timestamp);
  }

  private upsertAddress(addr: string, timestamp: number) {
    if (!addr || addr === '--' || addr === '') return;
    const stmt = this.db.prepare(`
      INSERT INTO address_activity (address, tx_count, first_seen, last_seen)
      VALUES (?, 1, ?, ?)
      ON CONFLICT(address) DO UPDATE SET
        tx_count = tx_count + 1,
        last_seen = MAX(last_seen, excluded.last_seen)
    `);
    stmt.run(addr, timestamp, timestamp);
  }

  insertBatch(blocks: IndexedBlock[], txs: IndexedTx[]) {
    const insertAll = this.db.transaction(() => {
      for (const b of blocks) this.insertBlock(b);
      for (const tx of txs) this.insertTx(tx);
    });
    insertAll();
  }

  getMeta(key: string): string | undefined {
    const row = this.db.prepare('SELECT value FROM meta WHERE key = ?').get(key) as { value: string } | undefined;
    return row?.value;
  }

  setMeta(key: string, value: string) {
    this.db.prepare('INSERT OR REPLACE INTO meta (key, value) VALUES (?, ?)').run(key, value);
  }

  getLastIndexedBlock(): number {
    const val = this.getMeta('last_block');
    return val ? parseInt(val, 10) : 0;
  }

  setLastIndexedBlock(n: number) {
    this.setMeta('last_block', String(n));
  }

  getBlock(number: number): IndexedBlock | undefined {
    return this.db.prepare('SELECT * FROM blocks WHERE number = ?').get(number) as IndexedBlock | undefined;
  }

  getBlockByHash(hash: string): IndexedBlock | undefined {
    return this.db.prepare('SELECT * FROM blocks WHERE hash = ?').get(hash) as IndexedBlock | undefined;
  }

  getRecentBlocks(limit: number = 20): IndexedBlock[] {
    return this.db.prepare('SELECT * FROM blocks ORDER BY number DESC LIMIT ?').all(limit) as IndexedBlock[];
  }

  getTx(hash: string): IndexedTx | undefined {
    return this.db.prepare('SELECT * FROM transactions WHERE hash = ?').get(hash) as IndexedTx | undefined;
  }

  getTxsByAddress(address: string, limit: number = 50, offset: number = 0): IndexedTx[] {
    return this.db.prepare(
      'SELECT * FROM transactions WHERE from_addr = ? OR to_addr = ? ORDER BY timestamp DESC LIMIT ? OFFSET ?'
    ).all(address, address, limit, offset) as IndexedTx[];
  }

  getTxCountByAddress(address: string): number {
    const row = this.db.prepare(
      'SELECT COUNT(*) as cnt FROM transactions WHERE from_addr = ? OR to_addr = ?'
    ).get(address, address) as { cnt: number };
    return row.cnt;
  }

  getRecentTxs(limit: number = 50): IndexedTx[] {
    return this.db.prepare('SELECT * FROM transactions ORDER BY timestamp DESC LIMIT ?').all(limit) as IndexedTx[];
  }

  getTxsByBlock(blockNumber: number): IndexedTx[] {
    return this.db.prepare('SELECT * FROM transactions WHERE block_number = ? ORDER BY tx_index').all(blockNumber) as IndexedTx[];
  }

  getStats(): { total_blocks: number; total_txs: number; active_addresses: number; last_block: number } {
    const blocks = (this.db.prepare('SELECT COUNT(*) as cnt FROM blocks').get() as { cnt: number }).cnt;
    const txs = (this.db.prepare('SELECT COUNT(*) as cnt FROM transactions').get() as { cnt: number }).cnt;
    const addrs = (this.db.prepare('SELECT COUNT(*) as cnt FROM address_activity').get() as { cnt: number }).cnt;
    return {
      total_blocks: blocks,
      total_txs: txs,
      active_addresses: addrs,
      last_block: this.getLastIndexedBlock(),
    };
  }

  getTopAddresses(limit: number = 20): AddressActivity[] {
    return this.db.prepare(
      'SELECT * FROM address_activity ORDER BY tx_count DESC LIMIT ?'
    ).all(limit) as AddressActivity[];
  }

  close() {
    this.db.close();
  }
}
