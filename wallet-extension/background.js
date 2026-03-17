// Aztibase Wallet — background service worker (vault + signer)

let vault = { secret: null, address: null, privKey: null, publicKey: null };
let rpcUrl = "https://rpc.aztibase.com";

const GOOGLE_CLIENT_ID = "REPLACE_WITH_YOUR_GOOGLE_CLIENT_ID.apps.googleusercontent.com";
const GITHUB_CLIENT_ID = "REPLACE_WITH_YOUR_GITHUB_CLIENT_ID";
const GITHUB_CLIENT_SECRET = "REPLACE_WITH_YOUR_GITHUB_CLIENT_SECRET";

// Restore session on service worker wake-up
chrome.storage.session?.get(["walletSession"], (result) => {
  if (result?.walletSession) {
    vault.secret = result.walletSession.secret;
    vault.address = result.walletSession.address;
  }
});

chrome.storage.local.get(["network"], (result) => {
  if (result?.network?.rpc) rpcUrl = result.network.rpc;
});

chrome.runtime.onInstalled.addListener(() => {
  chrome.storage.local.get("network", (result) => {
    if (!result.network) {
      chrome.storage.local.set({
        network: { name: "Testnet", rpc: "https://rpc.aztibase.com", chainId: "0xA27B" },
      });
    }
  });
});

// ── BLAKE3 (pure JS, single-chunk ≤1024 bytes) ──────────────────

const BLAKE3 = (() => {
  const IV = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
              0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
  const PERM = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];
  const CHUNK_START = 1, CHUNK_END = 2, ROOT = 8;

  function rotr(x, n) { return ((x >>> n) | (x << (32 - n))) >>> 0; }

  function g(s, a, b, c, d, mx, my) {
    s[a] = (s[a] + s[b] + mx) >>> 0; s[d] = rotr(s[d] ^ s[a], 16);
    s[c] = (s[c] + s[d]) >>> 0;      s[b] = rotr(s[b] ^ s[c], 12);
    s[a] = (s[a] + s[b] + my) >>> 0; s[d] = rotr(s[d] ^ s[a], 8);
    s[c] = (s[c] + s[d]) >>> 0;      s[b] = rotr(s[b] ^ s[c], 7);
  }

  function compress(cv, block, counter, blockLen, flags) {
    const s = [
      cv[0], cv[1], cv[2], cv[3], cv[4], cv[5], cv[6], cv[7],
      IV[0], IV[1], IV[2], IV[3],
      counter & 0xffffffff, (counter / 0x100000000) >>> 0, blockLen, flags,
    ];
    let m = block.slice();
    for (let r = 0; r < 7; r++) {
      g(s, 0, 4, 8, 12, m[0], m[1]);   g(s, 1, 5, 9, 13, m[2], m[3]);
      g(s, 2, 6, 10, 14, m[4], m[5]);  g(s, 3, 7, 11, 15, m[6], m[7]);
      g(s, 0, 5, 10, 15, m[8], m[9]);  g(s, 1, 6, 11, 12, m[10], m[11]);
      g(s, 2, 7, 8, 13, m[12], m[13]); g(s, 3, 4, 9, 14, m[14], m[15]);
      const p = new Array(16);
      for (let i = 0; i < 16; i++) p[i] = m[PERM[i]];
      m = p;
    }
    for (let i = 0; i < 8; i++) { s[i] ^= s[i + 8]; s[i + 8] ^= cv[i]; }
    return s;
  }

  function hash(input) {
    const data = input instanceof Uint8Array ? input : new TextEncoder().encode(input);
    let cv = IV.slice();
    const numBlocks = Math.max(1, Math.ceil(data.length / 64));
    for (let i = 0; i < numBlocks; i++) {
      const blk = new Uint8Array(64);
      const off = i * 64, rem = Math.min(64, data.length - off);
      if (rem > 0) blk.set(data.subarray(off, off + rem));
      const w = new Array(16);
      const dv = new DataView(blk.buffer);
      for (let j = 0; j < 16; j++) w[j] = dv.getUint32(j * 4, true);
      let flags = 0;
      if (i === 0) flags |= CHUNK_START;
      if (i === numBlocks - 1) flags |= CHUNK_END | ROOT;
      const res = compress(cv, w, 0, rem, flags);
      cv = res.slice(0, 8);
    }
    const out = new Uint8Array(32);
    const odv = new DataView(out.buffer);
    for (let i = 0; i < 8; i++) odv.setUint32(i * 4, cv[i], true);
    return out;
  }

  return { hash };
})();

