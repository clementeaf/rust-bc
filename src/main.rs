mod airdrop;
mod api;
mod billing;
mod block_storage;
mod blockchain;
mod cache;
mod chain_validation;
mod checkpoint;
mod middleware;
mod models;
mod network;
mod network_security;
mod transaction_validation;
mod pruning;
mod smart_contracts;
mod staking;
mod state_reconstructor;
mod state_snapshot;

use actix_cors::Cors;
use actix_web::middleware::Compress;
use actix_web::{web, App, HttpServer};
use airdrop::AirdropManager;
use api::{config_routes, AppState};
use billing::BillingManager;
use block_storage::BlockStorage;
use blockchain::Blockchain;
use cache::BalanceCache;
use middleware::RateLimitMiddleware;
use models::{Mempool, WalletManager};
use network::Node;
use pruning::PruningManager;
use staking::{StakingManager, Validator};
use state_reconstructor::ReconstructedState;
use state_snapshot::{StateSnapshot, StateSnapshotManager};
use transaction_validation::TransactionValidator;
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
        .unwrap_or_else(|| {
            env::var("API_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8080)
        });
    let p2p_port = args
        .get(2)
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or_else(|| {
            env::var("P2P_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(8081)
        });
    let db_name = args
        .get(3)
        .cloned()
        .unwrap_or_else(|| env::var("DB_NAME").unwrap_or_else(|_| "blockchain".to_string()));

    // Network ID: "mainnet" o "testnet" (default: "mainnet")
    let network_id = env::var("NETWORK_ID").unwrap_or_else(|_| "mainnet".to_string());

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
    let blocks_dir = format!("{}_blocks", db_name);
    let snapshots_dir = format!("{}_snapshots", db_name);
    let checkpoints_dir = format!("{}_checkpoints", db_name);

    println!("🚀 Iniciando Blockchain API Server...");
    println!("📊 Dificultad: {}", difficulty);
    println!("💾 Base de datos: {}", db_path);
    println!("📁 Directorio de bloques: {}", blocks_dir);
    println!("📸 Directorio de snapshots: {}", snapshots_dir);
    println!("🌐 Puerto API: {}", api_port);
    println!("📡 Puerto P2P: {}", p2p_port);
    println!("🌍 Network ID: {}", network_id);
    if !bootstrap_nodes.is_empty() {
        println!("🔗 Bootstrap nodes: {}", bootstrap_nodes.join(", "));
    }
    if !seed_nodes.is_empty() {
        println!("🌱 Seed nodes: {}", seed_nodes.join(", "));
    }
    println!(
        "🔍 Auto-discovery: intervalo {}s, max conexiones {}, delay inicial {}s",
        auto_discovery_interval, auto_discovery_max_connections, auto_discovery_initial_delay
    );

    // Inicializar BlockStorage (nuevo sistema)
    let block_storage = match BlockStorage::new(&blocks_dir) {
        Ok(storage) => {
            println!("✅ BlockStorage inicializado");
            Some(storage)
        }
        Err(e) => {
            eprintln!("⚠️  Error al inicializar BlockStorage: {}", e);
            None
        }
    };

    // Inicializar StateSnapshotManager
    let snapshot_manager = match StateSnapshotManager::new(&snapshots_dir) {
        Ok(manager) => {
            println!("✅ StateSnapshotManager inicializado");
            Some(manager)
        }
        Err(e) => {
            eprintln!("⚠️  Error al inicializar StateSnapshotManager: {}", e);
            None
        }
    };

    // Cargar blockchain: solo desde archivos (sin BD)
    let blockchain = if let Some(ref storage) = block_storage {
        // Intentar cargar desde archivos
        match storage.load_all_blocks() {
            Ok(blocks) if !blocks.is_empty() => {
                println!(
                    "✅ Blockchain cargada desde archivos: {} bloques",
                    blocks.len()
                );
                Blockchain {
                    chain: blocks,
                    difficulty,
                    target_block_time: 60,
                    difficulty_adjustment_interval: 10,
                    max_transactions_per_block: 1000,
                    max_block_size_bytes: 1_000_000,
                }
            }
            Ok(_) => {
                // No hay bloques en archivos, crear nueva blockchain
                println!("📦 Creando bloque génesis...");
                let mut bc = Blockchain::new(difficulty);
                bc.create_genesis_block();
                // Guardar bloque génesis en archivos
                if let Some(ref storage) = block_storage {
                    for block in &bc.chain {
                        if let Err(e) = storage.save_block(block) {
                            eprintln!("⚠️  Error al guardar bloque génesis: {}", e);
                        }
                    }
                }
                bc
            }
            Err(e) => {
                eprintln!(
                    "⚠️  Error al cargar bloques desde archivos: {}, creando nueva blockchain",
                    e
                );
                let mut bc = Blockchain::new(difficulty);
                bc.create_genesis_block();
                // Guardar bloque génesis
                if let Some(ref storage) = block_storage {
                    for block in &bc.chain {
                        if let Err(e) = storage.save_block(block) {
                            eprintln!("⚠️  Error al guardar bloque génesis: {}", e);
                        }
                    }
                }
                bc
            }
        }
    } else {
        // Sin BlockStorage, crear nueva blockchain
        eprintln!("⚠️  BlockStorage no disponible, creando nueva blockchain");
        let mut bc = Blockchain::new(difficulty);
        bc.create_genesis_block();
        bc
    };

    // Intentar cargar estado desde snapshot (más rápido que reconstruir)
    let reconstructed_state = if let Some(ref snapshot_mgr) = snapshot_manager {
        match snapshot_mgr.load_latest_snapshot() {
            Ok(Some(snapshot)) => {
                // Verificar que el snapshot corresponde al último bloque
                let latest_block = blockchain.get_latest_block();
                if snapshot.block_index == latest_block.index
                    && snapshot.block_hash == latest_block.hash
                {
                    println!(
                        "✅ Estado cargado desde snapshot (bloque {})",
                        snapshot.block_index
                    );
                    // Convertir snapshot a ReconstructedState
                    let mut state = ReconstructedState::new();
                    for (addr, wallet_snap) in snapshot.wallets {
                        state.wallets.insert(
                            addr.clone(),
                            state_reconstructor::WalletState {
                                balance: wallet_snap.balance,
                            },
                        );
                    }
                    state.contracts = snapshot.contracts;
                    state.validators = snapshot.validators;
                    // airdrop_tracking se reconstruye desde bloques
                    state.airdrop_tracking =
                        ReconstructedState::from_blockchain(&blockchain.chain).airdrop_tracking;
                    state
                } else {
                    println!(
                        "⚠️  Snapshot desactualizado (bloque {} vs {}), reconstruyendo...",
                        snapshot.block_index, latest_block.index
                    );
                    ReconstructedState::from_blockchain(&blockchain.chain)
                }
            }
            Ok(None) => {
                println!("📸 No hay snapshot disponible, reconstruyendo estado...");
                ReconstructedState::from_blockchain(&blockchain.chain)
            }
            Err(e) => {
                eprintln!("⚠️  Error al cargar snapshot: {}, reconstruyendo...", e);
                ReconstructedState::from_blockchain(&blockchain.chain)
            }
        }
    } else {
        // Sin snapshot manager, reconstruir normalmente
        let block_count = blockchain.chain.len();
        if block_count > 10 {
            println!("🔄 Reconstruyendo estado desde {} bloques...", block_count);
        }
        let state = ReconstructedState::from_blockchain(&blockchain.chain);
        if block_count > 10 {
            println!("✅ Estado reconstruido desde blockchain");
        }
        state
    };

    // Guardar snapshot si hay muchos bloques y no existe uno reciente
    let block_count = blockchain.chain.len();
    if block_count > 50 {
        if let Some(ref snapshot_mgr) = snapshot_manager {
            // Verificar si necesitamos crear un snapshot
            let should_create_snapshot = match snapshot_mgr.load_latest_snapshot() {
                Ok(Some(snapshot)) => {
                    let latest_block = blockchain.get_latest_block();
                    // Crear snapshot si el último tiene más de 100 bloques de diferencia
                    snapshot.block_index + 100 < latest_block.index
                }
                _ => true, // No hay snapshot, crear uno
            };

            if should_create_snapshot {
                let latest_block = blockchain.get_latest_block();
                let snapshot = StateSnapshot::from_state(
                    latest_block,
                    reconstructed_state.wallets.clone(),
                    reconstructed_state.contracts.clone(),
                    reconstructed_state.validators.clone(),
                );
                if let Err(e) = snapshot_mgr.save_snapshot(&snapshot, latest_block.index) {
                    eprintln!("⚠️  Error al guardar snapshot: {}", e);
                } else {
                    println!("📸 Snapshot guardado (bloque {})", latest_block.index);
                }
            }
        }
    }

    let mut wallet_manager = WalletManager::new();
    wallet_manager.sync_from_blockchain(&blockchain.chain);
    println!("✅ Wallets sincronizados desde blockchain");
    let wallet_manager_arc = Arc::new(Mutex::new(wallet_manager));

    // Base de datos eliminada - ya no se usa

    let blockchain_arc = Arc::new(Mutex::new(blockchain));
    let blockchain_for_network = blockchain_arc.clone();

    let mempool = Arc::new(Mutex::new(Mempool::new()));
    let balance_cache = Arc::new(BalanceCache::new());
    let billing_manager = Arc::new(BillingManager::new());

    // Los contratos se mantienen en memoria (ContractManager)
    // Se reconstruyen desde blockchain si es necesario
    let contract_manager = smart_contracts::ContractManager::new();
    let contract_manager = Arc::new(RwLock::new(contract_manager));

    // Crear Arc<BlockStorage> para AppState y Node (antes de crear Node)
    let block_storage_arc = block_storage.map(Arc::new);

    // Inicializar CheckpointManager (protección anti-51%) - ANTES de crear Node
    let checkpoint_interval = env::var("CHECKPOINT_INTERVAL")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(2000);
    let max_reorg_depth = env::var("MAX_REORG_DEPTH")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(2000);

    let checkpoint_manager = match crate::checkpoint::CheckpointManager::new(
        &checkpoints_dir,
        Some(checkpoint_interval),
        Some(max_reorg_depth),
    ) {
        Ok(manager) => {
            let count = manager.checkpoint_count();
            if count > 0 {
                println!(
                    "✅ CheckpointManager inicializado: {} checkpoints cargados",
                    count
                );
            } else {
                println!("✅ CheckpointManager inicializado (sin checkpoints previos)");
            }
            Some(Arc::new(Mutex::new(manager)))
        }
        Err(e) => {
            eprintln!("⚠️  Error al inicializar CheckpointManager: {}", e);
            None
        }
    };

    // Inicializar TransactionValidator
    let transaction_validator = Arc::new(Mutex::new(TransactionValidator::with_defaults()));

    let node_address = SocketAddr::from(([127, 0, 0, 1], p2p_port));
    let mut node_arc = Node::new(
        node_address,
        blockchain_for_network.clone(),
        Some(network_id.clone()),
        Some(bootstrap_nodes.clone()),
        Some(seed_nodes.clone()),
    );
    node_arc.set_resources(wallet_manager_arc.clone());
    if let Some(storage) = block_storage_arc.as_ref() {
        node_arc.set_block_storage(Arc::clone(storage));
    }
    node_arc.set_contract_manager(contract_manager.clone());
    if let Some(ref checkpoint_mgr) = checkpoint_manager {
        node_arc.set_checkpoint_manager(checkpoint_mgr.clone());
    }
    node_arc.set_transaction_validator(transaction_validator.clone());

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
    node_for_server.set_resources(wallet_manager_arc.clone());
    node_for_server.set_contract_manager(contract_manager.clone());
    if let Some(ref checkpoint_mgr) = checkpoint_manager {
        node_for_server.set_checkpoint_manager(checkpoint_mgr.clone());
    }
    node_for_server.set_transaction_validator(transaction_validator.clone());
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

    // Cargar validadores desde estado reconstruido
    let validators_from_state: Vec<Validator> =
        reconstructed_state.validators.values().cloned().collect();
    if !validators_from_state.is_empty() {
        println!(
            "📋 Cargando {} validadores desde estado reconstruido...",
            validators_from_state.len()
        );
        staking_manager.load_validators(validators_from_state);
        println!("✅ Validadores cargados exitosamente");
    }

    // Inicializar AirdropManager
    let max_eligible_nodes = env::var("AIRDROP_MAX_NODES")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(500);

    let airdrop_amount_per_node = env::var("AIRDROP_AMOUNT_PER_NODE")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(1000);

    let airdrop_wallet = env::var("AIRDROP_WALLET").unwrap_or_else(|_| "AIRDROP".to_string());

    let airdrop_manager = Arc::new(AirdropManager::new(
        max_eligible_nodes,
        airdrop_amount_per_node,
        airdrop_wallet.clone(),
    ));

    // El tracking de airdrop se reconstruye desde blockchain
    let airdrop_tracking = reconstructed_state.get_airdrop_tracking();
    if !airdrop_tracking.is_empty() {
        println!(
            "📋 Tracking de airdrop reconstruido: {} nodos",
            airdrop_tracking.len()
        );
    }

    // Inicializar PruningManager
    let pruning_manager = if block_storage_arc.is_some() && snapshot_manager.is_some() {
        let storage_clone = BlockStorage::new(&blocks_dir).ok();
        let snapshot_mgr_clone = StateSnapshotManager::new(&snapshots_dir).ok();
        if let (Some(storage), Some(snapshot_mgr)) = (storage_clone, snapshot_mgr_clone) {
            let keep_blocks = std::env::var("PRUNING_KEEP_BLOCKS")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1000);
            let snapshot_interval = std::env::var("SNAPSHOT_INTERVAL")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1000);
            Some(Arc::new(PruningManager::new(
                storage,
                snapshot_mgr,
                Some(keep_blocks),
                Some(snapshot_interval),
            )))
        } else {
            None
        }
    } else {
        None
    };

    let app_state = AppState {
        blockchain: blockchain_arc.clone(),
        wallet_manager: wallet_manager_arc.clone(),
        block_storage: block_storage_arc.clone(),
        node: Some(node_arc.clone()),
        mempool: mempool.clone(),
        balance_cache: balance_cache.clone(),
        billing_manager: billing_manager.clone(),
        contract_manager: contract_manager.clone(),
        staking_manager: staking_manager.clone(),
        airdrop_manager: airdrop_manager.clone(),
        pruning_manager: pruning_manager.clone(),
        checkpoint_manager: checkpoint_manager.clone(),
        transaction_validator: transaction_validator.clone(),
    };

    // Tarea periódica para crear snapshots cada 1000 bloques
    if pruning_manager.is_some() && snapshot_manager.is_some() {
        let blockchain_for_snapshot = blockchain_arc.clone();
        let pruning_mgr_clone = match pruning_manager.clone() {
            Some(pruning) => pruning,
            None => {
                eprintln!("⚠️  Pruning manager no disponible para tarea periódica");
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Pruning manager not available",
                ));
            }
        };

        match StateSnapshotManager::new(&snapshots_dir) {
            Ok(snapshot_mgr_clone) => {
                tokio::spawn(async move {
                    let mut last_snapshot_block = 0u64;
                    loop {
                        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

                        let (current_block_index, should_create) = {
                            let lock_result = blockchain_for_snapshot.lock();
                            match lock_result {
                                Ok(blockchain) => {
                                    let latest = blockchain.get_latest_block();
                                    let current = latest.index;
                                    let should = pruning_mgr_clone.should_create_snapshot(current)
                                        && current > last_snapshot_block;
                                    (current, should)
                                }
                                Err(_e) => {
                                    eprintln!("⚠️  Error al adquirir lock de blockchain en snapshot task");
                                    continue;
                                }
                            }
                        };

                        if should_create {
                            println!(
                                "📸 Creando snapshot automático en bloque {}",
                                current_block_index
                            );

                            // Reconstruir estado para el snapshot
                            let latest_block = {
                                let lock_result = blockchain_for_snapshot.lock();
                                match lock_result {
                                    Ok(blockchain) => {
                                        let block = blockchain.get_latest_block().clone();
                                        drop(blockchain);
                                        block
                                    }
                                    Err(_e) => {
                                        eprintln!("⚠️  Error al adquirir lock de blockchain para snapshot");
                                        continue;
                                    }
                                }
                            };

                            // Reconstruir estado desde blockchain
                            let reconstructed = {
                                let lock_result = blockchain_for_snapshot.lock();
                                match lock_result {
                                    Ok(blockchain) => {
                                        let reconstructed = crate::state_reconstructor::ReconstructedState::from_blockchain(
                                            &blockchain.chain,
                                        );
                                        drop(blockchain);
                                        reconstructed
                                    }
                                    Err(_e) => {
                                        eprintln!("⚠️  Error al adquirir lock de blockchain para reconstrucción");
                                        continue;
                                    }
                                }
                            };

                            let snapshot = StateSnapshot::from_state(
                                &latest_block,
                                reconstructed.wallets,
                                reconstructed.contracts,
                                reconstructed.validators,
                            );

                            if let Err(e) =
                                snapshot_mgr_clone.save_snapshot(&snapshot, latest_block.index)
                            {
                                eprintln!("⚠️  Error al guardar snapshot automático: {}", e);
                            } else {
                                println!(
                                    "✅ Snapshot automático guardado (bloque {})",
                                    latest_block.index
                                );
                                last_snapshot_block = current_block_index;

                                // Ejecutar pruning después de crear snapshot
                                if let Err(e) =
                                    pruning_mgr_clone.prune_old_blocks(current_block_index)
                                {
                                    eprintln!("⚠️  Error durante pruning automático: {}", e);
                                }
                            }
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!(
                    "⚠️  No se pudo crear StateSnapshotManager para tarea periódica: {}",
                    e
                );
            }
        }
    }

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
        tokio::time::sleep(tokio::time::Duration::from_secs(
            discovery_initial_delay_secs,
        ))
        .await;

        let mut interval =
            tokio::time::interval(tokio::time::Duration::from_secs(discovery_interval_secs));
        loop {
            interval.tick().await;

            // auto_discover_and_connect ya maneja:
            // 1. Reconexión a bootstrap si no hay peers (en discover_peers)
            // 2. Conexión a bootstrap si hay pocos peers (< 3)
            node_for_discovery
                .auto_discover_and_connect(discovery_max_connections)
                .await;
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
