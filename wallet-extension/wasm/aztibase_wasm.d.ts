/* tslint:disable */
/* eslint-disable */

/**
 * Derive the address from a secret key hex string.
 * Returns hex address on success, or `"error: ..."` on failure.
 */
export function addressFromSecret(secret_hex: string): string;

export function blake3Hash(data: Uint8Array): string;

export function browserPerTxLimit(): string;

export function browserSpendingLimit(): string;

/**
 * Build a JSON-RPC request for `aztb_estimateGas`.
 */
export function buildEstimateGasRequest(tx_type: string, id: number): string;

/**
 * Build a JSON-RPC request for `aztb_getActiveValidators`.
 */
export function buildGetActiveValidatorsRequest(id: number): string;

/**
 * Build a JSON-RPC request for `aztb_getBalance`.
 */
export function buildGetBalanceRequest(address_hex: string, id: number): string;

/**
 * Build a JSON-RPC request for `aztb_getNonce`.
 */
export function buildGetNonceRequest(address_hex: string, id: number): string;

/**
 * Build a JSON-RPC request for `aztb_getValidatorStake`.
 */
export function buildGetValidatorStakeRequest(address_hex: string, id: number): string;

export function buildHeaderRequest(from_round: bigint, count: bigint): string;

/**
 * Build a JSON-RPC request body to send a raw transaction.
 * Returns a JSON string ready to POST to the node's RPC endpoint.
 */
export function buildSendTxRequest(signed_tx_hex: string, id: number): string;

export function checkBrowserBalance(balance_str: string): any;

export function checkBrowserTx(amount_str: string): any;

/**
 * Generate a new Ed25519 keypair. Returns JSON: `{"secret": "hex", "public": "hex", "address": "hex"}`.
 */
export function generateKeypair(): string;

export function latestSyncedRound(state_json: string): any;

/**
 * Sign a Delegate transaction and return the raw signed envelope as hex.
 */
export function signDelegate(secret_hex: string, validator_hex: string, amount_str: string, nonce: bigint, gas_price: bigint): string;

/**
 * Sign a RegisterValidator transaction and return the raw signed envelope as hex.
 * Amount of 0 registers without initial stake (free registration).
 */
export function signRegisterValidator(secret_hex: string, amount_str: string, nonce: bigint, gas_price: bigint): string;

/**
 * Sign a Stake transaction and return the raw signed envelope as hex.
 */
export function signStake(secret_hex: string, amount_str: string, nonce: bigint, gas_price: bigint): string;

/**
 * Sign a Transfer transaction and return the raw signed envelope as a hex string.
 *
 * Parameters:
 * - `secret_hex`: 64-char hex Ed25519 secret key
 * - `to_hex`: 64-char hex recipient address
 * - `value_str`: transfer amount as decimal string (u128)
 * - `nonce`: sender nonce
 * - `gas_price`: gas price
 */
export function signTransfer(secret_hex: string, to_hex: string, value_str: string, nonce: bigint, gas_price: bigint): string;

/**
 * Sign an Undelegate transaction and return the raw signed envelope as hex.
 */
export function signUndelegate(secret_hex: string, nonce: bigint, gas_price: bigint): string;

/**
 * Sign an Unstake transaction and return the raw signed envelope as hex.
 */
export function signUnstake(secret_hex: string, amount_str: string, nonce: bigint, gas_price: bigint): string;

export function verifyHeaderChain(headers_json: string, cert_json: string, expected_start: bigint): any;

export function verifyLightClientProof(proof_json: string, leaf_hex: string): any;

export function verifyMerkleProof(proof_json: string, root_hex: string, leaf_hex: string): any;

export function verifyVerkleProof(proof_json: string, root_hex: string): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly addressFromSecret: (a: number, b: number, c: number) => void;
    readonly buildEstimateGasRequest: (a: number, b: number, c: number, d: number) => void;
    readonly buildGetActiveValidatorsRequest: (a: number, b: number) => void;
    readonly buildGetBalanceRequest: (a: number, b: number, c: number, d: number) => void;
    readonly buildGetNonceRequest: (a: number, b: number, c: number, d: number) => void;
    readonly buildGetValidatorStakeRequest: (a: number, b: number, c: number, d: number) => void;
    readonly buildSendTxRequest: (a: number, b: number, c: number, d: number) => void;
    readonly generateKeypair: (a: number) => void;
    readonly signDelegate: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: bigint, i: bigint) => void;
    readonly signRegisterValidator: (a: number, b: number, c: number, d: number, e: number, f: bigint, g: bigint) => void;
    readonly signStake: (a: number, b: number, c: number, d: number, e: number, f: bigint, g: bigint) => void;
    readonly signTransfer: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: bigint, i: bigint) => void;
    readonly signUndelegate: (a: number, b: number, c: number, d: bigint, e: bigint) => void;
    readonly signUnstake: (a: number, b: number, c: number, d: number, e: number, f: bigint, g: bigint) => void;
    readonly blake3Hash: (a: number, b: number, c: number) => void;
    readonly buildHeaderRequest: (a: number, b: bigint, c: bigint) => void;
    readonly latestSyncedRound: (a: number, b: number) => number;
    readonly verifyHeaderChain: (a: number, b: number, c: number, d: number, e: bigint) => number;
    readonly verifyLightClientProof: (a: number, b: number, c: number, d: number) => number;
    readonly verifyMerkleProof: (a: number, b: number, c: number, d: number, e: number, f: number) => number;
    readonly verifyVerkleProof: (a: number, b: number, c: number, d: number) => number;
    readonly browserPerTxLimit: (a: number) => void;
    readonly browserSpendingLimit: (a: number) => void;
    readonly checkBrowserBalance: (a: number, b: number) => number;
    readonly checkBrowserTx: (a: number, b: number) => number;
    readonly __wbindgen_export: (a: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export2: (a: number, b: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_export4: (a: number, b: number, c: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
