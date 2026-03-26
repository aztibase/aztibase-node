export const CHAIN = {
  name: "Aztibase Network",
  chainId: "0xA27B",
  rpc: "http://102.209.21.247:9944",
  rpcFallback: "https://rpc.aztibase.com",
  explorer: "https://aztibase.com/explorer",
  symbol: "AZTB",
  decimals: 18,
  blockTime: 400,
};

export const CONTRACTS = {
  WASZTB: "0x31dad086f9decca6429f47a837b3189fae9070d5000000000000000000000000",
  tUSDC: "0x1451ec1efa32cde88fe150450d6bb36d85c16073000000000000000000000000",
  Factory: "0xf213b3ea6724948b72330aa93932be1eb6561ff6000000000000000000000000",
  Router: "0x4472c05ec237ad8063d5887b6c4febc2cb3efb7a000000000000000000000000",
  PriceFeed: "0x603a19a8ecf8a303a705e4f9c96d4a80bbcce9ed000000000000000000000000",
};

export const TOKENS = [
  { symbol: "AZTB", name: "Aztibase", decimals: 18, native: true, address: null },
  { symbol: "WASZTB", name: "Wrapped AZTB", decimals: 18, native: false, address: CONTRACTS.WASZTB },
  { symbol: "tUSDC", name: "Test USDC", decimals: 6, native: false, address: CONTRACTS.tUSDC },
] as const;

export type Token = (typeof TOKENS)[number];

export const NAV_LINKS = [
  { label: "Swap", href: "/swap" },
  { label: "Faucet", href: "/faucet" },
  { label: "Explorer", href: "/explorer" },
  { label: "Docs", href: "/docs" },
  { label: "Litepaper", href: "/litepaper" },
];