// ── Hex / base64url helpers ──────────────────────────────────────

function bytesToHex(buf) {
  return Array.from(buf).map((b) => b.toString(16).padStart(2, "0")).join("");
}

function hexToBytes(hex) {
  const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
  const out = new Uint8Array(clean.length / 2);
  for (let i = 0; i < out.length; i++) out[i] = parseInt(clean.substr(i * 2, 2), 16);
  return out;
}

function base64urlToBytes(b64) {
  const str = b64.replace(/-/g, "+").replace(/_/g, "/");
  const bin = atob(str);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
  return out;
}

// ── Ed25519 via Web Crypto API (Chrome 113+) ─────────────────────

async function ensureKeyPair() {
  if (vault.privKey && vault.publicKey) return;
  if (!vault.secret) throw new Error("Wallet locked");

  const seed = hexToBytes(vault.secret);
  const pkcs8Prefix = new Uint8Array([
    48, 46, 2, 1, 0, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32,
  ]);
  const pkcs8 = new Uint8Array(pkcs8Prefix.length + 32);
  pkcs8.set(pkcs8Prefix);
  pkcs8.set(seed, pkcs8Prefix.length);

  vault.privKey = await crypto.subtle.importKey(
    "pkcs8", pkcs8, { name: "Ed25519" }, true, ["sign"]
  );

  const jwk = await crypto.subtle.exportKey("jwk", vault.privKey);
  vault.publicKey = base64urlToBytes(jwk.x);
}

async function ed25519Sign(message) {
  await ensureKeyPair();
  const sig = await crypto.subtle.sign("Ed25519", vault.privKey, message);
  return new Uint8Array(sig);
}

// ── Postcard varint (unsigned LEB128) ────────────────────────────

function encodeVarint(value) {
  const bytes = [];
  let v = BigInt(value);
  do {
    let byte = Number(v & 0x7fn);
    v >>= 7n;
    if (v > 0n) byte |= 0x80;
    bytes.push(byte);
  } while (v > 0n);
  return new Uint8Array(bytes);
}

// ── Transaction encoding ─────────────────────────────────────────

const TX_DOMAIN = new TextEncoder().encode("AZTB_TX_V1");
const ENVELOPE_MAGIC = 0xaa;
const PREFIX_TRANSFER = 0x01;
const PREFIX_STAKE = 0x10;
const PREFIX_UNSTAKE = 0x11;

function encodeTransferPayload(from32, to32, value, nonce, gasPrice) {
  const variantIdx = encodeVarint(0);
  const valEnc = encodeVarint(value);
  const nonceEnc = encodeVarint(nonce);
  const gasPriceEnc = encodeVarint(gasPrice);
  const enumPayload = new Uint8Array(
    variantIdx.length + 32 + 32 + valEnc.length + nonceEnc.length + gasPriceEnc.length
  );
  let off = 0;
  enumPayload.set(variantIdx, off); off += variantIdx.length;
  enumPayload.set(from32, off); off += 32;
  enumPayload.set(to32, off); off += 32;
  enumPayload.set(valEnc, off); off += valEnc.length;
  enumPayload.set(nonceEnc, off); off += nonceEnc.length;
  enumPayload.set(gasPriceEnc, off);

  const payload = new Uint8Array(1 + enumPayload.length);
  payload[0] = PREFIX_TRANSFER;
  payload.set(enumPayload, 1);
  return payload;
}

