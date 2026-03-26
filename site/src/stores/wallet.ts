"use client";
import { create } from "zustand";

interface AztibaseProvider {
  isAztibase: boolean;
  connect: () => Promise<{ address: string; connected: boolean; locked: boolean }>;
  disconnect: () => Promise<void>;
  getBalance: () => Promise<{ balance: string; nonce: number }>;
  getNonce: () => Promise<string | number>;
  getAddress: () => Promise<{ address: string }>;
  isConnected: () => Promise<{ connected: boolean; address: string | null }>;
  signAndSendTransfer: (to: string, amount: number, gasPrice: number) => Promise<{ txHash: string }>;
  signAndSendEvmCall: (to: string, data: string, gasLimit: number, value: number) => Promise<{ txHash: string }>;
  faucetDrip: () => Promise<unknown>;
}

declare global {
  interface Window {
    aztibase?: AztibaseProvider;
    __aztibaseManualKey?: string;
  }
}

type ConnectionMode = "extension" | "key" | null;

interface WalletState {
  address: string | null;
  connected: boolean;
  mode: ConnectionMode;
  secretKey: string | null;

  connectExtension: () => Promise<void>;
  connectWithKey: (hex: string) => Promise<void>;
  disconnect: () => void;
  setAddress: (addr: string, mode: ConnectionMode) => void;
}

export const useWalletStore = create<WalletState>((set) => ({
  address: null,
  connected: false,
  mode: null,
  secretKey: null,

  connectExtension: async () => {
    if (!window.aztibase) throw new Error("Aztibase Wallet extension not detected");
    const res = await window.aztibase.connect();
    if (res.locked) throw new Error("Wallet locked — open extension to unlock");
    if (!res.address) throw new Error("No wallet found");
    set({ address: res.address, connected: true, mode: "extension", secretKey: null });
  },

  connectWithKey: async (hex: string) => {
    const clean = hex.replace(/^0x/, "");
    if (clean.length !== 64) throw new Error("Key must be 64 hex characters");
    const ed = await import("@noble/ed25519");
    const hashes = await import("@noble/hashes/blake3.js");
    const blake3 = hashes.blake3;
    const keyBytes = new Uint8Array(clean.match(/.{2}/g)!.map((b: string) => parseInt(b, 16)));
    const pub = await ed.getPublicKeyAsync(keyBytes);
    const addr = blake3(pub);
    const addrHex = "0x" + Array.from(addr as Uint8Array).map((b) => b.toString(16).padStart(2, "0")).join("");
    set({ address: addrHex, connected: true, mode: "key", secretKey: clean });
  },

  disconnect: () => {
    if (window.aztibase) window.aztibase.disconnect().catch(() => {});
    set({ address: null, connected: false, mode: null, secretKey: null });
  },

  setAddress: (addr, mode) => set({ address: addr, connected: true, mode }),
}));
