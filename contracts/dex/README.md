# Aztibase DEX (AMM)

Constant-product AMM (Uniswap V2 pattern) for the Aztibase Network.

## Contracts

| Contract | Purpose |
|----------|---------|
| `WASZTB` | Wrapped AZTB — ERC20 wrapper for native token |
| `AztibaseFactory` | Creates and tracks trading pairs |
| `AztibasePair` | Liquidity pool for one token pair (x*y=k) |
| `AztibaseRouter` | User-facing swap and liquidity operations |
| `IERC20` | Standard ERC20 interface |

## How It Works

```
User → Router.swapExactTokensForTokens()
         → transfers input tokens to Pair
         → Pair.swap() executes constant-product math
         → output tokens sent to user
         → 0.3% fee retained in pool for LPs
```

## Fees

- **Swap fee**: 0.3% of input amount
- **LP reward**: 100% of fees go to liquidity providers
- **Protocol fee**: Optional, set via `Factory.setFeeTo()`

## Deployment Order

1. Deploy `WASZTB`
2. Deploy `AztibaseFactory`
3. Deploy `AztibaseRouter(factory, wasztb)`
4. Create pairs via `Factory.createPair(tokenA, tokenB)`
5. Add initial liquidity via `Router.addLiquidity()`

## Compile

```bash
solc --bin --abi --optimize contracts/dex/*.sol -o build/
```

## Key Functions

### For Traders
```solidity
// Swap exact input for minimum output
router.swapExactTokensForTokens(amountIn, amountOutMin, path, to, deadline)

// Swap for exact output with maximum input
router.swapTokensForExactTokens(amountOut, amountInMax, path, to, deadline)

// Get expected output
router.getAmountsOut(amountIn, path)
```

### For Liquidity Providers
```solidity
// Add liquidity (creates pair if needed)
router.addLiquidity(tokenA, tokenB, amountA, amountB, minA, minB, to, deadline)

// Remove liquidity
router.removeLiquidity(tokenA, tokenB, lpTokens, minA, minB, to, deadline)
```

### Price Queries
```solidity
// Get reserves for a pair
pair.getReserves() → (reserve0, reserve1, timestamp)

// Calculate output amount
router.getAmountOut(amountIn, reserveIn, reserveOut)
```

## License

MIT
