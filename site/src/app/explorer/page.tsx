"use client";
import { useEffect, useState, useCallback } from "react";
import { getBlockNumber, getBlockRange, getPeerCount, getValidators } from "@/lib/rpc";

interface Block {
  number: number;
  hash: string;
  transactions?: unknown[];
  timestamp?: string;
  gasUsed?: string;
}

function timeAgo(ts: string | undefined): string {
  if (!ts) return "";
  const sec = Math.floor((Date.now() - new Date(ts).getTime()) / 1000);
  if (sec < 0) return "just now";
  if (sec < 60) return `${sec}s ago`;
  if (sec < 3600) return `${Math.floor(sec / 60)}m ago`;
  if (sec < 86400) return `${Math.floor(sec / 3600)}h ago`;
  return `${Math.floor(sec / 86400)}d ago`;
}

export default function ExplorerPage() {
  const [blockHeight, setBlockHeight] = useState(0);
  const [blocks, setBlocks] = useState<Block[]>([]);
  const [peers, setPeers] = useState(0);
  const [validators, setValidators] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [search, setSearch] = useState("");
  const [avgBlockTime, setAvgBlockTime] = useState("--");

  const refresh = useCallback(async () => {
    try {
      const [height, peerCount] = await Promise.all([
        getBlockNumber(),
        getPeerCount().catch(() => 0),
      ]);
      setBlockHeight(height);
      setPeers(peerCount);
      setError(false);

      try {
        const vals = await getValidators() as unknown[];
        setValidators(Array.isArray(vals) ? vals.length : 0);
      } catch { setValidators(0); }

      if (height > 0) {
        const from = Math.max(0, height - 14);
        const count = Math.min(15, height - from + 1);
        try {
          const range = await getBlockRange(from, count) as Block[];
          if (Array.isArray(range)) {
            const sorted = range.reverse();
            setBlocks(sorted);

            if (sorted.length >= 2 && sorted[0].timestamp && sorted[sorted.length - 1].timestamp) {
              const newest = new Date(sorted[0].timestamp!).getTime();
              const oldest = new Date(sorted[sorted.length - 1].timestamp!).getTime();
              const diff = (newest - oldest) / (sorted.length - 1);
              if (diff > 0 && diff < 60000) {
                setAvgBlockTime(`${(diff / 1000).toFixed(1)}s`);
              }
            }
          }
        } catch { /* block range might not be available */ }
      }
    } catch {
      setError(true);
    }
    setLoading(false);
  }, []);

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 5000);
    return () => clearInterval(interval);
  }, [refresh]);

  const filteredBlocks = search.trim()
    ? blocks.filter(
        (b) =>
          b.number.toString().includes(search) ||
          b.hash?.toLowerCase().includes(search.toLowerCase())
      )
    : blocks;

  const stats = [
    {
      label: "Block Height",
      value: blockHeight.toLocaleString(),
      icon: (
        <svg viewBox="0 0 20 20" fill="none" className="w-4 h-4">
          <rect x="3" y="3" width="6" height="6" rx="1" stroke="currentColor" strokeWidth="1.5" />
          <rect x="11" y="3" width="6" height="6" rx="1" stroke="currentColor" strokeWidth="1.5" />
          <rect x="3" y="11" width="6" height="6" rx="1" stroke="currentColor" strokeWidth="1.5" />
          <rect x="11" y="11" width="6" height="6" rx="1" stroke="currentColor" strokeWidth="1.5" />
        </svg>
      ),
      color: "text-aztb-accent",
    },
    {
      label: "Avg Block Time",
      value: avgBlockTime,
      icon: (
        <svg viewBox="0 0 20 20" fill="none" className="w-4 h-4">
          <circle cx="10" cy="10" r="7" stroke="currentColor" strokeWidth="1.5" />
          <path d="M10 6v4l3 2" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
        </svg>
      ),
      color: "text-aztb-cyan",
    },
    {
      label: "Peers",
      value: peers.toString(),
      icon: (
        <svg viewBox="0 0 20 20" fill="none" className="w-4 h-4">
          <circle cx="7" cy="7" r="3" stroke="currentColor" strokeWidth="1.5" />
          <circle cx="14" cy="13" r="3" stroke="currentColor" strokeWidth="1.5" />
          <path d="M9.5 9l2 2" stroke="currentColor" strokeWidth="1.2" strokeLinecap="round" />
        </svg>
      ),
      color: "text-aztb-green",
    },
    {
      label: "Validators",
      value: validators.toString(),
      icon: (
        <svg viewBox="0 0 20 20" fill="none" className="w-4 h-4">
          <path d="M10 2l6 3v5c0 4-3 7-6 8-3-1-6-4-6-8V5l6-3z" stroke="currentColor" strokeWidth="1.5" strokeLinejoin="round" />
          <path d="M7.5 10l2 2 3.5-4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
      ),
      color: "text-aztb-400",
    },
  ];

  return (
    <div className="max-w-5xl mx-auto px-4 pt-10 pb-20">
      {/* Header */}
      <div className="flex flex-col sm:flex-row sm:items-center gap-4 mb-6">
        <div className="flex items-center gap-3 flex-1">
          <div className="w-8 h-8 rounded-lg bg-aztb-cyan/10 border border-aztb-cyan/20 flex items-center justify-center">
            <svg viewBox="0 0 20 20" fill="none" className="w-4 h-4 text-aztb-cyan">
              <rect x="3" y="3" width="6" height="6" rx="1" stroke="currentColor" strokeWidth="1.5" />
              <rect x="11" y="3" width="6" height="6" rx="1" stroke="currentColor" strokeWidth="1.5" />
              <rect x="3" y="11" width="6" height="6" rx="1" stroke="currentColor" strokeWidth="1.5" />
              <rect x="11" y="11" width="6" height="6" rx="1" stroke="currentColor" strokeWidth="1.5" />
            </svg>
          </div>
          <div>
            <h1 className="font-heading text-lg font-bold text-aztb-200">Block Explorer</h1>
            <div className="text-[0.6rem] text-aztb-500 font-mono uppercase tracking-wider flex items-center gap-2">
              {error ? (
                <><span className="w-1.5 h-1.5 rounded-full bg-aztb-red" />Disconnected</>
              ) : (
                <><span className="w-1.5 h-1.5 rounded-full bg-aztb-green animate-pulse" />Live — auto-refresh 5s</>
              )}
            </div>
          </div>
        </div>

        {/* Search */}
        <div className="relative w-full sm:w-72">
          <svg viewBox="0 0 20 20" fill="none" className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-aztb-500">
            <circle cx="9" cy="9" r="5.5" stroke="currentColor" strokeWidth="1.5" />
            <path d="M13 13l4 4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
          </svg>
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search by block number or hash..."
            className="input-dark w-full pl-9 pr-3 py-2 text-xs"
            spellCheck={false}
          />
        </div>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-2 md:grid-cols-4 gap-3 mb-6">
        {stats.map((s) => (
          <div key={s.label} className="card p-4 group hover:border-aztb-500/20 transition-all duration-300">
            <div className="flex items-center gap-2 mb-2">
              <span className={s.color}>{s.icon}</span>
              <span className="text-[0.6rem] text-aztb-500 font-mono uppercase tracking-wider">{s.label}</span>
            </div>
            <div className={`text-xl font-heading font-bold ${s.color}`}>{s.value}</div>
          </div>
        ))}
      </div>

      {/* Recent Blocks */}
      <div className="card overflow-hidden">
        <div className="px-5 py-3.5 border-b border-aztb-500/8 flex items-center justify-between">
          <span className="text-sm font-semibold text-aztb-300">Recent Blocks</span>
          <button
            onClick={refresh}
            className="text-xs text-aztb-500 hover:text-aztb-300 transition-colors flex items-center gap-1.5"
          >
            <svg viewBox="0 0 16 16" fill="none" className="w-3.5 h-3.5">
              <path d="M2 8a6 6 0 0111.47-2.47M14 8a6 6 0 01-11.47 2.47" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
              <path d="M14 2v4h-4M2 14v-4h4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
            Refresh
          </button>
        </div>

        {/* Table header */}
        <div className="hidden sm:grid grid-cols-[80px_1fr_100px_80px] gap-4 px-5 py-2 text-[0.6rem] text-aztb-500 font-mono uppercase tracking-wider border-b border-aztb-500/5">
          <span>Block</span>
          <span>Hash</span>
          <span className="text-right">Txs</span>
          <span className="text-right">Age</span>
        </div>

        {loading && blocks.length === 0 && (
          <div className="px-5 py-12 text-center">
            <div className="inline-block w-5 h-5 border-2 border-aztb-accent/30 border-t-aztb-accent rounded-full animate-spin mb-3" />
            <div className="text-sm text-aztb-500">Connecting to RPC...</div>
          </div>
        )}

        {!loading && blocks.length === 0 && (
          <div className="px-5 py-12 text-center text-sm text-aztb-500">
            No blocks loaded. Check RPC connection.
          </div>
        )}

        {filteredBlocks.length === 0 && search.trim() && blocks.length > 0 && (
          <div className="px-5 py-8 text-center text-sm text-aztb-500">
            No blocks matching &ldquo;{search}&rdquo;
          </div>
        )}

        {filteredBlocks.map((block) => {
          const txCount = block.transactions
            ? Array.isArray(block.transactions) ? block.transactions.length : 0
            : 0;
          return (
            <div
              key={block.hash || block.number}
              className="group px-5 py-3 border-b border-aztb-500/5 last:border-0 hover:bg-aztb-500/3 transition-colors
                         grid grid-cols-1 sm:grid-cols-[80px_1fr_100px_80px] gap-1 sm:gap-4 items-center"
            >
              <span className="text-sm font-mono font-semibold text-aztb-accent">
                #{block.number.toLocaleString()}
              </span>
              <span className="text-xs font-mono text-aztb-400 truncate opacity-70 group-hover:opacity-100 transition-opacity">
                {block.hash}
              </span>
              <span className="text-xs text-aztb-500 font-mono sm:text-right">
                {txCount > 0 ? (
                  <span className="inline-flex items-center gap-1">
                    <span className="w-1 h-1 rounded-full bg-aztb-green" />
                    {txCount} tx{txCount !== 1 ? "s" : ""}
                  </span>
                ) : (
                  <span className="text-aztb-600">empty</span>
                )}
              </span>
              <span className="text-xs text-aztb-500 font-mono sm:text-right">
                {timeAgo(block.timestamp)}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
