import { TwoFactorAuth, isWebAuthnAvailable } from "./twofa.js";

let wasm = null;
let secretHex = null;
let addressHex = null;
let rpcUrl = "https://rpc.aztibase.com";
let currentNonce = 0;
const twofa = new TwoFactorAuth();
let pending2FAResolve = null;
let socialAuthData = null;
let rpcConnected = false;
let reconnectTimer = null;

const DECIMALS = 0;
const BASE = 1n;

const LOCAL_RPC = "http://127.0.0.1:9944";
const PUBLIC_RPC = "https://rpc.aztibase.com";
const RPC_TIMEOUT = 5000;
const RECONNECT_INTERVAL = 15000;

// ── Connection management ────────────────────────────────────────

function setConnectionStatus(status, label) {
  const dot = document.getElementById("conn-dot");
  const banner = document.getElementById("conn-banner");
  const bannerText = document.getElementById("conn-banner-text");
  if (dot) {
    dot.className = "conn-status " + status;
    dot.title = label || status;
  }
  if (status === "disconnected") {
    if (banner) { banner.classList.add("show"); }
    if (bannerText) bannerText.textContent = label || "Cannot connect to network";
    const balEl = document.getElementById("main-balance");
    if (balEl && balEl.textContent === "0") {
      balEl.textContent = "Offline";
      balEl.classList.add("offline");
    }
  } else {
    if (banner) banner.classList.remove("show");
    const balEl = document.getElementById("main-balance");
    if (balEl) balEl.classList.remove("offline");
  }
  rpcConnected = status === "connected";
}

async function probeRpc(url, timeout) {
  try {
    const ctrl = new AbortController();
    const timer = setTimeout(() => ctrl.abort(), timeout || RPC_TIMEOUT);
    const resp = await fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ jsonrpc: "2.0", method: "aztb_blockNumber", params: [], id: 1 }),
      signal: ctrl.signal,
    });
    clearTimeout(timer);
    const json = await resp.json();
    return json.result !== undefined;
  } catch {
    return false;
  }
}

async function detectAndConnect() {
  setConnectionStatus("connecting", "Connecting...");

  const stored = await storageGet(["network"]);
  const savedUrl = stored.network?.rpc;

  // Try saved URL first
  if (savedUrl) {
    if (await probeRpc(savedUrl)) {
      rpcUrl = savedUrl;
      const name = getNetworkName(rpcUrl);
      setConnectionStatus("connected", name);
      updateNetworkUI(name);
      return;
    }
  }

  // Fallback: try local
  if (await probeRpc(LOCAL_RPC, 2000)) {
    rpcUrl = LOCAL_RPC;
    setConnectionStatus("connected", "Local Testnet");
    updateNetworkUI("Local Testnet");
    toast("Connected to local node", "info");
    return;
  }

  // Fallback: try public (if not already tried)
  if (savedUrl !== PUBLIC_RPC) {
    if (await probeRpc(PUBLIC_RPC)) {
      rpcUrl = PUBLIC_RPC;
      setConnectionStatus("connected", "Testnet");
      updateNetworkUI("Testnet");
      return;
    }
  }

  // All failed
  rpcUrl = savedUrl || PUBLIC_RPC;
  setConnectionStatus("disconnected", "Cannot connect — check RPC settings");
  startReconnect();
}

function startReconnect() {
  if (reconnectTimer) return;
  reconnectTimer = setInterval(async () => {
    if (rpcConnected) { clearInterval(reconnectTimer); reconnectTimer = null; return; }
    const ok = await probeRpc(rpcUrl, 3000);
    if (ok) {
      clearInterval(reconnectTimer);
      reconnectTimer = null;
      setConnectionStatus("connected", getNetworkName(rpcUrl));
      toast("Reconnected", "success");
      if (secretHex) { refreshBalance(); refreshStaking(); }
    }
  }, RECONNECT_INTERVAL);
}

async function retryConnection() {
  if (reconnectTimer) { clearInterval(reconnectTimer); reconnectTimer = null; }
  await detectAndConnect();
  if (rpcConnected && secretHex) {
    refreshBalance();
    refreshStaking();
  }
}

function getNetworkName(url) {
  if (url.includes("127.0.0.1") || url.includes("localhost")) return "Local Testnet";
  if (url.includes("rpc.aztibase.com")) return "Testnet";
  return "Custom";
}

function updateNetworkUI(name) {
  const netEl = document.getElementById("network-name");
  if (netEl) netEl.textContent = name;
  const rpcInput = document.getElementById("settings-rpc");
  if (rpcInput) rpcInput.value = rpcUrl;
  updateNetPresetButtons();
}

function updateNetPresetButtons() {
  document.querySelectorAll(".net-preset").forEach(btn => btn.classList.remove("active"));
  const action = rpcUrl.includes("127.0.0.1") || rpcUrl.includes("localhost")
    ? "netLocal"
    : rpcUrl.includes("rpc.aztibase.com") ? "netPublic" : "netCustom";
  const activeBtn = document.querySelector(`[data-action="${action}"]`);
  if (activeBtn) activeBtn.classList.add("active");
}

// ── Init ─────────────────────────────────────────────────────────

async function init() {
  try {
    const mod = await import("./wasm/aztibase_wasm.js");
    await mod.default();
    wasm = mod;
  } catch (e) {
    console.error("WASM load failed:", e);
  }

  await twofa.load();

  await detectAndConnect();

  const stored = await storageGet(["encryptedKey"]);
  if (stored.encryptedKey) {
    const session = await sessionGet(["sessionSecret", "sessionAddress"]);
    if (session.sessionSecret && session.sessionAddress) {
      secretHex = session.sessionSecret;
      addressHex = session.sessionAddress;
      showView("main");
      updateMainView();
    } else {
      showView("unlock");
    }
  } else {
    showView("onboard");
  }
}

