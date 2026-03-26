"use client";
import { useState, useEffect, useCallback, useRef } from "react";
import { useWalletStore } from "@/stores/wallet";
import { useSwapBalance } from "@/hooks/useBalance";
import { evmCall, rpc, getTransactionReceipt } from "@/lib/rpc";
import { CONTRACTS, TOKENS } from "@/config/chain";
import { formatBalance, hexToBigInt, pad32, toHex256 } from "@/lib/utils";
import WalletConnect from "@/components/WalletConnect";

const KECCAK_GET_AMOUNTS_OUT = "d06ca61f";

let _keccak: ((data: Uint8Array) => Uint8Array) | null = null;

async function loadKeccak() {
  if (!_keccak) {
    const m = await import("@noble/hashes/sha3.js");
    _keccak = m.keccak_256;
  }
  return _keccak;
}

function keccakSel(sig: string): string {
  if (!_keccak) return "00000000";
  return Array.from(_keccak(new TextEncoder().encode(sig)).slice(0, 4))
    .map((b) => b.toString(16).padStart(2, "0")).join("");
}

function hexToBytes(hex: string): Uint8Array {
  const clean = hex.replace(/^0x/, "");
  const b = new Uint8Array(clean.length / 2);
  for (let i = 0; i < b.length; i++) b[i] = parseInt(clean.slice(i * 2, i * 2 + 2), 16);
  return b;
}

function bytesToHex(b: Uint8Array): string {
  return Array.from(b).map((x) => x.toString(16).padStart(2, "0")).join("");
}

function concatBytes(...arrays: Uint8Array[]): Uint8Array {
  const total = arrays.reduce((s, a) => s + a.length, 0);
  const out = new Uint8Array(total);
  let off = 0;
  for (const a of arrays) { out.set(a, off); off += a.length; }
  return out;
}

function encodeVarint(n: bigint | number): Uint8Array {
  let v = typeof n === "bigint" ? n : BigInt(n >>> 0);
  const bytes: number[] = [];
  while (v > 0x7Fn) { bytes.push(Number(v & 0x7Fn) | 0x80); v >>= 7n; }
  bytes.push(Number(v));
  return new Uint8Array(bytes);
}

function encodeBytes(d: Uint8Array): Uint8Array {
  const l = encodeVarint(d.length);
  return concatBytes(l, d);
}

function encodeEvmCall(caller: Uint8Array, contract: Uint8Array, calldata: Uint8Array, nonce: number, gasLimit: number, value: number): Uint8Array {
  return concatBytes(
    encodeVarint(4), caller, contract, encodeBytes(calldata),
    encodeVarint(BigInt(nonce)), encodeVarint(BigInt(gasLimit)),
    encodeVarint(BigInt(value || 0)), encodeVarint(1n),
  );
}

function isNative(sym: string) { return sym === "AZTB"; }

function buildSwapCalldata(fromName: string, toName: string, amountWei: bigint, minOut: bigint, toAddrHex: string, deadline: bigint, path: string[]): string {
  const payingNative = isNative(fromName);
  const receivingNative = isNative(toName);
  let data: string;

  if (payingNative) {
    const sel = keccakSel("swapExactETHForTokens(uint256,address[],address,uint256)");
    data = sel + toHex256(minOut) + toHex256(128n) + pad32(toAddrHex.replace("0x", ""))
      + toHex256(deadline) + toHex256(BigInt(path.length))
      + path.map((a) => pad32(a)).join("");
  } else if (receivingNative) {
    const sel = keccakSel("swapExactTokensForETH(uint256,uint256,address[],address,uint256)");
    data = sel + toHex256(amountWei) + toHex256(minOut) + toHex256(160n)
      + pad32(toAddrHex.replace("0x", "")) + toHex256(deadline)
      + toHex256(BigInt(path.length))
      + path.map((a) => pad32(a)).join("");
  } else {
    const sel = keccakSel("swapExactTokensForTokens(uint256,uint256,address[],address,uint256)");
    data = sel + toHex256(amountWei) + toHex256(minOut) + toHex256(160n)
      + pad32(toAddrHex.replace("0x", "")) + toHex256(deadline)
      + toHex256(BigInt(path.length))
      + path.map((a) => pad32(a)).join("");
  }
  return "0x" + data;
}

