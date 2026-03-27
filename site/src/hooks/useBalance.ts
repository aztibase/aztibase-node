"use client";
import { useCallback, useEffect, useState } from "react";
import { getBalance, evmCall } from "@/lib/rpc";
import { hexToBigInt, toEvmAddress } from "@/lib/utils";
import { useWalletStore } from "@/stores/wallet";
import { CONTRACTS, TOKENS } from "@/config/chain";

const BALANCE_OF_SEL = "70a08231";

export interface BalanceResult {
  balance: bigint;
  loading: boolean;
  refresh: () => Promise<void>;
  decimals: number;
}

export function useNativeBalance(): BalanceResult {
  const { address, connected } = useWalletStore();
  const [balance, setBalance] = useState<bigint>(0n);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    if (!address) { setBalance(0n); return; }
    setLoading(true);
    try {
      setBalance(await getBalance(address));
    } catch {
      setBalance(0n);
    }
    setLoading(false);
  }, [address]);

  useEffect(() => {
    if (connected && address) refresh();
  }, [connected, address, refresh]);

  return { balance, loading, refresh, decimals: 0 };
}

export function useTokenBalance(contractAddress: string | null, decimals?: number): BalanceResult {
  const tokenDec = decimals ?? TOKENS.find((t) => t.address === contractAddress)?.decimals ?? 0;
  const { address, connected } = useWalletStore();
  const [balance, setBalance] = useState<bigint>(0n);
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    if (!address || !contractAddress) { setBalance(0n); return; }
    setLoading(true);
    try {
      const data = "0x" + BALANCE_OF_SEL + toEvmAddress(address);
      const res = await evmCall(contractAddress, data);
      if (!res || res === "0x") { setBalance(0n); }
      else { setBalance(hexToBigInt(res.replace("0x", "").slice(0, 64))); }
    } catch {
      setBalance(0n);
    }
    setLoading(false);
  }, [address, contractAddress]);

  useEffect(() => {
    if (connected && address) refresh();
  }, [connected, address, refresh]);

  return { balance, loading, refresh, decimals: tokenDec };
}

export function useSwapBalance(tokenSymbol: string): BalanceResult {
  const native = useNativeBalance();
  const wasztb = useTokenBalance(CONTRACTS.WASZTB);
  const tusdc = useTokenBalance(CONTRACTS.tUSDC);

  if (tokenSymbol === "AZTB") return native;
  if (tokenSymbol === "WASZTB") return wasztb;
  if (tokenSymbol === "tUSDC") return tusdc;
  return native;
}