// --- Crypto (Web Crypto API for key encryption) ---

async function deriveKey(password, salt) {
  const enc = new TextEncoder();
  const keyMaterial = await crypto.subtle.importKey(
    "raw", enc.encode(password), "PBKDF2", false, ["deriveKey"]
  );
  return crypto.subtle.deriveKey(
    { name: "PBKDF2", salt, iterations: 600000, hash: "SHA-256" },
    keyMaterial,
    { name: "AES-GCM", length: 256 },
    false,
    ["encrypt", "decrypt"]
  );
}

async function encryptSecret(secret, password) {
  const salt = crypto.getRandomValues(new Uint8Array(16));
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const key = await deriveKey(password, salt);
  const enc = new TextEncoder();
  const ciphertext = await crypto.subtle.encrypt(
    { name: "AES-GCM", iv }, key, enc.encode(secret)
  );
  return {
    salt: bufToHex(salt),
    iv: bufToHex(iv),
    ciphertext: bufToHex(new Uint8Array(ciphertext)),
  };
}

async function decryptSecret(encrypted, password) {
  const salt = hexToBuf(encrypted.salt);
  const iv = hexToBuf(encrypted.iv);
  const ciphertext = hexToBuf(encrypted.ciphertext);
  const key = await deriveKey(password, salt);
  const plaintext = await crypto.subtle.decrypt(
    { name: "AES-GCM", iv }, key, ciphertext
  );
  return new TextDecoder().decode(plaintext);
}

// --- Hex helpers ---

function bufToHex(buf) {
  return Array.from(buf).map(b => b.toString(16).padStart(2, "0")).join("");
}

function hexToBuf(hex) {
  const clean = hex.startsWith("0x") ? hex.slice(2) : hex;
  const bytes = new Uint8Array(clean.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(clean.substr(i * 2, 2), 16);
  }
  return bytes;
}

function toBaseUnits(amountStr) {
  return BigInt(amountStr.split(".")[0] || "0").toString();
}

function fromBaseUnits(baseStr) {
  return BigInt(baseStr || "0").toLocaleString();
}

function shortenAddress(addr) {
  const clean = addr.startsWith("0x") ? addr : "0x" + addr;
  return clean.slice(0, 10) + "..." + clean.slice(-8);
}

// --- Chrome storage helpers ---

function storageGet(keys) {
  return new Promise((resolve) => {
    if (chrome?.storage?.local) {
      chrome.storage.local.get(keys, resolve);
    } else {
      const result = {};
      for (const k of keys) {
        const v = localStorage.getItem(k);
        if (v) result[k] = JSON.parse(v);
      }
      resolve(result);
    }
  });
}

function storageSet(obj) {
  return new Promise((resolve) => {
    if (chrome?.storage?.local) {
      chrome.storage.local.set(obj, resolve);
    } else {
      for (const [k, v] of Object.entries(obj)) {
        localStorage.setItem(k, JSON.stringify(v));
      }
      resolve();
    }
  });
}

function storageRemove(keys) {
  return new Promise((resolve) => {
    if (chrome?.storage?.local) {
      chrome.storage.local.remove(keys, resolve);
    } else {
      for (const k of keys) localStorage.removeItem(k);
      resolve();
    }
  });
}

function sessionGet(keys) {
  return new Promise((resolve) => {
    if (chrome?.storage?.session) {
      chrome.storage.session.get(keys, resolve);
    } else {
      const result = {};
      for (const k of keys) {
        const v = sessionStorage.getItem(k);
        if (v) result[k] = JSON.parse(v);
      }
      resolve(result);
    }
  });
}

function sessionSet(obj) {
  return new Promise((resolve) => {
    if (chrome?.storage?.session) {
      chrome.storage.session.set(obj, resolve);
    } else {
      for (const [k, v] of Object.entries(obj)) {
        sessionStorage.setItem(k, JSON.stringify(v));
      }
      resolve();
    }
  });
}

function sessionRemove(keys) {
  return new Promise((resolve) => {
    if (chrome?.storage?.session) {
      chrome.storage.session.remove(keys, resolve);
    } else {
      for (const k of keys) sessionStorage.removeItem(k);
      resolve();
    }
  });
}

// --- View navigation ---

function showView(name) {
  document.querySelectorAll(".view").forEach(v => v.classList.remove("active"));
  const view = document.getElementById(`view-${name}`);
  if (view) view.classList.add("active");

  if (name === "main") {
    refreshBalance();
    refreshStaking();
    loadTxHistory();
    update2FAStatusText();
  } else if (name === "receive") {
    document.getElementById("receive-address").textContent =
      addressHex ? "0x" + addressHex : "No wallet";
  } else if (name === "settings") {
    update2FAStatusText();
    updateNetPresetButtons();
    const rpcInput = document.getElementById("settings-rpc");
    if (rpcInput) rpcInput.value = rpcUrl;
  } else if (name === "2fa-setup") {
    if (twofa.totpSecret && twofa.enabled) {
      document.getElementById("totp-enabled-badge").style.display = "block";
      document.getElementById("totp-setup-btn").style.display = "none";
    }
    if (twofa.webauthnCredentialId && twofa.enabled) {
      document.getElementById("webauthn-enabled-badge").style.display = "block";
      document.getElementById("btn-webauthn-setup").style.display = "none";
    }
    if (twofa.enabled) {
      document.getElementById("disable-2fa-section").style.display = "block";
    }
  }
}


function goBack() {
  if (secretHex) {
    showView("main");
  } else {
    showView("onboard");
  }
}

