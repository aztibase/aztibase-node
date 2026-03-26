"use client";
import { useState, useRef, useEffect, useCallback } from "react";
import { useWalletStore } from "@/stores/wallet";
import { shortenAddress, formatBalance } from "@/lib/utils";
import { getBalance } from "@/lib/rpc";

export default function WalletConnect() {
  const { address, connected, connectExtension, connectWithKey, disconnect } = useWalletStore();
  const [open, setOpen] = useState(false);
  const [keyInput, setKeyInput] = useState("");
  const [showKey, setShowKey] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const [balance, setBalance] = useState<bigint | null>(null);
  const ref = useRef<HTMLDivElement>(null);

  const fetchBalance = useCallback(async () => {
    if (!address) { setBalance(null); return; }
    try {
      setBalance(await getBalance(address));
    } catch {
      setBalance(null);
    }
  }, [address]);

  useEffect(() => {
    if (connected && address) {
      fetchBalance();
      const iv = setInterval(fetchBalance, 15_000);
      return () => clearInterval(iv);
    }
  }, [connected, address, fetchBalance]);

  useEffect(() => {
    function handleClick(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener("click", handleClick);
    return () => document.removeEventListener("click", handleClick);
  }, []);

  useEffect(() => {
    if (!window.aztibase) return;
    window.aztibase.isConnected().then((s: { connected: boolean; address: string | null }) => {
      if (s?.connected && s?.address) {
        useWalletStore.getState().setAddress(s.address, "extension");
      }
    }).catch(() => {});
  }, []);

  async function handleExtension() {
    setError(null);
    try {
      await connectExtension();
      setOpen(false);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Connection failed");
    }
  }

  async function handleKey() {
    setError(null);
    try {
      await connectWithKey(keyInput);
      setOpen(false);
      setKeyInput("");
      setShowKey(false);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Invalid key");
    }
  }

  function handleCopy() {
    if (!address) return;
    navigator.clipboard.writeText(address);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  if (connected && address) {
    return (
      <div className="relative" ref={ref}>
        <button
          onClick={() => setOpen(!open)}
          className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-aztb-500/10 border border-aztb-500/15
                     hover:border-aztb-500/25 transition-all duration-200 text-sm font-mono text-aztb-300"
        >
          <span className="w-1.5 h-1.5 rounded-full bg-aztb-green" />
          {balance !== null && (
            <span className="text-aztb-200 font-semibold">{formatBalance(balance, 0)} AZTB</span>
          )}
          <span className="text-aztb-500">|</span>
          {shortenAddress(address)}
          <svg viewBox="0 0 12 12" fill="none" className={`w-3 h-3 text-aztb-500 transition-transform duration-200 ${open ? "rotate-180" : ""}`}>
            <path d="M3 5l3 3 3-3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
        </button>
        {open && (
          <div className="absolute right-0 top-full mt-2 w-60 bg-aztb-800 border border-aztb-500/15 rounded-xl shadow-xl z-50 overflow-hidden">
            <div className="px-4 py-3 border-b border-aztb-500/8">
              <div className="text-[0.6rem] text-aztb-500 uppercase tracking-wider font-semibold mb-1">Connected</div>
              <div className="text-xs font-mono text-aztb-400 truncate mb-1.5">{address}</div>
              <div className="text-sm font-semibold text-aztb-200">
                {balance !== null ? `${formatBalance(balance, 0)} AZTB` : "Loading..."}
              </div>
            </div>
            <div className="p-1.5">
              <button
                onClick={handleCopy}
                className="w-full flex items-center gap-2.5 px-3 py-2 text-sm text-aztb-300 hover:bg-aztb-500/8 rounded-lg transition-colors"
              >
                {copied ? (
                  <>
                    <svg viewBox="0 0 16 16" fill="none" className="w-3.5 h-3.5 text-aztb-green shrink-0">
                      <path d="M3 8l4 4 6-7" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
                    </svg>
                    Copied
                  </>
                ) : (
                  <>
                    <svg viewBox="0 0 16 16" fill="none" className="w-3.5 h-3.5 text-aztb-500 shrink-0">
                      <rect x="5" y="5" width="8" height="8" rx="1.5" stroke="currentColor" strokeWidth="1.3" />
                      <path d="M3 11V3.5A1.5 1.5 0 014.5 2H11" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" />
                    </svg>
                    Copy Address
                  </>
                )}
              </button>
              <button
                onClick={() => { disconnect(); setOpen(false); }}
                className="w-full flex items-center gap-2.5 px-3 py-2 text-sm text-aztb-red hover:bg-aztb-red/8 rounded-lg transition-colors"
              >
                <svg viewBox="0 0 16 16" fill="none" className="w-3.5 h-3.5 shrink-0">
                  <path d="M6 2H4a2 2 0 00-2 2v8a2 2 0 002 2h2M10 12l4-4-4-4M14 8H6" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" strokeLinejoin="round" />
                </svg>
                Disconnect
              </button>
            </div>
          </div>
        )}
      </div>
    );
  }

  return (
    <div className="relative" ref={ref}>
      <button onClick={() => setOpen(!open)} className="btn-primary text-sm px-4 py-1.5">
        Connect Wallet
      </button>
      {open && (
        <div className="absolute right-0 top-full mt-2 w-72 bg-aztb-800 border border-aztb-500/15 rounded-xl p-3 shadow-xl z-50">
          <div className="text-sm font-semibold text-aztb-200 mb-3">Connect a wallet</div>

          <button
            onClick={handleExtension}
            className="w-full flex items-center gap-3 p-3 rounded-lg border border-aztb-500/10 hover:border-aztb-accent/25 hover:bg-aztb-accent/5 transition-all duration-200 text-left group"
          >
            <div className="w-9 h-9 rounded-lg bg-aztb-accent/10 border border-aztb-accent/15 flex items-center justify-center
                            group-hover:bg-aztb-accent/15 transition-colors">
              <svg viewBox="0 0 20 20" fill="none" className="w-4 h-4 text-aztb-accent">
                <path d="M10 2L2 6v8l8 4 8-4V6l-8-4z" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round" />
              </svg>
            </div>
            <div className="flex-1 min-w-0">
              <div className="text-sm font-semibold text-aztb-200">Aztibase Wallet</div>
              <div className="text-xs font-mono text-aztb-500">
                {typeof window !== "undefined" && window.aztibase ? "Browser Extension" : "Not detected"}
              </div>
            </div>
          </button>

          <div className="flex items-center gap-2 my-2.5 text-[0.6rem] text-aztb-500">
            <span className="flex-1 h-px bg-aztb-500/10" />
            or
            <span className="flex-1 h-px bg-aztb-500/10" />
          </div>

          <button
            onClick={() => setShowKey(!showKey)}
            className="w-full flex items-center gap-3 p-3 rounded-lg border border-aztb-500/10 hover:border-aztb-500/25 hover:bg-aztb-500/5 transition-all duration-200 text-left"
          >
            <div className="w-9 h-9 rounded-lg bg-aztb-500/8 border border-aztb-500/10 flex items-center justify-center">
              <svg viewBox="0 0 20 20" fill="none" className="w-4 h-4 text-aztb-400">
                <path d="M15 7a5 5 0 10-6.19 4.86L7 14v2h2l1-1h2l.81-1.81A5 5 0 0015 7z" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round" />
                <circle cx="13" cy="7" r="1" fill="currentColor" />
              </svg>
            </div>
            <div className="flex-1 min-w-0">
              <div className="text-sm font-semibold text-aztb-200">Import Key</div>
              <div className="text-xs font-mono text-aztb-500">Paste a private key</div>
            </div>
            <svg viewBox="0 0 12 12" fill="none" className={`w-3 h-3 text-aztb-500 transition-transform duration-200 ${showKey ? "rotate-180" : ""}`}>
              <path d="M3 5l3 3 3-3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
            </svg>
          </button>

          {showKey && (
            <div className="flex gap-2 mt-2">
              <input
                type="password"
                value={keyInput}
                onChange={(e) => setKeyInput(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && handleKey()}
                placeholder="64-char hex key"
                className="input-dark flex-1 text-xs min-w-0"
              />
              <button onClick={handleKey} className="btn-primary text-xs px-3 py-1.5">
                Go
              </button>
            </div>
          )}

          {error && (
            <div className="mt-2 text-xs text-aztb-red bg-aztb-red/8 border border-aztb-red/15 rounded-lg px-3 py-2">
              {error}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
