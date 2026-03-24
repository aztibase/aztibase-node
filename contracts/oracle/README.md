# Aztibase Price Feed Oracle

On-chain price feeds for the Aztibase Network.

## Architecture

```
CoinGecko API ──→ Updater Script ──→ PriceFeed.sol (on-chain)
                                          │
                    DEX / Lending / Any contract reads via getPrice()
```

Phase 1 (current): Trusted updater pushes prices from CoinGecko.
Phase 2 (future): RedStone signed data packages verified on-chain.

## Contract: PriceFeed.sol

Solidity ^0.8.20, deployed on Aztibase EVM.

### Read Prices

```solidity
interface IPriceFeed {
    function getPrice(bytes32 feedId) external view returns (uint256 price, uint256 timestamp, uint256 blockNumber);
    function getFreshPrice(bytes32 feedId, uint256 maxAge) external view returns (uint256 price, uint256 timestamp);
    function getAllFeeds() external view returns (bytes32[] memory);
    function DECIMALS() external view returns (uint8); // always 8
}
```

### Feed IDs

| Feed | Description |
|------|-------------|
| `keccak256("AZTB/USD")` | AZTB token price in USD |
| `keccak256("ETH/USD")` | Ethereum price in USD |
| `keccak256("BTC/USD")` | Bitcoin price in USD |
| `keccak256("USDC/USD")` | USDC peg (should be ~1.0) |

### Price Format

All prices are scaled to 8 decimals:
- `$3,500.00` = `350000000000`
- `$0.001` = `100000`
- `$1.00` = `100000000`

## Scripts

### updater.ts

Fetches prices from CoinGecko and pushes to the on-chain contract.

```bash
AZTB_RPC=http://127.0.0.1:9944 \
ORACLE_ADDRESS=0x... \
UPDATER_ADDRESS=0x... \
UPDATE_INTERVAL=60 \
npx tsx scripts/updater.ts
```

### deploy.ts

Deployment helper — shows feed IDs and deployment instructions.

```bash
AZTB_RPC=http://127.0.0.1:9944 \
AZTB_DEPLOYER=0x... \
npx tsx scripts/deploy.ts
```

## Compile

```bash
solc --bin --abi --optimize PriceFeed.sol -o build/
```

## License

MIT
