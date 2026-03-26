import { CHAIN } from "@/config/chain";

let rpcUrl = CHAIN.rpc;
let primaryConfirmed = false;
let reqId = 0;

async function rawFetch(url: string, method: string, params: unknown[]): Promise<unknown> {
  const res = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", method, params, id: ++reqId }),
    signal: AbortSignal.timeout(8_000),
  });
  const json = await res.json();
  if (json.error) throw new Error(json.error.message || JSON.stringify(json.error));
  return json.result;
}

export async function rpc<T = unknown>(method: string, params: unknown[] = []): Promise<T> {
  try {
    const result = (await rawFetch(rpcUrl, method, params)) as T;
    primaryConfirmed = true;
    return result;
  } catch (primaryErr) {
    if (!primaryConfirmed && CHAIN.rpcFallback && rpcUrl !== CHAIN.rpcFallback) {
      try {
        const result = (await rawFetch(CHAIN.rpcFallback, method, params)) as T;
        rpcUrl = CHAIN.rpcFallback;
        primaryConfirmed = true;
        return result;
      } catch {
        throw primaryErr;
      }
    }
    throw primaryErr;
  }
}

export function setRpcUrl(url: string) {
  rpcUrl = url;
  primaryConfirmed = false;
}

export function getRpcUrl() {
  return rpcUrl;
}

export async function getBalance(address: string): Promise<bigint> {
  const res = await rpc<string | number>("aztb_getBalance", [address]);
  if (res === null || res === undefined) return 0n;
  if (typeof res === "number") return BigInt(res);
  const str = String(res);
  if (str.startsWith("0x")) return BigInt(str);
  return BigInt(str);
}

export async function getNonce(address: string): Promise<number> {
  const res = await rpc<string | number>("aztb_getNonce", [address]);
  if (typeof res === "string") return parseInt(res, 16);
  return Number(res);
}

export async function getBlockNumber(): Promise<number> {
  const res = await rpc<string | number>("aztb_blockNumber");
  if (typeof res === "string") return parseInt(res, 16);
  return Number(res);
}

export async function evmCall(to: string, data: string): Promise<string> {
  return rpc<string>("aztb_call", [{ to, data }]);
}

export async function sendTransaction(envelope: string): Promise<string> {
  return rpc<string>("aztb_sendTransaction", [envelope]);
}

export async function getTransactionReceipt(hash: string) {
  return rpc<{ success: boolean; gasUsed: string; error?: string } | null>(
    "aztb_getTransactionReceipt",
    [hash]
  );
}

export async function faucetDrip(address: string) {
  return rpc("aztb_faucetDrip", [address]);
}

export async function getBlockByNumber(num: number | string) {
  return rpc("aztb_getBlockByNumber", [typeof num === "number" ? "0x" + num.toString(16) : num]);
}

export async function getBlockRange(from: number, count: number) {
  return rpc("aztb_getBlockRange", [
    "0x" + from.toString(16),
    "0x" + count.toString(16),
  ]);
}

export async function getNodeInfo() {
  return rpc("aztb_nodeInfo");
}

export async function getPeerCount(): Promise<number> {
  const res = await rpc<string | number>("aztb_peerCount");
  if (typeof res === "string") return parseInt(res, 16);
  return Number(res);
}

export async function getValidators() {
  return rpc("aztb_getActiveValidators");
}

export async function getConsensusState() {
  return rpc("aztb_consensusState");
}