function switchTab(tabId, clickedEl) {
  document.querySelectorAll(".tab").forEach(t => t.classList.remove("active"));
  if (clickedEl) clickedEl.classList.add("active");
  document.querySelectorAll(".tx-list").forEach(l => l.style.display = "none");
  const el = document.getElementById(`tab-${tabId}`);
  if (el) el.style.display = "block";
  if (tabId === "staking-tab") refreshStaking();
}

// --- Wallet operations ---

async function createWallet() {
  if (!wasm) { toast("WASM module not loaded", "error"); return; }

  const pass = document.getElementById("create-pass").value;
  const pass2 = document.getElementById("create-pass2").value;
  if (pass.length < 8) { toast("Password must be at least 8 characters", "error"); return; }
  if (pass !== pass2) { toast("Passwords do not match", "error"); return; }

  const keypairJson = wasm.generateKeypair();
  const kp = JSON.parse(keypairJson);

  secretHex = kp.secret;
  addressHex = kp.address;

  const encrypted = await encryptSecret(secretHex, pass);
  await storageSet({ encryptedKey: encrypted, address: addressHex });
  await sessionSet({ sessionSecret: secretHex, sessionAddress: addressHex });

  chrome.runtime?.sendMessage?.({
    type: "wallet_unlocked", secret: secretHex, address: addressHex,
  }).catch(() => {});

  document.getElementById("mnemonic-display").style.display = "block";
  document.getElementById("mnemonic-words").value =
    `Private key (hex) — save this securely:\n${kp.secret}`;
}


async function importKeyFile() {
  if (!wasm) { toast("WASM module not loaded", "error"); return; }

  const input = document.createElement("input");
  input.type = "file";
  input.accept = ".json";
  input.onchange = async (e) => {
    const file = e.target.files[0];
    if (!file) return;
    try {
      const text = await file.text();
      const data = JSON.parse(text);
      const sk = data.secret_key || data.secretKey || data.secret;
      if (!sk || sk.length !== 64) {
        toast("Invalid key file — no secret_key found", "error");
        return;
      }
      const addr = data.address;
      if (!addr) {
        toast("Invalid key file — no address found", "error");
        return;
      }

      const pass = prompt("Set a password to protect this key (min 8 chars):");
      if (!pass || pass.length < 8) {
        toast("Password must be at least 8 characters", "error");
        return;
      }

      secretHex = sk;
      addressHex = addr;

      const encrypted = await encryptSecret(secretHex, pass);
      await storageSet({ encryptedKey: encrypted, address: addressHex });
      await sessionSet({ sessionSecret: secretHex, sessionAddress: addressHex });

      chrome.runtime?.sendMessage?.({
        type: "wallet_unlocked", secret: secretHex, address: addressHex,
      }).catch(() => {});

      toast("Key file imported: " + shortenAddress(addressHex), "success");
      showView("main");
      refreshAll();
    } catch (err) {
      toast("Failed to read key file: " + err.message, "error");
    }
  };
  input.click();
}

function confirmMnemonic() {
  showView("main");
  updateMainView();
  toast("Wallet created", "success");
}

async function importWallet() {
  if (!wasm) { toast("WASM module not loaded", "error"); return; }

  const secret = document.getElementById("import-secret").value.trim();
  const pass = document.getElementById("import-pass").value;
  if (pass.length < 8) { toast("Password must be at least 8 characters", "error"); return; }

  const clean = secret.startsWith("0x") ? secret.slice(2) : secret;
  if (clean.length !== 64 || !/^[0-9a-fA-F]+$/.test(clean)) {
    toast("Invalid private key — must be 64 hex characters", "error");
    return;
  }

  const addr = wasm.addressFromSecret(clean);
  if (addr.startsWith("error:")) { toast(addr, "error"); return; }

  secretHex = clean;
  addressHex = addr;

  const encrypted = await encryptSecret(secretHex, pass);
  await storageSet({ encryptedKey: encrypted, address: addressHex });
  await sessionSet({ sessionSecret: secretHex, sessionAddress: addressHex });

  chrome.runtime?.sendMessage?.({
    type: "wallet_unlocked", secret: secretHex, address: addressHex,
  }).catch(() => {});

  showView("main");
  updateMainView();
  toast("Wallet imported", "success");
}

// --- Social sign-in (OAuth runs in background service worker) ---

async function signInGoogle() {
  try {
    const result = await chrome.runtime.sendMessage({ type: "oauth_google" });
    if (result?.error) throw new Error(result.error);

    socialAuthData = { provider: "google", userId: result.data.userId, email: result.data.email || "" };
    document.getElementById("social-provider").textContent = "Google";
    document.getElementById("social-email").textContent = result.data.email || result.data.userId;
    showView("social-auth");
  } catch (e) {
    toast("Google sign-in failed: " + e.message, "error");
  }
}

async function signInGitHub() {
  try {
    const result = await chrome.runtime.sendMessage({ type: "oauth_github" });
    if (result?.error) throw new Error(result.error);

    socialAuthData = {
      provider: "github",
      userId: String(result.data.userId),
      email: result.data.login || "",
    };
    document.getElementById("social-provider").textContent = "GitHub";
    document.getElementById("social-email").textContent = result.data.login || result.data.userId;
    showView("social-auth");
  } catch (e) {
    toast("GitHub sign-in failed: " + e.message, "error");
  }
}

