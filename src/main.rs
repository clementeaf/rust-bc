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
mod staking;

use actix_web::{web, App, HttpServer};
use actix_web::middleware::Compress;
use actix_cors::Cors;
use api::{config_routes, AppState};
use billing::BillingManager;
use blockchain::Blockchain;
use cache::BalanceCache;
use database::BlockchainDB;
use middleware::RateLimitMiddleware;
use models::{WalletManager, Mempool};
use network::Node;
use staking::StakingManager;
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};

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
    
    // Network ID: "mainnet" o "testnet" (default: "mainnet")
    let network_id = env::var("NETWORK_ID")
        .unwrap_or_else(|_| "mainnet".to_string());
    
    // Bootstrap nodes: lista separada por comas (ej: "127.0.0.1:8081,127.0.0.1:8083")
    let bootstrap_nodes_str = env::var("BOOTSTRAP_NODES").unwrap_or_default();
    let bootstrap_nodes: Vec<String> = if bootstrap_nodes_str.is_empty() {
        Vec::new()
    } else {
        bootstrap_nodes_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    
    // Seed nodes: lista separada por comas (siempre se intentan, incluso sin bootstrap)
    // Estas son nodos conocidos que siempre están disponibles para discovery
    let seed_nodes_str = env::var("SEED_NODES").unwrap_or_default();
    let seed_nodes: Vec<String> = if seed_nodes_str.is_empty() {
        Vec::new()
    } else {
        seed_nodes_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    
    // Auto-discovery: intervalo en segundos (default: 120 = 2 minutos)
    let auto_discovery_interval = env::var("AUTO_DISCOVERY_INTERVAL")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(120);
    
    // Auto-discovery: máximo número de conexiones por ciclo (default: 5)
    let auto_discovery_max_connections = env::var("AUTO_DISCOVERY_MAX_CONNECTIONS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(5);
    
    // Auto-discovery: delay inicial en segundos (default: 30)
    let auto_discovery_initial_delay = env::var("AUTO_DISCOVERY_INITIAL_DELAY")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30);
    
    let db_path = format!("{}.db", db_name);

    println!("🚀 Iniciando Blockchain API Server...");
    println!("📊 Dificultad: {}", difficulty);
    println!("💾 Base de datos: {}", db_path);
    println!("🌐 Puerto API: {}", api_port);
    println!("📡 Puerto P2P: {}", p2p_port);
    println!("🌍 Network ID: {}", network_id);
    if !bootstrap_nodes.is_empty() {
        println!("🔗 Bootstrap nodes: {}", bootstrap_nodes.join(", "));
    }
    if !seed_nodes.is_empty() {
        println!("🌱 Seed nodes: {}", seed_nodes.join(", "));
    }
    println!("🔍 Auto-discovery: intervalo {}s, max conexiones {}, delay inicial {}s", 
        auto_discovery_interval, auto_discovery_max_connections, auto_discovery_initial_delay);

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

    let mempool = Arc::new(Mutex::new(Mempool::new()));
    let balance_cache = Arc::new(BalanceCache::new());
    let billing_manager = Arc::new(BillingManager::new());
    
    // Cargar contratos desde base de datos
    let mut contract_manager = smart_contracts::ContractManager::new();
    match db_arc.lock() {
        Ok(db) => {
            match db.load_contracts() {
                Ok(contracts) => {
                    if !contracts.is_empty() {
                        println!("📋 Cargando {} contratos desde base de datos...", contracts.len());
                        for contract in contracts {
                            let _ = contract_manager.deploy_contract(contract);
                        }
                        println!("✅ Contratos cargados exitosamente");
                    }
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
    let contract_manager = Arc::new(RwLock::new(contract_manager));

    let node_address = SocketAddr::from(([127, 0, 0, 1], p2p_port));
    let mut node_arc = Node::new(
        node_address,
        blockchain_for_network.clone(),
        Some(network_id.clone()),
        Some(bootstrap_nodes.clone()),
        Some(seed_nodes.clone()),
    );
    node_arc.set_resources(wallet_manager_arc.clone(), db_arc.clone());
    node_arc.set_contract_manager(contract_manager.clone());
    
    // Clonar los recursos compartidos antes de crear el Arc
    let shared_peers = node_arc.peers.clone();
    let shared_contract_sync_metrics = node_arc.contract_sync_metrics.clone();
    let shared_pending_broadcasts = node_arc.pending_contract_broadcasts.clone();
    let shared_recent_receipts = node_arc.recent_contract_receipts.clone();
    let shared_rate_limits = node_arc.contract_rate_limits.clone();
    let shared_failed_peers = node_arc.failed_peers.clone();
    
    let node_arc = Arc::new(node_arc);
    
    // Crear segunda instancia para el servidor P2P que comparte los mismos recursos
    let mut node_for_server = Node::new(
        node_address,
        blockchain_for_network.clone(),
        Some(network_id.clone()),
        Some(bootstrap_nodes.clone()),
        Some(seed_nodes.clone()),
    );
    node_for_server.set_resources(wallet_manager_arc.clone(), db_arc.clone());
    node_for_server.set_contract_manager(contract_manager.clone());
    // Compartir los mismos recursos compartidos
    node_for_server.peers = shared_peers;
    node_for_server.contract_sync_metrics = shared_contract_sync_metrics;
    node_for_server.pending_contract_broadcasts = shared_pending_broadcasts;
    node_for_server.recent_contract_receipts = shared_recent_receipts;
    node_for_server.contract_rate_limits = shared_rate_limits;
    node_for_server.failed_peers = shared_failed_peers;

    // Crear StakingManager
    // Min stake: 1000 tokens (configurable vía MIN_STAKE env var)
    let min_stake = env::var("MIN_STAKE")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1000);
    
    // Unstaking period: 7 días (configurable vía UNSTAKING_PERIOD env var, en segundos)
    let unstaking_period = env::var("UNSTAKING_PERIOD")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(604800); // 7 días
    
    // Slash percentage: 5% (configurable vía SLASH_PERCENTAGE env var)
    let slash_percentage = env::var("SLASH_PERCENTAGE")
        .ok()
        .and_then(|s| s.parse::<u8>().ok())
        .unwrap_or(5);
    
    let staking_manager = Arc::new(StakingManager::new(
        Some(min_stake),
        Some(unstaking_period),
        Some(slash_percentage),
    ));

    // Cargar validadores desde base de datos
    match db_arc.lock() {
        Ok(db) => {
            match db.load_validators() {
                Ok(validators) => {
                    if !validators.is_empty() {
                        println!("📋 Cargando {} validadores desde base de datos...", validators.len());
                        staking_manager.load_validators(validators);
                        println!("✅ Validadores cargados exitosamente");
                    }
                }
                Err(e) => {
                    eprintln!("⚠️  Error al cargar validadores: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("⚠️  Error al acceder a BD para cargar validadores: {}", e);
        }
    }

    let app_state = AppState {
        blockchain: blockchain_arc.clone(),
        wallet_manager: wallet_manager_arc.clone(),
        db: db_arc.clone(),
        node: Some(node_arc.clone()),
        mempool: mempool.clone(),
        balance_cache: balance_cache.clone(),
        billing_manager: billing_manager.clone(),
        contract_manager: contract_manager.clone(),
        staking_manager: staking_manager.clone(),
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

    // Clonar node_arc para conectar a bootstrap nodes después de iniciar
    let node_for_bootstrap = node_arc.clone();
    let bootstrap_nodes_clone = bootstrap_nodes.clone();
    
    let server_handle = tokio::spawn(async move {
        if let Err(e) = node_for_server.start_server(p2p_port).await {
            eprintln!("Error en servidor P2P: {}", e);
        }
    });

    // Conectar a bootstrap nodes después de un breve delay
    if !bootstrap_nodes_clone.is_empty() {
        tokio::spawn(async move {
            // Esperar a que el servidor esté listo
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            node_for_bootstrap.connect_to_bootstrap_nodes().await;
        });
    }

    let rate_limit_config = middleware::RateLimitConfig {
        requests_per_minute: 20,
        requests_per_hour: 1000,
    };

    let api_bind = format!("127.0.0.1:{}", api_port);
    
    // Configurar límite de tamaño para JSON (256KB por defecto, aumentamos a 1MB)
    let json_config = web::JsonConfig::default()
        .limit(1_048_576) // 1MB
        .error_handler(|err, _req| {
            eprintln!("[JSON ERROR] Error al deserializar JSON: {:?}", err);
            actix_web::error::ErrorBadRequest(format!("JSON deserialization error: {}", err))
        });
    
    let api_handle = HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .supports_credentials()
            .max_age(3600);
        
        App::new()
            .wrap(cors)
            .wrap(Compress::default())
            .wrap(RateLimitMiddleware::new(rate_limit_config.clone()))
            .app_data(web::Data::new(app_state.clone()))
            .app_data(json_config.clone())
            .app_data(web::JsonConfig::default().error_handler(|err, _req| {
                eprintln!("[JSON CONFIG ERROR] Error en deserialización: {:?}", err);
                eprintln!("[JSON CONFIG ERROR] Request path: {}", _req.path());
                actix_web::error::ErrorBadRequest(format!("JSON error: {}", err))
            }))
            .configure(config_routes)
    })
    .workers(8)
    .bind(&api_bind)?
    .run();

    // Tarea periódica para limpiar peers desconectados (cada 60 segundos)
    let node_for_cleanup = node_arc.clone();
    let cleanup_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            node_for_cleanup.cleanup_disconnected_peers().await;
        }
    });

    // Tarea periódica para auto-discovery de peers
    let node_for_discovery = node_arc.clone();
    let discovery_interval_secs = auto_discovery_interval;
    let discovery_max_connections = auto_discovery_max_connections;
    let discovery_initial_delay_secs = auto_discovery_initial_delay;
    let discovery_handle = tokio::spawn(async move {
        // Esperar delay inicial para que los bootstrap nodes se conecten
        tokio::time::sleep(tokio::time::Duration::from_secs(discovery_initial_delay_secs)).await;
        
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(discovery_interval_secs));
        loop {
            interval.tick().await;
            
            // auto_discover_and_connect ya maneja:
            // 1. Reconexión a bootstrap si no hay peers (en discover_peers)
            // 2. Conexión a bootstrap si hay pocos peers (< 3)
            node_for_discovery.auto_discover_and_connect(discovery_max_connections).await;
        }
    });

    // El servidor API debe continuar incluso si el P2P falla
    tokio::select! {
        result = api_handle => {
            result?;
        }
        _ = cleanup_handle => {
            // Cleanup task terminó (no debería pasar)
        }
        _ = discovery_handle => {
            // Discovery task terminó (no debería pasar)
        }
        _ = server_handle => {
            println!("Servidor P2P detenido, pero servidor API continúa");
            // Esperar indefinidamente para que el servidor API continúe
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
            }
        }
    }

    Ok(())
}
