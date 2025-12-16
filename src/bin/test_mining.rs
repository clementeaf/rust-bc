/**
 * Binario de prueba para minería manual
 * Permite probar la minería con diferentes dificultades y ver el progreso en tiempo real
 * 
 * Uso: cargo run --bin test_mining --release -- [dificultad]
 * Ejemplo: cargo run --bin test_mining --release -- 1
 */
use rust_bc::blockchain::{Blockchain, Block};
use rust_bc::models::WalletManager;
use std::time::Instant;

fn main() {
    let difficulty: u8 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(1);
    
    println!("==========================================");
    println!("Test de Minería Manual - Dificultad: {}", difficulty);
    println!("==========================================");
    println!();
    
    println!("Iniciando blockchain con dificultad: {}", difficulty);
    let blockchain = Blockchain::new(difficulty);
    let mut wallet_manager = WalletManager::new();
    
    // Crear wallet para minero
    let wallet = wallet_manager.create_wallet();
    let miner_address = wallet.address.clone();
    
    println!("Dirección del minero: {}", miner_address);
    println!("Bloque génesis creado. Hash: {}", blockchain.chain[0].hash);
    println!();
    println!("Iniciando minería del bloque #1...");
    println!("(Esto puede tomar tiempo dependiendo de la dificultad)");
    println!();
    
    let start = Instant::now();
    let mut last_print = Instant::now();
    let mut hash_count = 0u64;
    
    // Obtener parámetros para minería
    let previous_hash = blockchain.get_latest_block().hash.clone();
    let index = blockchain.chain.len() as u64;
    let difficulty_target = blockchain.difficulty;
    
    // Crear transacción coinbase
    let coinbase = Blockchain::create_coinbase_transaction(&miner_address, Some(50));
    
    let mut test_block = Block::new(index, vec![coinbase], previous_hash, difficulty_target);
    
    println!("Dificultad: {}", difficulty_target);
    println!("Target: {} ceros al inicio del hash", difficulty_target);
    println!("Buscando hash válido...");
    println!();
    
    loop {
        test_block.hash = test_block.calculate_hash();
        hash_count += 1;
        
        // Mostrar progreso cada 100,000 hashes o cada segundo
        if hash_count % 100_000 == 0 || last_print.elapsed().as_secs() >= 1 {
            let elapsed = start.elapsed();
            let hashes_per_sec = if elapsed.as_secs() > 0 {
                hash_count as f64 / elapsed.as_secs_f64()
            } else {
                0.0
            };
            print!(
                "\rHashes probados: {} | Tiempo: {:.2}s | Velocidad: {:.0} H/s | Nonce: {}     ",
                hash_count,
                elapsed.as_secs_f64(),
                hashes_per_sec,
                test_block.nonce
            );
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            last_print = Instant::now();
        }
        
        // Verificar si el hash es válido
        if test_block.is_valid() {
            println!();
            println!();
            println!("✓ Hash válido encontrado!");
            println!("  Hash: {}", test_block.hash);
            println!("  Nonce: {}", test_block.nonce);
            println!("  Hashes totales: {}", hash_count);
            println!("  Tiempo total: {:.2} segundos", start.elapsed().as_secs_f64());
            if start.elapsed().as_secs() > 0 {
                println!(
                    "  Velocidad promedio: {:.0} hashes/segundo",
                    hash_count as f64 / start.elapsed().as_secs_f64()
                );
            }
            break;
        }
        
        test_block.nonce += 1;
        
        // Timeout de seguridad: 10 minutos máximo
        if start.elapsed().as_secs() > 600 {
            println!();
            println!();
            println!("⚠️  Timeout alcanzado (10 minutos). La dificultad puede ser demasiado alta.");
            println!("   Intenta con una dificultad menor (1-3)");
            println!("   Hashes probados: {}", hash_count);
            std::process::exit(1);
        }
    }
}