async function completeSocialAuth() {
  if (!socialAuthData) { toast("No auth data", "error"); return; }
  if (!wasm) { toast("WASM module not loaded", "error"); return; }

  const pass = document.getElementById("social-pass").value;
  const pass2 = document.getElementById("social-pass2").value;
  if (pass.length < 8) { toast("Password must be at least 8 characters", "error"); return; }
  if (pass !== pass2) { toast("Passwords do not match", "error"); return; }

  const enc = new TextEncoder();
  const salt = enc.encode("aztibase:" + socialAuthData.provider + ":" + socialAuthData.userId);
  const keyMaterial = await crypto.subtle.importKey(
    "raw", enc.encode(pass), "PBKDF2", false, ["deriveBits"]
  );
  const seedBits = await crypto.subtle.deriveBits(
    { name: "PBKDF2", salt, iterations: 600000, hash: "SHA-256" },
    keyMaterial,
    256
  );

  secretHex = bufToHex(new Uint8Array(seedBits));
  const addr = wasm.addressFromSecret(secretHex);
  if (addr.startsWith("error:")) { toast(addr, "error"); return; }
  addressHex = addr;

  const encrypted = await encryptSecret(secretHex, pass);
  await storageSet({
    encryptedKey: encrypted,
    address: addressHex,
    socialAuth: { provider: socialAuthData.provider, email: socialAuthData.email },
  });
  await sessionSet({ sessionSecret: secretHex, sessionAddress: addressHex });

  chrome.runtime?.sendMessage?.({
    type: "wallet_unlocked", secret: secretHex, address: addressHex,
  }).catch(() => {});

  const provider = socialAuthData.provider;
  socialAuthData = null;
  showView("main");
  updateMainView();
  toast("Wallet created via " + provider, "success");
}

async function unlockWallet() {
  const pass = document.getElementById("unlock-pass").value;
  const stored = await storageGet(["encryptedKey", "address"]);

  if (!stored.encryptedKey) { toast("No wallet found", "error"); return; }

  try {
    secretHex = await decryptSecret(stored.encryptedKey, pass);
    addressHex = stored.address;

    await sessionSet({ sessionSecret: secretHex, sessionAddress: addressHex });

    chrome.runtime?.sendMessage?.({
      type: "wallet_unlocked", secret: secretHex, address: addressHex,
    }).catch(() => {});

    showView("main");
    updateMainView();
  } catch {
    toast("Wrong password", "error");
  }
}

async function lockWallet() {
  secretHex = null;
  await sessionRemove(["sessionSecret", "sessionAddress"]);
  chrome.runtime?.sendMessage?.({ type: "wallet_locked" }).catch(() => {});
  showView("unlock");
  toast("Wallet locked", "info");
}

async function resetWallet() {
  if (!confirm("This will remove your wallet. Make sure you have backed up your key.")) return;
  const oldAddr = addressHex;
  secretHex = null;
  addressHex = null;
  const keysToRemove = ["encryptedKey", "address", "lastGenesisHash", "socialAuth"];
  if (oldAddr) keysToRemove.push("txHistory_" + oldAddr);
  await storageRemove(keysToRemove);
  await sessionRemove(["sessionSecret", "sessionAddress"]);
  showView("onboard");
  toast("Wallet reset", "info");
}

async function updateMainView() {
  if (addressHex) {
    document.getElementById("main-address").textContent = shortenAddress(addressHex);
    await detectChainReset();
    refreshBalance();
    refreshStaking();
  }
}

async function detectChainReset() {
  if (!addressHex || !rpcConnected) return;
  try {
    const block0 = await rpcCall("aztb_getBlockByNumber", [0]);
    const genesisHash = block0?.hash || block0?.block_hash || null;
    if (!genesisHash) return;

    const stored = await storageGet(["lastGenesisHash"]);
    if (stored.lastGenesisHash && stored.lastGenesisHash !== genesisHash) {
      const key = "txHistory_" + addressHex;
      await storageRemove([key]);
      const list = document.getElementById("tab-activity");
      if (list) list.innerHTML = '<div class="empty-state">No transactions yet</div>';
      toast("Chain reset detected — activity cleared", "info");
    }
    await storageSet({ lastGenesisHash: genesisHash });
  } catch {
    // non-critical
  }
}

// --- RPC calls ---

async function rpcCall(method, params) {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), RPC_TIMEOUT);
  const body = JSON.stringify({
    jsonrpc: "2.0",
    method,
    params,
    id: Date.now(),
  });
  try {
    const resp = await fetch(rpcUrl, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body,
      signal: ctrl.signal,
    });
    clearTimeout(timer);
    const text = await resp.text();
    let json;
    try {
      json = JSON.parse(text);
    } catch {
      throw new Error("RPC returned invalid response (is the endpoint correct?)");
    }
    if (json.error) throw new Error(json.error.message || JSON.stringify(json.error));

    if (!rpcConnected) {
      setConnectionStatus("connected", getNetworkName(rpcUrl));
    }
    return json.result;
  } catch (e) {
    clearTimeout(timer);
    if (e.name === "AbortError") {
      setConnectionStatus("disconnected", "RPC timeout");
      startReconnect();
      throw new Error("RPC request timed out");
    }
    if (e.message.includes("Failed to fetch") || e.message.includes("NetworkError") || e.message.includes("invalid response")) {
      setConnectionStatus("disconnected", "Cannot connect to " + getNetworkName(rpcUrl));
      startReconnect();
    }
    throw e;
  }
}

async function refreshBalance() {
  if (!addressHex) return;
  const balEl = document.getElementById("main-balance");
  try {
    const balance = await rpcCall("aztb_getBalance", [addressHex]);
    const display = fromBaseUnits(balance || "0");
    if (balEl) {
      balEl.textContent = display;
      balEl.classList.remove("offline");
    }
  } catch {
    if (balEl && !rpcConnected) {
      balEl.textContent = "Offline";
      balEl.classList.add("offline");
    }
  }
}

