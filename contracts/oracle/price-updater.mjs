#!/usr/bin/env node
/**
 * Price Feed Updater — fetches prices from CoinGecko and submits to the PriceFeed oracle.
 *
 * Usage:
 *   node price-updater.mjs --key <deployer-hex-secret> [--rpc <url>] [--interval <secs>]
 *
 * Env vars:
 *   AZTB_RPC            — Node RPC URL (default: testnet)
 *   AZTB_DEPLOYER_KEY   — Ed25519 secret key hex
 *   UPDATE_INTERVAL     — Seconds between updates (default: 60)
 */

import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// ── Config ──────────────────────────────────────────────────────────

function getArg(flag) {
  const idx = process.argv.indexOf(flag);
  if (idx === -1 || idx + 1 >= process.argv.length) return null;
  return process.argv[idx + 1];
}

const RPC = getArg('--rpc') || process.env.AZTB_RPC || 'http://102.209.21.247:9944';
const SECRET_KEY = getArg('--key') || process.env.AZTB_DEPLOYER_KEY || '';
const INTERVAL = parseInt(getArg('--interval') || process.env.UPDATE_INTERVAL || '60', 10);

const addressFile = resolve(__dirname, '..', 'deployed-addresses.json');
const addresses = JSON.parse(readFileSync(addressFile, 'utf-8'));
const network = RPC.includes('102.209') ? 'testnet' : 'local';
const ORACLE_ADDRESS = addresses[network]?.PriceFeed;

const COINGECKO_API = 'https://api.coingecko.com/api/v3/simple/price';
const FEEDS = {
  bitcoin:  'BTC/USD',
  ethereum: 'ETH/USD',
};
const DECIMALS = 8;

const TX_DOMAIN = new TextEncoder().encode('AZTB_TX_V1');
const ENVELOPE_MAGIC = 0xAA;
const PREFIX_EVM_CALL = 0x05;
const EVM_CALL_VARIANT = 4;

// ── Crypto loaders ──────────────────────────────────────────────────

let ed25519, blake3Hash, keccakFn;

async function loadCrypto() {
  ed25519 = await import('@noble/ed25519');
  blake3Hash = (await import('@noble/hashes/blake3.js')).blake3;
  keccakFn = (await import('@noble/hashes/sha3.js')).keccak_256;
}

// ── Hex helpers ─────────────────────────────────────────────────────

function hexToBytes(hex) {
  hex = hex.replace(/^0x/, '');
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

function bytesToHex(bytes) {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('');
}

// ── Postcard encoding ───────────────────────────────────────────────

function encodeVarint(n) {
  if (typeof n === 'bigint') {
    const bytes = [];
    while (n > 0x7Fn) { bytes.push(Number(n & 0x7Fn) | 0x80); n >>= 7n; }
    bytes.push(Number(n));
    return new Uint8Array(bytes);
  }
  const bytes = [];
  let val = n >>> 0;
  while (val > 0x7F) { bytes.push((val & 0x7F) | 0x80); val >>>= 7; }
  bytes.push(val);
  return new Uint8Array(bytes);
}

function encodeVarint64(n) { return encodeVarint(BigInt(n)); }
function encodeVarint128(n) { return encodeVarint(BigInt(n)); }

function encodeBytes(data) {
  const len = encodeVarint(data.length);
  const out = new Uint8Array(len.length + data.length);
  out.set(len);
  out.set(data, len.length);
  return out;
}

function concat(...arrays) {
  let total = 0;
  for (const a of arrays) total += a.length;
  const buf = new Uint8Array(total);
  let off = 0;
  for (const a of arrays) { buf.set(a, off); off += a.length; }
  return buf;
}

// ── Transaction building ────────────────────────────────────────────

function encodeEvmCall(caller, contract, calldata, nonce, gasLimit, value, gasPrice) {
  return concat(
    encodeVarint(EVM_CALL_VARIANT),
    caller,
    contract,
    encodeBytes(calldata),
    encodeVarint64(nonce),
    encodeVarint64(gasLimit),
    encodeVarint128(value),
    encodeVarint64(gasPrice),
  );
}

async function signPayload(payload, secretKeyHex) {
  const secretKey = hexToBytes(secretKeyHex);
  const publicKey = await ed25519.getPublicKeyAsync(secretKey);
  const msg = concat(TX_DOMAIN, payload);
  const signature = await ed25519.signAsync(msg, secretKey);
  return { publicKey, signature };
}

function buildEnvelope(payload, publicKey, signature) {
  const payloadLen = new Uint8Array(4);
  new DataView(payloadLen.buffer).setUint32(0, payload.length, true);
  return concat(new Uint8Array([ENVELOPE_MAGIC]), payloadLen, payload, publicKey, signature);
}

// ── ABI encoding ────────────────────────────────────────────────────

function keccak256(data) { return keccakFn(data); }

function selectorOf(sig) {
  return keccak256(new TextEncoder().encode(sig)).slice(0, 4);
}

function encodeUint256(n) {
  return hexToBytes(BigInt(n).toString(16).padStart(64, '0'));
}

function encodeBytes32(hex) {
  return hexToBytes(hex.replace('0x', '').padStart(64, '0'));
}

function feedId(name) {
  const hash = keccak256(new TextEncoder().encode(name));
  return bytesToHex(hash);
}

function encodeUpdatePrices(feedIds, prices, timestamps) {
  const sel = selectorOf('updatePrices(bytes32[],uint256[],uint256[])');
  const n = feedIds.length;

  // Dynamic ABI encoding: 3 array offsets + 3 * (length word + n items)
  const offsetFeeds = 3 * 32;
  const offsetPrices = offsetFeeds + 32 + n * 32;
  const offsetTimestamps = offsetPrices + 32 + n * 32;

  const parts = [
    sel,
    encodeUint256(offsetFeeds),
    encodeUint256(offsetPrices),
    encodeUint256(offsetTimestamps),
    // feedIds array
    encodeUint256(n),
    ...feedIds.map(id => encodeBytes32(id)),
    // prices array
    encodeUint256(n),
    ...prices.map(p => encodeUint256(p)),
    // timestamps array
    encodeUint256(n),
    ...timestamps.map(t => encodeUint256(t)),
  ];
  return concat(...parts);
}

// ── RPC ─────────────────────────────────────────────────────────────

async function rpc(method, params = []) {
  const res = await fetch(RPC, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', method, params, id: 1 }),
  });
  const json = await res.json();
  if (json.error) throw new Error(`RPC ${method}: ${json.error.message}`);
  return json.result;
}

