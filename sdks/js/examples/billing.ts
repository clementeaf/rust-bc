/**
 * Billing examples for Rust Blockchain SDK
 */

import { BlockchainClient } from '../src';

async function main() {
  const client = new BlockchainClient({
    baseUrl: 'http://127.0.0.1:8080/api/v1',
  });

  try {
    // Create an API key
    console.log('Creating API key...');
    const apiKey = await client.createAPIKey({
      tier: 'free',
    });
    console.log('API key created:', apiKey);

    // Set the API key for subsequent requests
    client.setApiKey(apiKey);

    // Get billing usage
    console.log('\nGetting billing usage...');
    const usage = await client.getBillingUsage();
    console.log('Usage statistics:');
    console.log('  Tier:', usage.tier);
    console.log('  Transactions this month:', usage.transactions_this_month);
    console.log('  Transaction limit:', usage.transaction_limit);
    console.log('  Wallets created:', usage.wallets_created);
    console.log('  Wallet limit:', usage.wallet_limit);
    console.log('  Requests this month:', usage.requests_this_month);

    // Create a wallet with API key
    console.log('\nCreating wallet with API key...');
    const wallet = await client.createWallet();
    console.log('Wallet created:', wallet.address);

    // Check updated usage
    console.log('\nChecking updated usage...');
    const updatedUsage = await client.getBillingUsage();
    console.log('Wallets created:', updatedUsage.wallets_created);

    // Example: Create API keys for different tiers
    console.log('\nCreating API keys for different tiers...');
    const tiers: Array<'free' | 'basic' | 'pro' | 'enterprise'> = ['free', 'basic', 'pro', 'enterprise'];
    
    for (const tier of tiers) {
      try {
        const key = await client.createAPIKey({ tier });
        console.log(`${tier} tier key:`, key.substring(0, 30) + '...');
      } catch (error) {
        console.error(`Failed to create ${tier} key:`, error instanceof Error ? error.message : error);
      }
    }

    // Note: In production, you would want to deactivate test keys
    // await client.deactivateAPIKey(apiKey);

  } catch (error) {
    console.error('Error:', error instanceof Error ? error.message : error);
  }
}

main();

