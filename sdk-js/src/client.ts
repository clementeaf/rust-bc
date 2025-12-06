/**
 * Core HTTP client for Rust Blockchain SDK
 */

import axios, { AxiosInstance, AxiosError } from 'axios';
import {
  ApiResponse,
  Block,
  Transaction,
  Wallet,
  BlockchainInfo,
  ChainVerification,
  Peer,
  MempoolTransaction,
  Stats,
  HealthCheck,
  CreateTransactionRequest,
  CreateBlockRequest,
  CreateAPIKeyRequest,
  UsageStats,
  SDKConfig,
  SmartContract,
  DeployContractRequest,
  ExecuteContractRequest,
} from './types';

export class BlockchainClient {
  private client: AxiosInstance;
  private apiKey?: string;

  constructor(config: SDKConfig = {}) {
    const baseURL = config.baseUrl || 'http://127.0.0.1:8080/api/v1';
    const timeout = config.timeout || 30000;

    this.client = axios.create({
      baseURL,
      timeout,
      headers: {
        'Content-Type': 'application/json',
      },
    });

    if (config.apiKey) {
      this.setApiKey(config.apiKey);
    }
  }

  /**
   * Set or update the API key
   */
  setApiKey(apiKey: string): void {
    this.apiKey = apiKey;
    this.client.defaults.headers.common['X-API-Key'] = apiKey;
  }

  /**
   * Remove the API key
   */
  removeApiKey(): void {
    this.apiKey = undefined;
    delete this.client.defaults.headers.common['X-API-Key'];
  }

  /**
   * Handle API errors
   */
  private handleError(error: unknown): never {
    if (axios.isAxiosError(error)) {
      const axiosError = error as AxiosError<ApiResponse<unknown>>;
      const message = axiosError.response?.data?.message || axiosError.message;
      throw new Error(message || 'Unknown error occurred');
    }
    throw error;
  }

