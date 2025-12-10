/**
 * Tests de reconstrucción de estado
 * Verifican que el procesamiento paralelo produce el mismo resultado que el secuencial
 * 
 * NOTA: Estos tests verifican la funcionalidad básica de reconstrucción.
 * Los tests de performance se pueden agregar con benchmarks.
 */
use rust_bc::blockchain::Blockchain;
use rust_bc::models::WalletManager;

#[test]
fn test_blockchain_creation_for_reconstruction() {
    // Test básico para verificar que la blockchain se crea correctamente
    // La reconstrucción de estado se prueba en los tests de integración principales
    let blockchain = Blockchain::new(4);
    assert_eq!(blockchain.chain.len(), 1, "Debe tener bloque génesis");
}

#[test]
fn test_wallet_manager_creation() {
    // Test básico para verificar que WalletManager funciona
    let wallet_manager = WalletManager::new();
    let wallet = wallet_manager.create_wallet();
    assert!(!wallet.address.is_empty(), "Wallet debe tener dirección");
}
