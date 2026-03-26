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
  WASZTB: "0x3990ff2c823c30416e0c8cbb689f4c5dfdcf1132000000000000000000000000",
  tUSDC: "0x6f6a5a8e7d1545955f3814139e6550d03766b4d2000000000000000000000000",
  Factory: "0x34b362ca4a307ea7756a3658861d6cf4f277841f000000000000000000000000",
  Router: "0x909f299b994cd4a3b8c6b6382d589f406ed68248000000000000000000000000",
  PriceFeed: "0x74af2626ed64f9966994a2f0828915b4524d0fae000000000000000000000000",
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
