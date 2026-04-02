/**
 * Core HTTP client for Rust Blockchain SDK
 */

import axios, { AxiosInstance, AxiosError } from 'axios';
import { unwrapGatewayData } from './envelope';
import {
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
  MempoolListResponse,
  MineBlockResponse,
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
   * Handle API errors (gateway and legacy bodies)
   */
  private handleError(error: unknown): never {
    if (axios.isAxiosError(error)) {
      const axiosError = error as AxiosError<Record<string, unknown>>;
      const data = axiosError.response?.data;
      let message = axiosError.message;
      if (data && typeof data === 'object') {
        if (typeof data.message === 'string' && data.message.length > 0) {
          message = data.message;
        } else if (
          data.error &&
          typeof data.error === 'object' &&
          data.error !== null &&
          'message' in data.error &&
          typeof (data.error as { message: unknown }).message === 'string'
        ) {
          message = (data.error as { message: string }).message;
        }
      }
      throw new Error(message || 'Unknown error occurred');
    }
    throw error;
  }

  /**
   * Health check endpoint (gateway envelope)
   */
  async health(): Promise<HealthCheck> {
    try {
      const response = await this.client.get<unknown>('/health');
      return unwrapGatewayData<HealthCheck>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get all blocks (gateway)
   */
  async getBlocks(): Promise<Block[]> {
    try {
      const response = await this.client.get<unknown>('/blocks');
      return unwrapGatewayData<Block[]>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get a block by hash (gateway)
   */
  async getBlockByHash(hash: string): Promise<Block> {
    try {
      const response = await this.client.get<unknown>(`/blocks/${hash}`);
      return unwrapGatewayData<Block>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get a block by index (gateway)
   */
  async getBlockByIndex(index: number): Promise<Block> {
    try {
      const response = await this.client.get<unknown>(`/blocks/index/${index}`);
      return unwrapGatewayData<Block>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Create a new block (gateway) — returns mined block hash string
   */
  async createBlock(request: CreateBlockRequest): Promise<string> {
    try {
      const response = await this.client.post<unknown>('/blocks', request);
      return unwrapGatewayData<string>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Create a new transaction (gateway)
   */
  async createTransaction(request: CreateTransactionRequest): Promise<Transaction> {
    try {
      const response = await this.client.post<unknown>('/transactions', request);
      return unwrapGatewayData<Transaction>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Create a new wallet (legacy)
   */
  async createWallet(): Promise<Wallet> {
    try {
      const response = await this.client.post<unknown>('/wallets/create');
      return unwrapGatewayData<Wallet>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get wallet balance (legacy)
   */
  async getWalletBalance(address: string): Promise<Wallet> {
    try {
      const response = await this.client.get<unknown>(`/wallets/${address}`);
      return unwrapGatewayData<Wallet>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get wallet transactions (legacy)
   */
  async getWalletTransactions(address: string): Promise<Transaction[]> {
    try {
      const response = await this.client.get<unknown>(`/wallets/${address}/transactions`);
      return unwrapGatewayData<Transaction[]>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Verify blockchain integrity (gateway)
   */
  async verifyChain(): Promise<ChainVerification> {
    try {
      const response = await this.client.get<unknown>('/chain/verify');
      return unwrapGatewayData<ChainVerification>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get blockchain info (gateway)
   */
  async getBlockchainInfo(): Promise<BlockchainInfo> {
    try {
      const response = await this.client.get<unknown>('/chain/info');
      return unwrapGatewayData<BlockchainInfo>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get connected peers (legacy)
   */
  async getPeers(): Promise<Peer[]> {
    try {
      const response = await this.client.get<unknown>('/peers');
      return unwrapGatewayData<Peer[]>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Connect to a peer (legacy)
   */
  async connectPeer(address: string): Promise<{ success: boolean; message: string }> {
    try {
      const response = await this.client.post<unknown>(`/peers/${address}/connect`);
      const message = unwrapGatewayData<string>(response.data);
      return { success: true, message };
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Sync blockchain with peers (legacy)
   */
  async syncBlockchain(): Promise<{ success: boolean; message: string }> {
    try {
      const response = await this.client.post<unknown>('/sync');
      const message = unwrapGatewayData<string>(response.data);
      return { success: true, message };
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Mine a block (legacy)
   */
  async mineBlock(minerAddress: string, maxTransactions?: number): Promise<MineBlockResponse> {
    try {
      const response = await this.client.post<unknown>('/mine', {
        miner_address: minerAddress,
        max_transactions: maxTransactions,
      });
      return unwrapGatewayData<MineBlockResponse>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get mempool transactions (gateway — unwraps `transactions` from payload)
   */
  async getMempool(): Promise<MempoolTransaction[]> {
    try {
      const response = await this.client.get<unknown>('/mempool');
      const data = unwrapGatewayData<MempoolListResponse | MempoolTransaction[]>(response.data);
      if (Array.isArray(data)) {
        return data;
      }
      return data.transactions;
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get blockchain statistics (legacy)
   */
  async getStats(): Promise<Stats> {
    try {
      const response = await this.client.get<unknown>('/stats');
      return unwrapGatewayData<Stats>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Create a new API key (legacy)
   */
  async createAPIKey(request: CreateAPIKeyRequest): Promise<string> {
    try {
      const response = await this.client.post<unknown>('/billing/create-key', request);
      return unwrapGatewayData<string>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Deactivate an API key (legacy)
   */
  async deactivateAPIKey(apiKey: string): Promise<{ success: boolean; message: string }> {
    try {
      const response = await this.client.post<unknown>('/billing/deactivate-key', {
        api_key: apiKey,
      });
      const message = unwrapGatewayData<string>(response.data);
      return { success: true, message };
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get billing usage statistics (legacy)
   */
  async getBillingUsage(): Promise<UsageStats> {
    try {
      const response = await this.client.get<unknown>('/billing/usage');
      return unwrapGatewayData<UsageStats>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Deploy a new smart contract (legacy)
   */
  async deployContract(request: DeployContractRequest): Promise<string> {
    try {
      const response = await this.client.post<unknown>('/contracts', request);
      return unwrapGatewayData<string>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get a contract by address (legacy)
   */
  async getContract(address: string): Promise<SmartContract> {
    try {
      const response = await this.client.get<unknown>(`/contracts/${address}`);
      return unwrapGatewayData<SmartContract>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get all contracts (legacy)
   */
  async getAllContracts(): Promise<SmartContract[]> {
    try {
      const response = await this.client.get<unknown>('/contracts');
      return unwrapGatewayData<SmartContract[]>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Execute a contract function (legacy)
   */
  async executeContractFunction(
    address: string,
    request: ExecuteContractRequest
  ): Promise<string> {
    try {
      const response = await this.client.post<unknown>(
        `/contracts/${address}/execute`,
        request
      );
      return unwrapGatewayData<string>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }

  /**
   * Get contract balance for a wallet address (legacy)
   */
  async getContractBalance(contractAddress: string, walletAddress: string): Promise<number> {
    try {
      const response = await this.client.get<unknown>(
        `/contracts/${contractAddress}/balance/${walletAddress}`
      );
      return unwrapGatewayData<number>(response.data);
    } catch (error) {
      this.handleError(error);
    }
  }
}
