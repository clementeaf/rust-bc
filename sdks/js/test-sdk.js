/**
 * Test completo del SDK JavaScript
 */

const { BlockchainClient } = require('./dist');

const API_URL = process.env.API_URL || 'http://127.0.0.1:8080/api/v1';

async function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function testSDK() {
  console.log('🧪 TEST COMPLETO DEL SDK');
  console.log('========================\n');

  const client = new BlockchainClient({
    baseUrl: API_URL,
  });

  let testsPassed = 0;
  let testsFailed = 0;

  // Test 1: Health Check
  try {
    console.log('📝 Test 1: Health Check');
    const health = await client.health();
    if (health.status === 'healthy') {
      console.log('   ✅ Health check OK\n');
      testsPassed++;
    } else {
      throw new Error('Health status not healthy');
    }
  } catch (error) {
    console.log(`   ❌ Error: ${error.message}\n`);
    testsFailed++;
  }

  await sleep(1000);

  // Test 2: Create Wallet
  let wallet;
  try {
    console.log('📝 Test 2: Create Wallet');
    wallet = await client.createWallet();
    if (wallet && wallet.address) {
      console.log(`   ✅ Wallet creado: ${wallet.address.substring(0, 40)}...\n`);
      testsPassed++;
    } else {
      throw new Error('Wallet creation failed');
    }
  } catch (error) {
    console.log(`   ❌ Error: ${error.message}\n`);
    testsFailed++;
    return;
  }

  await sleep(1000);

  // Test 3: Deploy Contract
  let contractAddress;
  try {
    console.log('📝 Test 3: Deploy Smart Contract');
    contractAddress = await client.deployContract({
      owner: wallet.address,
      contract_type: 'token',
      name: 'SDKTestToken',
      symbol: 'SDK',
      total_supply: 1000000,
      decimals: 18,
    });
    if (contractAddress && contractAddress.startsWith('contract_')) {
      console.log(`   ✅ Contrato desplegado: ${contractAddress.substring(0, 50)}...\n`);
      testsPassed++;
    } else {
      throw new Error('Contract deployment failed');
    }
  } catch (error) {
    console.log(`   ❌ Error: ${error.message}\n`);
    testsFailed++;
    return;
  }

  await sleep(1000);

  // Test 4: Get Contract
  try {
    console.log('📝 Test 4: Get Contract');
    const contract = await client.getContract(contractAddress);
    if (contract && contract.name === 'SDKTestToken') {
      console.log(`   ✅ Contrato obtenido: ${contract.name} (${contract.contract_type})\n`);
      testsPassed++;
    } else {
      throw new Error('Contract not found or incorrect');
    }
  } catch (error) {
    console.log(`   ❌ Error: ${error.message}\n`);
    testsFailed++;
  }

  await sleep(1000);

  // Test 5: Execute Contract Function (Mint)
  try {
    console.log('📝 Test 5: Execute Contract Function (Mint)');
    const result = await client.executeContractFunction(contractAddress, {
      function: 'mint',
      params: {
        to: wallet.address,
        amount: 500,
      },
    });
    if (result && result.includes('Minted')) {
      console.log(`   ✅ Función ejecutada: ${result}\n`);
      testsPassed++;
    } else {
      throw new Error('Function execution failed');
    }
  } catch (error) {
    console.log(`   ❌ Error: ${error.message}\n`);
    testsFailed++;
  }

  await sleep(1000);

  // Test 6: Get Contract Balance
  try {
    console.log('📝 Test 6: Get Contract Balance');
    const balance = await client.getContractBalance(contractAddress, wallet.address);
    if (typeof balance === 'number' && balance >= 0) {
      console.log(`   ✅ Balance obtenido: ${balance} tokens\n`);
      testsPassed++;
    } else {
      throw new Error('Balance retrieval failed');
    }
  } catch (error) {
    console.log(`   ❌ Error: ${error.message}\n`);
    testsFailed++;
  }

  await sleep(1000);

  // Test 7: Get All Contracts
  try {
    console.log('📝 Test 7: Get All Contracts');
    const contracts = await client.getAllContracts();
    if (Array.isArray(contracts) && contracts.length > 0) {
      console.log(`   ✅ Total de contratos: ${contracts.length}\n`);
      testsPassed++;
    } else {
      throw new Error('Failed to get contracts');
    }
  } catch (error) {
    console.log(`   ❌ Error: ${error.message}\n`);
    testsFailed++;
  }

  await sleep(1000);

  // Test 8: Get Blockchain Info
  try {
    console.log('📝 Test 8: Get Blockchain Info');
    const info = await client.getBlockchainInfo();
    if (info && info.block_count !== undefined) {
      console.log(`   ✅ Blockchain info: ${info.block_count} bloques, dificultad ${info.difficulty}\n`);
      testsPassed++;
    } else {
      throw new Error('Failed to get blockchain info');
    }
  } catch (error) {
    console.log(`   ❌ Error: ${error.message}\n`);
    testsFailed++;
  }

  // Resumen
  console.log('📊 RESUMEN DE TESTS');
  console.log('===================');
  console.log(`✅ Tests pasados: ${testsPassed}`);
  console.log(`❌ Tests fallidos: ${testsFailed}`);
  console.log(`📈 Tasa de éxito: ${((testsPassed / (testsPassed + testsFailed)) * 100).toFixed(1)}%`);
  console.log('');

  if (testsFailed === 0) {
    console.log('🎉 TODOS LOS TESTS PASARON');
  } else {
    console.log('⚠️  ALGUNOS TESTS FALLARON');
  }
}

testSDK().catch(console.error);

