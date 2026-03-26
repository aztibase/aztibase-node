"use client";
import { useState } from "react";
import { faucetDrip } from "@/lib/rpc";
import { useWalletStore } from "@/stores/wallet";

export default function FaucetPage() {
  const { address, connected } = useWalletStore();
  const [input, setInput] = useState("");
  const [result, setResult] = useState<{ msg: string; ok: boolean } | null>(null);
  const [loading, setLoading] = useState(false);

  const targetAddr = input || address || "";

  async function handleDrip() {
    if (!targetAddr || targetAddr.length < 10) {
      setResult({ msg: "Enter a valid address", ok: false });
      return;
    }
    setLoading(true);
    setResult(null);
    try {
      await faucetDrip(targetAddr);
      setResult({ msg: `Sent 1,000,000 AZTB to ${targetAddr.slice(0, 12)}...`, ok: true });
    } catch (e: unknown) {
      setResult({ msg: e instanceof Error ? e.message : "Faucet request failed", ok: false });
    }
    setLoading(false);
  }

  return (
    <div className="max-w-md mx-auto px-4 pt-12 pb-20">
      <div className="card p-6">
        {/* Header */}
        <div className="flex items-center gap-2 mb-5">
          <div className="w-6 h-6 rounded-lg bg-aztb-green/10 border border-aztb-green/20 flex items-center justify-center">
            <svg viewBox="0 0 16 16" fill="none" className="w-3 h-3 text-aztb-green">
              <path d="M8 2C8 2 4 7 4 10a4 4 0 108 0c0-3-4-8-4-8z" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round" />
            </svg>
          </div>
          <span className="font-heading text-xs font-bold tracking-wider text-aztb-300 uppercase flex-1">
            Testnet Faucet
          </span>
          <span className="badge-green text-[0.6rem]">
            <span className="w-1 h-1 rounded-full bg-aztb-green animate-pulse" />
            Active
          </span>
        </div>

        <p className="text-sm text-aztb-400 mb-5 leading-relaxed">
          Request <span className="text-aztb-200 font-semibold">1,000,000 AZTB</span> for testnet development.
          One drip per address every 60 seconds.
        </p>

        <div className="space-y-3">
          <div>
            <div className="label mb-1.5">Recipient Address</div>
            <input
              type="text"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              placeholder={connected && address ? address : "0x... (your address)"}
              className="input-dark w-full text-xs"
              spellCheck={false}
            />
            {connected && address && !input && (
              <div className="text-[0.6rem] text-aztb-500 mt-1 font-mono">
                Using connected wallet: {address.slice(0, 12)}...
              </div>
            )}
          </div>

          <button
            onClick={handleDrip}
            disabled={loading || !targetAddr}
            className="btn-primary w-full py-3 text-sm"
          >
            {loading ? (
              <span className="flex items-center justify-center gap-2">
                <span className="w-3.5 h-3.5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                Requesting...
              </span>
            ) : (
              "Request AZTB"
            )}
          </button>
        </div>

        {result && (
          <div
            className={`mt-4 text-sm px-4 py-3 rounded-xl font-mono text-xs ${
              result.ok
                ? "bg-aztb-green/8 border border-aztb-green/20 text-aztb-green"
                : "bg-aztb-red/8 border border-aztb-red/20 text-aztb-red"
            }`}
          >
            {result.ok && (
              <svg viewBox="0 0 16 16" fill="none" className="w-4 h-4 inline mr-2 -mt-0.5">
                <path d="M3 8l4 4 6-7" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
            )}
            {result.msg}
          </div>
        )}

        <div className="mt-5 pt-4 border-t border-aztb-500/8 space-y-2">
          {[
            ["RPC Endpoint", "rpc.aztibase.com"],
            ["Method", "aztb_faucetDrip"],
            ["Amount", "1,000,000 AZTB"],
            ["Cooldown", "60 seconds"],
          ].map(([k, v]) => (
            <div key={k} className="flex justify-between text-[0.6rem]">
              <span className="text-aztb-500 uppercase font-semibold tracking-wider">{k}</span>
              <span className="text-aztb-400 font-mono">{v}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
