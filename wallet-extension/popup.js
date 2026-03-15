import { TwoFactorAuth, isWebAuthnAvailable } from "./twofa.js";

let wasm = null;
let secretHex = null;
let addressHex = null;
let rpcUrl = "https://rpc.aztibase.com";
let currentNonce = 0;
const twofa = new TwoFactorAuth();
let pending2FAResolve = null;
let socialAuthData = null;

const DECIMALS = 0;
const BASE = 1n;

const LOCAL_RPC = "http://127.0.0.1:9944";
const PUBLIC_RPC = "https://rpc.aztibase.com";

async function detectRpc() {
  try {
    const resp = await fetch(LOCAL_RPC + "/health", { signal: AbortSignal.timeout(1500) });
    if (resp.ok) return { rpc: LOCAL_RPC, name: "Local Testnet" };
  } catch {}
  return { rpc: PUBLIC_RPC, name: "Testnet" };
}

async function init() {
  try {
    const mod = await import("./wasm/aztibase_wasm.js");
    await mod.default();
    wasm = mod;
  } catch (e) {
    console.error("WASM load failed:", e);
  }

  await twofa.load();

  const stored = await storageGet(["network", "encryptedKey"]);
  if (stored.network?.rpc) {
    rpcUrl = stored.network.rpc;
  } else {
    const detected = await detectRpc();
    rpcUrl = detected.rpc;
  }
  const rpcInput = document.getElementById("settings-rpc");
  if (rpcInput) rpcInput.value = rpcUrl;
  const netLabel = rpcUrl.includes("127.0.0.1") ? "Local Testnet" : (stored.network?.name || "Testnet");
  const netEl = document.getElementById("network-name");
  if (netEl) netEl.textContent = netLabel;

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
  } else if (name === "2fa-setup") {
    // Reflect current state
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
  const stored = localStorage.getItem("encryptedKey") ||
    (chrome?.storage?.local ? "check" : null);
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
  if (tabId === "staking") refreshStaking();
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
  secretHex = null;
  addressHex = null;
  await storageRemove(["encryptedKey", "address"]);
  await sessionRemove(["sessionSecret", "sessionAddress"]);
  showView("onboard");
  toast("Wallet reset", "info");
}

function updateMainView() {
  if (addressHex) {
    document.getElementById("main-address").textContent = shortenAddress(addressHex);
    refreshBalance();
    refreshStaking();
  }
}

// --- RPC calls ---

async function rpcCall(method, params) {
  const body = JSON.stringify({
    jsonrpc: "2.0",
    method,
    params,
    id: Date.now(),
  });
  const resp = await fetch(rpcUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body,
  });
  const json = await resp.json();
  if (json.error) throw new Error(json.error.message || JSON.stringify(json.error));
  return json.result;
}

async function refreshBalance() {
  if (!addressHex) return;
  try {
    const balance = await rpcCall("aztb_getBalance", [addressHex]);
    const display = fromBaseUnits(balance || "0");
    document.getElementById("main-balance").textContent = display;
  } catch (e) {
    console.warn("Balance fetch failed:", e);
  }
}

async function refreshNonce() {
  if (!addressHex) return;
  try {
    const nonce = await rpcCall("aztb_getNonce", [addressHex]);
    currentNonce = parseInt(nonce, 10) || 0;
  } catch (e) {
    console.warn("Nonce fetch failed:", e);
  }
}

