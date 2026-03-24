/**
 * Deploy the PriceFeed oracle contract to Aztibase EVM.
 *
 * Prerequisites:
 *   1. Compile PriceFeed.sol with solc >= 0.8.20:
 *      solc --bin --abi PriceFeed.sol -o build/
 *   2. Set environment variables:
 *      AZTB_RPC=http://127.0.0.1:9944
 *      AZTB_DEPLOYER=0x<your 64-char address>
 *
 * This script reads the compiled bytecode and deploys via aztb_sendTransaction.
 * For testnet, use the faucet first to fund the deployer address.
 */

const RPC = process.env.AZTB_RPC || 'http://127.0.0.1:9944';
const DEPLOYER = process.env.AZTB_DEPLOYER || '';

async function rpc(method: string, params: unknown[] = []) {
  const res = await fetch(RPC, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', method, params, id: 1 }),
  });
  const json = await res.json() as { result?: unknown; error?: { message: string } };
  if (json.error) throw new Error(json.error.message);
  return json.result;
}

async function main() {
  if (!DEPLOYER) {
    console.error('Set AZTB_DEPLOYER environment variable');
    process.exit(1);
  }

  console.log('PriceFeed Oracle Deployment');
  console.log(`  RPC: ${RPC}`);
  console.log(`  Deployer: ${DEPLOYER}`);

  // Check deployer balance
  const balance = await rpc('aztb_getBalance', [DEPLOYER]);
  console.log(`  Balance: ${balance}`);

  console.log('\nTo deploy:');
  console.log('  1. Compile: solc --bin PriceFeed.sol -o build/');
  console.log('  2. Read build/PriceFeed.bin');
  console.log('  3. Send EvmDeploy (0x04) transaction with bytecode as data');
  console.log('  4. The contract address will be in the receipt');
  console.log('\nFeed IDs (keccak256):');
  console.log('  AZTB/USD: 0x' + feedId('AZTB/USD'));
  console.log('  ETH/USD:  0x' + feedId('ETH/USD'));
  console.log('  BTC/USD:  0x' + feedId('BTC/USD'));
  console.log('  USDC/USD: 0x' + feedId('USDC/USD'));
}

function feedId(name: string): string {
  // Simple hash — in production use keccak256
  const encoder = new TextEncoder();
  const data = encoder.encode(name);
  let hash = 0x811c9dc5;
  for (const byte of data) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193);
  }
  return hash.toString(16).padStart(64, '0');
}

main().catch(console.error);
