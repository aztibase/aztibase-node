#!/usr/bin/env node
/**
 * Seed initial liquidity into the WASZTB/tUSDC pair on Aztibase testnet.
 *
 * Steps:
 *   1. Wrap native AZTB → WASZTB via deposit() with value
 *   2. Approve Router to spend WASZTB
 *   3. Approve Router to spend tUSDC
 *   4. addLiquidity(WASZTB, tUSDC, ...) via Router
 */

import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import { getPublicKeyAsync, signAsync } from '@noble/ed25519';
import { blake3 } from '@noble/hashes/blake3.js';
import { keccak_256 } from '@noble/hashes/sha3.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const addresses = JSON.parse(readFileSync(resolve(__dirname, 'deployed-addresses.json'), 'utf-8'));
const testnet = addresses.testnet;

const RPC = process.env.AZTB_RPC || 'http://102.209.21.247:9944';
const SECRET = process.env.AZTB_DEPLOYER_KEY || 'dbe91028bf5a3f64c5b095bda345ee6402d0ca8654161228859d616cbcd995c3';
const GAS_PRICE = 1;

// Liquidity amounts
const WASZTB_AMOUNT = 100_000n;   // 100K WASZTB (wrap this much native AZTB)
const TUSDC_AMOUNT = 100_000n * 1_000_000n;  // 100K tUSDC (6 decimals = 100_000_000_000)

const PREFIX_EVM_CALL = 0x05;
const EVM_CALL_VARIANT = 4;
const TX_DOMAIN = new TextEncoder().encode('AZTB_TX_V1');
const ENVELOPE_MAGIC = 0xAA;

// ── Hex helpers ─────────────────────────────────────────────────────

function hexToBytes(h) {
  h = h.replace(/^0x/, '');
  const b = new Uint8Array(h.length / 2);
  for (let i = 0; i < b.length; i++) b[i] = parseInt(h.slice(i * 2, i * 2 + 2), 16);
  return b;
}

function bytesToHex(b) {
  return Array.from(b).map(x => x.toString(16).padStart(2, '0')).join('');
}

// ── Postcard encoding ───────────────────────────────────────────────

function encodeVarint(n) {
  n = BigInt(n);
  const b = [];
  while (n > 0x7Fn) { b.push(Number(n & 0x7Fn) | 0x80); n >>= 7n; }
  b.push(Number(n));
  return new Uint8Array(b);
}

function encodeBytes(d) {
  const l = encodeVarint(d.length);
  const o = new Uint8Array(l.length + d.length);
  o.set(l);
  o.set(d, l.length);
  return o;
}

function concatBytes(...arrays) {
  let total = 0;
  for (const a of arrays) total += a.length;
  const buf = new Uint8Array(total);
  let off = 0;
  for (const a of arrays) { buf.set(a, off); off += a.length; }
  return buf;
}

// ── ABI encoding ────────────────────────────────────────────────────

function selector(sig) {
  return keccak_256(new TextEncoder().encode(sig)).slice(0, 4);
}

function abiAddress(addr32hex) {
  // Our 32-byte addresses: first 20 bytes are the EVM address
  const full = hexToBytes(addr32hex.replace(/^0x/, ''));
  const padded = new Uint8Array(32);
  // ABI: left-pad 20-byte address to 32 bytes
  padded.set(full.slice(0, 20), 12);
  return padded;
}

function abiUint256(n) {
  const hex = BigInt(n).toString(16).padStart(64, '0');
  return hexToBytes(hex);
}

// ── Transaction building ────────────────────────────────────────────

async function buildEvmCall(caller32, contract32, calldata, nonce, gasLimit, value) {
  const postcard = concatBytes(
    encodeVarint(EVM_CALL_VARIANT),
    caller32,
    contract32,
    encodeBytes(calldata),
    encodeVarint(nonce),
    encodeVarint(gasLimit),
    encodeVarint(value),
    encodeVarint(GAS_PRICE),
  );
  const payload = new Uint8Array(1 + postcard.length);
  payload[0] = PREFIX_EVM_CALL;
  payload.set(postcard, 1);
  return payload;
}

async function signAndSend(payload, secretKeyBytes) {
  const pub = await getPublicKeyAsync(secretKeyBytes);
  const msg = concatBytes(TX_DOMAIN, payload);
  const sig = await signAsync(msg, secretKeyBytes);

  const payloadLen = new Uint8Array(4);
  new DataView(payloadLen.buffer).setUint32(0, payload.length, true);

  const envelope = concatBytes(
    new Uint8Array([ENVELOPE_MAGIC]),
    payloadLen,
    payload,
    pub,
    sig,
  );

  const hex = '0x' + bytesToHex(envelope);
  const res = await fetch(RPC, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', method: 'aztb_sendTransaction', params: [hex], id: 1 }),
  });
  const json = await res.json();
  if (json.error) throw new Error(`RPC: ${json.error.message}`);
  return json.result;
}

