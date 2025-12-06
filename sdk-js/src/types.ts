/**
 * TypeScript types for Rust Blockchain SDK
 */

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  message?: string | null;
}

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
  length: number;
  difficulty: number;
  total_coinbase: number;
  latest_block_hash: string;
}

export interface ChainVerification {
  is_valid: boolean;
  errors: string[];
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

export interface HealthCheck {
  status: string;
  version: string;
  blockchain: {
    block_count: number;
    latest_block_index: number;
    mempool_size: number;
  };
  cache: {
    size: number;
    last_block_index: number;
  };
  network: {
    connected_peers: number;
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