async function getNonce(addrHex) {
  const result = await rpc('aztb_getNonce', [addrHex]);
  return typeof result === 'string' ? parseInt(result, 16) : Number(result);
}

async function sendTx(envelopeHex) {
  return rpc('aztb_sendTransaction', [envelopeHex]);
}

// ── Price fetching ──────────────────────────────────────────────────

async function fetchPrices() {
  const ids = Object.keys(FEEDS).join(',');
  const url = `${COINGECKO_API}?ids=${ids}&vs_currencies=usd`;
  const res = await fetch(url);
  if (!res.ok) throw new Error(`CoinGecko HTTP ${res.status}`);
  const data = await res.json();

  const prices = new Map();
  for (const [coin, feedName] of Object.entries(FEEDS)) {
    if (data[coin]?.usd) prices.set(feedName, data[coin].usd);
  }
  prices.set('AZTB/USD', 0.001);
  prices.set('USDC/USD', 1.0);
  return prices;
}

function scalePrice(price) {
  return BigInt(Math.round(price * 10 ** DECIMALS));
}

// ── Main loop ───────────────────────────────────────────────────────

async function main() {
  await loadCrypto();

  if (!SECRET_KEY) {
    console.error('Error: --key <hex-secret> or AZTB_DEPLOYER_KEY required');
    process.exit(1);
  }
  if (!ORACLE_ADDRESS) {
    console.error(`Error: No PriceFeed address found in deployed-addresses.json for network "${network}"`);
    process.exit(1);
  }

  const publicKey = await ed25519.getPublicKeyAsync(hexToBytes(SECRET_KEY));
  const callerAddr = '0x' + bytesToHex(blake3Hash(publicKey));

  console.log('[updater] Aztibase Price Feed Updater');
  console.log(`[updater] RPC:      ${RPC}`);
  console.log(`[updater] Oracle:   ${ORACLE_ADDRESS.slice(0, 22)}...`);
  console.log(`[updater] Caller:   ${callerAddr.slice(0, 22)}...`);
  console.log(`[updater] Interval: ${INTERVAL}s`);

  let consecutiveErrors = 0;

  while (true) {
    try {
      const prices = await fetchPrices();
      const now = Math.floor(Date.now() / 1000);

      const ids = [];
      const scaledPrices = [];
      const timestamps = [];

      for (const [name, price] of prices) {
        ids.push(feedId(name));
        scaledPrices.push(scalePrice(price));
        timestamps.push(BigInt(now));
        console.log(`  ${name}: $${price.toFixed(4)} (${scalePrice(price)})`);
      }

      const calldata = encodeUpdatePrices(ids, scaledPrices, timestamps);
      const nonce = await getNonce(callerAddr);

      const postcard = encodeEvmCall(
        hexToBytes(callerAddr.replace('0x', '')),
        hexToBytes(ORACLE_ADDRESS.replace('0x', '')),
        calldata,
        nonce,
        500_000,
        0,
        1,
      );
      const payload = concat(new Uint8Array([PREFIX_EVM_CALL]), postcard);
      const { publicKey: pk, signature } = await signPayload(payload, SECRET_KEY);
      const envelope = buildEnvelope(payload, pk, signature);
      const txHash = await sendTx('0x' + bytesToHex(envelope));

      console.log(`  [${new Date().toISOString()}] tx: ${txHash}`);
      consecutiveErrors = 0;
    } catch (e) {
      consecutiveErrors++;
      console.error(`[updater] Error (${consecutiveErrors}): ${e.message}`);
      if (consecutiveErrors >= 10) {
        console.error('[updater] Too many consecutive errors, exiting');
        process.exit(1);
      }
    }

    await new Promise(r => setTimeout(r, INTERVAL * 1000));
  }
}

main().catch(err => {
  console.error('Fatal:', err.message);
  process.exit(1);
});