async function waitReceipt(txHash, label) {
  for (let i = 0; i < 30; i++) {
    await new Promise(r => setTimeout(r, 1000));
    const r2 = await fetch(RPC, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ jsonrpc: '2.0', method: 'aztb_getTransactionReceipt', params: [txHash], id: 2 }),
    });
    const receipt = (await r2.json()).result;
    if (receipt) {
      const gas = parseInt(receipt.gasUsed, 16);
      if (receipt.success) {
        console.log(`    ${label}: OK (gas: ${gas})`);
      } else {
        console.log(`    ${label}: FAILED (gas: ${gas}, error: ${receipt.error || 'unknown'})`);
      }
      return receipt;
    }
  }
  throw new Error(`Timeout waiting for ${label}`);
}

// ── Main ────────────────────────────────────────────────────────────

async function main() {
  const keyBytes = hexToBytes(SECRET);
  const pub = await getPublicKeyAsync(keyBytes);
  const callerAddr = blake3(pub);
  const callerHex = '0x' + bytesToHex(callerAddr);

  const wasztb32 = hexToBytes(testnet.WASZTB.replace(/^0x/, ''));
  const router32 = hexToBytes(testnet.AztibaseRouter.replace(/^0x/, ''));
  const tusdc32 = hexToBytes(testnet.TestUSDC.replace(/^0x/, ''));

  console.log('Aztibase Liquidity Seeder');
  console.log(`  Deployer: ${callerHex}`);
  console.log(`  WASZTB:   ${testnet.WASZTB.slice(0, 18)}...`);
  console.log(`  tUSDC:    ${testnet.TestUSDC.slice(0, 18)}...`);
  console.log(`  Router:   ${testnet.AztibaseRouter.slice(0, 18)}...`);
  console.log(`  WASZTB amount: ${WASZTB_AMOUNT}`);
  console.log(`  tUSDC amount:  ${TUSDC_AMOUNT}`);

  let nonce = 9;

  // Step 1: Wrap AZTB → WASZTB via deposit() with value
  console.log('\n  Step 1: Wrap AZTB → WASZTB');
  const depositCall = selector('deposit()');
  const payload1 = await buildEvmCall(callerAddr, wasztb32, depositCall, nonce, 200000, WASZTB_AMOUNT);
  const tx1 = await signAndSend(payload1, keyBytes);
  console.log(`    Tx: ${tx1}`);
  const r1 = await waitReceipt(tx1, 'deposit');
  if (!r1.success) { console.log('    Aborting'); return; }
  nonce++;

  // Step 2: Approve Router to spend WASZTB
  console.log('\n  Step 2: Approve Router for WASZTB');
  const approveWasztb = concatBytes(
    selector('approve(address,uint256)'),
    abiAddress(testnet.AztibaseRouter),
    abiUint256(WASZTB_AMOUNT),
  );
  const payload2 = await buildEvmCall(callerAddr, wasztb32, approveWasztb, nonce, 100000, 0n);
  const tx2 = await signAndSend(payload2, keyBytes);
  console.log(`    Tx: ${tx2}`);
  const r2 = await waitReceipt(tx2, 'approve WASZTB');
  if (!r2.success) { console.log('    Aborting'); return; }
  nonce++;

  // Step 3: Approve Router to spend tUSDC
  console.log('\n  Step 3: Approve Router for tUSDC');
  const approveTusdc = concatBytes(
    selector('approve(address,uint256)'),
    abiAddress(testnet.AztibaseRouter),
    abiUint256(TUSDC_AMOUNT),
  );
  const payload3 = await buildEvmCall(callerAddr, tusdc32, approveTusdc, nonce, 100000, 0n);
  const tx3 = await signAndSend(payload3, keyBytes);
  console.log(`    Tx: ${tx3}`);
  const r3 = await waitReceipt(tx3, 'approve tUSDC');
  if (!r3.success) { console.log('    Aborting'); return; }
  nonce++;

  // Step 4: addLiquidity via Router
  console.log('\n  Step 4: addLiquidity (WASZTB/tUSDC)');
  const deadline = BigInt(Math.floor(Date.now() / 1000) + 3600); // 1 hour
  const addLiq = concatBytes(
    selector('addLiquidity(address,address,uint256,uint256,uint256,uint256,address,uint256)'),
    abiAddress(testnet.WASZTB),       // tokenA
    abiAddress(testnet.TestUSDC),     // tokenB
    abiUint256(WASZTB_AMOUNT),        // amountADesired
    abiUint256(TUSDC_AMOUNT),         // amountBDesired
    abiUint256(0),                    // amountAMin (0 for first deposit)
    abiUint256(0),                    // amountBMin (0 for first deposit)
    abiAddress(callerHex + '000000000000000000000000'),  // to (deployer, padded to 32 bytes)
    abiUint256(deadline),             // deadline
  );
  const payload4 = await buildEvmCall(callerAddr, router32, addLiq, nonce, 500000, 0n);
  const tx4 = await signAndSend(payload4, keyBytes);
  console.log(`    Tx: ${tx4}`);
  const r4 = await waitReceipt(tx4, 'addLiquidity');

  if (r4.success) {
    console.log('\n  Liquidity seeded successfully!');
    console.log('  Pair: WASZTB/tUSDC');
    console.log(`  WASZTB deposited: ${WASZTB_AMOUNT}`);
    console.log(`  tUSDC deposited:  ${TUSDC_AMOUNT}`);
  } else {
    console.log('\n  addLiquidity failed. Pair may need manual creation.');
  }
}

main().catch(err => {
  console.error('\nError:', err.message);
  process.exit(1);
});
