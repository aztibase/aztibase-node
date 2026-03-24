export type Hex = `0x${string}`;
export type Address = Hex;
export type Hash = Hex;

export interface Block {
  number: number;
  hash: Hash;
  parentHashes: Hash[];
  stateRoot: Hash;
  timestamp: number;
  transactions: Hash[] | Transaction[];
  txCount: number;
  gasUsed: number;
  gasLimit: number;
  proposer: Address;
}

export interface Transaction {
  hash: Hash;
  from: Address;
  to: Address;
  value: bigint;
  nonce: number;
  gasLimit: number;
  gasPrice: number;
  txType: TxKind;
  data: Hex;
  signature: Hex;
}

export interface TransactionReceipt {
  txHash: Hash;
  success: boolean;
  gasUsed: number;
  contractAddress: Address | null;
  error: string | null;
  inferenceHash: Hash | null;
}

export enum TxKind {
  Transfer = 0x01,
  ContractDeploy = 0x02,
  ContractCall = 0x03,
  EvmDeploy = 0x04,
  EvmCall = 0x05,
  AiInfer = 0x06,
  CreateAgent = 0x07,
  RegisterModel = 0x08,
  PostTask = 0x09,
  SubmitAttestation = 0x0a,
  CommitCompute = 0x0b,
  DeregisterCompute = 0x0c,
  DeregisterModel = 0x0d,
  CreateProposal = 0x0e,
  CastVote = 0x0f,
  Stake = 0x10,
  Unstake = 0x11,
  Delegate = 0x12,
  Undelegate = 0x13,
  SetAgentPolicy = 0x14,
  AgentExecute = 0x15,
  AnchorL2State = 0x16,
  BridgeDeposit = 0x17,
  BridgeWithdraw = 0x18,
  RegisterL2 = 0x19,
  RotateValidatorKey = 0x1a,
  FaucetDrip = 0x1b,
}

export interface Validator {
  validatorId: Address;
  selfStake: bigint;
  effectiveStake: bigint;
  isActive: boolean;
  commission: number;
}

export interface ConsensusState {
  round: number;
  wave: number;
  validators: number;
  committedBatches: number;
}

export interface NodeInfo {
  version: string;
  nodeType: string;
  chainId: Hex;
  peerCount: number;
}

export interface ChainHealth {
  score: number;
  level: 'normal' | 'warning' | 'critical';
  batchHeight: number;
  features: number[];
  featureNames: string[];
}

export interface AccountType {
  type: 'EOA' | 'Contract' | 'Agent';
}

export interface SentinelAction {
  kind: { type: string };
  score: number;
  dryRun: boolean;
}

export interface EpochSummary {
  epoch: number;
  validatorProfiles: unknown[];
  avgHealthScore: number;
}

export interface RpcError {
  code: number;
  message: string;
  data?: unknown;
}

export interface RpcResponse<T> {
  jsonrpc: '2.0';
  id: number;
  result?: T;
  error?: RpcError;
}

export interface ClientOptions {
  url: string;
  timeout?: number;
}
