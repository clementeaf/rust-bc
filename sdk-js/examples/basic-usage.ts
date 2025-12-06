/**
 * Basic usage examples for Rust Blockchain SDK
 */

import { BlockchainClient } from '../src';

async function main() {
  // Initialize client
  const client = new BlockchainClient({
    baseUrl: 'http://127.0.0.1:8080/api/v1',
  });

  try {
    // Health check
    console.log('Checking health...');
    const health = await client.health();
    console.log('Health status:', health.status);
    console.log('Block count:', health.blockchain.block_count);

    // Create a wallet
    console.log('\nCreating wallet...');
    const wallet = await client.createWallet();
    console.log('Wallet created:');
    console.log('  Address:', wallet.address);
    console.log('  Balance:', wallet.balance);
    console.log('  Public Key:', wallet.public_key);

    // Get wallet balance
    console.log('\nGetting wallet balance...');
    const balance = await client.getWalletBalance(wallet.address);
    console.log('Current balance:', balance.balance);

    // Get blockchain info
    console.log('\nGetting blockchain info...');
    const info = await client.getBlockchainInfo();
    console.log('Blockchain length:', info.length);
    console.log('Difficulty:', info.difficulty);
    console.log('Latest block hash:', info.latest_block_hash);

    // Get all blocks
    console.log('\nGetting all blocks...');
    const blocks = await client.getBlocks();
    console.log('Total blocks:', blocks.length);

    if (blocks.length > 0) {
      // Get a specific block
      console.log('\nGetting block by hash...');
      const block = await client.getBlockByHash(blocks[0].hash);
      console.log('Block index:', block.index);
      console.log('Block transactions:', block.transactions.length);
    }

    // Get statistics
    console.log('\nGetting statistics...');
    const stats = await client.getStats();
    console.log('Block count:', stats.blockchain.block_count);
    console.log('Total transactions:', stats.blockchain.total_transactions);
    console.log('Pending transactions:', stats.mempool.pending_transactions);
    console.log('Connected peers:', stats.network.connected_peers);

    // Verify chain
    console.log('\nVerifying chain...');
    const verification = await client.verifyChain();
    console.log('Chain valid:', verification.is_valid);
    if (verification.errors.length > 0) {
      console.log('Errors:', verification.errors);
    }

  } catch (error) {
    console.error('Error:', error instanceof Error ? error.message : error);
  }
}

main();