function encodeStakePayload(from32, value, nonce, gasPrice) {
  const valEnc = encodeVarint(value);
  const nonceEnc = encodeVarint(nonce);
  const gasPriceEnc = encodeVarint(gasPrice);
  const inner = new Uint8Array(32 + valEnc.length + nonceEnc.length + gasPriceEnc.length);
  let off = 0;
  inner.set(from32, off); off += 32;
  inner.set(valEnc, off); off += valEnc.length;
  inner.set(nonceEnc, off); off += nonceEnc.length;
  inner.set(gasPriceEnc, off);

  const payload = new Uint8Array(1 + inner.length);
  payload[0] = PREFIX_STAKE;
  payload.set(inner, 1);
  return payload;
}

function encodeUnstakePayload(from32, value, nonce, gasPrice) {
  const valEnc = encodeVarint(value);
  const nonceEnc = encodeVarint(nonce);
  const gasPriceEnc = encodeVarint(gasPrice);
  const inner = new Uint8Array(32 + valEnc.length + nonceEnc.length + gasPriceEnc.length);
  let off = 0;
  inner.set(from32, off); off += 32;
  inner.set(valEnc, off); off += valEnc.length;
  inner.set(nonceEnc, off); off += nonceEnc.length;
  inner.set(gasPriceEnc, off);

  const payload = new Uint8Array(1 + inner.length);
  payload[0] = PREFIX_UNSTAKE;
  payload.set(inner, 1);
  return payload;
}

function buildEnvelope(payload, publicKey, signature) {
  const payloadLen = payload.length;
  const buf = new Uint8Array(1 + 4 + payloadLen + 32 + 64);
  buf[0] = ENVELOPE_MAGIC;
  buf[1] = payloadLen & 0xff;
  buf[2] = (payloadLen >> 8) & 0xff;
  buf[3] = (payloadLen >> 16) & 0xff;
  buf[4] = (payloadLen >> 24) & 0xff;
  buf.set(payload, 5);
  buf.set(publicKey, 5 + payloadLen);
  buf.set(signature, 5 + payloadLen + 32);
  return buf;
}

async function signPayload(payload) {
  await ensureKeyPair();
  const msg = new Uint8Array(TX_DOMAIN.length + payload.length);
  msg.set(TX_DOMAIN);
  msg.set(payload, TX_DOMAIN.length);
  const sig = await ed25519Sign(msg);
  return buildEnvelope(payload, vault.publicKey, sig);
}

// ── RPC ──────────────────────────────────────────────────────────

async function rpc(method, params) {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), 8000);
  try {
    const resp = await fetch(rpcUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ jsonrpc: "2.0", method, params, id: Date.now() }),
      signal: ctrl.signal,
    });
    clearTimeout(timer);
    const text = await resp.text();
    let json;
    try { json = JSON.parse(text); } catch {
      throw new Error("RPC returned non-JSON response — check endpoint URL");
    }
    if (json.error) throw new Error(json.error.message || JSON.stringify(json.error));
    return json.result;
  } catch (e) {
    clearTimeout(timer);
    if (e.name === "AbortError") throw new Error("RPC request timed out — node may be offline");
    throw e;
  }
}

// ── Message handler ──────────────────────────────────────────────

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (msg.type === "wallet_unlocked") {
    handleUnlock(msg).then(
      () => sendResponse({ data: true }),
      (e) => sendResponse({ error: e.message })
    );
    return true;
  }

  if (msg.type === "wallet_locked") {
    handleLock();
    sendResponse({ data: true });
    return false;
  }

  if (msg.type === "network_changed") {
    if (msg.rpc) rpcUrl = msg.rpc;
    sendResponse({ data: true });
    return false;
  }

  if (msg.type === "oauth_google") {
    handleOAuthGoogle().then(
      (r) => sendResponse({ data: r }),
      (e) => sendResponse({ error: e.message })
    );
    return true;
  }

  if (msg.type === "oauth_github") {
    handleOAuthGitHub().then(
      (r) => sendResponse({ data: r }),
      (e) => sendResponse({ error: e.message })
    );
    return true;
  }

  if (msg.type === "provider_request") {
    handleProviderRequest(msg.method, msg.params).then(
      (r) => sendResponse({ data: r }),
      (e) => sendResponse({ error: e.message })
    );
    return true;
  }
});

