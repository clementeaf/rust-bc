use crate::blockchain::{Block, Blockchain};
use crate::billing::{BillingManager, BillingTier, UsageStats};
use crate::cache::BalanceCache;
use crate::database::BlockchainDB;
use crate::models::{Transaction, WalletManager, Mempool};
use crate::network::Node;
use actix_web::{web, HttpResponse, Result as ActixResult};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::{Arc, Mutex};

/**
 * Estado compartido de la aplicación
 */
#[derive(Clone)]
pub struct AppState {
    pub blockchain: Arc<Mutex<Blockchain>>,
    pub wallet_manager: Arc<Mutex<WalletManager>>,
    pub db: Arc<Mutex<BlockchainDB>>,
    pub node: Option<Arc<Node>>,
    pub mempool: Arc<Mutex<Mempool>>,
    pub balance_cache: Arc<BalanceCache>,
    pub billing_manager: Arc<BillingManager>,
}

/**
 * Request para crear una transacción
 */
#[derive(Deserialize)]
pub struct CreateTransactionRequest {
    pub from: String,
    pub to: String,
    pub amount: u64,
    #[serde(default)]
    pub fee: Option<u64>, // Fee opcional (default: 0)
    pub data: Option<String>,
    #[serde(default)]
    pub signature: Option<String>, // Firma opcional (si se proporciona, se usa en lugar de firmar automáticamente)
}

/**
 * Request para crear un bloque
 */
#[derive(Deserialize)]
pub struct CreateBlockRequest {
    pub transactions: Vec<CreateTransactionRequest>,
}

