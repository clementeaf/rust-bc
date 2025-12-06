/**
 * Transaction examples for Rust Blockchain SDK
 */

import { BlockchainClient } from '../src';

async function main() {
  const client = new BlockchainClient({
    baseUrl: 'http://127.0.0.1:8080/api/v1',
  });

  try {
    // Create two wallets
    console.log('Creating wallets...');
    const wallet1 = await client.createWallet();
    const wallet2 = await client.createWallet();
    
    console.log('Wallet 1:', wallet1.address);
    console.log('Wallet 2:', wallet2.address);

    // Mine some blocks to get funds for wallet1
    console.log('\nMining blocks for wallet1...');
    for (let i = 0; i < 3; i++) {
      const block = await client.mineBlock(wallet1.address, 1);
      console.log(`Mined block ${i + 1}:`, block.hash.substring(0, 20) + '...');
    }

    // Get updated balance
    const balance1 = await client.getWalletBalance(wallet1.address);
    console.log('\nWallet1 balance after mining:', balance1.balance);

    // Create a transaction
    console.log('\nCreating transaction...');
    const transaction = await client.createTransaction({
      from: wallet1.address,
      to: wallet2.address,
      amount: 50,
      fee: 1,
      data: 'Test transaction',
    });

    console.log('Transaction created:');
    console.log('  ID:', transaction.id);
    console.log('  From:', transaction.from);
    console.log('  To:', transaction.to);
    console.log('  Amount:', transaction.amount);
    console.log('  Fee:', transaction.fee);

    // Mine a block to include the transaction
    console.log('\nMining block to include transaction...');
    const block = await client.mineBlock(wallet1.address, 10);
    console.log('Block mined:', block.hash);

    // Check balances after transaction
    console.log('\nChecking balances after transaction...');
    const finalBalance1 = await client.getWalletBalance(wallet1.address);
    const finalBalance2 = await client.getWalletBalance(wallet2.address);
    
    console.log('Wallet1 final balance:', finalBalance1.balance);
    console.log('Wallet2 final balance:', finalBalance2.balance);

    // Get wallet transactions
    console.log('\nGetting wallet1 transactions...');
    const transactions = await client.getWalletTransactions(wallet1.address);
    console.log('Total transactions:', transactions.length);
    transactions.forEach((tx, index) => {
      console.log(`Transaction ${index + 1}:`, {
        id: tx.id.substring(0, 20) + '...',
        from: tx.from.substring(0, 20) + '...',
        to: tx.to.substring(0, 20) + '...',
        amount: tx.amount,
      });
    });

    // Get mempool
    console.log('\nGetting mempool...');
    const mempool = await client.getMempool();
    console.log('Pending transactions:', mempool.length);

  } catch (error) {
    console.error('Error:', error instanceof Error ? error.message : error);
  }
}

main();