async function handleUnlock(msg) {
  vault.secret = msg.secret;
  vault.address = msg.address;
  vault.privKey = null;
  vault.publicKey = null;

  chrome.storage.session?.set({
    walletSession: { secret: msg.secret, address: msg.address },
  });
}

function handleLock() {
  vault.secret = null;
  vault.address = null;
  vault.privKey = null;
  vault.publicKey = null;
  chrome.storage.session?.remove("walletSession");
}

function launchWebAuthFlow(url) {
  return new Promise((resolve, reject) => {
    chrome.identity.launchWebAuthFlow(
      { url, interactive: true },
      (responseUrl) => {
        if (chrome.runtime.lastError) {
          reject(new Error(chrome.runtime.lastError.message));
        } else {
          resolve(responseUrl);
        }
      }
    );
  });
}

async function handleOAuthGoogle() {
  const redirectUrl = chrome.identity.getRedirectURL();
  const authUrl = "https://accounts.google.com/o/oauth2/v2/auth"
    + "?client_id=" + encodeURIComponent(GOOGLE_CLIENT_ID)
    + "&response_type=token"
    + "&redirect_uri=" + encodeURIComponent(redirectUrl)
    + "&scope=openid%20email";

  const responseUrl = await launchWebAuthFlow(authUrl);

  const params = new URL(responseUrl.replace("#", "?")).searchParams;
  const token = params.get("access_token");
  if (!token) throw new Error("No access token received");

  const resp = await fetch("https://www.googleapis.com/oauth2/v3/userinfo", {
    headers: { Authorization: "Bearer " + token },
  });
  const info = await resp.json();
  if (!info.sub) throw new Error("Failed to get Google user ID");

  return { userId: info.sub, email: info.email || "" };
}

async function handleOAuthGitHub() {
  const redirectUrl = chrome.identity.getRedirectURL();
  const authUrl = "https://github.com/login/oauth/authorize"
    + "?client_id=" + encodeURIComponent(GITHUB_CLIENT_ID)
    + "&redirect_uri=" + encodeURIComponent(redirectUrl)
    + "&scope=read:user";

  const responseUrl = await launchWebAuthFlow(authUrl);

  const code = new URL(responseUrl).searchParams.get("code");
  if (!code) throw new Error("No authorization code received");

  const tokenResp = await fetch("https://github.com/login/oauth/access_token", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Accept: "application/json",
    },
    body: JSON.stringify({
      client_id: GITHUB_CLIENT_ID,
      client_secret: GITHUB_CLIENT_SECRET,
      code,
    }),
  });
  const tokenData = await tokenResp.json();
  if (tokenData.error) throw new Error(tokenData.error_description || tokenData.error);

  const userResp = await fetch("https://api.github.com/user", {
    headers: { Authorization: "Bearer " + tokenData.access_token },
  });
  const user = await userResp.json();
  if (!user.id) throw new Error("Failed to get GitHub user");

  return { userId: user.id, login: user.login };
}