async function refreshStaking() {
  if (!addressHex) return;
  try {
    const info = await rpcCall("aztb_getValidatorStake", [addressHex]);
    if (info && typeof info === "object") {
      const self_stake = info.self_stake || info.selfStake || 0;
      const delegated = info.total_delegated || info.totalDelegated || 0;
      document.getElementById("staking-self").textContent = fromBaseUnits(String(self_stake)) + " AZTB";
      const delEl = document.getElementById("staking-delegated");
      if (delEl) delEl.textContent = fromBaseUnits(String(delegated)) + " AZTB";
    } else if (info && info !== "0") {
      document.getElementById("staking-self").textContent = fromBaseUnits(String(info)) + " AZTB";
    }
  } catch {
    // staking info may not be available
  }

  try {
    const rewards = await rpcCall("aztb_getEpochRewards", [10]);
    const rewardsEl = document.getElementById("staking-rewards");
    if (rewardsEl && Array.isArray(rewards)) {
      const addrClean = addressHex.replace(/^0x/, '').toLowerCase();
      let total = 0n;
      for (const evt of rewards) {
        for (const c of (evt.credits || [])) {
          const cAddr = (c.validator || '').replace(/^0x/, '').toLowerCase();
          if (cAddr === addrClean) total += BigInt(c.amount);
        }
      }
      if (total > 0n) {
        rewardsEl.textContent = fromBaseUnits(total.toString()) + " AZTB";
      } else {
        rewardsEl.textContent = "0 AZTB";
      }
    }
  } catch {
    // rewards may not be available yet
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

async function sendStake() {
  if (!wasm || !secretHex) { toast("Wallet not ready", "error"); return; }

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
    toast(`Staked! TX: ${txHash.slice(0, 16)}...`, "success");
    addTxToHistory("Stake", addressHex, amount, "stake");
    refreshBalance();
    refreshStaking();
    showView("main");
  } catch (e) {
    toast(`Stake failed: ${e.message}`, "error");
  }
}

async function sendUnstake() {
  if (!wasm || !secretHex) { toast("Wallet not ready", "error"); return; }

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
    toast(`Unstaked! TX: ${txHash.slice(0, 16)}...`, "success");
    addTxToHistory("Unstake", addressHex, amount, "stake");
    showView("main");
  } catch (e) {
    toast(`Unstake failed: ${e.message}`, "error");
  }
}

async function sendDelegate() {
  if (!wasm || !secretHex) { toast("Wallet not ready", "error"); return; }

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
    toast(`Delegated! TX: ${txHash.slice(0, 16)}...`, "success");
    addTxToHistory("Delegate", validator, amount, "stake");
    showView("main");
  } catch (e) {
    toast(`Delegate failed: ${e.message}`, "error");
  }
}

async function sendUndelegate() {
  if (!wasm || !secretHex) { toast("Wallet not ready", "error"); return; }

  const authed = await require2FA();
  if (!authed) return;

  try {
    await refreshNonce();
    const signedHex = wasm.signUndelegate(secretHex, BigInt(currentNonce), 1n);
    if (signedHex.startsWith("error:")) { toast(signedHex, "error"); return; }

    const txHash = await rpcCall("aztb_sendRawTransaction", [signedHex]);
    toast(`Undelegated! TX: ${txHash.slice(0, 16)}...`, "success");
    addTxToHistory("Undelegate", addressHex, "--", "stake");
    showView("main");
  } catch (e) {
    toast(`Undelegate failed: ${e.message}`, "error");
  }
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

async function saveSettings() {
  rpcUrl = document.getElementById("settings-rpc").value.trim();
  await storageSet({
    network: {
      name: rpcUrl.includes("rpc.aztibase.com") ? "Testnet" : "Custom",
      rpc: rpcUrl,
      chainId: "0xA27B",
    },
  });
  document.getElementById("network-name").textContent =
    rpcUrl.includes("rpc.aztibase.com") ? "Testnet" : "Custom";
  chrome.runtime?.sendMessage?.({ type: "network_changed", rpc: rpcUrl }).catch(() => {});
  toast("Settings saved", "success");
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
  toast("Requesting faucet drip...", "info");
  try {
    const result = await rpcCall("aztb_faucetDrip", [addressHex]);
    if (result && result.amount) {
      toast("Received " + fromBaseUnits(String(result.amount)) + " AZTB", "success");
      addTxToHistory("Faucet Drip", "faucet", fromBaseUnits(String(result.amount)) + " AZTB");
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
  lockWallet, resetWallet, copyAddress, sendTransfer, sendStake,
  sendUnstake, sendDelegate, sendUndelegate, saveSettings, exportKey,
  startTotpSetup, confirmTotpSetup, startWebAuthnSetup, disable2FA,
  verify2FAWebAuthn, verify2FATotp, cancel2FA,
  signInGoogle, signInGitHub, completeSocialAuth, refreshAll, walletFaucet,
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
