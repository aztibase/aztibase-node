#!/usr/bin/env node
/**
 * Aztibase Contract Deployer
 *
 * Compiles, signs, and deploys EVM contracts to the Aztibase testnet.
 * Uses Ed25519 signing with the AZTB transaction envelope format.
 *
 * Usage:
 *   node deploy.mjs --contract <name> --key <hex-secret> [--rpc <url>] [--gas <limit>]
 *
 * Examples:
 *   node deploy.mjs --contract oracle  --key $DEPLOYER_KEY
 *   node deploy.mjs --contract dex-all --key $DEPLOYER_KEY --rpc http://102.209.21.247:9944
 */

import { readFileSync, writeFileSync, existsSync } from 'fs';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

const RPC = getArg('--rpc') || process.env.AZTB_RPC || 'http://102.209.21.247:9944';
const GAS_LIMIT = parseInt(getArg('--gas') || '3000000', 10);
const GAS_PRICE = 1;
const CONTRACT = getArg('--contract') || '';
const SECRET_KEY = getArg('--key') || process.env.AZTB_DEPLOYER_KEY || '';

const TX_DOMAIN = new TextEncoder().encode('AZTB_TX_V1');
const ENVELOPE_MAGIC = 0xAA;
const PREFIX_EVM_DEPLOY = 0x04;
const PREFIX_EVM_CALL = 0x05;
const EVM_DEPLOY_VARIANT = 3; // 0-indexed in TxKind enum

// ── Postcard varint encoding ────────────────────────────────────────

function encodeVarint(n) {
  if (typeof n === 'bigint') {
    const bytes = [];
    while (n > 0x7Fn) {
      bytes.push(Number(n & 0x7Fn) | 0x80);
      n >>= 7n;
    }
    bytes.push(Number(n));
    return new Uint8Array(bytes);
  }
  const bytes = [];
  let val = n >>> 0;
  while (val > 0x7F) {
    bytes.push((val & 0x7F) | 0x80);
    val >>>= 7;
  }
  bytes.push(val);
  return new Uint8Array(bytes);
}

function encodeVarint64(n) {
  return encodeVarint(BigInt(n));
}

function encodeVarint128(n) {
  return encodeVarint(BigInt(n));
}

function encodeBytes(data) {
  const len = encodeVarint(data.length);
  const out = new Uint8Array(len.length + data.length);
  out.set(len);
  out.set(data, len.length);
  return out;
}

// ── Postcard encode TxKind::EvmDeploy ───────────────────────────────

function encodeEvmDeploy(deployer, code, nonce, gasLimit, gasPrice) {
  const parts = [
    encodeVarint(EVM_DEPLOY_VARIANT),
    deployer,                          // [u8; 32] — fixed, no length prefix
    encodeBytes(code),                 // Vec<u8>  — varint length + bytes
    encodeVarint64(nonce),             // u64      — varint
    encodeVarint64(gasLimit),          // u64      — varint
    encodeVarint64(gasPrice),          // u64      — varint
  ];
  let total = 0;
  for (const p of parts) total += p.length;
  const buf = new Uint8Array(total);
  let offset = 0;
  for (const p of parts) {
    buf.set(p, offset);
    offset += p.length;
  }
  return buf;
}

// ── Postcard encode TxKind::EvmCall ─────────────────────────────────

const EVM_CALL_VARIANT = 4;

function encodeEvmCall(caller, contract, calldata, nonce, gasLimit, value, gasPrice) {
  const parts = [
    encodeVarint(EVM_CALL_VARIANT),
    caller,
    contract,
    encodeBytes(calldata),
    encodeVarint64(nonce),
    encodeVarint64(gasLimit),
    encodeVarint128(value),
    encodeVarint64(gasPrice),
  ];
  let total = 0;
  for (const p of parts) total += p.length;
  const buf = new Uint8Array(total);
  let offset = 0;
  for (const p of parts) {
    buf.set(p, offset);
    offset += p.length;
  }
  return buf;
}

// ── Build payload with prefix ───────────────────────────────────────

function buildPayload(postcardEncoded, prefix) {
  const buf = new Uint8Array(1 + postcardEncoded.length);
  buf[0] = prefix;
  buf.set(postcardEncoded, 1);
  return buf;
}