async function refreshNonce() {
  if (!addressHex) return;
  try {
    const nonce = await rpcCall("aztb_getNonce", [addressHex]);
    currentNonce = parseInt(nonce, 10) || 0;
  } catch {
    // nonce fetch failed — will retry on next tx
  }
}

async function refreshStaking() {
  if (!addressHex) return;

  const selfEl = document.getElementById("staking-self");
  const delEl = document.getElementById("staking-delegated");
  const rewardsEl = document.getElementById("staking-rewards");

  try {
    const info = await rpcCall("aztb_getValidatorStake", [addressHex]);
    const btn = document.getElementById("btn-become-validator");
    if (info && typeof info === "object") {
      const self_stake = info.self_stake || info.selfStake || 0;
      const delegated = info.total_delegated || info.totalDelegated || 0;
      if (selfEl) selfEl.textContent = fromBaseUnits(String(self_stake)) + " AZTB";
      if (delEl) delEl.textContent = fromBaseUnits(String(delegated)) + " AZTB";
      if (btn) { btn.textContent = "Registered \u2713"; btn.disabled = true; }
    } else if (info && info !== "0") {
      if (selfEl) selfEl.textContent = fromBaseUnits(String(info)) + " AZTB";
      if (btn) { btn.textContent = "Registered \u2713"; btn.disabled = true; }
    } else {
      if (selfEl) selfEl.textContent = "0 AZTB";
      if (delEl) delEl.textContent = "0 AZTB";
      if (btn) { btn.textContent = "Become Validator"; btn.disabled = false; }
    }
  } catch {
    if (selfEl) selfEl.textContent = "0 AZTB";
    if (delEl) delEl.textContent = "0 AZTB";
  }

  try {
    const rewards = await rpcCall("aztb_getEpochRewards", [10]);
    if (rewardsEl && Array.isArray(rewards)) {
      const addrClean = addressHex.replace(/^0x/, '').toLowerCase();
      let total = 0n;
      for (const evt of rewards) {
        for (const c of (evt.credits || [])) {
          const cAddr = (c.validator || '').replace(/^0x/, '').toLowerCase();
          if (cAddr === addrClean) total += BigInt(c.amount);
        }
      }
      rewardsEl.textContent = (total > 0n ? fromBaseUnits(total.toString()) : "0") + " AZTB";
    }
  } catch {
    if (rewardsEl && !rpcConnected) rewardsEl.textContent = "-- AZTB";
  }
}

// --- 2FA gate ---

function require2FA() {
  if (!twofa.enabled) return Promise.resolve(true);

  return new Promise((resolve) => {
    pending2FAResolve = resolve;
    const modal = document.getElementById("twofa-modal");
    modal.style.display = "block";

    const hasWebAuthn = twofa.method === "webauthn" || twofa.method === "both";
    const hasTotp = twofa.method === "totp" || twofa.method === "both";

    document.getElementById("twofa-webauthn-section").style.display = hasWebAuthn ? "block" : "none";
    document.getElementById("twofa-totp-section").style.display = hasTotp ? "block" : "none";
    document.getElementById("twofa-code").value = "";
  });
}

async function verify2FAWebAuthn() {
  try {
    const ok = await twofa.verify(null);
    if (ok) {
      document.getElementById("twofa-modal").style.display = "none";
      if (pending2FAResolve) { pending2FAResolve(true); pending2FAResolve = null; }
    } else {
      toast("Biometric verification failed", "error");
    }
  } catch (e) {
    toast(`WebAuthn error: ${e.message}`, "error");
  }
}

async function verify2FATotp() {
  const code = document.getElementById("twofa-code").value.trim();
  if (code.length !== 6) { toast("Enter 6-digit code", "error"); return; }

  const ok = await twofa.verify(code);
  if (ok) {
    document.getElementById("twofa-modal").style.display = "none";
    if (pending2FAResolve) { pending2FAResolve(true); pending2FAResolve = null; }
  } else {
    toast("Invalid code", "error");
  }
}

function cancel2FA() {
  document.getElementById("twofa-modal").style.display = "none";
  if (pending2FAResolve) { pending2FAResolve(false); pending2FAResolve = null; }
}

// --- 2FA Setup ---

async function startTotpSetup() {
  const account = addressHex ? shortenAddress(addressHex) : "wallet";
  const result = await twofa.setupTotp(account);

  document.getElementById("totp-qr").innerHTML = result.qrSvg;
  document.getElementById("totp-qr").style.display = "block";
  document.getElementById("totp-secret-text").value = result.secret;
  document.getElementById("totp-secret-display").style.display = "block";
  document.getElementById("totp-confirm").style.display = "block";
  document.getElementById("totp-setup-btn").style.display = "none";
}

async function confirmTotpSetup() {
  const code = document.getElementById("totp-confirm-code").value.trim();
  if (code.length !== 6) { toast("Enter 6-digit code from your authenticator", "error"); return; }

  const ok = await twofa.confirmTotp(code);
  if (ok) {
    document.getElementById("totp-enabled-badge").style.display = "block";
    document.getElementById("totp-confirm").style.display = "none";
    document.getElementById("disable-2fa-section").style.display = "block";
    update2FAStatusText();
    toast("TOTP enabled", "success");
  } else {
    toast("Invalid code — try again", "error");
  }
}

async function startWebAuthnSetup() {
  try {
    const available = await isWebAuthnAvailable();
    if (!available) {
      toast("Biometric / security key not available on this device", "error");
      return;
    }
    await twofa.setupWebAuthn(addressHex || "aztibase-wallet");
    document.getElementById("webauthn-enabled-badge").style.display = "block";
    document.getElementById("btn-webauthn-setup").style.display = "none";
    document.getElementById("disable-2fa-section").style.display = "block";
    update2FAStatusText();
    toast("Passkey registered", "success");
  } catch (e) {
    toast(`Passkey setup failed: ${e.message}`, "error");
  }
}

