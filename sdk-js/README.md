# Rust Blockchain SDK

JavaScript/TypeScript SDK for interacting with the Rust Blockchain API.

## Installation

```bash
npm install @rust-bc/sdk
# or
yarn add @rust-bc/sdk
```

## Quick Start

```typescript
import { BlockchainClient } from '@rust-bc/sdk';

// Initialize client
const client = new BlockchainClient({
  baseUrl: 'http://127.0.0.1:8080/api/v1',
  apiKey: 'your-api-key-here', // Optional
});

// Create a wallet
const wallet = await client.createWallet();
console.log('Wallet address:', wallet.address);
console.log('Balance:', wallet.balance);

// Get wallet balance
const balance = await client.getWalletBalance(wallet.address);
console.log('Current balance:', balance.balance);

// Create a transaction
const transaction = await client.createTransaction({
  from: wallet.address,
  to: 'recipient-address',
  amount: 100,
  fee: 1,
});
console.log('Transaction ID:', transaction.id);

// Get all blocks
const blocks = await client.getBlocks();
console.log('Total blocks:', blocks.length);

// Get a specific block
const block = await client.getBlockByHash(blocks[0].hash);
console.log('Block transactions:', block.transactions.length);
```

## API Reference

### Initialization

```typescript
const client = new BlockchainClient({
  baseUrl?: string;      // Default: 'http://127.0.0.1:8080/api/v1'
  apiKey?: string;       // Optional API key
  timeout?: number;      // Default: 30000ms
});
```

### Wallet Operations

```typescript
// Create a new wallet
const wallet = await client.createWallet();

// Get wallet balance
const wallet = await client.getWalletBalance(address);

// Get wallet transactions
const transactions = await client.getWalletTransactions(address);
```

### Transaction Operations

```typescript
// Create a transaction
const transaction = await client.createTransaction({
  from: string;
  to: string;
  amount: number;
  fee?: number;
  data?: string;
  signature?: string;
});
```

### Block Operations

```typescript
// Get all blocks
const blocks = await client.getBlocks();

// Get block by hash
const block = await client.getBlockByHash(hash);

// Get block by index
const block = await client.getBlockByIndex(index);

// Create a new block
const block = await client.createBlock({
  transactions: CreateTransactionRequest[];
});
```

### Blockchain Operations

```typescript
// Verify blockchain integrity
const verification = await client.verifyChain();

// Get blockchain info
const info = await client.getBlockchainInfo();

// Get statistics
const stats = await client.getStats();
```

### Mining Operations

```typescript
// Mine a block
const block = await client.mineBlock(minerAddress, maxTransactions?);
```

### Network Operations

```typescript
// Get connected peers
const peers = await client.getPeers();

// Connect to a peer
await client.connectPeer(address);

// Sync blockchain
await client.syncBlockchain();
```

### Mempool Operations

```typescript
// Get pending transactions
const transactions = await client.getMempool();
```

### Billing Operations

```typescript
// Create API key
const apiKey = await client.createAPIKey({
  tier: 'free' | 'basic' | 'pro' | 'enterprise'
});

// Deactivate API key
await client.deactivateAPIKey(apiKey);

// Get usage statistics
const usage = await client.getBillingUsage();
```

### Utility Operations

```typescript
// Health check
const health = await client.health();

// Set/update API key
client.setApiKey('new-api-key');

// Remove API key
client.removeApiKey();
```

## Error Handling

All methods throw errors that can be caught:

```typescript
try {
  const wallet = await client.createWallet();
} catch (error) {
  console.error('Error:', error.message);
}
```

## Examples

See the `examples/` directory for more detailed examples:

- `basic-usage.ts` - Basic wallet and transaction operations
- `mining.ts` - Mining operations
- `billing.ts` - Billing and API key management
- `network.ts` - Network and peer operations

## TypeScript Support

This SDK is written in TypeScript and includes full type definitions. All types are exported:

```typescript
import { Block, Transaction, Wallet, Stats } from '@rust-bc/sdk';
```

## License

MIT