// ── Ed25519 signing (pure JS, minimal) ──────────────────────────────

let ed25519;

async function loadEd25519() {
  try {
    ed25519 = await import('@noble/ed25519');
  } catch {
    console.error('Missing @noble/ed25519. Install with: npm install @noble/ed25519');
    process.exit(1);
  }
}

async function signPayload(payload, secretKeyHex) {
  const secretKey = hexToBytes(secretKeyHex);
  const publicKey = await ed25519.getPublicKeyAsync(secretKey);

  const msg = new Uint8Array(TX_DOMAIN.length + payload.length);
  msg.set(TX_DOMAIN);
  msg.set(payload, TX_DOMAIN.length);

  const signature = await ed25519.signAsync(msg, secretKey);
  return { publicKey, signature };
}

function buildEnvelope(payload, publicKey, signature) {
  const payloadLen = new Uint8Array(4);
  new DataView(payloadLen.buffer).setUint32(0, payload.length, true);

  const total = 1 + 4 + payload.length + 32 + 64;
  const buf = new Uint8Array(total);
  let off = 0;
  buf[off++] = ENVELOPE_MAGIC;
  buf.set(payloadLen, off); off += 4;
  buf.set(payload, off); off += payload.length;
  buf.set(publicKey, off); off += 32;
  buf.set(signature, off);
  return buf;
}

// ── BLAKE3 address derivation ───────────────────────────────────────

let blake3Hash;

async function loadBlake3() {
  const mod = await import('@noble/hashes/blake3.js');
  blake3Hash = mod.blake3;
}

function addressFromPubkey(pubkey) {
  return blake3Hash(pubkey);
}

// ── RPC helpers ─────────────────────────────────────────────────────

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

async function getNonce(addressHex) {
  const result = await rpc('aztb_getNonce', [addressHex]);
  return typeof result === 'string' ? parseInt(result, 16) : Number(result);
}

async function getBalance(addressHex) {
  return rpc('aztb_getBalance', [addressHex]);
}

async function sendTx(envelopeHex) {
  return rpc('aztb_sendTransaction', [envelopeHex]);
}

async function getReceipt(txHash) {
  return rpc('aztb_getTransactionReceipt', [txHash]);
}

async function waitForReceipt(txHash, timeoutMs = 30000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const receipt = await getReceipt(txHash);
    if (receipt) return receipt;
    await new Promise(r => setTimeout(r, 1000));
  }
  throw new Error(`Timeout waiting for receipt of ${txHash}`);
}

// ── Contract bytecode loading ───────────────────────────────────────

function loadBytecode(name, buildDir) {
  const binPath = resolve(buildDir, `${name}.bin`);
  if (!existsSync(binPath)) {
    throw new Error(`Bytecode not found: ${binPath}`);
  }
  const hex = readFileSync(binPath, 'utf-8').trim();
  return hexToBytes(hex);
}

function loadAbi(name, buildDir) {
  const abiPath = resolve(buildDir, `${name}.abi`);
  if (!existsSync(abiPath)) return null;
  return JSON.parse(readFileSync(abiPath, 'utf-8'));
}

// ── Deploy a single contract ────────────────────────────────────────

async function deployContract(name, code, secretKey, nonce) {
  const publicKey = await ed25519.getPublicKeyAsync(hexToBytes(secretKey));
  const deployerAddr = addressFromPubkeyHex(publicKey);

  console.log(`\n  Deploying ${name}...`);
  console.log(`    Bytecode: ${code.length} bytes`);
  console.log(`    Nonce: ${nonce}`);
  console.log(`    Gas limit: ${GAS_LIMIT}`);

  const postcard = encodeEvmDeploy(
    hexToBytes(deployerAddr.replace('0x', '')),
    code,
    nonce,
    GAS_LIMIT,
    GAS_PRICE,
  );
  const payload = buildPayload(postcard, PREFIX_EVM_DEPLOY);
  const { publicKey: pk, signature } = await signPayload(payload, secretKey);
  const envelope = buildEnvelope(payload, pk, signature);
  const envelopeHex = '0x' + bytesToHex(envelope);

  const txHash = await sendTx(envelopeHex);
  console.log(`    Tx hash: ${txHash}`);

  const receipt = await waitForReceipt(txHash);
  const contractAddr = receipt.contract_address || receipt.contractAddress;
  const success = receipt.success !== false && receipt.status !== 0;

  if (success && contractAddr) {
    console.log(`    Contract: ${contractAddr}`);
  } else {
    console.log(`    FAILED:`, JSON.stringify(receipt, null, 2));
    throw new Error(`Deployment of ${name} failed`);
  }

  return { name, txHash, contractAddr, receipt };
}