  /**
   * Health check endpoint
   */
  async health(): Promise<HealthCheck> {
    try {
      const response = await this.client.get<ApiResponse<HealthCheck>>('/health');
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Health check failed');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get all blocks
   */
  async getBlocks(): Promise<Block[]> {
    try {
      const response = await this.client.get<ApiResponse<Block[]>>('/blocks');
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Failed to get blocks');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get a block by hash
   */
  async getBlockByHash(hash: string): Promise<Block> {
    try {
      const response = await this.client.get<ApiResponse<Block>>(`/blocks/${hash}`);
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Block not found');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get a block by index
   */
  async getBlockByIndex(index: number): Promise<Block> {
    try {
      const response = await this.client.get<ApiResponse<Block>>(`/blocks/index/${index}`);
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Block not found');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Create a new block
   */
  async createBlock(request: CreateBlockRequest): Promise<Block> {
    try {
      const response = await this.client.post<ApiResponse<Block>>('/blocks', request);
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Failed to create block');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Create a new transaction
   */
  async createTransaction(request: CreateTransactionRequest): Promise<Transaction> {
    try {
      const response = await this.client.post<ApiResponse<Transaction>>('/transactions', request);
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Failed to create transaction');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Create a new wallet
   */
  async createWallet(): Promise<Wallet> {
    try {
      const response = await this.client.post<ApiResponse<Wallet>>('/wallets/create');
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Failed to create wallet');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get wallet balance
   */
  async getWalletBalance(address: string): Promise<Wallet> {
    try {
      const response = await this.client.get<ApiResponse<Wallet>>(`/wallets/${address}`);
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Wallet not found');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get wallet transactions
   */
  async getWalletTransactions(address: string): Promise<Transaction[]> {
    try {
      const response = await this.client.get<ApiResponse<Transaction[]>>(
        `/wallets/${address}/transactions`
      );
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Failed to get wallet transactions');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Verify blockchain integrity
   */
  async verifyChain(): Promise<ChainVerification> {
    try {
      const response = await this.client.get<ApiResponse<ChainVerification>>('/chain/verify');
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Chain verification failed');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get blockchain info
   */
  async getBlockchainInfo(): Promise<BlockchainInfo> {
    try {
      const response = await this.client.get<ApiResponse<BlockchainInfo>>('/chain/info');
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Failed to get blockchain info');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get connected peers
   */
  async getPeers(): Promise<Peer[]> {
    try {
      const response = await this.client.get<ApiResponse<Peer[]>>('/peers');
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Failed to get peers');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Connect to a peer
   */
  async connectPeer(address: string): Promise<{ success: boolean; message: string }> {
    try {
      const response = await this.client.post<ApiResponse<string>>(`/peers/${address}/connect`);
      if (response.data.success) {
        return { success: true, message: response.data.message || 'Connected successfully' };
      }
      throw new Error(response.data.message || 'Failed to connect to peer');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Sync blockchain with peers
   */
  async syncBlockchain(): Promise<{ success: boolean; message: string }> {
    try {
      const response = await this.client.post<ApiResponse<string>>('/sync');
      if (response.data.success) {
        return { success: true, message: response.data.message || 'Sync completed' };
      }
      throw new Error(response.data.message || 'Sync failed');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Mine a block
   */
  async mineBlock(minerAddress: string, maxTransactions?: number): Promise<Block> {
    try {
      const response = await this.client.post<ApiResponse<Block>>('/mine', {
        miner_address: minerAddress,
        max_transactions: maxTransactions,
      });
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Mining failed');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get mempool transactions
   */
  async getMempool(): Promise<MempoolTransaction[]> {
    try {
      const response = await this.client.get<ApiResponse<MempoolTransaction[]>>('/mempool');
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Failed to get mempool');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get blockchain statistics
   */
  async getStats(): Promise<Stats> {
    try {
      const response = await this.client.get<ApiResponse<Stats>>('/stats');
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Failed to get stats');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Create a new API key
   */
  async createAPIKey(request: CreateAPIKeyRequest): Promise<string> {
    try {
      const response = await this.client.post<ApiResponse<string>>('/billing/create-key', request);
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Failed to create API key');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Deactivate an API key
   */
  async deactivateAPIKey(apiKey: string): Promise<{ success: boolean; message: string }> {
    try {
      const response = await this.client.post<ApiResponse<string>>('/billing/deactivate-key', {
        api_key: apiKey,
      });
      if (response.data.success) {
        return { success: true, message: response.data.message || 'API key deactivated' };
      }
      throw new Error(response.data.message || 'Failed to deactivate API key');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get billing usage statistics
   */
  async getBillingUsage(): Promise<UsageStats> {
    try {
      const response = await this.client.get<ApiResponse<UsageStats>>('/billing/usage');
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Failed to get billing usage');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Deploy a new smart contract
   */
  async deployContract(request: DeployContractRequest): Promise<string> {
    try {
      const response = await this.client.post<ApiResponse<string>>('/contracts', request);
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Failed to deploy contract');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get a contract by address
   */
  async getContract(address: string): Promise<SmartContract> {
    try {
      const response = await this.client.get<ApiResponse<SmartContract>>(`/contracts/${address}`);
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Contract not found');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get all contracts
   */
  async getAllContracts(): Promise<SmartContract[]> {
    try {
      const response = await this.client.get<ApiResponse<SmartContract[]>>('/contracts');
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Failed to get contracts');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Execute a contract function
   */
  async executeContractFunction(
    address: string,
    request: ExecuteContractRequest
  ): Promise<string> {
    try {
      const response = await this.client.post<ApiResponse<string>>(
        `/contracts/${address}/execute`,
        request
      );
      if (response.data.success && response.data.data) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Failed to execute contract function');
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get contract balance for a wallet address
   */
  async getContractBalance(contractAddress: string, walletAddress: string): Promise<number> {
    try {
      const response = await this.client.get<ApiResponse<number>>(
        `/contracts/${contractAddress}/balance/${walletAddress}`
      );
      if (response.data.success && response.data.data !== undefined) {
        return response.data.data;
      }
      throw new Error(response.data.message || 'Failed to get contract balance');
    } catch (error) {
      this.handleError(error);
    }
  }
}

