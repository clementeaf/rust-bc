mod api;
mod billing;
mod billing_middleware;
mod blockchain;
mod cache;
mod database;
mod middleware;
mod models;
mod network;
mod smart_contracts;

use actix_web::{web, App, HttpServer};
use actix_web::middleware::Compress;
use api::{config_routes, AppState};
use billing::BillingManager;
use blockchain::Blockchain;
use cache::BalanceCache;
use database::BlockchainDB;
use middleware::RateLimitMiddleware;
use models::{WalletManager, Mempool};
use network::Node;
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

/**
 * Función principal - Inicia el servidor API
 */
#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    let difficulty = env::var("DIFFICULTY")
        .ok()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(1);
    
    let args: Vec<String> = env::args().collect();
    let api_port = args
        .get(1)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or_else(|| env::var("API_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8080));
    let p2p_port = args
        .get(2)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or_else(|| env::var("P2P_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8081));
    let db_name = args
        .get(3)
        .cloned()
        .unwrap_or_else(|| env::var("DB_NAME").unwrap_or_else(|_| "blockchain".to_string()));
    
    let db_path = format!("{}.db", db_name);

    println!("🚀 Iniciando Blockchain API Server...");
    println!("📊 Dificultad: {}", difficulty);
    println!("💾 Base de datos: {}", db_path);
    println!("🌐 Puerto API: {}", api_port);
    println!("📡 Puerto P2P: {}", p2p_port);

    let db = match BlockchainDB::new(&db_path) {
        Ok(db) => {
            println!("✅ Base de datos conectada");
            db
        }
        Err(e) => {
            eprintln!("❌ Error al conectar con la base de datos: {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Error de base de datos: {}", e),
            ));
        }
    };

    let blockchain = match db.load_blockchain(difficulty) {
        Ok(mut bc) => {
            if bc.chain.is_empty() {
                println!("📦 Creando bloque génesis...");
                bc.create_genesis_block();
                if let Err(e) = db.save_blockchain(&bc) {
                    eprintln!("⚠️  Error al guardar blockchain inicial: {}", e);
                }
            }
            println!("✅ Blockchain cargada: {} bloques", bc.chain.len());
            bc
        }
        Err(e) => {
            eprintln!("⚠️  Error al cargar blockchain, creando nueva: {}", e);
            Blockchain::new(difficulty)
        }
    };

    let mut wallet_manager = WalletManager::new();
    wallet_manager.sync_from_blockchain(&blockchain.chain);
    println!("✅ Wallets sincronizados desde blockchain");
    let wallet_manager_arc = Arc::new(Mutex::new(wallet_manager));
    let db_arc = Arc::new(Mutex::new(db));

    let blockchain_arc = Arc::new(Mutex::new(blockchain));
    let blockchain_for_network = blockchain_arc.clone();

    let node_address = SocketAddr::from(([127, 0, 0, 1], p2p_port));
    let mut node_arc = Node::new(node_address, blockchain_for_network.clone());
    node_arc.set_resources(wallet_manager_arc.clone(), db_arc.clone());
    let node_arc = Arc::new(node_arc);
    
    let mut node_for_server = Node::new(node_address, blockchain_for_network.clone());
    node_for_server.set_resources(wallet_manager_arc.clone(), db_arc.clone());

    let mempool = Arc::new(Mutex::new(Mempool::new()));
    let balance_cache = Arc::new(BalanceCache::new());
    let billing_manager = Arc::new(BillingManager::new());
    
    // Cargar contratos desde base de datos
    let mut contract_manager = smart_contracts::ContractManager::new();
    match db_arc.lock() {
        Ok(db) => {
            match db.load_contracts() {
                Ok(contracts) => {
                    println!("📋 Cargando {} contratos desde base de datos...", contracts.len());
                    for contract in contracts {
                        let _ = contract_manager.deploy_contract(contract);
                    }
                    println!("✅ Contratos cargados exitosamente");
                }
                Err(e) => {
                    eprintln!("⚠️  Error al cargar contratos: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("⚠️  Error al acceder a BD para cargar contratos: {}", e);
        }
    }
    let contract_manager = Arc::new(Mutex::new(contract_manager));

    let app_state = AppState {
        blockchain: blockchain_arc.clone(),
        wallet_manager: wallet_manager_arc.clone(),
        db: db_arc.clone(),
        node: Some(node_arc.clone()),
        mempool: mempool.clone(),
        balance_cache: balance_cache.clone(),
        billing_manager: billing_manager.clone(),
        contract_manager: contract_manager.clone(),
    };

    println!("🌐 Servidor API iniciado en http://127.0.0.1:{}", api_port);
    println!("📡 Servidor P2P iniciado en 127.0.0.1:{}", p2p_port);
    println!("📚 Documentación de API:");
    println!("   GET  /api/v1/blocks");
    println!("   GET  /api/v1/blocks/{{hash}}");
    println!("   POST /api/v1/blocks");
    println!("   POST /api/v1/transactions");
    println!("   GET  /api/v1/wallets/{{address}}");
    println!("   GET  /api/v1/chain/verify");
    println!("   GET  /api/v1/chain/info");
    println!("\n💡 Presiona Ctrl+C para detener el servidor\n");

    let server_handle = tokio::spawn(async move {
        if let Err(e) = node_for_server.start_server(p2p_port).await {
            eprintln!("Error en servidor P2P: {}", e);
        }
    });

    let rate_limit_config = middleware::RateLimitConfig {
        requests_per_minute: 20,
        requests_per_hour: 1000,
    };

    let api_bind = format!("127.0.0.1:{}", api_port);
    let api_handle = HttpServer::new(move || {
        App::new()
            .wrap(Compress::default())
            .wrap(RateLimitMiddleware::new(rate_limit_config.clone()))
            .app_data(web::Data::new(app_state.clone()))
            .configure(config_routes)
    })
    .workers(8)
    .bind(&api_bind)?
    .run();

    tokio::select! {
        result = api_handle => {
            result?;
        }
        _ = server_handle => {
            println!("Servidor P2P detenido");
        }
    }

    Ok(())
}
