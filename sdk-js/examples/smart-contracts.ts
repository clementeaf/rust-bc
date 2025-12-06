/**
 * Smart Contracts examples for Rust Blockchain SDK
 */

import { BlockchainClient } from '../src';

async function main() {
  const client = new BlockchainClient({
    baseUrl: 'http://127.0.0.1:8080/api/v1',
  });

  try {
    // Create a wallet for the contract owner
    console.log('Creating wallet for contract owner...');
    const ownerWallet = await client.createWallet();
    console.log('Owner wallet:', ownerWallet.address);

    // Deploy a token contract
    console.log('\nDeploying token contract...');
    const contractAddress = await client.deployContract({
      owner: ownerWallet.address,
      contract_type: 'token',
      name: 'MyToken',
      symbol: 'MTK',
      total_supply: 1000000,
      decimals: 18,
    });
    console.log('Contract deployed at:', contractAddress);

    // Get contract details
    console.log('\nGetting contract details...');
    const contract = await client.getContract(contractAddress);
    console.log('Contract name:', contract.name);
    console.log('Contract symbol:', contract.symbol);
    console.log('Total supply:', contract.total_supply);
    console.log('Owner:', contract.owner);

    // Create another wallet to receive tokens
    console.log('\nCreating recipient wallet...');
    const recipientWallet = await client.createWallet();
    console.log('Recipient wallet:', recipientWallet.address);

    // Mint tokens to the owner
    console.log('\nMinting tokens to owner...');
    const mintResult = await client.executeContractFunction(contractAddress, {
      function: 'mint',
      params: {
        to: ownerWallet.address,
        amount: 1000,
      },
    });
    console.log('Mint result:', mintResult);

    // Check owner balance
    console.log('\nChecking owner balance...');
    const ownerBalance = await client.getContractBalance(contractAddress, ownerWallet.address);
    console.log('Owner balance:', ownerBalance);

    // Transfer tokens from owner to recipient
    console.log('\nTransferring tokens...');
    const transferResult = await client.executeContractFunction(contractAddress, {
      function: 'transfer',
      params: {
        from: ownerWallet.address,
        to: recipientWallet.address,
        amount: 100,
      },
    });
    console.log('Transfer result:', transferResult);

    // Check balances after transfer
    console.log('\nChecking balances after transfer...');
    const ownerBalanceAfter = await client.getContractBalance(contractAddress, ownerWallet.address);
    const recipientBalance = await client.getContractBalance(contractAddress, recipientWallet.address);
    console.log('Owner balance:', ownerBalanceAfter);
    console.log('Recipient balance:', recipientBalance);

    // Burn some tokens
    console.log('\nBurning tokens from owner...');
    const burnResult = await client.executeContractFunction(contractAddress, {
      function: 'burn',
      params: {
        from: ownerWallet.address,
        amount: 50,
      },
    });
    console.log('Burn result:', burnResult);

    // Final balance check
    console.log('\nFinal owner balance:');
    const finalBalance = await client.getContractBalance(contractAddress, ownerWallet.address);
    console.log('Final balance:', finalBalance);

    // Get all contracts
    console.log('\nGetting all contracts...');
    const allContracts = await client.getAllContracts();
    console.log('Total contracts:', allContracts.length);
    allContracts.forEach((contract, index) => {
      console.log(`Contract ${index + 1}:`, {
        address: contract.address.substring(0, 30) + '...',
        name: contract.name,
        type: contract.contract_type,
      });
    });

  } catch (error) {
    console.error('Error:', error instanceof Error ? error.message : error);
  }
}

main();

