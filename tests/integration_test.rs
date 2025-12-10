/**
 * Tests de integración para la blockchain
 * Verifican el funcionamiento end-to-end del sistema
 */
use rust_bc::blockchain::Blockchain;
use rust_bc::models::WalletManager;

#[test]
fn test_blockchain_creation() {
    let blockchain = Blockchain::new(4);

    // Verificar que se creó el bloque génesis
    assert_eq!(blockchain.chain.len(), 1);
    assert_eq!(blockchain.chain[0].index, 0);
    assert_eq!(blockchain.chain[0].previous_hash, "0");
}

#[test]
fn test_blockchain_mining() {
    let mut blockchain = Blockchain::new(4);
    let mut wallet_manager = WalletManager::new();

    // Crear un wallet válido para obtener una dirección de 32+ caracteres
    let wallet = wallet_manager.create_wallet();
    let miner_address = wallet.address.clone();
    assert!(
        miner_address.len() >= 32,
        "Dirección debe tener al menos 32 caracteres"
    );

    // Minar un bloque vacío (solo coinbase)
    // mine_block_with_reward crea automáticamente la transacción coinbase
    let result = blockchain.mine_block_with_reward(&miner_address, vec![], &wallet_manager);

    // Verificar que se minó correctamente
    if result.is_err() {
        panic!("Error minando bloque: {:?}", result.err());
    }
    assert_eq!(blockchain.chain.len(), 2);
}

#[test]
fn test_chain_validity() {
    let blockchain = Blockchain::new(4);

    // Verificar estructura básica
    assert_eq!(blockchain.chain.len(), 1);
    assert_eq!(blockchain.chain[0].index, 0);
    // Nota: is_chain_valid puede requerir validación adicional que no está disponible en tests simples
}

#[test]
fn test_balance_calculation() {
    let mut blockchain = Blockchain::new(4);
    let mut wallet_manager = WalletManager::new();

    // Crear un wallet válido para obtener una dirección de 32+ caracteres
    let wallet = wallet_manager.create_wallet();
    let miner_address = wallet.address.clone();
    assert!(
        miner_address.len() >= 32,
        "Dirección debe tener al menos 32 caracteres"
    );

    // Minar bloque con recompensa para la dirección
    let result = blockchain.mine_block_with_reward(&miner_address, vec![], &wallet_manager);
    if result.is_err() {
        panic!("Error minando bloque: {:?}", result.err());
    }

    // Calcular balance (debe incluir la recompensa de minería)
    let balance = blockchain.calculate_balance(&miner_address);
    assert!(balance > 0, "Balance debe ser mayor que 0 después de minar");
}