async function disable2FA() {
  if (!confirm("Disable all 2FA? Transactions will no longer require verification.")) return;
  await twofa.disable();
  document.getElementById("totp-enabled-badge").style.display = "none";
  document.getElementById("webauthn-enabled-badge").style.display = "none";
  document.getElementById("totp-setup-btn").style.display = "block";
  document.getElementById("totp-qr").style.display = "none";
  document.getElementById("totp-secret-display").style.display = "none";
  document.getElementById("totp-confirm").style.display = "none";
  document.getElementById("btn-webauthn-setup").style.display = "block";
  document.getElementById("disable-2fa-section").style.display = "none";
  update2FAStatusText();
  toast("2FA disabled", "info");
}

function update2FAStatusText() {
  const el = document.getElementById("2fa-status-text");
  if (!el) return;
  if (!twofa.enabled) {
    el.textContent = "Not configured";
    el.style.color = "var(--text-muted)";
  } else if (twofa.method === "both") {
    el.textContent = "TOTP + Passkey enabled";
    el.style.color = "var(--green)";
  } else if (twofa.method === "totp") {
    el.textContent = "TOTP enabled";
    el.style.color = "var(--green)";
  } else if (twofa.method === "webauthn") {
    el.textContent = "Passkey enabled";
    el.style.color = "var(--green)";
  }
}

// --- Send operations ---

async function sendTransfer() {
  if (!wasm || !secretHex) { toast("Wallet not ready", "error"); return; }
  if (!rpcConnected) { toast("Not connected to network", "error"); return; }

  const to = document.getElementById("send-to").value.trim();
  const amount = document.getElementById("send-amount").value.trim();
  const gasPrice = parseInt(document.getElementById("send-gas").value || "1", 10);

  if (!to || !amount) { toast("Fill in recipient and amount", "error"); return; }

  const authed = await require2FA();
  if (!authed) return;

  try {
    await refreshNonce();
    const baseAmount = toBaseUnits(amount);
    const signedHex = wasm.signTransfer(secretHex, to, baseAmount, BigInt(currentNonce), BigInt(gasPrice));
    if (signedHex.startsWith("error:")) { toast(signedHex, "error"); return; }

    const txHash = await rpcCall("aztb_sendRawTransaction", [signedHex]);
    toast(`Sent! TX: ${txHash.slice(0, 16)}...`, "success");
    addTxToHistory("Send", to, amount, "negative");
    showView("main");
  } catch (e) {
    toast(`Send failed: ${e.message}`, "error");
  }
}

async function becomeValidator() {
  if (!wasm || !secretHex) { toast("Wallet not ready", "error"); return; }
  if (!rpcConnected) { toast("Not connected to network", "error"); return; }

  const authed = await require2FA();
  if (!authed) return;

  try {
    await refreshNonce();
    const signedHex = wasm.signRegisterValidator(secretHex, "0", BigInt(currentNonce), 1n);
    if (signedHex.startsWith("error:")) { toast(signedHex, "error"); return; }

    const txHash = await rpcCall("aztb_sendRawTransaction", [signedHex]);
    toast("Registering as validator...", "info");
    const receipt = await pollReceipt(txHash);
    if (receipt && receipt.success === false) {
      toast(`Registration failed: ${receipt.error || "execution rejected"}`, "error");
    } else {
      toast("You are now a validator! Active at next epoch.", "success");
      addTxToHistory("Become Validator", addressHex, "0", "stake");
      const btn = document.getElementById("btn-become-validator");
      if (btn) { btn.textContent = "Registered"; btn.disabled = true; }
    }
    refreshBalance();
    refreshStaking();
  } catch (e) {
    toast(`Registration failed: ${e.message}`, "error");
  }
}

async function sendStake() {
  if (!wasm || !secretHex) { toast("Wallet not ready", "error"); return; }
  if (!rpcConnected) { toast("Not connected to network", "error"); return; }

  const amount = document.getElementById("stake-amount").value.trim();
  const gasPrice = parseInt(document.getElementById("stake-gas").value || "1", 10);
  if (!amount) { toast("Enter stake amount", "error"); return; }

  const authed = await require2FA();
  if (!authed) return;

  try {
    await refreshNonce();
    const baseAmount = toBaseUnits(amount);
    const signedHex = wasm.signStake(secretHex, baseAmount, BigInt(currentNonce), BigInt(gasPrice));
    if (signedHex.startsWith("error:")) { toast(signedHex, "error"); return; }

    const txHash = await rpcCall("aztb_sendRawTransaction", [signedHex]);
    toast(`Stake TX broadcast: ${txHash.slice(0, 16)}...`, "info");
    const receipt = await pollReceipt(txHash);
    if (receipt && receipt.success === false) {
      toast(`Stake failed: ${receipt.error || "execution rejected"}`, "error");
      addTxToHistory("Stake (failed)", addressHex, amount, "stake");
    } else {
      toast("Staked successfully!", "success");
      addTxToHistory("Stake", addressHex, amount, "stake");
    }
    refreshBalance();
    refreshStaking();
    showView("main");
  } catch (e) {
    toast(`Stake failed: ${e.message}`, "error");
  }
}