function TokenSelector({ value, onChange, exclude }: { value: string; onChange: (v: string) => void; exclude?: string }) {
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    function close(e: MouseEvent) {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener("click", close);
    return () => document.removeEventListener("click", close);
  }, []);

  const selected = TOKENS.find((t) => t.symbol === value);

  return (
    <div className="relative" ref={ref}>
      <button onClick={() => setOpen(!open)} className="flex items-center gap-2 bg-aztb-500/8 border border-aztb-500/12 rounded-lg px-3 py-2 hover:border-aztb-500/25 transition-colors">
        <span className="text-sm font-mono font-semibold text-aztb-200">{selected?.symbol}</span>
        <svg viewBox="0 0 12 12" fill="none" className="w-3 h-3 text-aztb-500"><path d="M3 5l3 3 3-3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" /></svg>
      </button>
      {open && (
        <div className="absolute right-0 top-full mt-1 w-44 bg-aztb-800 border border-aztb-500/15 rounded-xl p-1.5 shadow-xl z-50">
          {TOKENS.filter((t) => t.symbol !== exclude).map((t) => (
            <button key={t.symbol} onClick={() => { onChange(t.symbol); setOpen(false); }}
              className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-left transition-colors ${t.symbol === value ? "bg-aztb-accent/10 text-aztb-200" : "text-aztb-400 hover:bg-aztb-500/8 hover:text-aztb-200"}`}>
              <div className="flex-1 min-w-0">
                <div className="text-sm font-mono font-semibold">{t.symbol}</div>
                <div className="text-[0.6rem] text-aztb-500">{t.name}</div>
              </div>
              {t.symbol === value && (
                <svg viewBox="0 0 16 16" fill="none" className="w-3.5 h-3.5 text-aztb-accent shrink-0"><path d="M3 8l4 4 6-7" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" /></svg>
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export default function SwapPage() {
  const { address, connected, mode, secretKey } = useWalletStore();
  const [tokenFrom, setTokenFrom] = useState("AZTB");
  const [tokenTo, setTokenTo] = useState("tUSDC");
  const [amountIn, setAmountIn] = useState("");
  const [amountOut, setAmountOut] = useState("");
  const [rate, setRate] = useState("");
  const [loading, setLoading] = useState(false);
  const [slippage, setSlippage] = useState("0.5");
  const [showSettings, setShowSettings] = useState(false);
  const [swapping, setSwapping] = useState(false);
  const [swapStatus, setSwapStatus] = useState<{ msg: string; ok: boolean } | null>(null);

  const fromBal = useSwapBalance(tokenFrom);
  const toBal = useSwapBalance(tokenTo);

  function tokenAddr(sym: string) {
    if (sym === "AZTB") return CONTRACTS.WASZTB;
    return CONTRACTS[sym as keyof typeof CONTRACTS] || CONTRACTS.WASZTB;
  }

  const fetchQuote = useCallback(async (val: number) => {
    if (!val || val <= 0) { setAmountOut(""); setRate(""); return; }
    setLoading(true);
    try {
      const amountWei = BigInt(Math.floor(val)) * 10n ** 18n;
      const path = [tokenAddr(tokenFrom), tokenAddr(tokenTo)];
      const data = "0x" + KECCAK_GET_AMOUNTS_OUT + toHex256(amountWei) + toHex256(64n) + toHex256(BigInt(path.length)) +
        path.map((a) => pad32(a)).join("");
      const res = await evmCall(CONTRACTS.Router, data);
      const outHex = res.replace("0x", "").slice(192, 256);
      const outWei = hexToBigInt(outHex);
      const outFloat = Number(outWei) / 1e18;
      setAmountOut(outFloat.toFixed(6));
      setRate(`1 ${tokenFrom} = ${(outFloat / val).toFixed(6)} ${tokenTo}`);
    } catch {
      setAmountOut((val * 0.997).toFixed(6));
      setRate(`~0.997 (est.)`);
    }
    setLoading(false);
  }, [tokenFrom, tokenTo]);

  useEffect(() => {
    const val = parseFloat(amountIn);
    if (!val) { setAmountOut(""); setRate(""); return; }
    const t = setTimeout(() => fetchQuote(val), 400);
    return () => clearTimeout(t);
  }, [amountIn, fetchQuote]);

  useEffect(() => { loadKeccak(); }, []);

  function flip() {
    setTokenFrom(tokenTo); setTokenTo(tokenFrom);
    setAmountIn(""); setAmountOut(""); setRate(""); setSwapStatus(null);
  }

  function handleMax() {
    if (fromBal.balance > 0n) {
      const divisor = 10 ** fromBal.decimals;
      setAmountIn((Number(fromBal.balance) / divisor).toString());
    }
  }

  async function waitForReceipt(txHash: string): Promise<void> {
    for (let i = 0; i < 15; i++) {
      await new Promise((r) => setTimeout(r, 2000));
      try {
        const receipt = await getTransactionReceipt(txHash);
        if (receipt) {
          if (receipt.success) {
            setSwapStatus({ msg: `Swap confirmed! Gas: ${parseInt(receipt.gasUsed, 16)}`, ok: true });
          } else {
            setSwapStatus({ msg: `Swap failed: ${receipt.error || "reverted"}`, ok: false });
          }
          return;
        }
      } catch { /* keep polling */ }
    }
    setSwapStatus({ msg: "Tx submitted but receipt not found yet. Check explorer.", ok: true });
  }

  async function handleSwap() {
    if (!address) return;
    const val = parseFloat(amountIn);
    if (!val || val <= 0) return;

    setSwapping(true);
    setSwapStatus(null);

    const fromName = tokenFrom;
    const toName = tokenTo;
    const payingNative = isNative(fromName);
    const amountWei = BigInt(Math.floor(val));
    const minOut = 0n;
    const deadline = BigInt(Math.floor(Date.now() / 1000) + 600);
    const path = [tokenAddr(fromName), tokenAddr(toName)];

    try {
      const canUseExtension = mode === "extension" && window.aztibase && typeof window.aztibase.signAndSendEvmCall === "function";

      if (canUseExtension) {
        const ext = window.aztibase!;
        if (!payingNative) {
          setSwapStatus({ msg: `Approving Router to spend ${fromName}...`, ok: true });
          const approveSel = keccakSel("approve(address,uint256)");
          const approveCalldata = "0x" + approveSel
            + pad32(CONTRACTS.Router.replace("0x", "").replace(/0+$/, ""))
            + toHex256(amountWei * 10n);
          await ext.signAndSendEvmCall(tokenAddr(fromName), approveCalldata, 100000, 0);
          await new Promise((r) => setTimeout(r, 2000));
        }

        setSwapStatus({ msg: "Sending swap...", ok: true });
        const swapCalldata = buildSwapCalldata(fromName, toName, amountWei, minOut, address, deadline, path);
        const nativeValue = payingNative ? Number(amountWei) : 0;
        const swapRes = await ext.signAndSendEvmCall(CONTRACTS.Router, swapCalldata, 500000, nativeValue);
        const txHash = swapRes.txHash || String(swapRes);
        setSwapStatus({ msg: `Tx submitted: ${txHash.slice(0, 16)}...`, ok: true });
        await waitForReceipt(txHash);
      } else if (mode === "extension" && !canUseExtension) {
        setSwapStatus({ msg: "Extension outdated — please update your wallet extension to enable swap.", ok: false });
        setSwapping(false);
        return;
      } else if (mode === "key" && secretKey) {
        const ed = await import("@noble/ed25519");
        const hashes = await import("@noble/hashes/blake3.js");
        const keyBytes = hexToBytes(secretKey!);
        const pub = await ed.getPublicKeyAsync(keyBytes);
        const callerAddr = hashes.blake3(pub);
        const callerHex = "0x" + bytesToHex(callerAddr as Uint8Array);
        const domain = new TextEncoder().encode("AZTB_TX_V1");

        const nonceRes = await rpc<string | number>("aztb_getNonce", [callerHex]);
        let nonce = typeof nonceRes === "string" ? parseInt(nonceRes, 16) : Number(nonceRes);

        if (!payingNative) {
          setSwapStatus({ msg: `Approving Router to spend ${fromName}...`, ok: true });
          const approveSel = keccakSel("approve(address,uint256)");
          const approveData = hexToBytes(
            approveSel + pad32(CONTRACTS.Router.replace("0x", "").replace(/0+$/, ""))
            + toHex256(amountWei * 10n)
          );
          const tokenContract = hexToBytes(tokenAddr(fromName).replace("0x", ""));
          const approvePostcard = encodeEvmCall(callerAddr as Uint8Array, tokenContract, approveData, nonce, 100000, 0);
          const approvePayload = new Uint8Array(1 + approvePostcard.length);
          approvePayload[0] = 0x05;
          approvePayload.set(approvePostcard, 1);
          const approveMsg = concatBytes(domain, approvePayload);
          const approveSig = await ed.signAsync(approveMsg, keyBytes);
          const approvePayloadLen = new Uint8Array(4);
          new DataView(approvePayloadLen.buffer).setUint32(0, approvePayload.length, true);
          const approveEnv = concatBytes(new Uint8Array([0xAA]), approvePayloadLen, approvePayload, pub, approveSig);
          await rpc("aztb_sendTransaction", ["0x" + bytesToHex(approveEnv)]);
          nonce++;
          await new Promise((r) => setTimeout(r, 2000));
        }

        setSwapStatus({ msg: "Sending swap...", ok: true });
        const swapCalldata = buildSwapCalldata(fromName, toName, amountWei, minOut, callerHex, deadline, path);
        const swapData = hexToBytes(swapCalldata.replace("0x", ""));
        const routerBytes = hexToBytes(CONTRACTS.Router.replace("0x", ""));
        const nativeValue = payingNative ? Number(amountWei) : 0;
        const postcard = encodeEvmCall(callerAddr as Uint8Array, routerBytes, swapData, nonce, 500000, nativeValue);
        const payload = new Uint8Array(1 + postcard.length);
        payload[0] = 0x05;
        payload.set(postcard, 1);
        const msg = concatBytes(domain, payload);
        const sig = await ed.signAsync(msg, keyBytes);
        const payloadLen = new Uint8Array(4);
        new DataView(payloadLen.buffer).setUint32(0, payload.length, true);
        const envelope = concatBytes(new Uint8Array([0xAA]), payloadLen, payload, pub, sig);
        const txHash = await rpc<string>("aztb_sendTransaction", ["0x" + bytesToHex(envelope)]);
        setSwapStatus({ msg: `Tx submitted: ${String(txHash).slice(0, 16)}...`, ok: true });
        await waitForReceipt(String(txHash));
      }
    } catch (e: unknown) {
      setSwapStatus({ msg: `Swap error: ${e instanceof Error ? e.message : String(e)}`, ok: false });
    }

    setSwapping(false);
    fromBal.refresh();
    toBal.refresh();
  }

  return (
    <div className="max-w-md mx-auto px-4 pt-12 pb-20">
      <div className="card p-6">
        {/* Header */}
        <div className="flex items-center gap-2 mb-5">
          <div className="w-6 h-6 rounded-lg bg-aztb-accent/10 border border-aztb-accent/20 flex items-center justify-center">
            <svg viewBox="0 0 16 16" fill="none" className="w-3 h-3 text-aztb-accent">
              <path d="M4 6l4 4 4-4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
              <path d="M4 10l4-4 4 4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" opacity="0.4" />
            </svg>
          </div>
          <span className="font-heading text-xs font-bold tracking-wider text-aztb-300 uppercase flex-1">Swap</span>
          <button onClick={() => setShowSettings(!showSettings)}
            className={`p-1.5 rounded-lg transition-colors ${showSettings ? "bg-aztb-accent/10 text-aztb-accent" : "text-aztb-500 hover:text-aztb-300 hover:bg-aztb-500/8"}`}>
            <svg viewBox="0 0 16 16" fill="none" className="w-4 h-4">
              <circle cx="8" cy="8" r="2.5" stroke="currentColor" strokeWidth="1.3" />
              <path d="M8 2v2.5M8 11.5V14M2 8h2.5M11.5 8H14M3.76 3.76l1.77 1.77M10.47 10.47l1.77 1.77M3.76 12.24l1.77-1.77M10.47 5.53l1.77-1.77" stroke="currentColor" strokeWidth="1" strokeLinecap="round" opacity="0.4" />
            </svg>
          </button>
          <span className="text-[0.6rem] text-aztb-500 font-mono bg-aztb-500/8 px-2 py-0.5 rounded">0.3% fee</span>
        </div>

        {/* Slippage settings */}
        {showSettings && (
          <div className="mb-4 p-3 rounded-lg bg-aztb-950 border border-aztb-500/8">
            <div className="label mb-2">Slippage Tolerance</div>
            <div className="flex gap-2">
              {["0.1", "0.5", "1.0"].map((v) => (
                <button key={v} onClick={() => setSlippage(v)}
                  className={`flex-1 py-1.5 rounded-lg text-xs font-mono transition-colors ${slippage === v ? "bg-aztb-accent/15 text-aztb-accent border border-aztb-accent/25" : "bg-aztb-500/8 text-aztb-400 border border-transparent hover:border-aztb-500/15"}`}>
                  {v}%
                </button>
              ))}
              <div className="relative flex-1">
                <input type="text" value={!["0.1", "0.5", "1.0"].includes(slippage) ? slippage : ""}
                  onChange={(e) => { const v = e.target.value.replace(/[^0-9.]/g, ""); if (v) setSlippage(v); }}
                  placeholder="Custom" className="w-full py-1.5 rounded-lg text-xs font-mono text-center bg-aztb-500/8 text-aztb-400 border border-transparent focus:border-aztb-accent/25 outline-none" />
                <span className="absolute right-2 top-1/2 -translate-y-1/2 text-[0.6rem] text-aztb-500">%</span>
              </div>
            </div>
          </div>
        )}

        {/* From */}
        <div className="bg-aztb-950 border border-aztb-500/8 rounded-xl p-4 mb-1">
          <div className="flex justify-between text-[0.65rem] text-aztb-500 mb-2.5">
            <span className="uppercase font-semibold tracking-wider">You pay</span>
            <button onClick={handleMax} className="font-mono hover:text-aztb-300 transition-colors flex items-center gap-1">
              Balance: {connected ? formatBalance(fromBal.balance, fromBal.decimals) : "--"}
              {connected && fromBal.balance > 0n && <span className="text-aztb-accent font-semibold ml-1">MAX</span>}
            </button>
          </div>
          <div className="flex items-center gap-3">
            <input type="text" inputMode="decimal" placeholder="0.0" value={amountIn}
              onChange={(e) => setAmountIn(e.target.value.replace(/[^0-9.]/g, ""))}
              className="flex-1 bg-transparent text-2xl font-semibold text-aztb-200 outline-none min-w-0 placeholder:text-aztb-700" />
            <TokenSelector value={tokenFrom} onChange={setTokenFrom} exclude={tokenTo} />
          </div>
        </div>

        {/* Flip */}
        <div className="flex justify-center -my-2.5 relative z-10">
          <button onClick={flip}
            className="w-9 h-9 rounded-xl border border-aztb-500/12 bg-aztb-800 text-aztb-400 hover:bg-aztb-700 hover:text-aztb-200 hover:border-aztb-500/25 active:scale-95 transition-all duration-200 flex items-center justify-center">
            <svg viewBox="0 0 16 16" fill="none" className="w-4 h-4"><path d="M8 3v10M5 10l3 3 3-3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" /></svg>
          </button>
        </div>

        {/* To */}
        <div className="bg-aztb-950 border border-aztb-500/8 rounded-xl p-4 mt-1">
          <div className="flex justify-between text-[0.65rem] text-aztb-500 mb-2.5">
            <span className="uppercase font-semibold tracking-wider">You receive</span>
            <span className="font-mono">Balance: {connected ? formatBalance(toBal.balance, toBal.decimals) : "--"}</span>
          </div>
          <div className="flex items-center gap-3">
            <input type="text" placeholder="0.0" value={loading ? "..." : amountOut} readOnly
              className="flex-1 bg-transparent text-2xl font-semibold text-aztb-300 outline-none min-w-0 placeholder:text-aztb-700" />
            <TokenSelector value={tokenTo} onChange={setTokenTo} exclude={tokenFrom} />
          </div>
        </div>

        {/* Rate */}
        {rate && (
          <div className="mt-3 px-4 py-2.5 rounded-xl bg-aztb-500/4 border border-aztb-500/5 flex items-center justify-between">
            <span className="text-xs text-aztb-400 font-mono">{rate}</span>
            <span className="text-[0.6rem] text-aztb-500 font-mono">Slippage: {slippage}%</span>
          </div>
        )}

        {/* Action */}
        <div className="mt-4">
          {!connected ? (
            <div className="w-full"><WalletConnect /></div>
          ) : (
            <button onClick={handleSwap} disabled={!amountIn || !amountOut || loading || swapping} className="btn-primary w-full py-3.5 text-sm">
              {swapping ? (
                <span className="flex items-center justify-center gap-2">
                  <span className="w-3.5 h-3.5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  Swapping...
                </span>
              ) : loading ? (
                <span className="flex items-center justify-center gap-2">
                  <span className="w-3.5 h-3.5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  Fetching quote...
                </span>
              ) : !amountIn ? "Enter an amount" : "Swap"}
            </button>
          )}
        </div>

        {/* Swap result */}
        {swapStatus && (
          <div className={`mt-3 text-xs px-4 py-3 rounded-xl font-mono ${
            swapStatus.ok
              ? "bg-aztb-green/8 border border-aztb-green/20 text-aztb-green"
              : "bg-aztb-red/8 border border-aztb-red/20 text-aztb-red"
          }`}>
            {swapStatus.ok && (
              <svg viewBox="0 0 16 16" fill="none" className="w-3.5 h-3.5 inline mr-1.5 -mt-0.5">
                <path d="M3 8l4 4 6-7" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
              </svg>
            )}
            {swapStatus.msg}
          </div>
        )}

        {/* Contract info */}
        <div className="mt-4 pt-3 border-t border-aztb-500/8 space-y-1.5">
          {[
            ["Router", CONTRACTS.Router],
            ["Factory", CONTRACTS.Factory],
            ["WASZTB", CONTRACTS.WASZTB],
          ].map(([label, addr]) => (
            <div key={label} className="flex justify-between text-[0.6rem]">
              <span className="text-aztb-500 uppercase font-semibold tracking-wider">{label}</span>
              <span className="text-aztb-500/70 font-mono">{addr.slice(0, 10)}...{addr.slice(-4)}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