// ── Call a contract (for initialization) ────────────────────────────

async function callContract(contractAddr, calldata, secretKey, nonce, value = 0) {
  const publicKey = await ed25519.getPublicKeyAsync(hexToBytes(secretKey));
  const callerAddr = addressFromPubkeyHex(publicKey);

  const postcard = encodeEvmCall(
    hexToBytes(callerAddr.replace('0x', '')),
    hexToBytes(contractAddr.replace('0x', '')),
    calldata,
    nonce,
    GAS_LIMIT,
    value,
    GAS_PRICE,
  );
  const payload = buildPayload(postcard, PREFIX_EVM_CALL);
  const { publicKey: pk, signature } = await signPayload(payload, secretKey);
  const envelope = buildEnvelope(payload, pk, signature);
  const envelopeHex = '0x' + bytesToHex(envelope);

  const txHash = await sendTx(envelopeHex);
  const receipt = await waitForReceipt(txHash);
  return { txHash, receipt };
}

// ── ABI encoding helpers ────────────────────────────────────────────

function encodeSelector(sig) {
  // keccak256 of function signature, first 4 bytes
  // We use a minimal keccak256 implementation
  return keccak256(new TextEncoder().encode(sig)).slice(0, 4);
}

function encodeAddress(addrHex) {
  const raw = addrHex.replace('0x', '').replace(/0+$/, '');
  const padded = raw.padStart(64, '0');
  return hexToBytes(padded);
}

function encodeUint256(n) {
  const hex = BigInt(n).toString(16).padStart(64, '0');
  return hexToBytes(hex);
}

function abiEncode(selector, ...params) {
  let total = 4;
  for (const p of params) total += p.length;
  const buf = new Uint8Array(total);
  buf.set(selector);
  let off = 4;
  for (const p of params) {
    buf.set(p, off);
    off += p.length;
  }
  return buf;
}

// ── Keccak256 (minimal, for ABI selectors) ──────────────────────────

let keccakFn;

async function loadKeccak() {
  const mod = await import('@noble/hashes/sha3.js');
  keccakFn = mod.keccak_256;
}