async function sendUnstake() {
  if (!wasm || !secretHex) { toast("Wallet not ready", "error"); return; }
  if (!rpcConnected) { toast("Not connected to network", "error"); return; }

  const amount = document.getElementById("unstake-amount").value.trim();
  if (!amount) { toast("Enter unstake amount", "error"); return; }

  const authed = await require2FA();
  if (!authed) return;

  try {
    await refreshNonce();
    const baseAmount = toBaseUnits(amount);
    const signedHex = wasm.signUnstake(secretHex, baseAmount, BigInt(currentNonce), 1n);
    if (signedHex.startsWith("error:")) { toast(signedHex, "error"); return; }

    const txHash = await rpcCall("aztb_sendRawTransaction", [signedHex]);
    toast(`Unstake TX broadcast: ${txHash.slice(0, 16)}...`, "info");
    const receipt = await pollReceipt(txHash);
    if (receipt && receipt.success === false) {
      toast(`Unstake failed: ${receipt.error || "execution rejected"}`, "error");
      addTxToHistory("Unstake (failed)", addressHex, amount, "stake");
    } else {
      toast("Unstaked successfully!", "success");
      addTxToHistory("Unstake", addressHex, amount, "stake");
    }
    showView("main");
  } catch (e) {
    toast(`Unstake failed: ${e.message}`, "error");
  }
}

async function sendDelegate() {
  if (!wasm || !secretHex) { toast("Wallet not ready", "error"); return; }
  if (!rpcConnected) { toast("Not connected to network", "error"); return; }

  const validator = document.getElementById("delegate-validator").value.trim();
  const amount = document.getElementById("delegate-amount").value.trim();
  if (!validator || !amount) { toast("Fill in validator and amount", "error"); return; }

  const authed = await require2FA();
  if (!authed) return;

  try {
    await refreshNonce();
    const baseAmount = toBaseUnits(amount);
    const signedHex = wasm.signDelegate(secretHex, validator, baseAmount, BigInt(currentNonce), 1n);
    if (signedHex.startsWith("error:")) { toast(signedHex, "error"); return; }

    const txHash = await rpcCall("aztb_sendRawTransaction", [signedHex]);
    toast(`Delegate TX broadcast: ${txHash.slice(0, 16)}...`, "info");
    const receipt = await pollReceipt(txHash);
    if (receipt && receipt.success === false) {
      toast(`Delegate failed: ${receipt.error || "execution rejected"}`, "error");
      addTxToHistory("Delegate (failed)", validator, amount, "stake");
    } else {
      toast("Delegated successfully!", "success");
      addTxToHistory("Delegate", validator, amount, "stake");
    }
    showView("main");
  } catch (e) {
    toast(`Delegate failed: ${e.message}`, "error");
  }
}

async function sendUndelegate() {
  if (!wasm || !secretHex) { toast("Wallet not ready", "error"); return; }
  if (!rpcConnected) { toast("Not connected to network", "error"); return; }

  const authed = await require2FA();
  if (!authed) return;

  try {
    await refreshNonce();
    const signedHex = wasm.signUndelegate(secretHex, BigInt(currentNonce), 1n);
    if (signedHex.startsWith("error:")) { toast(signedHex, "error"); return; }

    const txHash = await rpcCall("aztb_sendRawTransaction", [signedHex]);
    toast(`Undelegate TX broadcast: ${txHash.slice(0, 16)}...`, "info");
    const receipt = await pollReceipt(txHash);
    if (receipt && receipt.success === false) {
      toast(`Undelegate failed: ${receipt.error || "execution rejected"}`, "error");
      addTxToHistory("Undelegate (failed)", addressHex, "--", "stake");
    } else {
      toast("Undelegated successfully!", "success");
      addTxToHistory("Undelegate", addressHex, "--", "stake");
    }
    showView("main");
  } catch (e) {
    toast(`Undelegate failed: ${e.message}`, "error");
  }
}

async function pollReceipt(txHash, retries = 5) {
  for (let i = 0; i < retries; i++) {
    await new Promise(r => setTimeout(r, 2000));
    try {
      const receipt = await rpcCall("aztb_getTransactionReceipt", [txHash]);
      if (receipt) return receipt;
    } catch { /* not yet available */ }
  }
  return null;
}

// --- Activity list ---

function renderTxItem(entry) {
  const iconClass = entry.type === "negative" ? "send" : entry.type === "positive" ? "receive" : "stake";
  const symbol = entry.type === "negative" ? "&uarr;" : entry.type === "positive" ? "&darr;" : "&diams;";
  const amountClass = entry.type === "negative" ? "negative" : entry.type === "positive" ? "positive" : "";
  const item = document.createElement("div");
  item.className = "tx-item";
  item.innerHTML = `
    <div class="tx-icon ${iconClass}">${symbol}</div>
    <div class="tx-info">
      <div class="tx-label">${entry.label}</div>
      <div class="tx-addr">${shortenAddress(entry.addr)}</div>
    </div>
    <div class="tx-amount ${amountClass}">${entry.amount} AZTB</div>
  `;
  return item;
}

async function loadTxHistory() {
  if (!addressHex) return;
  const key = "txHistory_" + addressHex;
  const stored = await storageGet([key]);
  const entries = stored[key] || [];
  const list = document.getElementById("tab-activity");
  list.innerHTML = "";
  if (entries.length === 0) {
    list.innerHTML = '<div class="empty-state">No transactions yet</div>';
    return;
  }
  for (const entry of entries) {
    list.appendChild(renderTxItem(entry));
  }
}

async function clearHistory() {
  if (!addressHex) return;
  const key = "txHistory_" + addressHex;
  await storageSet({ [key]: [] });
  const list = document.getElementById("tab-activity");
  if (list) list.innerHTML = '<div class="empty-state">No transactions yet</div>';
  toast("Activity history cleared", "success");
}