async function handleProviderRequest(method, params) {
  // Restore from session if service worker restarted
  if (!vault.secret) {
    const stored = await new Promise((r) => {
      if (chrome.storage.session) {
        chrome.storage.session.get(["walletSession"], r);
      } else {
        r({});
      }
    });
    if (stored?.walletSession) {
      vault.secret = stored.walletSession.secret;
      vault.address = stored.walletSession.address;
      vault.privKey = null;
      vault.publicKey = null;
    }
  }

  // Reload network config
  if (!rpcUrl || rpcUrl === "https://rpc.aztibase.com") {
    const net = await new Promise((r) =>
      chrome.storage.local.get(["network"], r)
    );
    if (net?.network?.rpc) rpcUrl = net.network.rpc;
  }

  switch (method) {
    case "connect":
    case "getAddress": {
      if (vault.address) {
        return { address: vault.address, connected: true, locked: false };
      }
      const stored = await new Promise((r) =>
        chrome.storage.local.get(["address"], r)
      );
      if (stored?.address) {
        return { address: stored.address, connected: false, locked: true };
      }
      throw new Error("No wallet found. Open the Aztibase extension to create one.");
    }

    case "isConnected":
      return { connected: !!vault.secret, address: vault.address || null };

    case "disconnect":
      return { disconnected: true };

    case "getBalance": {
      if (!vault.address) throw new Error("Wallet not connected");
      const [balance, nonce] = await Promise.all([
        rpc("aztb_getBalance", [vault.address]),
        rpc("aztb_getNonce", [vault.address]),
      ]);
      return { balance: balance || "0", nonce: nonce || 0 };
    }

    case "getNonce": {
      if (!vault.address) throw new Error("Wallet not connected");
      return await rpc("aztb_getNonce", [vault.address]);
    }

    case "signAndSendTransfer": {
      if (!vault.secret) throw new Error("Wallet locked. Open extension and unlock first.");
      const { to, amount, gasPrice } = params;
      await ensureKeyPair();

      const nonce = await rpc("aztb_getNonce", [vault.address]);
      const from32 = BLAKE3.hash(vault.publicKey);
      const to32 = hexToBytes(to.replace(/^0x/, ""));
      const gp = parseInt(gasPrice) || 1;
      const val = parseInt(amount);

      const payload = encodeTransferPayload(from32, to32, val, parseInt(nonce), gp);
      const envelope = await signPayload(payload);
      const txHex = bytesToHex(envelope);

      const result = await rpc("aztb_sendRawTransaction", [txHex]);
      return { txHash: result?.tx_hash || result || "submitted" };
    }

    case "signAndSendStake": {
      if (!vault.secret) throw new Error("Wallet locked. Open extension and unlock first.");
      const { amount, gasPrice } = params;
      await ensureKeyPair();

      const nonce = await rpc("aztb_getNonce", [vault.address]);
      const from32 = BLAKE3.hash(vault.publicKey);
      const gp = parseInt(gasPrice) || 1;
      const val = parseInt(amount);

      const payload = encodeStakePayload(from32, val, parseInt(nonce), gp);
      const envelope = await signPayload(payload);
      const txHex = bytesToHex(envelope);

      const result = await rpc("aztb_sendRawTransaction", [txHex]);
      return { txHash: result?.tx_hash || result || "submitted" };
    }

    case "signAndSendUnstake": {
      if (!vault.secret) throw new Error("Wallet locked. Open extension and unlock first.");
      const { amount, gasPrice } = params;
      await ensureKeyPair();

      const nonce = await rpc("aztb_getNonce", [vault.address]);
      const from32 = BLAKE3.hash(vault.publicKey);
      const gp = parseInt(gasPrice) || 1;
      const val = parseInt(amount);

      const payload = encodeUnstakePayload(from32, val, parseInt(nonce), gp);
      const envelope = await signPayload(payload);
      const txHex = bytesToHex(envelope);

      const result = await rpc("aztb_sendRawTransaction", [txHex]);
      return { txHash: result?.tx_hash || result || "submitted" };
    }

    case "faucetDrip": {
      if (!vault.address) throw new Error("Wallet not connected");
      return await rpc("aztb_faucetDrip", [vault.address]);
    }

    default:
      throw new Error("Unknown method: " + method);
  }
}
