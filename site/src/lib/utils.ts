export function shortenAddress(addr: string, chars = 6): string {
  if (!addr || addr.length < chars * 2 + 2) return addr || "";
  return addr.slice(0, chars + 2) + "..." + addr.slice(-chars);
}

export function formatBalance(wei: bigint, decimals = 18, display = 4): string {
  if (wei === 0n) return "0";
  const whole = wei / 10n ** BigInt(decimals);
  const frac = wei % 10n ** BigInt(decimals);
  const fracStr = frac.toString().padStart(decimals, "0").slice(0, display).replace(/0+$/, "");
  if (!fracStr) return whole.toString();
  return whole.toString() + "." + fracStr;
}

export function hexToBigInt(hex: string): bigint {
  const clean = hex.replace(/^0x/, "");
  if (!clean || clean === "0") return 0n;
  return BigInt("0x" + clean);
}

export function pad32(hex: string): string {
  return hex.replace(/^0x/, "").padStart(64, "0");
}

export function toHex256(n: bigint): string {
  return n.toString(16).padStart(64, "0");
}

export function cn(...classes: (string | false | null | undefined)[]): string {
  return classes.filter(Boolean).join(" ");
}

export function toEvmAddress(addr: string): string {
  const clean = addr.replace(/^0x/, "").replace(/0+$/, "");
  const evm20 = clean.slice(0, 40);
  return evm20.padStart(64, "0");
}