async function addTxToHistory(label, addr, amount, type) {
  const list = document.getElementById("tab-activity");
  const empty = list.querySelector(".empty-state");
  if (empty) empty.remove();

  const entry = { label, addr, amount, type, ts: Date.now() };
  list.prepend(renderTxItem(entry));

  if (!addressHex) return;
  const key = "txHistory_" + addressHex;
  const stored = await storageGet([key]);
  const entries = stored[key] || [];
  entries.unshift(entry);
  if (entries.length > 50) entries.length = 50;
  await storageSet({ [key]: entries });
}

// --- Settings ---

function netLocal() {
  document.getElementById("settings-rpc").value = LOCAL_RPC;
  document.querySelectorAll(".net-preset").forEach(b => b.classList.remove("active"));
  document.querySelector('[data-action="netLocal"]')?.classList.add("active");
}

function netPublic() {
  document.getElementById("settings-rpc").value = PUBLIC_RPC;
  document.querySelectorAll(".net-preset").forEach(b => b.classList.remove("active"));
  document.querySelector('[data-action="netPublic"]')?.classList.add("active");
}

function netCustom() {
  document.querySelectorAll(".net-preset").forEach(b => b.classList.remove("active"));
  document.querySelector('[data-action="netCustom"]')?.classList.add("active");
  document.getElementById("settings-rpc").focus();
}

async function saveSettings() {
  const newUrl = document.getElementById("settings-rpc").value.trim();
  if (!newUrl) { toast("Enter an RPC URL", "error"); return; }

  const name = getNetworkName(newUrl);
  rpcUrl = newUrl;
  await storageSet({
    network: { name, rpc: rpcUrl, chainId: "0xA27B" },
  });
  document.getElementById("network-name").textContent = name;
  chrome.runtime?.sendMessage?.({ type: "network_changed", rpc: rpcUrl }).catch(() => {});

  // Test the new connection
  setConnectionStatus("connecting", "Testing...");
  const ok = await probeRpc(rpcUrl, 4000);
  if (ok) {
    setConnectionStatus("connected", name);
    toast("Connected to " + name, "success");
    if (secretHex) { refreshBalance(); refreshStaking(); }
  } else {
    setConnectionStatus("disconnected", "Cannot connect to " + name);
    toast("Cannot reach " + newUrl, "error");
    startReconnect();
  }
}

async function exportKey() {
  const pass = document.getElementById("export-pass").value;
  const stored = await storageGet(["encryptedKey"]);
  if (!stored.encryptedKey) { toast("No wallet", "error"); return; }

  try {
    const key = await decryptSecret(stored.encryptedKey, pass);
    document.getElementById("export-key-display").value = key;
    document.getElementById("export-result").style.display = "block";
    setTimeout(() => {
      document.getElementById("export-result").style.display = "none";
      document.getElementById("export-key-display").value = "";
    }, 30000);
  } catch {
    toast("Wrong password", "error");
  }
}

// --- Copy ---

function copyAddress() {
  const addr = addressHex ? "0x" + addressHex : "";
  if (!addr) return;
  navigator.clipboard.writeText(addr).then(() => {
    toast("Address copied", "info");
  });
}

// --- Toast ---

function toast(msg, type) {
  const el = document.getElementById("toast");
  el.textContent = msg;
  el.className = `toast ${type} show`;
  setTimeout(() => { el.className = "toast"; }, 3000);
}

// --- Delegated click handler (MV3 CSP forbids inline onclick) ---

async function refreshAll() {
  if (!rpcConnected) {
    await retryConnection();
    return;
  }
  const btn = document.querySelector('.refresh-btn');
  if (btn) btn.classList.add('spinning');
  await Promise.all([refreshBalance(), refreshStaking(), refreshNonce()]);
  if (btn) {
    btn.classList.remove('spinning');
    setTimeout(() => btn.classList.add('spinning'), 0);
    setTimeout(() => btn.classList.remove('spinning'), 600);
  }
  toast("Refreshed", "info");
}

async function walletFaucet() {
  if (!addressHex) return;
  if (!rpcConnected) { toast("Not connected to network", "error"); return; }
  toast("Requesting faucet drip...", "info");
  try {
    const result = await rpcCall("aztb_faucetDrip", [addressHex]);
    if (result && result.amount) {
      toast("Received " + fromBaseUnits(String(result.amount)) + " AZTB", "success");
      addTxToHistory("Faucet Drip", "faucet", fromBaseUnits(String(result.amount)));
    } else {
      toast("Faucet drip sent", "success");
    }
    setTimeout(refreshAll, 2000);
  } catch (e) {
    toast("Faucet error: " + e.message, "error");
  }
}

const actions = {
  goBack, createWallet, confirmMnemonic, importWallet, unlockWallet,
  lockWallet, resetWallet, clearHistory, copyAddress, importKeyFile, sendTransfer, becomeValidator, sendStake,
  sendUnstake, sendDelegate, sendUndelegate, saveSettings, exportKey,
  startTotpSetup, confirmTotpSetup, startWebAuthnSetup, disable2FA,
  verify2FAWebAuthn, verify2FATotp, cancel2FA,
  signInGoogle, signInGitHub, completeSocialAuth, refreshAll, walletFaucet,
  retryConnection, netLocal, netPublic, netCustom,
};

document.addEventListener("click", (e) => {
  const el = e.target.closest("[data-view],[data-action],[data-tab]");
  if (!el) return;

  if (el.dataset.view) {
    showView(el.dataset.view);
    return;
  }
  if (el.dataset.tab) {
    switchTab(el.dataset.tab, el);
    return;
  }
  if (el.dataset.action) {
    const fn = actions[el.dataset.action];
    if (typeof fn === "function") fn();
  }
});

// --- Init ---
init();
