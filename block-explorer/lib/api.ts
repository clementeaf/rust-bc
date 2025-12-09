/**
 * API client for Block Explorer
 */

import axios from 'axios';

const API_URL = process.env.API_URL || 'http://127.0.0.1:8080/api/v1';

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

export interface Stats {
  blockchain: {
    block_count: number;
    total_transactions: number;
    difficulty: number;
    latest_block_hash: string;
    latest_block_index: number;
    total_coinbase: number;
    unique_addresses: number;
  };
  mempool: {
    pending_transactions: number;
    total_fees_pending: number;
  };
  network: {
    connected_peers: number;
  };
}

const client = axios.create({
  baseURL: API_URL,
  timeout: 10000,
});

export async function getBlocks(): Promise<Block[]> {
  const response = await client.get<ApiResponse<Block[]>>('/blocks');
  if (response.data.success && response.data.data) {
    return response.data.data;
  }
  throw new Error(response.data.message || 'Failed to get blocks');
}

export async function getBlockByHash(hash: string): Promise<Block> {
  const response = await client.get<ApiResponse<Block>>(`/blocks/${hash}`);
  if (response.data.success && response.data.data) {
    return response.data.data;
  }
  throw new Error(response.data.message || 'Block not found');
}

export async function getBlockByIndex(index: number): Promise<Block> {
  const response = await client.get<ApiResponse<Block>>(`/blocks/index/${index}`);
  if (response.data.success && response.data.data) {
    return response.data.data;
  }
  throw new Error(response.data.message || 'Block not found');
}

export async function getWallet(address: string): Promise<Wallet> {
  const response = await client.get<ApiResponse<Wallet>>(`/wallets/${address}`);
  if (response.data.success && response.data.data) {
    return response.data.data;
  }
  throw new Error(response.data.message || 'Wallet not found');
}

export async function getWalletTransactions(address: string): Promise<Transaction[]> {
  const response = await client.get<ApiResponse<Transaction[]>>(`/wallets/${address}/transactions`);
  if (response.data.success && response.data.data) {
    return response.data.data;
  }
  throw new Error(response.data.message || 'Failed to get transactions');
}

export async function getStats(): Promise<Stats> {
  const response = await client.get<ApiResponse<Stats>>('/stats');
  if (response.data.success && response.data.data) {
    return response.data.data;
  }
  throw new Error(response.data.message || 'Failed to get stats');
}

export async function getMempool(): Promise<Transaction[]> {
  const response = await client.get<ApiResponse<Transaction[]>>('/mempool');
  if (response.data.success && response.data.data) {
    return response.data.data;
  }
  throw new Error(response.data.message || 'Failed to get mempool');
}

export interface Validator {
  address: string;
  staked_amount: number;
  is_active: boolean;
  total_rewards: number;
  created_at: number;
  last_validated_block: number;
  validation_count: number;
  slash_count: number;
  unstaking_requested: boolean;
  unstake_start_time: number | null;
}

export interface SmartContract {
  address: string;
  code: string;
  state: Record<string, unknown>;
  created_at: number;
  updated_at: number;
  update_sequence: number;
}

export async function getValidators(): Promise<Validator[]> {
  const response = await client.get<ApiResponse<Validator[]>>('/staking/validators');
  if (response.data.success && response.data.data) {
    return response.data.data;
  }
  throw new Error(response.data.message || 'Failed to get validators');
}

export async function getValidator(address: string): Promise<Validator> {
  const response = await client.get<ApiResponse<Validator>>(`/staking/validator/${address}`);
  if (response.data.success && response.data.data) {
    return response.data.data;
  }
  throw new Error(response.data.message || 'Validator not found');
}

export async function getAllContracts(): Promise<SmartContract[]> {
  const response = await client.get<ApiResponse<SmartContract[]>>('/contracts');
  if (response.data.success && response.data.data) {
    return response.data.data;
  }
  throw new Error(response.data.message || 'Failed to get contracts');
}

export async function getContract(address: string): Promise<SmartContract> {
  const response = await client.get<ApiResponse<SmartContract>>(`/contracts/${address}`);
  if (response.data.success && response.data.data) {
    return response.data.data;
  }
  throw new Error(response.data.message || 'Contract not found');
}

export async function searchByHash(hash: string): Promise<{ type: 'block' | 'transaction' | 'wallet' | 'contract'; data: unknown }> {
  try {
    const block = await getBlockByHash(hash);
    return { type: 'block', data: block };
  } catch {
    try {
      const contract = await getContract(hash);
      return { type: 'contract', data: contract };
    } catch {
      try {
        const wallet = await getWallet(hash);
        return { type: 'wallet', data: wallet };
      } catch {
        throw new Error('Not found');
      }
    }
  }
}