function keccak256(data) {
  return keccakFn(data);
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

function addressFromPubkeyHex(pubkey) {
  const addr = addressFromPubkey(pubkey);
  if (addr) return '0x' + bytesToHex(addr);
  // Fallback: compute BLAKE3 manually (should not reach here in production)
  throw new Error('BLAKE3 not available for address derivation');
}

// ── CLI helpers ─────────────────────────────────────────────────────

function getArg(flag) {
  const idx = process.argv.indexOf(flag);
  if (idx === -1 || idx + 1 >= process.argv.length) return null;
  return process.argv[idx + 1];
}

// ── Deploy plans ────────────────────────────────────────────────────

const DEPLOY_PLANS = {
  oracle: {
    steps: [
      { name: 'PriceFeed', buildDir: resolve(__dirname, 'oracle/build') },
    ],
  },
  wasztb: {
    steps: [
      { name: 'WASZTB', buildDir: resolve(__dirname, 'dex/build') },
    ],
  },
  'dex-all': {
    steps: [
      { name: 'WASZTB',          buildDir: resolve(__dirname, 'dex/build') },
      { name: 'AztibaseFactory', buildDir: resolve(__dirname, 'dex/build') },
      { name: 'AztibaseRouter',  buildDir: resolve(__dirname, 'dex/build'), ctorArgs: ['AztibaseFactory', 'WASZTB'] },
    ],
  },
  router: {
    steps: [
      { name: 'AztibaseRouter', buildDir: resolve(__dirname, 'dex/build'), ctorArgs: ['AztibaseFactory', 'WASZTB'] },
    ],
  },
  tusdc: {
    steps: [
      { name: 'TestUSDC', buildDir: resolve(__dirname, 'dex/build') },
    ],
  },
  all: {
    steps: [
      { name: 'PriceFeed',       buildDir: resolve(__dirname, 'oracle/build') },
      { name: 'WASZTB',          buildDir: resolve(__dirname, 'dex/build') },
      { name: 'AztibaseFactory', buildDir: resolve(__dirname, 'dex/build') },
      { name: 'AztibaseRouter',  buildDir: resolve(__dirname, 'dex/build'), ctorArgs: ['AztibaseFactory', 'WASZTB'] },
      { name: 'TestUSDC',        buildDir: resolve(__dirname, 'dex/build') },
    ],
  },
};

// ── Main ────────────────────────────────────────────────────────────

async function main() {
  await loadEd25519();
  await loadBlake3();
  await loadKeccak();

  if (!CONTRACT || !DEPLOY_PLANS[CONTRACT]) {
    console.log('Aztibase Contract Deployer');
    console.log('Available contracts:', Object.keys(DEPLOY_PLANS).join(', '));
    console.log('\nUsage: node deploy.mjs --contract <name> --key <hex-secret>');
    console.log('       node deploy.mjs --contract all --key <hex-secret>');
    process.exit(0);
  }

  if (!SECRET_KEY) {
    console.error('Error: --key <hex-secret> or AZTB_DEPLOYER_KEY required');
    process.exit(1);
  }

  const publicKey = await ed25519.getPublicKeyAsync(hexToBytes(SECRET_KEY));
  const deployerAddr = addressFromPubkeyHex(publicKey);
  const plan = DEPLOY_PLANS[CONTRACT];

  console.log('Aztibase Contract Deployer');
  console.log(`  RPC:      ${RPC}`);
  console.log(`  Deployer: ${deployerAddr}`);
  console.log(`  Plan:     ${CONTRACT} (${plan.steps.length} contracts)`);

  const balance = await getBalance(deployerAddr);
  console.log(`  Balance:  ${balance}`);

  let nonce = await getNonce(deployerAddr);
  console.log(`  Nonce:    ${nonce}`);

  const addressFile = resolve(__dirname, 'deployed-addresses.json');
  const network = RPC.includes('102.209') ? 'testnet' : 'local';
  const existing = existsSync(addressFile) ? JSON.parse(readFileSync(addressFile, 'utf-8')) : {};
  const deployed = { ...(existing[network] || {}) };
  delete deployed.deployedAt;
  delete deployed.deployer;

  for (const step of plan.steps) {
    let code = loadBytecode(step.name, step.buildDir);

    if (step.ctorArgs) {
      const args = step.ctorArgs.map(ref => {
        if (deployed[ref]) return deployed[ref];
        throw new Error(`${step.name} needs ${ref} address but it was not deployed yet`);
      });
      const encodedArgs = args.map(addr => encodeAddress(addr.replace('0x', '')));
      const argBytes = new Uint8Array(encodedArgs.length * 32);
      let off = 0;
      for (const enc of encodedArgs) {
        argBytes.set(enc, off);
        off += 32;
      }
      const combined = new Uint8Array(code.length + argBytes.length);
      combined.set(code);
      combined.set(argBytes, code.length);
      code = combined;
    }

    const result = await deployContract(step.name, code, SECRET_KEY, nonce);
    deployed[step.name] = result.contractAddr;
    nonce++;
  }

  console.log('\n  Deployment Summary');
  console.log('  ─────────────────');
  for (const [name, addr] of Object.entries(deployed)) {
    console.log(`  ${name.padEnd(20)} ${addr}`);
  }

  const saved = existsSync(addressFile) ? JSON.parse(readFileSync(addressFile, 'utf-8')) : {};
  saved[network] = {
    ...saved[network],
    ...deployed,
    deployedAt: new Date().toISOString(),
    deployer: deployerAddr,
  };
  writeFileSync(addressFile, JSON.stringify(saved, null, 2) + '\n');
  console.log(`\n  Addresses saved to ${addressFile}`);
}

main().catch(err => {
  console.error('\nDeploy failed:', err.message);
  process.exit(1);
});
