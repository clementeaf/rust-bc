/**
 * TypeScript types for Rust Blockchain SDK
 */

/** Gateway envelope (`/api/v1` handlers: blocks, chain, transactions, mempool, health, version). */
export interface GatewayApiResponse<T> {
  status: string;
  status_code: number;
  message: string;
  data?: T | null;
  error?: ErrorDto | null;
  timestamp: string;
  trace_id: string;
}

export interface ErrorDto {
  code: string;
  message: string;
  field?: string | null;
}

/** Legacy envelope (handlers not yet migrated to the gateway). */
export interface LegacyApiResponse<T> {
  success: boolean;
  data?: T;
  message?: string | null;
}

/** @deprecated Use `GatewayApiResponse<T>` or `LegacyApiResponse<T>`. */
export type ApiResponse<T> = LegacyApiResponse<T>;

export interface Block {
  index: number;
  timestamp: number;
  transactions: Transaction[];
  previous_hash: string;
  hash: string;
  nonce: number;
  merkle_root: string;
}

export interface Transaction {
  id: string;
  from: string;
  to: string;
  amount: number;
  fee: number;
  data?: string | null;
  timestamp: number;
  signature: string;
}

export interface Wallet {
  address: string;
  balance: number;
  public_key: string;
}

export interface BlockchainInfo {
  block_count: number;
  difficulty: number;
  latest_block_hash: string;
  is_valid: boolean;
}

/** GET /chain/verify (gateway). */
export interface ChainVerification {
  valid: boolean;
  block_count: number;
}

export interface Peer {
  address: string;
  port: number;
}

export interface MempoolTransaction extends Transaction {
  // Same as Transaction
}

export interface Stats {
  blockchain: {
    block_count: number;
    total_transactions: number;
    difficulty: number;
    latest_block_hash: string;
    latest_block_index: number;
    total_coinbase: number;
    unique_addresses: number;
    avg_block_time_seconds: number;
    target_block_time: number;
    max_transactions_per_block: number;
    max_block_size_bytes: number;
  };
  mempool: {
    pending_transactions: number;
    total_fees_pending: number;
  };
  network: {
    connected_peers: number;
  };
}

/** GET /health (gateway). */
export interface HealthCheck {
  status: string;
  uptime_seconds: number;
  blockchain: {
    height: number;
    last_block_hash: string;
    validators_count: number;
  };
}

export interface CreateTransactionRequest {
  from: string;
  to: string;
  amount: number;
  fee?: number;
  data?: string;
  signature?: string;
}

export interface CreateBlockRequest {
  transactions: CreateTransactionRequest[];
}

/** GET /mempool (gateway) — `data` payload. */
export interface MempoolListResponse {
  count: number;
  transactions: Transaction[];
}

/** POST /mine (legacy) — `data` payload. */
export interface MineBlockResponse {
  hash: string;
  reward: number;
  transactions_count: number;
  validator?: string | null;
  consensus: string;
}

export interface CreateAPIKeyRequest {
  tier: 'free' | 'basic' | 'pro' | 'enterprise';
}

export interface UsageStats {
  transactions_this_month: number;
  wallets_created: number;
  requests_this_month: number;
  tier: string;
  transaction_limit: number;
  wallet_limit: number;
}

export interface SDKConfig {
  baseUrl?: string;
  apiKey?: string;
  timeout?: number;
}

export interface SmartContract {
  address: string;
  owner: string;
  contract_type: string;
  name: string;
  symbol?: string | null;
  total_supply?: number | null;
  decimals?: number | null;
  state: {
    balances: Record<string, number>;
    metadata: Record<string, string>;
  };
  bytecode?: number[] | null;
  abi?: string | null;
  created_at: number;
  updated_at: number;
}

export interface DeployContractRequest {
  owner: string;
  contract_type: string;
  name: string;
  symbol?: string;
  total_supply?: number;
  decimals?: number;
}

export interface ExecuteContractRequest {
  function: 'transfer' | 'mint' | 'burn' | 'custom';
  params: {
    from?: string;
    to?: string;
    amount?: number;
    name?: string;
    params?: string[];
  };
}

// ── Fabric-style types ───────────────────────────────────────────────────────

/** POST /gateway/submit request body. */
export interface GatewaySubmitRequest {
  chaincode_id: string;
  channel_id?: string;
  transaction: {
    id: string;
    input_did: string;
    output_recipient: string;
    amount: number;
  };
}

/** POST /gateway/submit response payload. */
export interface GatewaySubmitResponse {
  tx_id: string;
  block_height: number;
  valid?: boolean;
}

/** POST /chaincode/{id}/simulate request body. */
export interface SimulateRequest {
  function: string;
  version?: string;
}

/** POST /chaincode/{id}/simulate response payload. */
export interface SimulateResponse {
  result: string;
  rwset: {
    reads: { key: string; version: number }[];
    writes: { key: string; value: string }[];
  };
}

/** Organization registration. */
export interface Organization {
  org_id: string;
  name: string;
  msp_id: string;
  root_cert?: string;
}

/** Endorsement policy. */
export type EndorsementPolicy =
  | { AnyOf: string[] }
  | { AllOf: string[] }
  | { NOutOf: { n: number; orgs: string[] } };

/** Channel creation request. */
export interface CreateChannelRequest {
  channel_id: string;
  member_orgs?: string[];
}

/** Channel info response. */
export interface ChannelInfo {
  channel_id: string;
}

/** Private data hash response from PUT. */
export interface PrivateDataWriteResponse {
  collection: string;
  key: string;
  hash: string;
}