/**
 * Response estándar de la API
 */
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> ApiResponse<T> {
        ApiResponse {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    pub fn error(message: String) -> ApiResponse<T> {
        ApiResponse {
            success: false,
            data: None,
            message: Some(message),
        }
    }
}

/**
 * Obtiene todos los bloques
 */
pub async fn get_blocks(state: web::Data<AppState>) -> ActixResult<HttpResponse> {
    let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    let response = ApiResponse::success(blockchain.chain.clone());
    Ok(HttpResponse::Ok().json(response))
}

/**
 * Obtiene un bloque por hash
 */
pub async fn get_block_by_hash(
    state: web::Data<AppState>,
    hash: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    match blockchain.get_block_by_hash(&hash) {
        Some(block) => {
            let response = ApiResponse::success(block.clone());
            Ok(HttpResponse::Ok().json(response))
        }
        None => {
            let response: ApiResponse<Block> = ApiResponse::error("Bloque no encontrado".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * Obtiene un bloque por índice
 */
pub async fn get_block_by_index(
    state: web::Data<AppState>,
    index: web::Path<u64>,
) -> ActixResult<HttpResponse> {
    let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    match blockchain.get_block_by_index(*index) {
        Some(block) => {
            let response = ApiResponse::success(block.clone());
            Ok(HttpResponse::Ok().json(response))
        }
        None => {
            let response: ApiResponse<Block> = ApiResponse::error("Bloque no encontrado".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * Crea un nuevo bloque con transacciones
 */
pub async fn create_block(
    state: web::Data<AppState>,
    req: web::Json<CreateBlockRequest>,
) -> ActixResult<HttpResponse> {
    let mut blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    let mut wallet_manager = state.wallet_manager.lock().unwrap_or_else(|e| e.into_inner());

    let transactions: Result<Vec<Transaction>, String> = req
        .transactions
        .iter()
        .map(|tx_req| {
            let fee = tx_req.fee.unwrap_or(0);
            let mut tx = Transaction::new_with_fee(
                tx_req.from.clone(),
                tx_req.to.clone(),
                tx_req.amount,
                fee,
                tx_req.data.clone(),
            );

            if tx_req.from != "0" {
                let wallet = wallet_manager
                    .get_wallet_for_signing(&tx_req.from)
                    .ok_or_else(|| "Wallet no encontrado para firmar".to_string())?;
                wallet.sign_transaction(&mut tx);
            }

            Ok(tx)
        })
        .collect();

    match transactions {
        Ok(txs) => {
            let mut mempool = state.mempool.lock().unwrap_or_else(|e| e.into_inner());
            for tx in &txs {
                if tx.from != "0" {
                    mempool.remove_transaction(&tx.id);
                }
            }
            drop(mempool);

            match blockchain.add_block(txs.clone(), &wallet_manager) {
                Ok(hash) => {
                    for tx in &txs {
                        if tx.from == "0" {
                            let _ = wallet_manager.process_coinbase_transaction(tx);
                        } else {
                            let _ = wallet_manager.process_transaction(tx);
                        }
                    }

                    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
                    let latest = blockchain.get_latest_block();
                    let latest_index = latest.index;
                    let latest_block_clone = latest.clone();
                    let _ = db.save_block(&latest_block_clone);
                    drop(db);

                    if let Some(node) = &state.node {
                        let node_clone = node.clone();
                        tokio::spawn(async move {
                            node_clone.broadcast_block(&latest_block_clone).await;
                        });
                    }

                    drop(blockchain);
                    state.balance_cache.invalidate(latest_index);

                    let response = ApiResponse::success(hash);
                    Ok(HttpResponse::Created().json(response))
                }
                Err(e) => {
                    let response: ApiResponse<String> = ApiResponse::error(e);
                    Ok(HttpResponse::BadRequest().json(response))
                }
            }
        }
        Err(e) => {
            let response: ApiResponse<String> = ApiResponse::error(e);
            Ok(HttpResponse::BadRequest().json(response))
        }
    }
}

/**
 * Crea una transacción (se agrega al próximo bloque)
 */
pub async fn create_transaction(
    state: web::Data<AppState>,
    req: web::Json<CreateTransactionRequest>,
    http_req: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    let api_key = http_req
        .headers()
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    if let Some(key) = &api_key {
        match state.billing_manager.can_make_transaction(key) {
            Ok(can_make) => {
                if !can_make {
                    let response: ApiResponse<Transaction> = ApiResponse::error(
                        "Límite de transacciones alcanzado para tu tier".to_string(),
                    );
                    return Ok(HttpResponse::PaymentRequired().json(response));
                }
            }
            Err(e) => {
                let response: ApiResponse<Transaction> = ApiResponse::error(e);
                return Ok(HttpResponse::Unauthorized().json(response));
            }
        }
    }

    let fee = req.fee.unwrap_or(0);
    let mut tx = Transaction::new_with_fee(
        req.from.clone(),
        req.to.clone(),
        req.amount,
        fee,
        req.data.clone(),
    );

    if !tx.is_valid() {
        let response: ApiResponse<Transaction> =
            ApiResponse::error("Transacción inválida".to_string());
        return Ok(HttpResponse::BadRequest().json(response));
    }

    if req.from != "0" {
        let wallet_manager = state.wallet_manager.lock().unwrap_or_else(|e| e.into_inner());
        let wallet = match wallet_manager.get_wallet_for_signing(&req.from) {
            Some(w) => w,
            None => {
                let response: ApiResponse<Transaction> =
                    ApiResponse::error("Wallet no encontrado para firmar".to_string());
                return Ok(HttpResponse::BadRequest().json(response));
            }
        };
        
        if let Some(sig) = &req.signature {
            if !sig.is_empty() {
                tx.signature = sig.clone();
            } else {
                wallet.sign_transaction(&mut tx);
            }
        } else {
            wallet.sign_transaction(&mut tx);
        }
        drop(wallet_manager);

        let (balance, validation_ok) = {
            let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
            let wallet_manager = state.wallet_manager.lock().unwrap_or_else(|e| e.into_inner());
            let bal = blockchain.calculate_balance(&req.from);
            let valid = blockchain.validate_transaction(&tx, &wallet_manager).is_ok();
            (bal, valid)
        };

        if !validation_ok {
            let response: ApiResponse<Transaction> = 
                ApiResponse::error("Transacción inválida".to_string());
            return Ok(HttpResponse::BadRequest().json(response));
        }

        let mut mempool = state.mempool.lock().unwrap_or_else(|e| e.into_inner());
        
        if mempool.has_double_spend(&tx) {
            let response: ApiResponse<Transaction> = 
                ApiResponse::error("Doble gasto detectado en mempool".to_string());
            return Ok(HttpResponse::BadRequest().json(response));
        }
        
        let pending_spent = mempool.calculate_pending_spent(&req.from);
        let total_required = tx.amount + tx.fee;
        let available_balance = balance.saturating_sub(pending_spent);
        
        if available_balance < total_required {
            let response: ApiResponse<Transaction> = 
                ApiResponse::error(format!("Saldo insuficiente. Disponible: {}, Requerido: {} (incluyendo {} pendientes)", 
                    available_balance, total_required, pending_spent));
            return Ok(HttpResponse::BadRequest().json(response));
        }
        
        if let Err(e) = mempool.add_transaction(tx.clone()) {
            let response: ApiResponse<Transaction> = ApiResponse::error(e);
            return Ok(HttpResponse::BadRequest().json(response));
        }
        drop(mempool);
    }

    if let Some(key) = &api_key {
        if let Err(e) = state.billing_manager.record_transaction(key) {
            let response: ApiResponse<Transaction> = ApiResponse::error(e);
            return Ok(HttpResponse::InternalServerError().json(response));
        }
    }

    if let Some(node) = &state.node {
        let tx_clone = tx.clone();
        let node_clone = node.clone();
        tokio::spawn(async move {
            node_clone.broadcast_transaction(&tx_clone).await;
        });
    }

    let response = ApiResponse::success(tx.clone());
    Ok(HttpResponse::Created().json(response))
}

/**
 * Obtiene el balance de un wallet usando caché cuando es posible
 */
pub async fn get_wallet_balance(
    state: web::Data<AppState>,
    address: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    let latest_block_index = if blockchain.chain.is_empty() {
        0
    } else {
        blockchain.get_latest_block().index
    };
    
    let balance = match state.balance_cache.get(&address, latest_block_index) {
        Some(cached_balance) => cached_balance,
        None => {
            let calculated_balance = blockchain.calculate_balance(&address);
            state.balance_cache.set(address.clone(), calculated_balance, latest_block_index);
            calculated_balance
        }
    };
    drop(blockchain);

    #[derive(Serialize)]
    struct BalanceResponse {
        address: String,
        balance: u64,
    }

    let response_data = BalanceResponse {
        address: address.clone(),
        balance,
    };

    let response = ApiResponse::success(response_data);
    Ok(HttpResponse::Ok().json(response))
}

/**
 * Crea un nuevo wallet con keypair criptográfico
 */
pub async fn create_wallet(
    state: web::Data<AppState>,
    http_req: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    let api_key = http_req
        .headers()
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    if let Some(key) = &api_key {
        match state.billing_manager.can_create_wallet(key) {
            Ok(can_create) => {
                if !can_create {
                    let response: ApiResponse<models::Wallet> = ApiResponse::error(
                        "Límite de wallets alcanzado para tu tier".to_string(),
                    );
                    return Ok(HttpResponse::PaymentRequired().json(response));
                }
            }
            Err(e) => {
                let response: ApiResponse<models::Wallet> = ApiResponse::error(e);
                return Ok(HttpResponse::Unauthorized().json(response));
            }
        }
    }

    let mut wallet_manager = state.wallet_manager.lock().unwrap_or_else(|e| e.into_inner());
    let wallet = wallet_manager.create_wallet();
    let address = wallet.address.clone();

    if let Some(key) = &api_key {
        if let Err(e) = state.billing_manager.record_wallet_creation(key) {
            let response: ApiResponse<models::Wallet> = ApiResponse::error(e);
            return Ok(HttpResponse::InternalServerError().json(response));
        }
    }

    if let Some(node) = &state.node {
        let node_clone = node.clone();
        tokio::spawn(async move {
            node_clone.broadcast_wallet_creation(&address).await;
        });
    }

    let response = ApiResponse::success(wallet);
    Ok(HttpResponse::Created().json(response))
}

/**
 * Obtiene todas las transacciones de un wallet
 */
pub async fn get_wallet_transactions(
    _state: web::Data<AppState>,
    address: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let blockchain = _state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    let transactions = blockchain.get_transactions_for_wallet(&address);

    let response = ApiResponse::success(transactions);
    Ok(HttpResponse::Ok().json(response))
}

/**
 * Verifica la validez de la cadena
 */
pub async fn verify_chain(state: web::Data<AppState>) -> ActixResult<HttpResponse> {
    let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    let is_valid = blockchain.is_chain_valid();

    #[derive(Serialize)]
    struct VerifyResponse {
        valid: bool,
        block_count: usize,
    }

    let response_data = VerifyResponse {
        valid: is_valid,
        block_count: blockchain.chain.len(),
    };

    let response = ApiResponse::success(response_data);
    Ok(HttpResponse::Ok().json(response))
}

/**
 * Obtiene información de la blockchain
 */
pub async fn get_blockchain_info(state: web::Data<AppState>) -> ActixResult<HttpResponse> {
    let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());

    #[derive(Serialize)]
    struct InfoResponse {
        block_count: usize,
        difficulty: u8,
        latest_block_hash: String,
        is_valid: bool,
    }

    let latest_hash = blockchain.get_latest_block().hash.clone();
    let response_data = InfoResponse {
        block_count: blockchain.chain.len(),
        difficulty: blockchain.difficulty,
        latest_block_hash: latest_hash,
        is_valid: blockchain.is_chain_valid(),
    };

    let response = ApiResponse::success(response_data);
    Ok(HttpResponse::Ok().json(response))
}

/**
 * Conecta a un peer
 */
pub async fn connect_peer(
    state: web::Data<AppState>,
    address: web::Path<String>,
) -> ActixResult<HttpResponse> {
    if let Some(node) = &state.node {
        let address_str = address.clone();
        let node_clone = node.clone();
        
        actix_web::rt::spawn(async move {
            let _ = node_clone.connect_to_peer(&address_str).await;
        });

        let response = ApiResponse::success(format!("Conectando a {}", address));
        Ok(HttpResponse::Ok().json(response))
    } else {
        let response: ApiResponse<String> = ApiResponse::error("Nodo P2P no disponible".to_string());
        Ok(HttpResponse::ServiceUnavailable().json(response))
    }
}

/**
 * Obtiene la lista de peers conectados
 */
pub async fn get_peers(state: web::Data<AppState>) -> ActixResult<HttpResponse> {
    if let Some(node) = &state.node {
        let peers: Vec<String> = {
            let peers_guard = node.peers.lock().unwrap_or_else(|e| e.into_inner());
            peers_guard.iter().cloned().collect()
        };

        let response = ApiResponse::success(peers);
        Ok(HttpResponse::Ok().json(response))
    } else {
        let response: ApiResponse<Vec<String>> = ApiResponse::error("Nodo P2P no disponible".to_string());
        Ok(HttpResponse::ServiceUnavailable().json(response))
    }
}

/**
 * Sincroniza la blockchain con todos los peers
 */
pub async fn sync_blockchain(state: web::Data<AppState>) -> ActixResult<HttpResponse> {
    if let Some(node) = &state.node {
        let node_clone = node.clone();
        
        actix_web::rt::spawn(async move {
            let _ = node_clone.sync_with_all_peers().await;
        });

        let response = ApiResponse::success("Sincronización iniciada".to_string());
        Ok(HttpResponse::Ok().json(response))
    } else {
        let response: ApiResponse<String> = ApiResponse::error("Nodo P2P no disponible".to_string());
        Ok(HttpResponse::ServiceUnavailable().json(response))
    }
}

/**
 * Request para minar un bloque
 */
#[derive(Deserialize)]
pub struct MineBlockRequest {
    pub miner_address: String,
    #[serde(default)]
    pub max_transactions: Option<usize>,
}

/**
 * Mina un nuevo bloque con recompensa automática
 */
pub async fn mine_block(
    state: web::Data<AppState>,
    req: web::Json<MineBlockRequest>,
) -> ActixResult<HttpResponse> {
    let max_txs = req.max_transactions.unwrap_or(10);
    let transactions = {
        let mut mempool = state.mempool.lock().unwrap_or_else(|e| e.into_inner());
        mempool.get_transactions_for_block(max_txs)
    };

    let miner_address = req.miner_address.clone();
    let blockchain_state = state.blockchain.clone();
    let wallet_manager_state = state.wallet_manager.clone();
    
    let (hash, latest, reward) = actix_web::web::block(move || {
        let mut blockchain = blockchain_state.lock().unwrap_or_else(|e| e.into_inner());
        let wallet_manager = wallet_manager_state.lock().unwrap_or_else(|e| e.into_inner());
        
        let reward = blockchain.calculate_mining_reward();
        match blockchain.mine_block_with_reward(&miner_address, transactions, &wallet_manager) {
            Ok(h) => {
                let latest = blockchain.get_latest_block().clone();
                Ok((h, latest, reward))
            }
            Err(e) => Err(e),
        }
    })
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Mining error: {}", e)))?
    .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    {
        let mut wallet_manager = state.wallet_manager.lock().unwrap_or_else(|e| e.into_inner());
        for tx in &latest.transactions {
            if tx.from == "0" {
                let _ = wallet_manager.process_coinbase_transaction(tx);
            } else {
                let _ = wallet_manager.process_transaction(tx);
            }
        }
    }

    if let Ok(db) = state.db.lock() {
        let _ = db.save_block(&latest);
    }

    if let Some(node) = &state.node {
        let latest_block = latest.clone();
        let node_clone = node.clone();
        tokio::spawn(async move {
            node_clone.broadcast_block(&latest_block).await;
        });
    }

    state.balance_cache.invalidate(latest.index);

    #[derive(Serialize)]
    struct MineResponse {
        hash: String,
        reward: u64,
        transactions_count: usize,
    }

    let response_data = MineResponse {
        hash,
        reward,
        transactions_count: latest.transactions.len(),
    };

    let response = ApiResponse::success(response_data);
    Ok(HttpResponse::Created().json(response))
}

/**
 * Obtiene todas las transacciones del mempool
 */
pub async fn get_mempool(state: web::Data<AppState>) -> ActixResult<HttpResponse> {
    let mempool = state.mempool.lock().unwrap_or_else(|e| e.into_inner());
    let transactions = mempool.get_all_transactions().to_vec();
    
    #[derive(Serialize)]
    struct MempoolResponse {
        count: usize,
        transactions: Vec<Transaction>,
    }

    let response_data = MempoolResponse {
        count: transactions.len(),
        transactions,
    };

    let response = ApiResponse::success(response_data);
    Ok(HttpResponse::Ok().json(response))
}

/**
 * Health check endpoint para monitoreo del sistema
 */
pub async fn health_check(state: web::Data<AppState>) -> ActixResult<HttpResponse> {
    let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    let db = state.db.lock().unwrap_or_else(|e| e.into_inner());
    let mempool = state.mempool.lock().unwrap_or_else(|e| e.into_inner());
    let (cache_size, cache_block_index) = state.balance_cache.stats();
    
    let peers_count = if let Some(node) = &state.node {
        let peers = node.peers.lock().unwrap_or_else(|e| e.into_inner());
        peers.len()
    } else {
        0
    };

    let block_count = blockchain.chain.len();
    let mempool_size = mempool.len();
    let latest_block_index = if block_count > 0 {
        blockchain.get_latest_block().index
    } else {
        0
    };

    drop(blockchain);
    drop(db);
    drop(mempool);

    #[derive(Serialize)]
    struct HealthResponse {
        status: String,
        version: String,
        blockchain: HealthBlockchain,
        cache: HealthCache,
        network: HealthNetwork,
    }

    #[derive(Serialize)]
    struct HealthBlockchain {
        block_count: usize,
        latest_block_index: u64,
        mempool_size: usize,
    }

    #[derive(Serialize)]
    struct HealthCache {
        size: usize,
        last_block_index: u64,
    }

    #[derive(Serialize)]
    struct HealthNetwork {
        connected_peers: usize,
    }

    let response_data = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        blockchain: HealthBlockchain {
            block_count,
            latest_block_index,
            mempool_size,
        },
        cache: HealthCache {
            size: cache_size,
            last_block_index: cache_block_index,
        },
        network: HealthNetwork {
            connected_peers: peers_count,
        },
    };

    let response = ApiResponse::success(response_data);
    Ok(HttpResponse::Ok().json(response))
}

/**
 * Obtiene estadísticas del sistema
 */
pub async fn get_stats(state: web::Data<AppState>) -> ActixResult<HttpResponse> {
    // Obtener datos de blockchain (liberar lock rápidamente)
    let (block_count, difficulty, latest_block_hash, latest_block_index, total_transactions, total_coinbase, unique_addresses_count, avg_block_time, target_block_time, max_transactions_per_block, max_block_size_bytes) = {
        let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
        let block_count = blockchain.chain.len();
        let difficulty = blockchain.difficulty;
        let latest_block = blockchain.get_latest_block();
        let latest_block_hash = latest_block.hash.clone();
        let latest_block_index = latest_block.index;
        
        let total_transactions: usize = blockchain.chain.iter()
            .map(|b| b.transactions.len())
            .sum();
        
        let total_coinbase: u64 = blockchain.chain.iter()
            .flat_map(|b| &b.transactions)
            .filter(|tx| tx.from == "0")
            .map(|tx| tx.amount)
            .sum();
        
        let mut unique_addresses = std::collections::HashSet::new();
        for block in &blockchain.chain {
            for tx in &block.transactions {
                if !tx.from.is_empty() && tx.from != "0" {
                    unique_addresses.insert(tx.from.clone());
                }
                if !tx.to.is_empty() {
                    unique_addresses.insert(tx.to.clone());
                }
            }
        }
        
        let mut block_times = Vec::new();
        if blockchain.chain.len() > 1 {
            for i in 1..blockchain.chain.len() {
                // Usar saturating_sub para evitar overflow si timestamps están desordenados
                let time_diff = blockchain.chain[i].timestamp.saturating_sub(blockchain.chain[i-1].timestamp);
                block_times.push(time_diff);
            }
        }
        
        let avg_block_time = if !block_times.is_empty() {
            block_times.iter().sum::<u64>() as f64 / block_times.len() as f64
        } else {
            0.0
        };
        
        (block_count, difficulty, latest_block_hash, latest_block_index, total_transactions, total_coinbase, unique_addresses.len(), avg_block_time, blockchain.target_block_time, blockchain.max_transactions_per_block, blockchain.max_block_size_bytes)
    };
    
    // Obtener datos de mempool (liberar lock rápidamente)
    let (mempool_size, total_fees) = {
        let mempool = state.mempool.lock().unwrap_or_else(|e| e.into_inner());
        let size = mempool.len();
        let fees: u64 = mempool.get_all_transactions()
            .iter()
            .map(|tx| tx.fee)
            .sum();
        (size, fees)
    };
    
    // Obtener datos de red
    let peers_count = if let Some(node) = &state.node {
        let peers = node.peers.lock().unwrap_or_else(|e| e.into_inner());
        peers.len()
    } else {
        0
    };
    
    #[derive(Serialize)]
    struct StatsResponse {
        blockchain: BlockchainStats,
        mempool: MempoolStats,
        network: NetworkStats,
    }
    
    #[derive(Serialize)]
    struct BlockchainStats {
        block_count: usize,
        total_transactions: usize,
        difficulty: u8,
        latest_block_hash: String,
        latest_block_index: u64,
        total_coinbase: u64,
        unique_addresses: usize,
        avg_block_time_seconds: f64,
        target_block_time: u64,
        max_transactions_per_block: usize,
        max_block_size_bytes: usize,
    }
    
    #[derive(Serialize)]
    struct MempoolStats {
        pending_transactions: usize,
        total_fees_pending: u64,
    }
    
    #[derive(Serialize)]
    struct NetworkStats {
        connected_peers: usize,
    }
    
    let response_data = StatsResponse {
        blockchain: BlockchainStats {
            block_count,
            total_transactions,
            difficulty,
            latest_block_hash,
            latest_block_index,
            total_coinbase,
            unique_addresses: unique_addresses_count,
            avg_block_time_seconds: avg_block_time,
            target_block_time,
            max_transactions_per_block,
            max_block_size_bytes,
        },
        mempool: MempoolStats {
            pending_transactions: mempool_size,
            total_fees_pending: total_fees,
        },
        network: NetworkStats {
            connected_peers: peers_count,
        },
    };
    
    let response = ApiResponse::success(response_data);
    Ok(HttpResponse::Ok().json(response))
}

/**
 * Request para crear una API key
 */
#[derive(Deserialize)]
pub struct CreateAPIKeyRequest {
    pub tier: String,
}

/**
 * Crea una nueva API key
 */
pub async fn create_api_key(
    state: web::Data<AppState>,
    req: web::Json<CreateAPIKeyRequest>,
) -> ActixResult<HttpResponse> {
    let tier = match BillingTier::from_str(&req.tier) {
        Some(t) => t,
        None => {
            let response: ApiResponse<String> = ApiResponse::error(
                "Tier inválido. Opciones: free, basic, pro, enterprise".to_string(),
            );
            return Ok(HttpResponse::BadRequest().json(response));
        }
    };

    match state.billing_manager.create_api_key(tier) {
        Ok(key) => {
            let response: ApiResponse<String> = ApiResponse::success(key);
            Ok(HttpResponse::Created().json(response))
        }
        Err(e) => {
            let response: ApiResponse<String> = ApiResponse::error(e);
            Ok(HttpResponse::InternalServerError().json(response))
        }
    }
}

/**
 * Obtiene estadísticas de uso de una API key
 */
pub async fn get_billing_usage(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    let api_key = req
        .headers()
        .get("X-API-Key")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            actix_web::error::ErrorUnauthorized("API key requerida en header X-API-Key")
        })?;

    match state.billing_manager.get_usage(&api_key) {
        Ok(usage) => {
            let response: ApiResponse<UsageStats> = ApiResponse::success(usage);
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            let response: ApiResponse<String> = ApiResponse::error(e);
            Ok(HttpResponse::Unauthorized().json(response))
        }
    }
}

/**
 * Configura las rutas de la API
 */
pub fn config_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .route("/billing/create-key", web::post().to(create_api_key))
            .route("/billing/usage", web::get().to(get_billing_usage))
            .route("/blocks", web::get().to(get_blocks))
            .route("/blocks/{hash}", web::get().to(get_block_by_hash))
            .route("/blocks/index/{index}", web::get().to(get_block_by_index))
            .route("/blocks", web::post().to(create_block))
            .route("/transactions", web::post().to(create_transaction))
            .route("/wallets/{address}", web::get().to(get_wallet_balance))
            .route("/wallets/create", web::post().to(create_wallet))
            .route("/wallets/{address}/transactions", web::get().to(get_wallet_transactions))
            .route("/chain/verify", web::get().to(verify_chain))
            .route("/chain/info", web::get().to(get_blockchain_info))
            .route("/peers", web::get().to(get_peers))
            .route("/peers/{address}/connect", web::post().to(connect_peer))
            .route("/sync", web::post().to(sync_blockchain))
            .route("/mine", web::post().to(mine_block))
            .route("/mempool", web::get().to(get_mempool))
            .route("/stats", web::get().to(get_stats))
            .route("/health", web::get().to(health_check)),
    );
}

