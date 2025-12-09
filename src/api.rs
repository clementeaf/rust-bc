use crate::airdrop::AirdropManager;
use crate::blockchain::{Block, Blockchain};
use crate::block_storage::BlockStorage;
use crate::billing::{BillingManager, BillingTier, UsageStats};
use crate::cache::BalanceCache;
use crate::models::{Transaction, Wallet, WalletManager, Mempool};
use crate::network::Node;
use crate::smart_contracts::{ContractManager, ContractFunction, SmartContract, NFTMetadata};
use crate::staking::StakingManager;
use actix_web::{web, HttpResponse, Result as ActixResult};
use actix_web::web::Bytes;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/**
 * Estado compartido de la aplicación
 */
#[derive(Clone)]
pub struct AppState {
    pub blockchain: Arc<Mutex<Blockchain>>,
    pub wallet_manager: Arc<Mutex<WalletManager>>,
    pub block_storage: Option<Arc<BlockStorage>>,
    pub node: Option<Arc<Node>>,
    pub mempool: Arc<Mutex<Mempool>>,
    pub balance_cache: Arc<BalanceCache>,
    pub billing_manager: Arc<BillingManager>,
    pub contract_manager: Arc<RwLock<ContractManager>>,
    pub staking_manager: Arc<StakingManager>,
    pub airdrop_manager: Arc<AirdropManager>,
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

                    let latest = blockchain.get_latest_block();
                    let latest_index = latest.index;
                    let latest_block_clone = latest.clone();
                    
                    // Guardar en BlockStorage (nuevo sistema)
                    if let Some(ref storage) = state.block_storage {
                        if let Err(e) = storage.save_block(&latest_block_clone) {
                            eprintln!("⚠️  Error al guardar bloque en archivos: {}", e);
                        }
                    }
                    

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

