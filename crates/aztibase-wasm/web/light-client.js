/**
 * Aztibase Light Client — Browser Transport Bridge
 *
 * Connects to an Aztibase full node via WebSocket, syncs headers
 * using the WASM verification module, and caches them in IndexedDB.
 */

const LIGHT_SYNC_VERSION = 1;
const MAX_HEADERS_PER_REQUEST = 100;
const IDB_NAME = "aztibase_light";
const IDB_VERSION = 1;
const HEADERS_STORE = "headers";
const META_STORE = "meta";

class HeaderCache {
  constructor() {
    this.db = null;
  }

  async open() {
    return new Promise((resolve, reject) => {
      const req = indexedDB.open(IDB_NAME, IDB_VERSION);
      req.onupgradeneeded = (e) => {
        const db = e.target.result;
        if (!db.objectStoreNames.contains(HEADERS_STORE)) {
          db.createObjectStore(HEADERS_STORE, { keyPath: "round" });
        }
        if (!db.objectStoreNames.contains(META_STORE)) {
          db.createObjectStore(META_STORE, { keyPath: "key" });
        }
      };
      req.onsuccess = (e) => {
        this.db = e.target.result;
        resolve(this);
      };
      req.onerror = () => reject(req.error);
    });
  }

  async storeHeaders(headers) {
    if (!this.db || headers.length === 0) return;
    const tx = this.db.transaction(HEADERS_STORE, "readwrite");
    const store = tx.objectStore(HEADERS_STORE);
    for (const h of headers) {
      store.put(h);
    }
    return new Promise((resolve, reject) => {
      tx.oncomplete = resolve;
      tx.onerror = () => reject(tx.error);
    });
  }

  async getHeader(round) {
    if (!this.db) return null;
    const tx = this.db.transaction(HEADERS_STORE, "readonly");
    const store = tx.objectStore(HEADERS_STORE);
    return new Promise((resolve, reject) => {
      const req = store.get(round);
      req.onsuccess = () => resolve(req.result || null);
      req.onerror = () => reject(req.error);
    });
  }

  async getLastSyncedRound() {
    if (!this.db) return 0;
    const tx = this.db.transaction(META_STORE, "readonly");
    const store = tx.objectStore(META_STORE);
    return new Promise((resolve, reject) => {
      const req = store.get("last_synced_round");
      req.onsuccess = () => resolve(req.result ? req.result.value : 0);
      req.onerror = () => reject(req.error);
    });
  }

  async setLastSyncedRound(round) {
    if (!this.db) return;
    const tx = this.db.transaction(META_STORE, "readwrite");
    const store = tx.objectStore(META_STORE);
    store.put({ key: "last_synced_round", value: round });
    return new Promise((resolve, reject) => {
      tx.oncomplete = resolve;
      tx.onerror = () => reject(tx.error);
    });
  }

  async headerCount() {
    if (!this.db) return 0;
    const tx = this.db.transaction(HEADERS_STORE, "readonly");
    const store = tx.objectStore(HEADERS_STORE);
    return new Promise((resolve, reject) => {
      const req = store.count();
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error);
    });
  }
}

class LightClient {
  constructor(wasmModule) {
    this.wasm = wasmModule;
    this.ws = null;
    this.cache = new HeaderCache();
    this.lastSyncedRound = 0;
    this.targetRound = 0;
    this.connected = false;
    this.onSync = null;
    this.onError = null;
    this.pendingResolve = null;
  }

  async connect(wsUrl) {
    await this.cache.open();
    this.lastSyncedRound = await this.cache.getLastSyncedRound();

    return new Promise((resolve, reject) => {
      this.ws = new WebSocket(wsUrl);
      this.ws.binaryType = "arraybuffer";

      this.ws.onopen = () => {
        this.connected = true;
        resolve();
      };

      this.ws.onmessage = (event) => {
        this._handleMessage(event.data);
      };

      this.ws.onerror = (err) => {
        if (this.onError) this.onError(err);
        if (!this.connected) reject(err);
      };

      this.ws.onclose = () => {
        this.connected = false;
      };
    });
  }

  async sync() {
    if (!this.connected) throw new Error("Not connected");

    const from = this.lastSyncedRound + 1;
    const count = Math.min(MAX_HEADERS_PER_REQUEST, this.targetRound - this.lastSyncedRound);
    if (count <= 0) return { synced: 0, round: this.lastSyncedRound };

    const request = JSON.stringify({
      type: "RequestHeaders",
      version: LIGHT_SYNC_VERSION,
      from_round: from,
      count: count,
    });

    this.ws.send(request);

    return new Promise((resolve) => {
      this.pendingResolve = resolve;
    });
  }

  async _handleMessage(data) {
    let msg;
    if (data instanceof ArrayBuffer) {
      msg = JSON.parse(new TextDecoder().decode(data));
    } else {
      msg = JSON.parse(data);
    }

    if (msg.type === "ResponseHeaders") {
      await this._handleHeaderResponse(msg);
    } else if (msg.type === "TargetRound") {
      this.targetRound = msg.round;
    }
  }

  async _handleHeaderResponse(msg) {
    const headers = msg.headers || [];
    const cert = msg.finality_cert;

    if (headers.length === 0) {
      if (this.pendingResolve) {
        this.pendingResolve({ synced: 0, round: this.lastSyncedRound });
        this.pendingResolve = null;
      }
      return;
    }

    if (cert) {
      const result = this.wasm.verifyHeaderChain(
        JSON.stringify(headers),
        JSON.stringify(cert),
        this.lastSyncedRound + 1
      );

      if (result !== true) {
        const error = `Header chain verification failed: ${result}`;
        if (this.onError) this.onError(new Error(error));
        if (this.pendingResolve) {
          this.pendingResolve({ synced: 0, round: this.lastSyncedRound, error });
          this.pendingResolve = null;
        }
        return;
      }
    }

    await this.cache.storeHeaders(headers);
    const newRound = headers[headers.length - 1].round;
    this.lastSyncedRound = newRound;
    await this.cache.setLastSyncedRound(newRound);

    if (this.onSync) {
      this.onSync({ synced: headers.length, round: newRound });
    }

    if (this.pendingResolve) {
      this.pendingResolve({ synced: headers.length, round: newRound });
      this.pendingResolve = null;
    }
  }

  verifyProof(proofJson, leafHex) {
    return this.wasm.verifyLightClientProof(proofJson, leafHex);
  }

  async getBalance(address) {
    if (!this.connected) throw new Error("Not connected");
    this.ws.send(JSON.stringify({
      type: "RequestBalance",
      address: address,
    }));
    return new Promise((resolve) => {
      const handler = (event) => {
        let msg;
        if (event.data instanceof ArrayBuffer) {
          msg = JSON.parse(new TextDecoder().decode(event.data));
        } else {
          msg = JSON.parse(event.data);
        }
        if (msg.type === "BalanceResponse" && msg.address === address) {
          this.ws.removeEventListener("message", handler);
          resolve(msg.balance);
        }
      };
      this.ws.addEventListener("message", handler);
    });
  }

  disconnect() {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
      this.connected = false;
    }
  }

  get synced() {
    return this.lastSyncedRound;
  }

  get target() {
    return this.targetRound;
  }

  get needsSync() {
    return this.lastSyncedRound < this.targetRound;
  }
}

if (typeof module !== "undefined" && module.exports) {
  module.exports = { LightClient, HeaderCache };
}
