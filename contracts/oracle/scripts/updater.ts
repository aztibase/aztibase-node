/**
 * Price Feed Updater — fetches prices from CoinGecko and pushes to the on-chain oracle.
 *
 * Runs as a long-lived process, updating prices every INTERVAL seconds.
 * Uses the free CoinGecko API (no key required, 10-30 req/min).
 *
 * Environment:
 *   AZTB_RPC          — Node RPC URL (default: http://127.0.0.1:9944)
 *   ORACLE_ADDRESS    — Deployed PriceFeed contract address
 *   UPDATER_ADDRESS   — Address authorized as updater on the contract
 *   UPDATE_INTERVAL   — Seconds between updates (default: 60)
 */

const RPC = process.env.AZTB_RPC || 'http://127.0.0.1:9944';
const ORACLE_ADDRESS = process.env.ORACLE_ADDRESS || '';
const UPDATER_ADDRESS = process.env.UPDATER_ADDRESS || '';
const INTERVAL = parseInt(process.env.UPDATE_INTERVAL || '60', 10);

const COINGECKO_API = 'https://api.coingecko.com/api/v3/simple/price';
const FEEDS = ['bitcoin', 'ethereum'];
const FEED_MAP: Record<string, string> = {
  bitcoin: 'BTC/USD',
  ethereum: 'ETH/USD',
};
const DECIMALS = 8;

interface PriceResponse {
  [coin: string]: { usd: number };
}

async function fetchPrices(): Promise<Map<string, number>> {
  const ids = FEEDS.join(',');
  const url = `${COINGECKO_API}?ids=${ids}&vs_currencies=usd`;
  const res = await fetch(url);
  if (!res.ok) throw new Error(`CoinGecko API error: ${res.status}`);
  const data = await res.json() as PriceResponse;

  const prices = new Map<string, number>();
  for (const [coin, vals] of Object.entries(data)) {
    const feedName = FEED_MAP[coin];
    if (feedName && vals.usd) {
      prices.set(feedName, vals.usd);
    }
  }

  // Add synthetic AZTB/USD price (placeholder until real market exists)
  prices.set('AZTB/USD', 0.001);
  // USDC is pegged
  prices.set('USDC/USD', 1.0);

  return prices;
}

function scalePrice(price: number): bigint {
  return BigInt(Math.round(price * 10 ** DECIMALS));
}

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

async function updateLoop() {
  console.log('[updater] Aztibase Price Feed Updater');
  console.log(`[updater] RPC: ${RPC}`);
  console.log(`[updater] Oracle: ${ORACLE_ADDRESS}`);
  console.log(`[updater] Updater: ${UPDATER_ADDRESS}`);
  console.log(`[updater] Interval: ${INTERVAL}s`);
  console.log(`[updater] Feeds: ${FEEDS.map(f => FEED_MAP[f]).join(', ')}, AZTB/USD, USDC/USD`);

  if (!ORACLE_ADDRESS || !UPDATER_ADDRESS) {
    console.log('\n[updater] No ORACLE_ADDRESS or UPDATER_ADDRESS set.');
    console.log('[updater] Running in dry-run mode — fetching prices only.\n');
  }

  while (true) {
    try {
      const prices = await fetchPrices();
      const now = Math.floor(Date.now() / 1000);

      console.log(`[updater] ${new Date().toISOString()}`);
      for (const [feed, price] of prices) {
        const scaled = scalePrice(price);
        console.log(`  ${feed}: $${price.toFixed(4)} (raw: ${scaled})`);
      }

      if (ORACLE_ADDRESS && UPDATER_ADDRESS) {
        // In production: encode updatePrices() calldata and send via aztb_sendTransaction
        // For now, log what would be sent
        console.log(`  → Would call updatePrices() on ${ORACLE_ADDRESS.slice(0, 12)}...`);
      }
    } catch (e) {
      console.error('[updater] Error:', (e as Error).message);
    }

    await new Promise(r => setTimeout(r, INTERVAL * 1000));
  }
}

updateLoop().catch(console.error);
