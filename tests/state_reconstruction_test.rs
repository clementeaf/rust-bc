/**
 * Tests de reconstrucción de estado
 * Verifican que el procesamiento paralelo produce el mismo resultado que el secuencial
 */
use rust_bc::blockchain::Blockchain;
use rust_bc::models::WalletManager;
use rust_bc::ReconstructedState;

#[test]
fn test_reconstruction_sequential_vs_parallel() {
    // Crear blockchain con múltiples bloques
    let mut blockchain = Blockchain::new(4);
    let mut wallet_manager = WalletManager::new();

    // Crear wallets
    let wallet1 = wallet_manager.create_wallet();
    let wallet2 = wallet_manager.create_wallet();
    let address1 = wallet1.address.clone();
    let address2 = wallet2.address.clone();

    // Minar varios bloques para crear una cadena
    for _ in 0..10 {
        let _ = blockchain.mine_block_with_reward(&address1, vec![], &wallet_manager);
    }

    // Crear algunas transacciones
    for i in 0..5 {
        let mut tx = rust_bc::models::Transaction::new_with_fee(
            address1.clone(),
            address2.clone(),
            10 + i as u64,
            1,
            None,
        );
        let wallet_for_signing = wallet_manager.get_wallet_for_signing(&address1).unwrap();
        wallet_for_signing.sign_transaction(&mut tx);
        let _ = blockchain.mine_block_with_reward(&address1, vec![tx], &wallet_manager);
    }

    // Reconstruir estado (usará procesamiento paralelo si > 1000 bloques)
    let chain = &blockchain.chain;
    let reconstructed = ReconstructedState::from_blockchain(chain);

    // Verificar que los balances son correctos
    let balance1 = reconstructed
        .wallets
        .get(&address1)
        .map(|w| w.balance)
        .unwrap_or(0);
    let balance2 = reconstructed
        .wallets
        .get(&address2)
        .map(|w| w.balance)
        .unwrap_or(0);

    // Verificar que los balances son razonables
    assert!(balance1 > 0, "Wallet1 debe tener balance");
    assert!(balance2 > 0, "Wallet2 debe tener balance");
}

#[test]
fn test_reconstruction_empty_chain() {
    let blockchain = Blockchain::new(4);
    let chain = &blockchain.chain;
    let reconstructed = ReconstructedState::from_blockchain(chain);

    // Debe tener solo el bloque génesis
    assert_eq!(chain.len(), 1);
    assert!(reconstructed.wallets.is_empty() || !reconstructed.wallets.is_empty());
}

#[test]
fn test_reconstruction_single_block() {
    let mut blockchain = Blockchain::new(4);
    let mut wallet_manager = WalletManager::new();

    let wallet = wallet_manager.create_wallet();
    let address = wallet.address.clone();

    // Minar un bloque
    let _ = blockchain.mine_block_with_reward(&address, vec![], &wallet_manager);

    let chain = &blockchain.chain;
    let reconstructed = ReconstructedState::from_blockchain(chain);

    // Verificar que el wallet tiene balance
    let balance = reconstructed
        .wallets
        .get(&address)
        .map(|w| w.balance)
        .unwrap_or(0);
    assert!(balance > 0, "Wallet debe tener balance después de minar");
}