    // Verificar límite de billing LO MÁS TEMPRANO POSIBLE
    // Esto previene procesamiento innecesario si el límite ya se alcanzó
    // Las transacciones coinbase (from == "0") son del sistema y no deben contarse
    if req.from != "0" {
        if let Some(key) = &api_key {
            match state.billing_manager.check_transaction_limit(key) {
                Ok(()) => {}
                Err(e) => {
                    // Si falla por límite, retornar error de pago requerido inmediatamente
                    if e.contains("Límite de transacciones alcanzado") {
                        let response: ApiResponse<Transaction> = ApiResponse::error(e);
                        return Ok(HttpResponse::PaymentRequired().json(response));
                    }
                    // Otros errores (key inválida, etc.)
                    let response: ApiResponse<Transaction> = ApiResponse::error(e);
                    return Ok(HttpResponse::Unauthorized().json(response));
                }
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
        
        // Agregar al mempool
        if let Err(e) = mempool.add_transaction(tx.clone()) {
            drop(mempool);
            let response: ApiResponse<Transaction> = ApiResponse::error(e);
            return Ok(HttpResponse::BadRequest().json(response));
        }
        drop(mempool);
        
        // Registrar en billing SOLO si se agregó exitosamente al mempool
        // Ya verificamos el límite al inicio, así que solo incrementamos el contador
        // Usar try_record_transaction para verificación atómica final (por si hubo race condition)
        if let Some(key) = &api_key {
            match state.billing_manager.try_record_transaction(key) {
                Ok(()) => {}
                Err(e) => {
                    // Si falla por límite aquí, significa que hubo una race condition
                    // La transacción ya está en el mempool, pero el límite se aplicó correctamente
                    if e.contains("Límite de transacciones alcanzado") {
                        let response: ApiResponse<Transaction> = ApiResponse::error(e);
                        return Ok(HttpResponse::PaymentRequired().json(response));
                    }
                    // Otros errores (key inválida, etc.)
                    let response: ApiResponse<Transaction> = ApiResponse::error(e);
                    return Ok(HttpResponse::Unauthorized().json(response));
                }
            }
        }
    }
    // Las transacciones coinbase (from == "0") no se registran en billing

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
                    let response: ApiResponse<Wallet> = ApiResponse::error(
                        "Límite de wallets alcanzado para tu tier".to_string(),
                    );
                    return Ok(HttpResponse::PaymentRequired().json(response));
                }
            }
            Err(e) => {
                let response: ApiResponse<Wallet> = ApiResponse::error(e);
                return Ok(HttpResponse::Unauthorized().json(response));
            }
        }
    }

    let mut wallet_manager = state.wallet_manager.lock().unwrap_or_else(|e| e.into_inner());
    let wallet = wallet_manager.create_wallet();
    let _address = wallet.address.clone();

    if let Some(key) = &api_key {
        if let Err(e) = state.billing_manager.record_wallet_creation(key) {
            let response: ApiResponse<Wallet> = ApiResponse::error(e);
            return Ok(HttpResponse::InternalServerError().json(response));
        }
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
        
        // Ejecutar conexión y esperar resultado para poder retornar errores
        match node_clone.connect_to_peer(&address_str).await {
            Ok(_) => {
                let response = ApiResponse::success(format!("Conectado a {}", address));
                Ok(HttpResponse::Ok().json(response))
            }
            Err(e) => {
                let error_msg = format!("Error conectando a {}: {}", address, e);
                // Si es Network ID mismatch, retornar BadRequest
                if e.to_string().contains("Network ID mismatch") {
                    let response: ApiResponse<String> = ApiResponse::error(error_msg);
                    Ok(HttpResponse::BadRequest().json(response))
                } else {
                    let response: ApiResponse<String> = ApiResponse::error(error_msg);
                    Ok(HttpResponse::InternalServerError().json(response))
                }
            }
        }
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

    let blockchain_state = state.blockchain.clone();
    let wallet_manager_state = state.wallet_manager.clone();
    let staking_manager_state = state.staking_manager.clone();
    let airdrop_manager_state = state.airdrop_manager.clone();
    
    // Seleccionar validador usando PoS
    let previous_hash = {
        let blockchain = blockchain_state.lock().unwrap_or_else(|e| e.into_inner());
        blockchain.get_latest_block().hash.clone()
    };
    
    let validator_address = staking_manager_state.select_validator(&previous_hash);
    
    let miner_address_clone = req.miner_address.clone();
    let (hash, latest, reward, validator) = actix_web::web::block(move || {
        let mut blockchain = blockchain_state.lock().unwrap_or_else(|e| e.into_inner());
        let wallet_manager = wallet_manager_state.lock().unwrap_or_else(|e| e.into_inner());
        
        // Si hay validadores, usar PoS; si no, usar PoW con miner_address
        let validator_addr = validator_address.clone();
        let address_to_use = validator_addr.as_ref().unwrap_or(&miner_address_clone);
        
        let reward = blockchain.calculate_mining_reward();
        match blockchain.mine_block_with_reward(address_to_use, transactions, &wallet_manager) {
            Ok(h) => {
                let latest = blockchain.get_latest_block().clone();
                
                // Registrar validación si usamos PoS
                if let Some(validator_addr) = &validator_addr {
                    staking_manager_state.record_validation(validator_addr, latest.index, reward);
                }
                
                // Registrar tracking de airdrop
                airdrop_manager_state.record_block_validation(
                    address_to_use,
                    latest.index,
                    latest.timestamp,
                );
                
                Ok((h, latest, reward, validator_addr))
            }
            Err(e) => Err(e),
        }
    })
    .await
    .map_err(|e| actix_web::error::ErrorInternalServerError(format!("Mining error: {}", e)))?
    .map_err(|e| actix_web::error::ErrorBadRequest(e))?;

    // Guardar tracking en base de datos

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

    // Guardar en BlockStorage
    if let Some(ref storage) = state.block_storage {
        if let Err(e) = storage.save_block(&latest) {
            eprintln!("⚠️  Error al guardar bloque en archivos: {}", e);
        }
    }

    // Verificar transacciones de airdrop en el bloque minado
    let airdrop_wallet = state.airdrop_manager.get_airdrop_wallet().to_string();
    (*state.airdrop_manager).verify_pending_claims_in_block(&latest.transactions, latest.index, &airdrop_wallet);
    

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
        validator: Option<String>,
        consensus: String,
    }

    let consensus = if validator.is_some() { "PoS" } else { "PoW" };
    let response_data = MineResponse {
        hash,
        reward,
        transactions_count: latest.transactions.len(),
        validator,
        consensus: consensus.to_string(),
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
    let mempool = state.mempool.lock().unwrap_or_else(|e| e.into_inner());
    let (cache_size, cache_block_index) = state.balance_cache.stats();
    
    // Obtener block count desde blockchain en lugar de BD
    let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    let block_count = blockchain.chain.len() as u64;
    
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
 * Request para desactivar una API key
 */
#[derive(Deserialize)]
pub struct DeactivateKeyRequest {
    pub api_key: String,
}

/**
 * Desactiva una API key
 */
pub async fn deactivate_api_key(
    state: web::Data<AppState>,
    req: web::Json<DeactivateKeyRequest>,
) -> ActixResult<HttpResponse> {
    match state.billing_manager.deactivate_key(&req.api_key) {
        Ok(_) => {
            let response: ApiResponse<String> = ApiResponse::success("API key desactivada".to_string());
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            let response: ApiResponse<String> = ApiResponse::error(e);
            Ok(HttpResponse::BadRequest().json(response))
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
 * Request para crear un smart contract
 */
#[derive(Deserialize)]
pub struct DeployContractRequest {
    pub owner: String,
    pub contract_type: String,
    pub name: String,
    pub symbol: Option<String>,
    pub total_supply: Option<u64>,
    pub decimals: Option<u8>,
}

/**
 * Request para ejecutar una función de contrato
 */
#[derive(Deserialize)]
pub struct ExecuteContractRequest {
    pub function: String, // "transfer", "mint", "burn", "custom"
    pub params: serde_json::Value,
}

/**
 * Versión de debug que recibe el body crudo para investigar problemas de deserialización
 */
pub async fn deploy_contract_debug(
    state: web::Data<AppState>,
    body: Bytes,
) -> ActixResult<HttpResponse> {
    eprintln!("[DEPLOY DEBUG] ========================================");
    eprintln!("[DEPLOY DEBUG] Request recibido en endpoint debug");
    eprintln!("[DEPLOY DEBUG] Body length: {}", body.len());
    let body_str = String::from_utf8_lossy(&body);
    eprintln!("[DEPLOY DEBUG] Body (first 500 chars): {}", &body_str[..body_str.len().min(500)]);
    
    // Llamar directamente a deploy_contract con el body
    deploy_contract(state, body).await
}

/**
 * Despliega un nuevo smart contract
 * NOTA: Este endpoint tiene problemas con el extractor JSON de Actix-Web.
 * Usar /contracts/debug como alternativa funcional.
 */
pub async fn deploy_contract(
    state: web::Data<AppState>,
    body: Bytes,
) -> ActixResult<HttpResponse> {
    eprintln!("[DEPLOY] ========================================");
    eprintln!("[DEPLOY] FUNCIÓN deploy_contract() EJECUTADA");
    eprintln!("[DEPLOY] Body recibido, length: {}", body.len());
    
    // Parsear JSON manualmente (igual que el endpoint debug)
    let req: DeployContractRequest = match serde_json::from_slice(&body) {
        Ok(r) => {
            eprintln!("[DEPLOY] JSON parseado exitosamente");
            r
        },
        Err(e) => {
            eprintln!("[DEPLOY] ERROR al parsear JSON: {}", e);
            let response: ApiResponse<String> = ApiResponse::error(format!("Invalid JSON: {}", e));
            return Ok(HttpResponse::BadRequest().json(response));
        }
    };
    
    eprintln!("[DEPLOY] Request recibido exitosamente");
    eprintln!("[DEPLOY] Tipo: {}, Owner: {}", req.contract_type, req.owner);
    eprintln!("[DEPLOY] Name: {}, Symbol: {:?}", req.name, req.symbol);
    
    eprintln!("[DEPLOY] Creando SmartContract::new()...");
    let contract = SmartContract::new(
        req.owner.clone(),
        req.contract_type.clone(),
        req.name.clone(),
        req.symbol.clone(),
        req.total_supply,
        req.decimals,
    );

    eprintln!("[DEPLOY] Adquiriendo write lock de contract_manager...");
    let address = {
        let mut contract_manager = state.contract_manager.write().unwrap_or_else(|e| e.into_inner());
        eprintln!("[DEPLOY] Write lock adquirido, llamando deploy_contract()...");
        match contract_manager.deploy_contract(contract.clone()) {
            Ok(addr) => {
                eprintln!("[DEPLOY] deploy_contract() exitoso, address: {}", addr);
                addr
            },
            Err(e) => {
                eprintln!("[DEPLOY] ERROR en deploy_contract(): {}", e);
                let response: ApiResponse<String> = ApiResponse::error(e);
                return Ok(HttpResponse::BadRequest().json(response));
            }
        }
    };
    eprintln!("[DEPLOY] Write lock liberado");
    
    // Los contratos se mantienen en memoria (ContractManager)
    // No necesitan persistencia adicional ya que se reconstruyen desde blockchain

    // Broadcast del contrato a todos los peers
    if let Some(node) = &state.node {
        eprintln!("[DEPLOY] Iniciando broadcast a peers...");
        let node_clone = node.clone();
        let contract_clone = contract.clone();
        tokio::spawn(async move {
            node_clone.broadcast_contract(&contract_clone).await;
        });
    }

    eprintln!("[DEPLOY] Creando respuesta exitosa...");
    let address_clone = address.clone();
    let response: ApiResponse<String> = ApiResponse::success(address);
    eprintln!("[DEPLOY] Deploy completado exitosamente, address: {}", address_clone);
    Ok(HttpResponse::Created().json(response))
}

/**
 * Obtiene un contrato por dirección
 */
pub async fn get_contract(
    state: web::Data<AppState>,
    address: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let contract_manager = state.contract_manager.read().unwrap_or_else(|e| e.into_inner());
    match contract_manager.get_contract(&address) {
        Some(contract) => {
            let response = ApiResponse::success(contract.clone());
            Ok(HttpResponse::Ok().json(response))
        }
        None => {
            let response: ApiResponse<SmartContract> = ApiResponse::error("Contract not found".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * Ejecuta una función de contrato
 */
use std::collections::HashMap;
use std::time::Instant;

/**
 * Información de rate limiting por caller
 */
struct CallerRateLimitInfo {
    requests_per_second: Vec<Instant>, // Timestamps de requests en el último segundo
    requests_per_minute: Vec<Instant>,  // Timestamps de requests en el último minuto
}

impl CallerRateLimitInfo {
    fn new() -> Self {
        CallerRateLimitInfo {
            requests_per_second: Vec::new(),
            requests_per_minute: Vec::new(),
        }
    }
    
    /**
     * Limpia timestamps antiguos
     */
    fn cleanup(&mut self, now: Instant) {
        let one_second_ago = now - std::time::Duration::from_secs(1);
        let one_minute_ago = now - std::time::Duration::from_secs(60);
        
        self.requests_per_second.retain(|&time| time > one_second_ago);
        self.requests_per_minute.retain(|&time| time > one_minute_ago);
    }
}

/**
 * Rate limiting específico para funciones ERC-20
 * Tracking por caller con timestamps para límites precisos
 */
lazy_static::lazy_static! {
    static ref ERC20_RATE_LIMIT: std::sync::Mutex<HashMap<String, CallerRateLimitInfo>> = 
        std::sync::Mutex::new(HashMap::new());
}

/**
 * Verifica rate limiting para un caller específico
 * @param caller - Dirección del caller
 * @returns Ok(()) si está dentro del límite, Err si excede
 */
fn check_erc20_rate_limit(caller: &str) -> Result<(), String> {
    const MAX_REQUESTS_PER_SECOND: u32 = 10;
    const MAX_REQUESTS_PER_MINUTE: u32 = 100;
    
    let mut limits = ERC20_RATE_LIMIT.lock().unwrap_or_else(|e| e.into_inner());
    let now = Instant::now();
    
    // Obtener o crear entrada para el caller específico
    let caller_info = limits.entry(caller.to_string())
        .or_insert_with(CallerRateLimitInfo::new);
    
    // Limpiar timestamps antiguos para este caller
    caller_info.cleanup(now);
    
    // Verificar límite por segundo para ESTE caller específico
    if caller_info.requests_per_second.len() >= MAX_REQUESTS_PER_SECOND as usize {
        return Err("Rate limit exceeded: too many requests per second".to_string());
    }
    
    // Verificar límite por minuto para ESTE caller específico
    if caller_info.requests_per_minute.len() >= MAX_REQUESTS_PER_MINUTE as usize {
        return Err("Rate limit exceeded: too many requests per minute".to_string());
    }
    
    // Registrar esta request
    caller_info.requests_per_second.push(now);
    caller_info.requests_per_minute.push(now);
    
    Ok(())
}

pub async fn execute_contract_function(
    state: web::Data<AppState>,
    address: web::Path<String>,
    req: web::Json<ExecuteContractRequest>,
) -> ActixResult<HttpResponse> {
    // Extraer caller de la request primero para rate limiting
    // Para ERC-20, el caller debe venir en params para transfer, approve, transferFrom
    // Para NFT, el caller puede venir en params o ser "from" para transferNFT
    // Si no viene, intentamos obtenerlo de "from" para compatibilidad con código antiguo
    let caller = req.params.get("caller")
        .and_then(|v| v.as_str())
        .or_else(|| {
            // Para compatibilidad: si hay "from" y la función es transfer o transferNFT, usar "from" como caller
            if req.function == "transfer" || req.function == "transferNFT" {
                req.params.get("from").and_then(|v| v.as_str())
            } else {
                None
            }
        });
    
    // Rate limiting específico para funciones ERC-20 y NFT (por caller)
    if let Some(caller_addr) = caller {
        if matches!(req.function.as_str(), "transfer" | "transferFrom" | "approve" | "mint" | "burn" | 
                   "mintNFT" | "transferNFT" | "approveNFT" | "transferFromNFT" | "burnNFT") {
            match check_erc20_rate_limit(caller_addr) {
                Ok(()) => {
                    // Rate limit OK, continuar
                }
                Err(e) => {
                    let response: ApiResponse<String> = ApiResponse::error(e);
                    return Ok(HttpResponse::TooManyRequests().json(response));
                }
            }
        }
    }

    let function = match req.function.as_str() {
        // ERC-20: transfer(to, amount) - caller es el from
        "transfer" => {
            let to = match req.params.get("to").and_then(|v| v.as_str()) {
                Some(t) => t.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'to' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let amount = match req.params.get("amount").and_then(|v| v.as_u64()) {
                Some(a) => a,
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'amount' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            ContractFunction::Transfer { to, amount }
        }
        // ERC-20: transferFrom(from, to, amount) - caller es el spender
        "transferFrom" => {
            let from = match req.params.get("from").and_then(|v| v.as_str()) {
                Some(f) => f.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'from' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let to = match req.params.get("to").and_then(|v| v.as_str()) {
                Some(t) => t.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'to' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let amount = match req.params.get("amount").and_then(|v| v.as_u64()) {
                Some(a) => a,
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'amount' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            ContractFunction::TransferFrom { from, to, amount }
        }
        // ERC-20: approve(spender, amount) - caller es el owner
        "approve" => {
            let spender = match req.params.get("spender").and_then(|v| v.as_str()) {
                Some(s) => s.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'spender' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let amount = match req.params.get("amount").and_then(|v| v.as_u64()) {
                Some(a) => a,
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'amount' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            ContractFunction::Approve { spender, amount }
        }
        "mint" => {
            let to = match req.params.get("to").and_then(|v| v.as_str()) {
                Some(t) => t.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'to' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let amount = match req.params.get("amount").and_then(|v| v.as_u64()) {
                Some(a) => a,
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'amount' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            ContractFunction::Mint { to, amount }
        }
        "burn" => {
            let from = match req.params.get("from").and_then(|v| v.as_str()) {
                Some(f) => f.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'from' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let amount = match req.params.get("amount").and_then(|v| v.as_u64()) {
                Some(a) => a,
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'amount' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            ContractFunction::Burn { from, amount }
        }
        // NFT: mintNFT(to, token_id, token_uri)
        "mintNFT" => {
            let to = match req.params.get("to").and_then(|v| v.as_str()) {
                Some(t) => t.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'to' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let token_id = match req.params.get("token_id").and_then(|v| v.as_u64()) {
                Some(id) => id,
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'token_id' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let token_uri = match req.params.get("token_uri").and_then(|v| v.as_str()) {
                Some(uri) => uri.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'token_uri' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            ContractFunction::MintNFT { to, token_id, token_uri }
        }
        // NFT: transferNFT(from, to, token_id) - caller es el from o approved
        "transferNFT" => {
            let from = match req.params.get("from").and_then(|v| v.as_str()) {
                Some(f) => f.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'from' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let to = match req.params.get("to").and_then(|v| v.as_str()) {
                Some(t) => t.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'to' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let token_id = match req.params.get("token_id").and_then(|v| v.as_u64()) {
                Some(id) => id,
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'token_id' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            ContractFunction::TransferNFT { from, to, token_id }
        }
        // NFT: approveNFT(to, token_id) - caller es el owner
        "approveNFT" => {
            let to = match req.params.get("to").and_then(|v| v.as_str()) {
                Some(t) => t.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'to' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let token_id = match req.params.get("token_id").and_then(|v| v.as_u64()) {
                Some(id) => id,
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'token_id' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            ContractFunction::ApproveNFT { to, token_id }
        }
        // NFT: transferFromNFT(from, to, token_id) - caller es el spender
        "transferFromNFT" => {
            let from = match req.params.get("from").and_then(|v| v.as_str()) {
                Some(f) => f.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'from' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let to = match req.params.get("to").and_then(|v| v.as_str()) {
                Some(t) => t.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'to' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let token_id = match req.params.get("token_id").and_then(|v| v.as_u64()) {
                Some(id) => id,
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'token_id' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            ContractFunction::TransferFromNFT { from, to, token_id }
        }
        // NFT: burnNFT(owner, token_id)
        "burnNFT" => {
            let owner = match req.params.get("owner").and_then(|v| v.as_str()) {
                Some(o) => o.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'owner' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let token_id = match req.params.get("token_id").and_then(|v| v.as_u64()) {
                Some(id) => id,
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'token_id' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            ContractFunction::BurnNFT { owner, token_id }
        }
        // NFT: mintNFT(to, token_id, token_uri)
        "mintNFT" => {
            let to = match req.params.get("to").and_then(|v| v.as_str()) {
                Some(t) => t.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'to' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let token_id = match req.params.get("token_id").and_then(|v| v.as_u64()) {
                Some(id) => id,
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'token_id' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let token_uri = match req.params.get("token_uri").and_then(|v| v.as_str()) {
                Some(uri) => uri.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'token_uri' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            ContractFunction::MintNFT { to, token_id, token_uri }
        }
        // NFT: transferNFT(from, to, token_id) - caller es el from o approved
        "transferNFT" => {
            let from = match req.params.get("from").and_then(|v| v.as_str()) {
                Some(f) => f.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'from' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let to = match req.params.get("to").and_then(|v| v.as_str()) {
                Some(t) => t.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'to' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let token_id = match req.params.get("token_id").and_then(|v| v.as_u64()) {
                Some(id) => id,
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'token_id' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            ContractFunction::TransferNFT { from, to, token_id }
        }
        // NFT: approveNFT(to, token_id) - caller es el owner
        "approveNFT" => {
            let to = match req.params.get("to").and_then(|v| v.as_str()) {
                Some(t) => t.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'to' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let token_id = match req.params.get("token_id").and_then(|v| v.as_u64()) {
                Some(id) => id,
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'token_id' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            ContractFunction::ApproveNFT { to, token_id }
        }
        // NFT: transferFromNFT(from, to, token_id) - caller es el spender
        "transferFromNFT" => {
            let from = match req.params.get("from").and_then(|v| v.as_str()) {
                Some(f) => f.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'from' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let to = match req.params.get("to").and_then(|v| v.as_str()) {
                Some(t) => t.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'to' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let token_id = match req.params.get("token_id").and_then(|v| v.as_u64()) {
                Some(id) => id,
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'token_id' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            ContractFunction::TransferFromNFT { from, to, token_id }
        }
        "custom" => {
            let name = match req.params.get("name").and_then(|v| v.as_str()) {
                Some(n) => n.to_string(),
                None => {
                    let response: ApiResponse<String> = ApiResponse::error("Missing 'name' parameter".to_string());
                    return Ok(HttpResponse::BadRequest().json(response));
                }
            };
            let params = req.params.get("params")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                .unwrap_or_default();
            ContractFunction::Custom { name, params }
        }
        _ => {
            let response: ApiResponse<String> = ApiResponse::error(
                format!("Unknown function: {}", req.function)
            );
            return Ok(HttpResponse::BadRequest().json(response));
        }
    };

    // Adquirir lock solo cuando necesitamos ejecutar
    let mut contract_manager = state.contract_manager.write().unwrap_or_else(|e| e.into_inner());
    
    // Ejecutar función con mejor manejo de errores
    let execution_result = contract_manager.execute_contract_function(&address, function, caller);
    
    match execution_result {
        Ok(result) => {
            // Guardar estado actualizado en BD y broadcast
            let contract_for_broadcast = contract_manager.get_contract(&address).cloned();
            drop(contract_manager); // Liberar lock antes de operaciones I/O
            
            if let Some(contract_clone) = contract_for_broadcast {
                // Broadcast de la actualización del contrato a todos los peers
                if let Some(node) = &state.node {
                    let node_clone = node.clone();
                    let contract_for_broadcast = contract_clone.clone();
                    
                    // Verificar peers antes de hacer broadcast
                    let peers_count = {
                        let peers_guard = node_clone.peers.lock().unwrap();
                        peers_guard.len()
                    };
                    
                    if peers_count > 0 {
                        tokio::spawn(async move {
                            node_clone.broadcast_contract_update(&contract_for_broadcast).await;
                        });
                    }
                }
            }

            let response: ApiResponse<String> = ApiResponse::success(result);
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            // Mejorar mensajes de error para debugging
            let error_msg = if e.contains("Insufficient") {
                e
            } else if e.contains("not found") {
                format!("Contract execution failed: {}", e)
            } else {
                format!("Execution error: {}", e)
            };
            
            let response: ApiResponse<String> = ApiResponse::error(error_msg);
            Ok(HttpResponse::BadRequest().json(response))
        }
    }
}

/**
 * Obtiene todos los contratos
 */
pub async fn get_all_contracts(
    state: web::Data<AppState>,
) -> ActixResult<HttpResponse> {
    let contract_manager = state.contract_manager.read().unwrap_or_else(|e| e.into_inner());
    let contracts: Vec<&SmartContract> = contract_manager.get_all_contracts();
    let contracts_cloned: Vec<SmartContract> = contracts.iter().map(|c| (*c).clone()).collect();
    let response = ApiResponse::success(contracts_cloned);
    Ok(HttpResponse::Ok().json(response))
}

/**
 * Obtiene el balance de un contrato para una dirección
 */
pub async fn get_contract_balance(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> ActixResult<HttpResponse> {
    let (contract_address, wallet_address) = path.into_inner();
    let contract_manager = state.contract_manager.read().unwrap_or_else(|e| e.into_inner());
    
    match contract_manager.get_contract(&contract_address) {
        Some(contract) => {
            let balance = contract.get_balance(&wallet_address);
            let response = ApiResponse::success(balance);
            Ok(HttpResponse::Ok().json(response))
        }
        None => {
            let response: ApiResponse<u64> = ApiResponse::error("Contract not found".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * ERC-20: Obtiene el allowance (owner, spender)
 */
pub async fn get_contract_allowance(
    state: web::Data<AppState>,
    path: web::Path<(String, String, String)>,
) -> ActixResult<HttpResponse> {
    let (contract_address, owner_address, spender_address) = path.into_inner();
    let contract_manager = state.contract_manager.read().unwrap_or_else(|e| e.into_inner());

    match contract_manager.get_contract(&contract_address) {
        Some(contract) => {
            let allowance = contract.allowance(&owner_address, &spender_address);
            let response: ApiResponse<u64> = ApiResponse::success(allowance);
            Ok(HttpResponse::Ok().json(response))
        }
        None => {
            let response: ApiResponse<u64> = ApiResponse::error("Contract not found".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * ERC-20: Obtiene el total supply del token
 */
pub async fn get_contract_total_supply(
    state: web::Data<AppState>,
    address: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let contract_address = address.into_inner();
    let contract_manager = state.contract_manager.read().unwrap_or_else(|e| e.into_inner());

    match contract_manager.get_contract(&contract_address) {
        Some(contract) => {
            let total_supply = contract.total_supply();
            let response: ApiResponse<u64> = ApiResponse::success(total_supply);
            Ok(HttpResponse::Ok().json(response))
        }
        None => {
            let response: ApiResponse<u64> = ApiResponse::error("Contract not found".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * NFT: Obtiene el owner de un token
 */
pub async fn get_nft_owner(
    state: web::Data<AppState>,
    path: web::Path<(String, u64)>,
) -> ActixResult<HttpResponse> {
    let (contract_address, token_id) = path.into_inner();
    let contract_manager = state.contract_manager.read().unwrap_or_else(|e| e.into_inner());

    match contract_manager.get_contract(&contract_address) {
        Some(contract) => {
            match contract.owner_of(token_id) {
                Some(owner) => {
                    let response: ApiResponse<String> = ApiResponse::success(owner);
                    Ok(HttpResponse::Ok().json(response))
                }
                None => {
                    let response: ApiResponse<String> = ApiResponse::error(format!("Token ID {} does not exist", token_id));
                    Ok(HttpResponse::NotFound().json(response))
                }
            }
        }
        None => {
            let response: ApiResponse<String> = ApiResponse::error("Contract not found".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * NFT: Obtiene la URI/metadata de un token
 */
pub async fn get_nft_token_uri(
    state: web::Data<AppState>,
    path: web::Path<(String, u64)>,
) -> ActixResult<HttpResponse> {
    let (contract_address, token_id) = path.into_inner();
    let contract_manager = state.contract_manager.read().unwrap_or_else(|e| e.into_inner());

    match contract_manager.get_contract(&contract_address) {
        Some(contract) => {
            match contract.token_uri(token_id) {
                Some(uri) => {
                    let response: ApiResponse<String> = ApiResponse::success(uri);
                    Ok(HttpResponse::Ok().json(response))
                }
                None => {
                    let response: ApiResponse<String> = ApiResponse::error(format!("Token ID {} does not exist", token_id));
                    Ok(HttpResponse::NotFound().json(response))
                }
            }
        }
        None => {
            let response: ApiResponse<String> = ApiResponse::error("Contract not found".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * NFT: Obtiene la dirección aprobada para un token
 */
pub async fn get_nft_approved(
    state: web::Data<AppState>,
    path: web::Path<(String, u64)>,
) -> ActixResult<HttpResponse> {
    let (contract_address, token_id) = path.into_inner();
    let contract_manager = state.contract_manager.read().unwrap_or_else(|e| e.into_inner());

    match contract_manager.get_contract(&contract_address) {
        Some(contract) => {
            match contract.get_approved(token_id) {
                Some(approved) => {
                    let response: ApiResponse<String> = ApiResponse::success(approved);
                    Ok(HttpResponse::Ok().json(response))
                }
                None => {
                    // No hay aprobación, devolver string vacío (comportamiento estándar ERC-721)
                    let response: ApiResponse<String> = ApiResponse::success(String::new());
                    Ok(HttpResponse::Ok().json(response))
                }
            }
        }
        None => {
            let response: ApiResponse<String> = ApiResponse::error("Contract not found".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * NFT: Obtiene el balance de NFTs de una dirección
 */
pub async fn get_nft_balance(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> ActixResult<HttpResponse> {
    let (contract_address, wallet_address) = path.into_inner();
    let contract_manager = state.contract_manager.read().unwrap_or_else(|e| e.into_inner());

    match contract_manager.get_contract(&contract_address) {
        Some(contract) => {
            let balance = contract.balance_of_nft(&wallet_address);
            let response: ApiResponse<u64> = ApiResponse::success(balance);
            Ok(HttpResponse::Ok().json(response))
        }
        None => {
            let response: ApiResponse<u64> = ApiResponse::error("Contract not found".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * NFT: Obtiene el total supply de NFTs minteados
 */
pub async fn get_nft_total_supply(
    state: web::Data<AppState>,
    address: web::Path<String>,
) -> ActixResult<HttpResponse> {
    let contract_address = address.into_inner();
    let contract_manager = state.contract_manager.read().unwrap_or_else(|e| e.into_inner());

    match contract_manager.get_contract(&contract_address) {
        Some(contract) => {
            let total_supply = contract.total_supply_nft();
            let response: ApiResponse<u64> = ApiResponse::success(total_supply);
            Ok(HttpResponse::Ok().json(response))
        }
        None => {
            let response: ApiResponse<u64> = ApiResponse::error("Contract not found".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * NFT: Lista todos los tokens de un owner (enumeración)
 */
pub async fn get_nft_tokens_of_owner(
    state: web::Data<AppState>,
    path: web::Path<(String, String)>,
) -> ActixResult<HttpResponse> {
    let (contract_address, owner_address) = path.into_inner();
    let contract_manager = state.contract_manager.read().unwrap_or_else(|e| e.into_inner());

    match contract_manager.get_contract(&contract_address) {
        Some(contract) => {
            let tokens = contract.tokens_of_owner(&owner_address);
            let response: ApiResponse<Vec<u64>> = ApiResponse::success(tokens);
            Ok(HttpResponse::Ok().json(response))
        }
        None => {
            let response: ApiResponse<Vec<u64>> = ApiResponse::error("Contract not found".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * NFT: Obtiene un token por índice (enumeración)
 */
pub async fn get_nft_token_by_index(
    state: web::Data<AppState>,
    path: web::Path<(String, usize)>,
) -> ActixResult<HttpResponse> {
    let (contract_address, index) = path.into_inner();
    let contract_manager = state.contract_manager.read().unwrap_or_else(|e| e.into_inner());

    match contract_manager.get_contract(&contract_address) {
        Some(contract) => {
            match contract.token_by_index(index) {
                Some(token_id) => {
                    let response: ApiResponse<u64> = ApiResponse::success(token_id);
                    Ok(HttpResponse::Ok().json(response))
                }
                None => {
                    let response: ApiResponse<u64> = ApiResponse::error(format!("Token at index {} does not exist", index));
                    Ok(HttpResponse::NotFound().json(response))
                }
            }
        }
        None => {
            let response: ApiResponse<u64> = ApiResponse::error("Contract not found".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * NFT: Obtiene metadata estructurada de un token
 */
pub async fn get_nft_metadata(
    state: web::Data<AppState>,
    path: web::Path<(String, u64)>,
) -> ActixResult<HttpResponse> {
    let (contract_address, token_id) = path.into_inner();
    let contract_manager = state.contract_manager.read().unwrap_or_else(|e| e.into_inner());

    match contract_manager.get_contract(&contract_address) {
        Some(contract) => {
            match contract.get_nft_metadata(token_id) {
                Some(metadata) => {
                    let response: ApiResponse<NFTMetadata> = ApiResponse::success(metadata.clone());
                    Ok(HttpResponse::Ok().json(response))
                }
                None => {
                    let response: ApiResponse<NFTMetadata> = ApiResponse::error(format!("Metadata for token ID {} does not exist", token_id));
                    Ok(HttpResponse::NotFound().json(response))
                }
            }
        }
        None => {
            let response: ApiResponse<NFTMetadata> = ApiResponse::error("Contract not found".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * NFT: Establece metadata estructurada para un token
 */
#[derive(Deserialize)]
pub struct SetNFTMetadataRequest {
    pub metadata: NFTMetadata,
}

pub async fn set_nft_metadata(
    state: web::Data<AppState>,
    path: web::Path<(String, u64)>,
    req: web::Json<SetNFTMetadataRequest>,
) -> ActixResult<HttpResponse> {
    let (contract_address, token_id) = path.into_inner();
    let mut contract_manager = state.contract_manager.write().unwrap_or_else(|e| e.into_inner());

    match contract_manager.get_contract_mut(&contract_address) {
        Some(contract) => {
            match contract.set_nft_metadata(token_id, req.metadata.clone()) {
                Ok(()) => {
                    // Guardar en BD
                    drop(contract_manager);
                    
                    let response: ApiResponse<String> = ApiResponse::success(format!("Metadata set for token {}", token_id));
                    Ok(HttpResponse::Ok().json(response))
                }
                Err(e) => {
                    let response: ApiResponse<String> = ApiResponse::error(e);
                    Ok(HttpResponse::BadRequest().json(response))
                }
            }
        }
        None => {
            let response: ApiResponse<String> = ApiResponse::error("Contract not found".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * Request para staking
 */
#[derive(Deserialize)]
pub struct StakeRequest {
    pub address: String,
    pub amount: u64,
}

/**
 * Request para unstaking
 */
#[derive(Deserialize)]
pub struct UnstakeRequest {
    pub address: String,
    #[serde(default)]
    pub amount: Option<u64>, // Opcional, si es None retira todo
}

/**
 * Stakear tokens para convertirse en validador
 */
pub async fn stake(state: web::Data<AppState>, req: web::Json<StakeRequest>) -> ActixResult<HttpResponse> {
    // Verificar balance antes de stakear
    let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
    let balance = blockchain.calculate_balance(&req.address);
    drop(blockchain);
    
    if balance < req.amount {
        let response: ApiResponse<String> = ApiResponse::error(
            format!("Saldo insuficiente. Disponible: {}, Requerido: {}", balance, req.amount)
        );
        return Ok(HttpResponse::BadRequest().json(response));
    }
    
    let wallet_manager = state.wallet_manager.lock().unwrap_or_else(|e| e.into_inner());
    
    match state.staking_manager.stake(&req.address, req.amount, &wallet_manager) {
        Ok(_) => {
            // Crear transacción especial de staking: from -> "STAKING"
            // Esta transacción "lockea" los tokens en el sistema de staking
            let mut wallet = wallet_manager
                .get_wallet_for_signing(&req.address)
                .ok_or_else(|| {
                    let response: ApiResponse<String> = ApiResponse::error("Wallet no encontrado para firmar".to_string());
                    return actix_web::error::ErrorBadRequest("Wallet not found");
                })?;
            
            let mut tx = Transaction::new_with_fee(
                req.address.clone(),
                "STAKING".to_string(), // Dirección especial para staking
                req.amount,
                0,
                Some(format!("Staking: {} tokens", req.amount)),
            );
            
            wallet.sign_transaction(&mut tx);
            let _ = wallet;
            drop(wallet_manager);
            
            // Agregar al mempool
            let mut mempool = state.mempool.lock().unwrap_or_else(|e| e.into_inner());
            if let Err(e) = mempool.add_transaction(tx.clone()) {
                drop(mempool);
                let response: ApiResponse<String> = ApiResponse::error(e);
                return Ok(HttpResponse::BadRequest().json(response));
            }
            drop(mempool);
            
            // Los validadores se reconstruyen desde blockchain, no necesitan persistencia adicional
            
            let response: ApiResponse<String> = ApiResponse::success(
                format!("Staked {} tokens successfully. Transaction added to mempool.", req.amount)
            );
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            let response: ApiResponse<String> = ApiResponse::error(e);
            Ok(HttpResponse::BadRequest().json(response))
        }
    }
}

/**
 * Solicitar unstaking (retiro de tokens)
 */
pub async fn request_unstake(state: web::Data<AppState>, req: web::Json<UnstakeRequest>) -> ActixResult<HttpResponse> {
    match state.staking_manager.request_unstake(&req.address, req.amount) {
        Ok(amount) => {
            // Los validadores se reconstruyen desde blockchain, no necesitan persistencia adicional
            
            let response: ApiResponse<u64> = ApiResponse::success(amount);
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            let response: ApiResponse<String> = ApiResponse::error(e);
            Ok(HttpResponse::BadRequest().json(response))
        }
    }
}

/**
 * Completar unstaking después del período de lock
 */
pub async fn complete_unstake(state: web::Data<AppState>, address: web::Path<String>) -> ActixResult<HttpResponse> {
    match state.staking_manager.complete_unstake(&address) {
        Ok(amount) => {
            // Crear transacción especial de unstaking: "STAKING" -> address
            // Esta transacción devuelve los tokens del sistema de staking al usuario
            let mut tx = Transaction::new_with_fee(
                "STAKING".to_string(), // Dirección especial para staking
                address.clone(),
                amount,
                0,
                Some(format!("Unstaking: {} tokens", amount)),
            );
            
            // Las transacciones desde "STAKING" no requieren firma (son del sistema)
            // Agregar al mempool
            let mut mempool = state.mempool.lock().unwrap_or_else(|e| e.into_inner());
            if let Err(e) = mempool.add_transaction(tx.clone()) {
                drop(mempool);
                let response: ApiResponse<String> = ApiResponse::error(e);
                return Ok(HttpResponse::BadRequest().json(response));
            }
            drop(mempool);
            
            // Los validadores se reconstruyen desde blockchain, no necesitan persistencia adicional
            
            let response: ApiResponse<u64> = ApiResponse::success(amount);
            Ok(HttpResponse::Ok().json(response))
        }
        Err(e) => {
            let response: ApiResponse<String> = ApiResponse::error(e);
            Ok(HttpResponse::BadRequest().json(response))
        }
    }
}

/**
 * Obtener lista de validadores activos
 */
pub async fn get_validators(state: web::Data<AppState>) -> ActixResult<HttpResponse> {
    let validators = state.staking_manager.get_active_validators();
    let response: ApiResponse<Vec<crate::staking::Validator>> = ApiResponse::success(validators);
    Ok(HttpResponse::Ok().json(response))
}

/**
 * Obtener información de un validador específico
 */
pub async fn get_validator(state: web::Data<AppState>, address: web::Path<String>) -> ActixResult<HttpResponse> {
    match state.staking_manager.get_validator(&address) {
        Some(validator) => {
            let response: ApiResponse<crate::staking::Validator> = ApiResponse::success(validator);
            Ok(HttpResponse::Ok().json(response))
        }
        None => {
            let response: ApiResponse<String> = ApiResponse::error("Validator not found".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * Obtener información de staking del usuario
 */
pub async fn get_my_stake(state: web::Data<AppState>, address: web::Path<String>) -> ActixResult<HttpResponse> {
    match state.staking_manager.get_validator(&address) {
        Some(validator) => {
            let response: ApiResponse<crate::staking::Validator> = ApiResponse::success(validator);
            Ok(HttpResponse::Ok().json(response))
        }
        None => {
            let response: ApiResponse<String> = ApiResponse::error("You are not a validator".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * Request para reclamar airdrop
 */
#[derive(Deserialize)]
pub struct ClaimAirdropRequest {
    pub node_address: String,
}

/**
 * Reclamar airdrop para un nodo elegible
 */
pub async fn claim_airdrop(
    state: web::Data<AppState>,
    req: web::Json<ClaimAirdropRequest>,
    peer_addr: actix_web::HttpRequest,
) -> ActixResult<HttpResponse> {
    let node_address = req.node_address.clone();

    // Rate limiting: máximo 10 claims por minuto por IP
    let client_ip = peer_addr.connection_info().peer_addr()
        .unwrap_or("unknown")
        .to_string();
    
    if !(*state.airdrop_manager).check_rate_limit(&client_ip, 10) {
        let response: ApiResponse<String> = ApiResponse::error(
            "Rate limit exceeded. Maximum 10 claims per minute.".to_string(),
        );
        return Ok(HttpResponse::TooManyRequests().json(response));
    }

    // Verificar elegibilidad
    if !state.airdrop_manager.is_eligible(&node_address) {
        let response: ApiResponse<String> = ApiResponse::error(
            "Node is not eligible for airdrop or has already claimed".to_string(),
        );
        return Ok(HttpResponse::BadRequest().json(response));
    }

    // Obtener información del tracking
    let tracking = match state.airdrop_manager.get_node_tracking(&node_address) {
        Some(t) => t,
        None => {
            let response: ApiResponse<String> = ApiResponse::error("Node tracking not found".to_string());
            return Ok(HttpResponse::NotFound().json(response));
        }
    };

    // Calcular cantidad de airdrop basada en tier y participación
    let airdrop_amount = state.airdrop_manager.calculate_airdrop_amount(&tracking);
    let airdrop_wallet = state.airdrop_manager.get_airdrop_wallet().to_string();

    let airdrop_wallet_balance = {
        let blockchain = state.blockchain.lock().unwrap_or_else(|e| e.into_inner());
        blockchain.calculate_balance(&airdrop_wallet)
    };

    if airdrop_wallet_balance < airdrop_amount {
        let response: ApiResponse<String> = ApiResponse::error(
            format!(
                "Insufficient airdrop wallet balance. Required: {}, Available: {}",
                airdrop_amount, airdrop_wallet_balance
            ),
        );
        return Ok(HttpResponse::PaymentRequired().json(response));
    }

    // Crear transacción de airdrop
    let mut wallet_manager = state.wallet_manager.lock().unwrap_or_else(|e| e.into_inner());
    
    let wallet_for_signing = match wallet_manager.get_wallet_for_signing(&airdrop_wallet) {
        Some(w) => w,
        None => {
            drop(wallet_manager);
            let response: ApiResponse<String> = ApiResponse::error(
                format!(
                    "Airdrop wallet '{}' not found. Please ensure the wallet exists (create it via /api/v1/wallets/create) and has sufficient balance. You can configure AIRDROP_WALLET environment variable to use a specific wallet address.",
                    airdrop_wallet
                ),
            );
            return Ok(HttpResponse::BadRequest().json(response));
        }
    };

    let mut airdrop_tx = Transaction::new_with_fee(
        airdrop_wallet.clone(),
        node_address.clone(),
        airdrop_amount,
        0,
        None,
    );

    wallet_for_signing.sign_transaction(&mut airdrop_tx);
    let transaction_id = airdrop_tx.id.clone();
    drop(wallet_manager);

    // Agregar transacción al mempool
    {
        let mut mempool = state.mempool.lock().unwrap_or_else(|e| e.into_inner());
        let _ = mempool.add_transaction(airdrop_tx.clone());
    }

    // Marcar como reclamado y agregar a pending claims
    (*state.airdrop_manager).mark_as_claimed(&node_address, transaction_id.clone());
    (*state.airdrop_manager).add_pending_claim(&node_address, transaction_id.clone());

    // Agregar al historial
    let claim_record = crate::airdrop::ClaimRecord {
        node_address: node_address.clone(),
        claim_timestamp: SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        airdrop_amount,
        transaction_id: transaction_id.clone(),
        block_index: None,
        tier_id: tracking.eligibility_tier,
        verified: false,
        verification_timestamp: None,
    };
    state.airdrop_manager.add_claim_to_history(claim_record);

    // El tracking de airdrop se reconstruye desde blockchain, no necesita persistencia adicional

    let response: ApiResponse<serde_json::Value> = ApiResponse::success(serde_json::json!({
        "node_address": node_address,
        "airdrop_amount": airdrop_amount,
        "transaction_id": transaction_id,
        "tier": tracking.eligibility_tier,
        "message": "Airdrop claimed successfully. Transaction added to mempool. Verification pending."
    }));

    Ok(HttpResponse::Ok().json(response))
}

/**
 * Obtener información de tracking de un nodo
 */
pub async fn get_node_tracking(
    state: web::Data<AppState>,
    address: web::Path<String>,
) -> ActixResult<HttpResponse> {
    match state.airdrop_manager.get_node_tracking(&address) {
        Some(tracking) => {
            let response: ApiResponse<crate::airdrop::NodeTracking> = ApiResponse::success(tracking);
            Ok(HttpResponse::Ok().json(response))
        }
        None => {
            let response: ApiResponse<String> = ApiResponse::error("Node tracking not found".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * Obtener estadísticas del airdrop
 */
pub async fn get_airdrop_statistics(state: web::Data<AppState>) -> ActixResult<HttpResponse> {
    let stats = state.airdrop_manager.get_statistics();
    let response: ApiResponse<crate::airdrop::AirdropStatistics> = ApiResponse::success(stats);
    Ok(HttpResponse::Ok().json(response))
}

/**
 * Obtener lista de nodos elegibles
 */
pub async fn get_eligible_nodes(state: web::Data<AppState>) -> ActixResult<HttpResponse> {
    let nodes = state.airdrop_manager.get_eligible_nodes();
    let response: ApiResponse<Vec<crate::airdrop::NodeTracking>> = ApiResponse::success(nodes);
    Ok(HttpResponse::Ok().json(response))
}

/**
 * Obtener información de elegibilidad de un nodo (sin hacer claim)
 */
pub async fn get_eligibility_info(
    state: web::Data<AppState>,
    address: web::Path<String>,
) -> ActixResult<HttpResponse> {
    match state.airdrop_manager.get_eligibility_info(&address) {
        Some(info) => {
            let response: ApiResponse<crate::airdrop::EligibilityInfo> = ApiResponse::success(info);
            Ok(HttpResponse::Ok().json(response))
        }
        None => {
            let response: ApiResponse<String> = ApiResponse::error("Node tracking not found".to_string());
            Ok(HttpResponse::NotFound().json(response))
        }
    }
}

/**
 * Obtener historial de claims
 */
pub async fn get_claim_history(
    state: web::Data<AppState>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> ActixResult<HttpResponse> {
    let limit = query.get("limit")
        .and_then(|s| s.parse::<u64>().ok());
    let node_address = query.get("node_address")
        .map(|s| s.as_str());
    
    let history = state.airdrop_manager.get_claim_history(limit, node_address);
    let response: ApiResponse<Vec<crate::airdrop::ClaimRecord>> = ApiResponse::success(history);
    Ok(HttpResponse::Ok().json(response))
}

/**
 * Obtener información de tiers disponibles
 */
pub async fn get_airdrop_tiers(state: web::Data<AppState>) -> ActixResult<HttpResponse> {
    let tiers = state.airdrop_manager.get_tiers();
    let response: ApiResponse<Vec<crate::airdrop::AirdropTier>> = ApiResponse::success(tiers);
    Ok(HttpResponse::Ok().json(response))
}

/**
 * Configura las rutas de la API
 */
pub fn config_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api/v1")
            .route("/billing/create-key", web::post().to(create_api_key))
            .route("/billing/deactivate-key", web::post().to(deactivate_api_key))
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
            .route("/health", web::get().to(health_check))
            // IMPORTANTE: Rutas exactas ANTES de rutas con parámetros
            .route("/contracts/debug", web::post().to(deploy_contract_debug))
            .route("/contracts", web::get().to(get_all_contracts))
            .route("/contracts", web::post().to(deploy_contract))
            // Rutas con parámetros DESPUÉS de rutas exactas
            .route("/contracts/{address}", web::get().to(get_contract))
            .route("/contracts/{address}/execute", web::post().to(execute_contract_function))
            .route("/contracts/{address}/balance/{wallet}", web::get().to(get_contract_balance))
            .route("/contracts/{address}/allowance/{owner}/{spender}", web::get().to(get_contract_allowance))
            .route("/contracts/{address}/totalSupply", web::get().to(get_contract_total_supply))
            // NFT endpoints
            .route("/contracts/{address}/nft/{token_id}/owner", web::get().to(get_nft_owner))
            .route("/contracts/{address}/nft/{token_id}/uri", web::get().to(get_nft_token_uri))
            .route("/contracts/{address}/nft/{token_id}/approved", web::get().to(get_nft_approved))
            .route("/contracts/{address}/nft/{token_id}/metadata", web::get().to(get_nft_metadata))
            .route("/contracts/{address}/nft/{token_id}/metadata", web::post().to(set_nft_metadata))
            .route("/contracts/{address}/nft/balance/{wallet}", web::get().to(get_nft_balance))
            .route("/contracts/{address}/nft/totalSupply", web::get().to(get_nft_total_supply))
            .route("/contracts/{address}/nft/tokens/{owner}", web::get().to(get_nft_tokens_of_owner))
            .route("/contracts/{address}/nft/index/{index}", web::get().to(get_nft_token_by_index))
            // Staking endpoints
            .route("/staking/stake", web::post().to(stake))
            .route("/staking/unstake", web::post().to(request_unstake))
            .route("/staking/complete-unstake/{address}", web::post().to(complete_unstake))
            .route("/staking/validators", web::get().to(get_validators))
            .route("/staking/validator/{address}", web::get().to(get_validator))
            .route("/staking/my-stake/{address}", web::get().to(get_my_stake))
            // Airdrop endpoints
            .route("/airdrop/claim", web::post().to(claim_airdrop))
            .route("/airdrop/tracking/{address}", web::get().to(get_node_tracking))
            .route("/airdrop/statistics", web::get().to(get_airdrop_statistics))
            .route("/airdrop/eligible", web::get().to(get_eligible_nodes))
            .route("/airdrop/eligibility/{address}", web::get().to(get_eligibility_info))
            .route("/airdrop/history", web::get().to(get_claim_history))
            .route("/airdrop/tiers", web::get().to(get_airdrop_tiers)),
    );
}

