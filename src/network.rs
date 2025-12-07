use crate::blockchain::{Block, Blockchain};
use crate::database::BlockchainDB;
use crate::models::{Transaction, WalletManager};
use crate::smart_contracts::{ContractManager, SmartContract};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/**
 * Tipos de mensajes en la red P2P
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Ping,
    Pong,
    GetBlocks,
    Blocks(Vec<Block>),
    NewBlock(Block),
    NewTransaction(Transaction),
    GetPeers,
    Peers(Vec<String>),
    Version {
        version: String,
        block_count: usize,
        latest_hash: String,
        p2p_address: Option<String>, // Dirección P2P del nodo que envía el mensaje
    },
    // Mensajes de contratos
    GetContracts,
    GetContractsSince {
        timestamp: u64,
    },
    Contracts(Vec<SmartContract>),
    NewContract(SmartContract),
    UpdateContract(SmartContract),
}

/**
 * Nodo en la red P2P
 */
/**
 * Métricas de sincronización de contratos
 */
#[derive(Debug, Clone, Default)]
pub struct ContractSyncMetrics {
    pub last_sync_timestamp: u64,
    pub contracts_synced: usize,
    pub sync_errors: usize,
    pub last_sync_duration_ms: u64,
}

#[derive(Clone)]
pub struct Node {
    #[allow(dead_code)]
    pub address: SocketAddr,
    pub peers: Arc<Mutex<HashSet<String>>>,
    pub blockchain: Arc<Mutex<Blockchain>>,
    pub wallet_manager: Option<Arc<Mutex<WalletManager>>>,
    pub db: Option<Arc<Mutex<BlockchainDB>>>,
    pub contract_manager: Option<Arc<RwLock<ContractManager>>>,
    pub listening: bool,
    pub contract_sync_metrics: Arc<Mutex<HashMap<String, ContractSyncMetrics>>>,
    pub pending_contract_broadcasts: Arc<Mutex<Vec<(String, SmartContract)>>>,
    // Tracking de contratos recibidos recientemente para prevenir loops
    pub recent_contract_receipts: Arc<Mutex<HashMap<String, (u64, String)>>>, // (contract_address, timestamp, source_peer)
    // Rate limiting para contratos por peer
    pub contract_rate_limits: Arc<Mutex<HashMap<String, (u64, usize)>>>, // (peer_address, (timestamp, count))
}

impl Node {
    /**
     * Crea un nuevo nodo
     */
    pub fn new(address: SocketAddr, blockchain: Arc<Mutex<Blockchain>>) -> Node {
        Node {
            address,
            peers: Arc::new(Mutex::new(HashSet::new())),
            blockchain,
            wallet_manager: None,
            db: None,
            contract_manager: None,
            listening: false,
            contract_sync_metrics: Arc::new(Mutex::new(HashMap::new())),
            pending_contract_broadcasts: Arc::new(Mutex::new(Vec::new())),
            recent_contract_receipts: Arc::new(Mutex::new(HashMap::new())),
            contract_rate_limits: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /**
     * Configura el wallet manager y la base de datos para el nodo
     */
    pub fn set_resources(
        &mut self,
        wallet_manager: Arc<Mutex<WalletManager>>,
        db: Arc<Mutex<BlockchainDB>>,
    ) {
        self.wallet_manager = Some(wallet_manager);
        self.db = Some(db);
    }

    /**
     * Configura el contract manager para el nodo
     */
    pub fn set_contract_manager(&mut self, contract_manager: Arc<RwLock<ContractManager>>) {
        self.contract_manager = Some(contract_manager);
    }

    /**
     * Inicia el servidor P2P
     */
    pub async fn start_server(&mut self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(&addr).await?;
        self.listening = true;

        println!("🌐 Servidor P2P iniciado en {}", addr);

        // Clonar recursos compartidos antes del loop
        let peers = self.peers.clone();
        let blockchain = self.blockchain.clone();
        let wallet_manager = self.wallet_manager.clone();
        let db = self.db.clone();
        let contract_manager = self.contract_manager.clone();
        let my_p2p_address = format!("{}:{}", self.address.ip(), self.address.port());
        let recent_receipts = self.recent_contract_receipts.clone();
        let rate_limits = self.contract_rate_limits.clone();
        let pending_broadcasts = self.pending_contract_broadcasts.clone();

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    println!("📡 Nueva conexión desde: {}", peer_addr);
                    let peers_clone = peers.clone();
                    let blockchain_clone = blockchain.clone();
                    let wallet_manager_clone = wallet_manager.clone();
                    let db_clone = db.clone();
                    let contract_manager_clone = contract_manager.clone();
                    let my_p2p_address_clone = my_p2p_address.clone();
                    let recent_receipts_clone = recent_receipts.clone();
                    let rate_limits_clone = rate_limits.clone();
                    let pending_broadcasts_clone = pending_broadcasts.clone();

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(
                            stream, 
                            peers_clone, 
                            blockchain_clone, 
                            wallet_manager_clone, 
                            db_clone, 
                            contract_manager_clone, 
                            Some(my_p2p_address_clone),
                            recent_receipts_clone,
                            rate_limits_clone,
                            pending_broadcasts_clone,
                        ).await {
                            eprintln!("Error manejando conexión: {}", e);
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Error aceptando conexión: {}", e);
                }
            }
        }
    }

    /**
     * Maneja una conexión entrante
     */
    async fn handle_connection(
        mut stream: TcpStream,
        peers: Arc<Mutex<HashSet<String>>>,
        blockchain: Arc<Mutex<Blockchain>>,
        wallet_manager: Option<Arc<Mutex<WalletManager>>>,
        db: Option<Arc<Mutex<BlockchainDB>>>,
        contract_manager: Option<Arc<RwLock<ContractManager>>>,
        my_p2p_address: Option<String>,
        recent_receipts: Arc<Mutex<HashMap<String, (u64, String)>>>,
        rate_limits: Arc<Mutex<HashMap<String, (u64, usize)>>>,
        pending_broadcasts: Arc<Mutex<Vec<(String, SmartContract)>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let peer_addr = stream.peer_addr()?;
        let peer_addr_str = format!("{}:{}", peer_addr.ip(), peer_addr.port());
        let mut buffer = [0; 8192]; // Aumentado a 8KB para contratos más grandes
        let mut first_message = true;
        
        // Limpiar rate limit antiguo (más de 1 minuto)
        {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let mut limits = rate_limits.lock().unwrap();
            limits.retain(|_, (ts, _)| now - *ts < 60);
        }
        
        // Procesar contratos pendientes para este peer
        {
            let mut pending = pending_broadcasts.lock().unwrap();
            let mut to_remove = Vec::new();
            for (i, (peer, contract)) in pending.iter().enumerate() {
                if peer == &peer_addr_str {
                    // Intentar enviar el contrato pendiente (se intentará en background)
                    to_remove.push(i);
                    
                    // Remover de BD si está disponible
                    if let Some(db) = &db {
                        let db_guard = db.lock().unwrap();
                        if let Err(e) = db_guard.remove_pending_broadcast(peer, &contract.address) {
                            eprintln!("⚠️  Error removiendo broadcast pendiente de BD: {}", e);
                        }
                    }
                }
            }
            // Remover en orden inverso para mantener índices válidos
            for i in to_remove.into_iter().rev() {
                pending.remove(i);
            }
        }

        loop {
            let n = stream.read(&mut buffer).await?;
            if n == 0 {
                break;
            }

            let message_str = String::from_utf8_lossy(&buffer[..n]);
            if let Ok(message) = serde_json::from_str::<Message>(&message_str) {
                // Si es el primer mensaje y es Version, responder con nuestra dirección P2P
                if first_message {
                    if let Message::Version { p2p_address, .. } = &message {
                        // Agregar el peer que se conectó a nuestra lista
                        if let Some(their_p2p_addr) = p2p_address {
                            let mut peers_guard = peers.lock().unwrap();
                            peers_guard.insert(their_p2p_addr.clone());
                            println!("📡 Peer agregado desde conexión entrante: {}", their_p2p_addr);
                        }
                        first_message = false;
                    }
                }
                
                let response = Self::process_message(
                    message, 
                    &peers, 
                    &blockchain, 
                    wallet_manager.clone(), 
                    db.clone(), 
                    contract_manager.clone(), 
                    my_p2p_address.clone(),
                    Some(peer_addr_str.clone()),
                    recent_receipts.clone(),
                    rate_limits.clone(),
                ).await?;
                
                if let Some(response_msg) = response {
                    let response_json = serde_json::to_string(&response_msg)?;
                    stream.write_all(response_json.as_bytes()).await?;
                }
            }
        }

        Ok(())
    }

    /**
     * Procesa un mensaje recibido
     */
    async fn process_message(
        message: Message,
        peers: &Arc<Mutex<HashSet<String>>>,
        blockchain: &Arc<Mutex<Blockchain>>,
        wallet_manager: Option<Arc<Mutex<WalletManager>>>,
        db: Option<Arc<Mutex<BlockchainDB>>>,
        contract_manager: Option<Arc<RwLock<ContractManager>>>,
        my_p2p_address: Option<String>,
        source_peer: Option<String>,
        recent_receipts: Arc<Mutex<HashMap<String, (u64, String)>>>,
        rate_limits: Arc<Mutex<HashMap<String, (u64, usize)>>>,
    ) -> Result<Option<Message>, Box<dyn std::error::Error>> {
        match message {
            Message::Ping => Ok(Some(Message::Pong)),
            
            Message::GetBlocks => {
                let blockchain = blockchain.lock().unwrap();
                let blocks = blockchain.chain.clone();
                Ok(Some(Message::Blocks(blocks)))
            }
            
            Message::Blocks(blocks) => {
                let mut blockchain = blockchain.lock().unwrap();
                
                // Resolver conflicto usando la regla de la cadena más larga
                if blocks.len() > blockchain.chain.len() {
                    if Self::is_valid_chain(&blocks) {
                        // Validar transacciones si tenemos wallet_manager
                        let should_replace = if let Some(wm) = &wallet_manager {
                            let wm_guard = wm.lock().unwrap();
                            blockchain.resolve_conflict(&blocks, &wm_guard)
                        } else {
                            // Sin wallet_manager, solo validar estructura
                            blockchain.chain = blocks.clone();
                            true
                        };
                        
                        if should_replace {
                            println!("✅ Blockchain sincronizada: {} bloques (reemplazada por cadena más larga)", blocks.len());
                            
                            // Sincronizar wallets desde la nueva blockchain
                            if let Some(wm) = &wallet_manager {
                                let mut wm_guard = wm.lock().unwrap();
                                wm_guard.sync_from_blockchain(&blockchain.chain);
                            }
                            
                            // Guardar en base de datos si está disponible
                            if let Some(db) = &db {
                                let db_guard = db.lock().unwrap();
                                if let Err(e) = db_guard.save_blockchain(&blockchain) {
                                    eprintln!("⚠️  Error guardando blockchain en BD: {}", e);
                                }
                            }
                        } else {
                            println!("⚠️  Cadena recibida no pasó validación de transacciones");
                        }
                    } else {
                        println!("⚠️  Cadena recibida no es válida");
                    }
                } else if blocks.len() == blockchain.chain.len() {
                    // Misma longitud: verificar si hay diferencias (fork)
                    let my_latest = blockchain.get_latest_block().hash.clone();
                    let their_latest = blocks.last().map(|b| b.hash.clone()).unwrap_or_default();
                    
                    if my_latest != their_latest {
                        println!("⚠️  Fork detectado: misma longitud pero diferentes últimos bloques");
                        // Mantenemos nuestra cadena (regla de la cadena más larga)
                    }
                }
                Ok(None)
            }
            
            Message::NewBlock(block) => {
                let mut blockchain = blockchain.lock().unwrap();
                let latest = blockchain.get_latest_block();
                
                // Verificar si el bloque ya existe
                let block_exists = blockchain.chain.iter().any(|b| b.hash == block.hash);
                if block_exists {
                    println!("ℹ️  Bloque ya existe en nuestra cadena");
                    return Ok(None);
                }
                
                // Verificar que el bloque es el siguiente en la cadena
                if block.previous_hash != latest.hash {
                    // Si el índice es mayor, necesitamos sincronizar primero
                    if block.index > latest.index {
                        println!("📥 Bloque recibido tiene índice mayor ({} > {}), puede necesitar sincronización", 
                            block.index, latest.index);
                        // Guardar el bloque para agregarlo después de sincronizar
                        // Por ahora rechazamos, pero el peer debería sincronizar cuando se conecte
                        return Ok(None);
                    }
                    
                    // Si el índice es igual pero el hash es diferente, hay un fork
                    if block.index == latest.index {
                        println!("⚠️  Fork detectado: mismo índice pero diferentes hashes");
                        // En un fork, mantenemos nuestra cadena (regla de la cadena más larga se aplica después)
                        return Ok(None);
                    }
                    
                    // Si el índice es menor, el bloque es antiguo y ya debería estar en nuestra cadena
                    // Pero puede ser que tengamos diferentes génesis, verificar si el bloque existe
                    let block_exists_by_index = blockchain.chain.iter().any(|b| b.index == block.index);
                    if !block_exists_by_index && block.index < latest.index {
                        println!("⚠️  Bloque recibido es anterior pero no está en nuestra cadena (posible génesis diferente)");
                        // Intentar encontrar el bloque en nuestra cadena por hash
                        let block_found = blockchain.chain.iter().any(|b| b.hash == block.hash);
                        if !block_found {
                            println!("💡 Bloque no encontrado, puede requerir sincronización completa");
                        }
                    }
                    return Ok(None);
                }
                
                // Validar el bloque
                if !block.is_valid() {
                    println!("⚠️  Bloque recibido no es válido");
                    return Ok(None);
                }
                
                // Validar transacciones si tenemos wallet_manager
                if let Some(wm) = &wallet_manager {
                    let wallet_manager_guard = wm.lock().unwrap();
                    for tx in &block.transactions {
                        if tx.from != "0" {
                            if let Err(e) = blockchain.validate_transaction(tx, &wallet_manager_guard) {
                                println!("⚠️  Transacción inválida en bloque recibido: {}", e);
                                return Ok(None);
                            }
                        }
                    }
                }
                
                // Agregar el bloque
                let block_clone = block.clone();
                blockchain.chain.push(block_clone.clone());
                println!("✅ Nuevo bloque recibido y agregado: {} transacciones", block_clone.transactions.len());
                
                // Procesar transacciones si tenemos wallet_manager
                if let Some(wm) = &wallet_manager {
                    let mut wallet_manager_guard = wm.lock().unwrap();
                    for tx in &block_clone.transactions {
                        if tx.from == "0" {
                            // Coinbase transaction
                            if let Err(e) = wallet_manager_guard.process_coinbase_transaction(tx) {
                                eprintln!("⚠️  Error procesando transacción coinbase: {}", e);
                            }
                        } else {
                            // Transfer transaction
                            if let Err(e) = wallet_manager_guard.process_transaction(tx) {
                                eprintln!("⚠️  Error procesando transacción: {}", e);
                            }
                        }
                    }
                }
                
                // Guardar en base de datos si está disponible
                if let Some(db) = &db {
                    let db_guard = db.lock().unwrap();
                    if let Err(e) = db_guard.save_block(&block_clone) {
                        eprintln!("⚠️  Error guardando bloque en BD: {}", e);
                    }
                }
                
                Ok(None)
            }
            
            Message::NewTransaction(tx) => {
                println!("📨 Nueva transacción recibida: {} -> {} ({} unidades)", 
                    tx.from, tx.to, tx.amount);
                Ok(None)
            }
            
            Message::GetPeers => {
                let peers = peers.lock().unwrap();
                let peer_list: Vec<String> = peers.iter().cloned().collect();
                Ok(Some(Message::Peers(peer_list)))
            }
            
            Message::Peers(peer_list) => {
                let mut peers = peers.lock().unwrap();
                for peer in peer_list {
                    peers.insert(peer);
                }
                Ok(None)
            }
            
            Message::Version { block_count: their_count, latest_hash: their_hash, p2p_address, .. } => {
                // Si el peer envió su dirección P2P, agregarlo a nuestra lista
                if let Some(their_p2p_addr) = p2p_address {
                    let mut peers_guard = peers.lock().unwrap();
                    peers_guard.insert(their_p2p_addr);
                }
                let blockchain = blockchain.lock().unwrap();
                let latest = blockchain.get_latest_block();
                let my_count = blockchain.chain.len();
                let my_hash = latest.hash.clone();
                
                // Si tienen más bloques o mismo número pero diferente hash, indicar que pueden sincronizar
                if their_count > my_count || (their_count == my_count && their_hash != my_hash) {
                    // El peer que recibió este mensaje debería sincronizar
                    // Por ahora solo respondemos con nuestra versión
                }
                
                Ok(Some(Message::Version {
                    version: "1.0.0".to_string(),
                    block_count: my_count,
                    latest_hash: my_hash,
                    p2p_address: my_p2p_address,
                }))
            }
            
            Message::Pong => Ok(None),
            
            // Mensajes de contratos
            Message::GetContracts => {
                if let Some(cm) = &contract_manager {
                    let cm_guard = cm.read().unwrap();
                    let contracts: Vec<SmartContract> = cm_guard.get_all_contracts()
                        .iter()
                        .map(|c| (*c).clone())
                        .collect();
                    Ok(Some(Message::Contracts(contracts)))
                } else {
                    Ok(Some(Message::Contracts(Vec::new())))
                }
            }
            
            Message::GetContractsSince { timestamp } => {
                if let Some(cm) = &contract_manager {
                    let cm_guard = cm.read().unwrap();
                    let contracts: Vec<SmartContract> = cm_guard.get_all_contracts()
                        .iter()
                        .filter(|c| c.updated_at > timestamp || (c.updated_at == timestamp && c.update_sequence > 0))
                        .map(|c| (*c).clone())
                        .collect();
                    Ok(Some(Message::Contracts(contracts)))
                } else {
                    Ok(Some(Message::Contracts(Vec::new())))
                }
            }
            
            Message::Contracts(contracts) => {
                if let Some(cm) = &contract_manager {
                    let mut cm_guard = cm.write().unwrap();
                    let mut synced = 0;
                    let mut errors = 0;
                    
                    for contract in contracts {
                        // Validar integridad del contrato
                        if !contract.validate_integrity() {
                            eprintln!("⚠️  Contrato recibido tiene hash de integridad inválido: {}", contract.address);
                            errors += 1;
                            continue;
                        }
                        
                        // Verificar si el contrato ya existe
                        if cm_guard.get_contract(&contract.address).is_none() {
                            // Contrato nuevo, agregarlo
                            if let Ok(_) = cm_guard.deploy_contract(contract.clone()) {
                                synced += 1;
                                
                                // Guardar en BD si está disponible
                                if let Some(db) = &db {
                                    let db_guard = db.lock().unwrap();
                                    if let Err(e) = db_guard.save_contract(&contract) {
                                        eprintln!("⚠️  Error guardando contrato sincronizado en BD: {}", e);
                                    }
                                }
                            }
                        } else {
                            // Contrato existe, verificar si necesita actualización
                            if let Some(existing) = cm_guard.get_contract(&contract.address) {
                                // Validar que el owner no haya cambiado ilegalmente
                                if contract.owner != existing.owner {
                                    eprintln!("⚠️  Intento de actualizar contrato con owner diferente rechazado: {}", contract.address);
                                    errors += 1;
                                    continue;
                                }
                                
                                // Comparar por updated_at y update_sequence para resolver race conditions
                                let should_update = contract.updated_at > existing.updated_at 
                                    || (contract.updated_at == existing.updated_at && contract.update_sequence > existing.update_sequence);
                                
                                if should_update {
                                    // Actualizar el contrato
                                    if let Some(existing_mut) = cm_guard.get_contract_mut(&contract.address) {
                                        *existing_mut = contract.clone();
                                        synced += 1;
                                        
                                        // Guardar en BD
                                        if let Some(db) = &db {
                                            let db_guard = db.lock().unwrap();
                                            if let Err(e) = db_guard.save_contract(&contract) {
                                                eprintln!("⚠️  Error guardando contrato actualizado en BD: {}", e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    if synced > 0 {
                        println!("✅ {} contratos sincronizados desde peer", synced);
                    }
                    if errors > 0 {
                        println!("⚠️  {} contratos rechazados por validación", errors);
                    }
                }
                Ok(None)
            }
            
            Message::NewContract(contract) => {
                // Validar tamaño del contrato (máximo 1MB)
                let contract_size = serde_json::to_string(&contract).unwrap_or_default().len();
                if contract_size > 1_000_000 {
                    eprintln!("⚠️  Contrato recibido excede tamaño máximo ({} bytes): {}", contract_size, contract.address);
                    return Ok(None);
                }
                
                // Rate limiting: máximo 10 contratos por minuto por peer
                if let Some(ref peer) = source_peer {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let mut limits = rate_limits.lock().unwrap();
                    let (last_ts, count) = limits.entry(peer.clone()).or_insert((now, 0));
                    
                    if now - *last_ts < 60 {
                        if *count >= 10 {
                            eprintln!("⚠️  Rate limit excedido para peer {}: {} contratos en último minuto", peer, count);
                            return Ok(None);
                        }
                        *count += 1;
                    } else {
                        *last_ts = now;
                        *count = 1;
                    }
                }
                
                // Prevenir loops: verificar si recibimos este contrato recientemente del mismo peer
                if let Some(ref peer) = source_peer {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let mut receipts = recent_receipts.lock().unwrap();
                    
                    // Limpiar entradas antiguas (más de 5 minutos)
                    receipts.retain(|_, (ts, _)| now - *ts < 300);
                    
                    if let Some((ts, prev_peer)) = receipts.get(&contract.address) {
                        if *prev_peer == *peer && now - *ts < 60 {
                            println!("ℹ️  Contrato {} recibido recientemente del mismo peer {}, ignorando para prevenir loop", contract.address, peer);
                            return Ok(None);
                        }
                    }
                    
                    receipts.insert(contract.address.clone(), (now, peer.clone()));
                }
                
                if let Some(cm) = &contract_manager {
                    // Validar integridad del contrato
                    if !contract.validate_integrity() {
                        eprintln!("⚠️  Contrato recibido tiene hash de integridad inválido: {}", contract.address);
                        return Ok(None);
                    }
                    
                    let mut cm_guard = cm.write().unwrap();
                    
                    // Verificar si el contrato ya existe
                    if cm_guard.get_contract(&contract.address).is_none() {
                        // Contrato nuevo, agregarlo
                        if let Ok(_) = cm_guard.deploy_contract(contract.clone()) {
                            println!("✅ Nuevo contrato recibido y agregado: {} ({})", contract.name, contract.address);
                            
                            // Guardar en BD si está disponible
                            if let Some(db) = &db {
                                let db_guard = db.lock().unwrap();
                                if let Err(e) = db_guard.save_contract(&contract) {
                                    eprintln!("⚠️  Error guardando contrato en BD: {}", e);
                                }
                            }
                        } else {
                            println!("⚠️  Error al agregar contrato recibido");
                        }
                    } else {
                        println!("ℹ️  Contrato ya existe en nuestra red");
                    }
                }
                Ok(None)
            }
            
            Message::UpdateContract(contract) => {
                println!("📥 Mensaje UpdateContract recibido para contrato: {} ({})", contract.name, contract.address);
                
                // Validar tamaño del contrato (máximo 1MB)
                let contract_size = serde_json::to_string(&contract).unwrap_or_default().len();
                if contract_size > 1_000_000 {
                    eprintln!("⚠️  Actualización de contrato excede tamaño máximo ({} bytes): {}", contract_size, contract.address);
                    return Ok(None);
                }
                
                // Rate limiting: máximo 20 actualizaciones por minuto por peer
                if let Some(ref peer) = source_peer {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let mut limits = rate_limits.lock().unwrap();
                    let (last_ts, count) = limits.entry(peer.clone()).or_insert((now, 0));
                    
                    if now - *last_ts < 60 {
                        if *count >= 20 {
                            eprintln!("⚠️  Rate limit excedido para peer {}: {} actualizaciones en último minuto", peer, count);
                            return Ok(None);
                        }
                        *count += 1;
                    } else {
                        *last_ts = now;
                        *count = 1;
                    }
                }
                
                // Prevenir loops: verificar si recibimos esta actualización recientemente del mismo peer
                if let Some(ref peer) = source_peer {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    let mut receipts = recent_receipts.lock().unwrap();
                    
                    // Limpiar entradas antiguas (más de 5 minutos)
                    receipts.retain(|_, (ts, _)| now - *ts < 300);
                    
                    let receipt_key = format!("{}:{}:{}", contract.address, contract.updated_at, contract.update_sequence);
                    if let Some((ts, prev_peer)) = receipts.get(&receipt_key) {
                        if *prev_peer == *peer && now - *ts < 30 {
                            println!("ℹ️  Actualización de contrato {} recibida recientemente del mismo peer {}, ignorando para prevenir loop", contract.address, peer);
                            return Ok(None);
                        }
                    }
                    
                    receipts.insert(receipt_key, (now, peer.clone()));
                }
                
                if let Some(cm) = &contract_manager {
                    // Validar integridad del contrato
                    if !contract.validate_integrity() {
                        eprintln!("⚠️  Contrato recibido tiene hash de integridad inválido: {}", contract.address);
                        return Ok(None);
                    }
                    
                    let mut cm_guard = cm.write().unwrap();
                    
                    if let Some(existing_mut) = cm_guard.get_contract_mut(&contract.address) {
                        // Validar que el owner no haya cambiado ilegalmente
                        if contract.owner != existing_mut.owner {
                            eprintln!("⚠️  Intento de actualizar contrato con owner diferente rechazado: {}", contract.address);
                            return Ok(None);
                        }
                        
                        // Comparar por updated_at y update_sequence para resolver race conditions
                        let should_update = contract.updated_at > existing_mut.updated_at 
                            || (contract.updated_at == existing_mut.updated_at && contract.update_sequence > existing_mut.update_sequence);
                        
                        println!("🔍 Comparando actualización: nuestro updated_at={}, sequence={}, recibido updated_at={}, sequence={}, should_update={}", 
                            existing_mut.updated_at, existing_mut.update_sequence, 
                            contract.updated_at, contract.update_sequence, should_update);
                        
                        if should_update {
                            let old_balance = existing_mut.get_balance(&contract.state.balances.keys().next().unwrap_or(&String::new()));
                            *existing_mut = contract.clone();
                            let new_balance = existing_mut.get_balance(&contract.state.balances.keys().next().unwrap_or(&String::new()));
                            
                            println!("✅ Contrato actualizado desde peer: {} ({}), balance cambió de {} a {}", 
                                contract.name, contract.address, old_balance, new_balance);
                            
                            // Guardar en BD
                            if let Some(db) = &db {
                                let db_guard = db.lock().unwrap();
                                if let Err(e) = db_guard.save_contract(&contract) {
                                    eprintln!("⚠️  Error guardando contrato actualizado en BD: {}", e);
                                }
                            }
                        } else {
                            println!("ℹ️  Contrato recibido es más antiguo o igual, ignorando actualización");
                        }
                    } else {
                        // Contrato no existe, agregarlo como nuevo (validar integridad ya hecho arriba)
                        println!("ℹ️  Contrato no existe localmente, agregándolo como nuevo");
                        if let Ok(_) = cm_guard.deploy_contract(contract.clone()) {
                            println!("✅ Contrato recibido (no existía) y agregado: {} ({})", contract.name, contract.address);
                            
                            // Guardar en BD
                            if let Some(db) = &db {
                                let db_guard = db.lock().unwrap();
                                if let Err(e) = db_guard.save_contract(&contract) {
                                    eprintln!("⚠️  Error guardando contrato en BD: {}", e);
                                }
                            }
                        }
                    }
                } else {
                    println!("⚠️  ContractManager no disponible para procesar UpdateContract");
                }
                Ok(None)
            }
        }
    }

    /**
     * Verifica si una cadena es válida
     */
    fn is_valid_chain(chain: &[Block]) -> bool {
        for i in 1..chain.len() {
            if chain[i].previous_hash != chain[i - 1].hash {
                return false;
            }
            if !chain[i].is_valid() {
                return false;
            }
        }
        true
    }

    /**
     * Conecta a un peer
     */
    pub async fn connect_to_peer(&self, address: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(address).await?;
        
        let version_msg = {
            let blockchain = self.blockchain.lock().unwrap();
            let latest = blockchain.get_latest_block();
            let p2p_addr = format!("{}:{}", self.address.ip(), self.address.port());
            Message::Version {
                version: "1.0.0".to_string(),
                block_count: blockchain.chain.len(),
                latest_hash: latest.hash.clone(),
                p2p_address: Some(p2p_addr),
            }
        };

        let msg_json = serde_json::to_string(&version_msg)?;
        stream.write_all(msg_json.as_bytes()).await?;

        let mut buffer = [0; 4096];
        let n = stream.read(&mut buffer).await?;
        let response_str = String::from_utf8_lossy(&buffer[..n]);
        
        if let Ok(Message::Version { block_count: their_count, latest_hash: their_hash, p2p_address, .. }) = serde_json::from_str(&response_str) {
            // Si el peer envió su dirección P2P, usarla; si no, usar la dirección de conexión
            let peer_p2p_addr = p2p_address.unwrap_or_else(|| address.to_string());
            
            // Agregar el peer a nuestra lista ANTES de sincronizar
            {
                let mut peers = self.peers.lock().unwrap();
                peers.insert(peer_p2p_addr.clone());
                println!("📡 Peer agregado en connect_to_peer: {}", peer_p2p_addr);
            }
            
            let blockchain = self.blockchain.lock().unwrap();
            let my_count = blockchain.chain.len();
            let my_latest = blockchain.get_latest_block().hash.clone();
            drop(blockchain);
            
            // Sincronizar si el peer tiene más bloques
            if their_count > my_count {
                println!("📥 Sincronizando blockchain desde {} (ellos: {}, nosotros: {})", 
                    address, their_count, my_count);
                self.request_blocks(address).await?;
            } 
            // Si tienen el mismo número pero diferente hash
            else if their_count == my_count && their_hash != my_latest {
                if their_count == 1 {
                    // Ambos tienen solo el génesis pero diferentes - sincronizar para obtener el correcto
                    println!("⚠️  Diferentes bloques génesis detectados, sincronizando para obtener el correcto...");
                    self.request_blocks(address).await?;
                } else {
                    println!("⚠️  Fork detectado con {}: mismo número de bloques pero diferentes hashes", address);
                    println!("   Nuestro hash: {}...", &my_latest[..16]);
                    println!("   Su hash: {}...", &their_hash[..16]);
                    // En caso de fork, mantenemos nuestra cadena (regla de la cadena más larga)
                }
            }
            // Si tenemos más bloques, el peer debería sincronizar desde nosotros
            else if my_count > their_count {
                println!("ℹ️  Tenemos más bloques que {} (nosotros: {}, ellos: {})", address, my_count, their_count);
            }
            
            // Sincronizar contratos
            if let Some(_) = &self.contract_manager {
                println!("📋 Sincronizando contratos desde {}...", address);
                if let Err(e) = self.request_contracts(address).await {
                    eprintln!("⚠️  Error sincronizando contratos desde {}: {}", address, e);
                }
            }
        } else {
            // Si no recibimos Version válido, aún así agregar el peer
            let mut peers = self.peers.lock().unwrap();
            peers.insert(address.to_string());
        }

        println!("✅ Conectado a peer: {}", address);
        Ok(())
    }

    /**
     * Sincroniza con todos los peers conectados
     */
    pub async fn sync_with_all_peers(&self) -> Result<(), Box<dyn std::error::Error>> {
        let peers: Vec<String> = {
            let peers_guard = self.peers.lock().unwrap();
            peers_guard.iter().cloned().collect()
        };

        for peer_addr in peers.iter() {
            if let Err(e) = self.sync_with_peer(peer_addr).await {
                eprintln!("Error sincronizando con {}: {}", peer_addr, e);
            }
        }

        Ok(())
    }

    /**
     * Verifica si un peer está conectado enviando un ping
     */
    async fn ping_peer(&self, address: &str) -> bool {
        match TcpStream::connect(address).await {
            Ok(mut stream) => {
                let ping_msg = Message::Ping;
                if let Ok(msg_json) = serde_json::to_string(&ping_msg) {
                    if stream.write_all(msg_json.as_bytes()).await.is_ok() {
                        let mut buffer = [0; 256];
                        match tokio::time::timeout(
                            tokio::time::Duration::from_secs(5),
                            stream.read(&mut buffer)
                        ).await {
                            Ok(Ok(n)) if n > 0 => {
                                if let Ok(Message::Pong) = serde_json::from_slice(&buffer[..n]) {
                                    return true;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            Err(_) => {}
        }
        false
    }

    /**
     * Limpia peers desconectados verificando su conectividad
     */
    pub async fn cleanup_disconnected_peers(&self) {
        let peers_to_check: Vec<String> = {
            let peers_guard = self.peers.lock().unwrap();
            peers_guard.iter().cloned().collect()
        };

        let mut disconnected = Vec::new();

        for peer_addr in peers_to_check.iter() {
            if !self.ping_peer(peer_addr).await {
                println!("🔌 Peer desconectado detectado: {}", peer_addr);
                disconnected.push(peer_addr.clone());
            }
        }

        if !disconnected.is_empty() {
            let mut peers_guard = self.peers.lock().unwrap();
            for peer in disconnected {
                peers_guard.remove(&peer);
                println!("🗑️  Peer removido de la lista: {}", peer);
            }
        }
    }

    /**
     * Sincroniza con un peer específico
     */
    pub async fn sync_with_peer(&self, address: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Obtener información de nuestra blockchain antes de conectar
        let (my_count, my_latest) = {
            let blockchain = self.blockchain.lock().unwrap();
            let latest = blockchain.get_latest_block();
            (blockchain.chain.len(), latest.hash.clone())
        };
        
        let mut stream = TcpStream::connect(address).await?;
        
        // Enviar mensaje de versión para comparar
        let p2p_addr = format!("{}:{}", self.address.ip(), self.address.port());
        let version_msg = Message::Version {
            version: "1.0.0".to_string(),
            block_count: my_count,
            latest_hash: my_latest.clone(),
            p2p_address: Some(p2p_addr),
        };

        let msg_json = serde_json::to_string(&version_msg)?;
        stream.write_all(msg_json.as_bytes()).await?;

        let mut buffer = [0; 4096];
        let n = stream.read(&mut buffer).await?;
        let response_str = String::from_utf8_lossy(&buffer[..n]);
        
        if let Ok(Message::Version { block_count: their_count, latest_hash: their_hash, .. }) = serde_json::from_str(&response_str) {
            // Sincronizar si tienen más bloques
            if their_count > my_count {
                println!("📥 Sincronizando desde {} (ellos: {}, nosotros: {})", address, their_count, my_count);
                return self.request_blocks(address).await;
            }
            
            // Si tienen el mismo número pero diferente hash
            if their_count == my_count && their_hash != my_latest {
                if their_count == 1 {
                    // Ambos tienen solo el génesis pero diferentes - sincronizar para obtener el correcto
                    println!("⚠️  Diferentes bloques génesis detectados, sincronizando para obtener el correcto...");
                    return self.request_blocks(address).await;
                } else {
                    println!("⚠️  Fork detectado con {}: mismo número pero diferentes hashes", address);
                }
            }
            
            // Si tenemos más bloques, el peer debería sincronizar desde nosotros
            if my_count > their_count {
                println!("ℹ️  Tenemos más bloques que {} (nosotros: {}, ellos: {})", address, my_count, their_count);
            }
        }
        
        Ok(())
    }

    /**
     * Solicita bloques a un peer
     */
    pub async fn request_blocks(&self, address: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(address).await?;
        
        let get_blocks_msg = Message::GetBlocks;
        let msg_json = serde_json::to_string(&get_blocks_msg)?;
        stream.write_all(msg_json.as_bytes()).await?;

        let mut buffer = [0; 4096];
        let n = stream.read(&mut buffer).await?;
        let response_str = String::from_utf8_lossy(&buffer[..n]);
        
        if let Ok(Message::Blocks(blocks)) = serde_json::from_str(&response_str) {
            let mut blockchain = self.blockchain.lock().unwrap();
            
            // Si nuestra cadena está vacía o solo tiene génesis, aceptar la cadena recibida si es válida
            if blockchain.chain.is_empty() || (blockchain.chain.len() == 1 && blocks.len() >= 1) {
                if Self::is_valid_chain(&blocks) {
                    let should_replace = if let Some(wm) = &self.wallet_manager {
                        let wm_guard = wm.lock().unwrap();
                        // Si tenemos solo génesis, reemplazar completamente
                        if blockchain.chain.len() == 1 {
                            blockchain.chain = blocks.clone();
                            true
                        } else {
                            blockchain.resolve_conflict(&blocks, &wm_guard)
                        }
                    } else {
                        blockchain.chain = blocks.clone();
                        true
                    };
                    
                    if should_replace {
                        println!("✅ Blockchain sincronizada: {} bloques", blocks.len());
                        
                        // Sincronizar wallets desde la nueva blockchain
                        if let Some(wm) = &self.wallet_manager {
                            let mut wm_guard = wm.lock().unwrap();
                            wm_guard.sync_from_blockchain(&blockchain.chain);
                        }
                        
                        if let Some(db) = &self.db {
                            let db_guard = db.lock().unwrap();
                            if let Err(e) = db_guard.save_blockchain(&blockchain) {
                                eprintln!("⚠️  Error guardando blockchain sincronizada en BD: {}", e);
                            }
                        }
                    }
                }
                return Ok(());
            }
            
            // Resolver conflicto usando la regla de la cadena más larga
            if blocks.len() > blockchain.chain.len() && Self::is_valid_chain(&blocks) {
                let should_replace = if let Some(wm) = &self.wallet_manager {
                    let wm_guard = wm.lock().unwrap();
                    blockchain.resolve_conflict(&blocks, &wm_guard)
                } else {
                    blockchain.chain = blocks.clone();
                    true
                };
                
                if should_replace {
                    println!("✅ Blockchain sincronizada: {} bloques (reemplazada por cadena más larga)", blocks.len());
                    
                    // Sincronizar wallets desde la nueva blockchain
                    if let Some(wm) = &self.wallet_manager {
                        let mut wm_guard = wm.lock().unwrap();
                        wm_guard.sync_from_blockchain(&blockchain.chain);
                    }
                    
                    if let Some(db) = &self.db {
                        let db_guard = db.lock().unwrap();
                        if let Err(e) = db_guard.save_blockchain(&blockchain) {
                            eprintln!("⚠️  Error guardando blockchain sincronizada en BD: {}", e);
                        }
                    }
                } else {
                    println!("⚠️  Cadena recibida no pasó validación de transacciones");
                }
            } else if blocks.len() == blockchain.chain.len() {
                // Misma longitud: verificar si hay fork
                let my_latest = blockchain.get_latest_block().hash.clone();
                let their_latest = blocks.last().map(|b| b.hash.clone()).unwrap_or_default();
                
                if my_latest != their_latest {
                    println!("⚠️  Fork detectado durante sincronización: misma longitud pero diferentes últimos bloques");
                }
            } else if blocks.len() < blockchain.chain.len() {
                println!("ℹ️  Cadena recibida es más corta que la nuestra (ellos: {}, nosotros: {})", 
                    blocks.len(), blockchain.chain.len());
            }
        }

        Ok(())
    }

    /**
     * Envía un nuevo bloque a todos los peers
     */
    pub async fn broadcast_block(&self, block: &Block) {
        let peers: Vec<String> = {
            let peers_guard = self.peers.lock().unwrap();
            peers_guard.iter().cloned().collect()
        };

        for peer_addr in peers.iter() {
            if let Err(e) = self.send_block_to_peer(peer_addr, block).await {
                eprintln!("Error enviando bloque a {}: {} (el peer puede necesitar sincronización)", peer_addr, e);
            }
        }
    }

    /**
     * Envía un bloque a un peer específico
     */
    async fn send_block_to_peer(
        &self,
        address: &str,
        block: &Block,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(address).await?;
        let msg = Message::NewBlock(block.clone());
        let msg_json = serde_json::to_string(&msg)?;
        stream.write_all(msg_json.as_bytes()).await?;
        
        // Esperar un poco para que el peer procese el mensaje
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        Ok(())
    }

    /**
     * Envía una transacción a todos los peers
     */
    pub async fn broadcast_transaction(&self, tx: &Transaction) {
        let peers: Vec<String> = {
            let peers_guard = self.peers.lock().unwrap();
            peers_guard.iter().cloned().collect()
        };

        for peer_addr in peers.iter() {
            if let Err(e) = self.send_transaction_to_peer(peer_addr, tx).await {
                eprintln!("Error enviando transacción a {}: {}", peer_addr, e);
            }
        }
    }

    /**
     * Envía una transacción a un peer específico
     */
    async fn send_transaction_to_peer(
        &self,
        address: &str,
        tx: &Transaction,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect(address).await?;
        let msg = Message::NewTransaction(tx.clone());
        let msg_json = serde_json::to_string(&msg)?;
        stream.write_all(msg_json.as_bytes()).await?;
        Ok(())
    }

    /**
     * Solicita contratos a un peer (sincronización completa)
     */
    pub async fn request_contracts(&self, address: &str) -> Result<(), Box<dyn std::error::Error>> {
        let start_time = std::time::Instant::now();
        let mut stream = TcpStream::connect(address).await?;
        
        // Intentar sincronización incremental primero
        let last_sync = {
            let metrics = self.contract_sync_metrics.lock().unwrap();
            metrics.get(address)
                .map(|m| m.last_sync_timestamp)
                .unwrap_or(0)
        };
        
        let get_contracts_msg = if last_sync > 0 {
            Message::GetContractsSince { timestamp: last_sync }
        } else {
            Message::GetContracts
        };
        
        let msg_json = serde_json::to_string(&get_contracts_msg)?;
        stream.write_all(msg_json.as_bytes()).await?;

        let mut buffer = [0; 8192]; // Buffer más grande para contratos
        let n = stream.read(&mut buffer).await?;
        let response_str = String::from_utf8_lossy(&buffer[..n]);
        
        if let Ok(Message::Contracts(contracts)) = serde_json::from_str(&response_str) {
            if let Some(cm) = &self.contract_manager {
                let mut cm_guard = cm.write().unwrap();
                let mut synced = 0;
                let mut errors = 0;
                
                for contract in contracts {
                    // Validar integridad
                    if !contract.validate_integrity() {
                        eprintln!("⚠️  Contrato recibido tiene hash de integridad inválido: {}", contract.address);
                        errors += 1;
                        continue;
                    }
                    
                    // Verificar si el contrato ya existe
                    if cm_guard.get_contract(&contract.address).is_none() {
                        // Contrato nuevo, agregarlo
                        if let Ok(_) = cm_guard.deploy_contract(contract.clone()) {
                            synced += 1;
                            
                            // Guardar en BD si está disponible
                            if let Some(db) = &self.db {
                                let db_guard = db.lock().unwrap();
                                if let Err(e) = db_guard.save_contract(&contract) {
                                    eprintln!("⚠️  Error guardando contrato sincronizado en BD: {}", e);
                                }
                            }
                        }
                    } else {
                        // Contrato existe, verificar si necesita actualización
                        if let Some(existing) = cm_guard.get_contract(&contract.address) {
                            // Validar owner
                            if contract.owner != existing.owner {
                                eprintln!("⚠️  Intento de actualizar contrato con owner diferente rechazado: {}", contract.address);
                                errors += 1;
                                continue;
                            }
                            
                            // Comparar por updated_at y update_sequence
                            let should_update = contract.updated_at > existing.updated_at 
                                || (contract.updated_at == existing.updated_at && contract.update_sequence > existing.update_sequence);
                            
                            if should_update {
                                // Actualizar el contrato
                                if let Some(existing_mut) = cm_guard.get_contract_mut(&contract.address) {
                                    *existing_mut = contract.clone();
                                    synced += 1;
                                    
                                    // Guardar en BD
                                    if let Some(db) = &self.db {
                                        let db_guard = db.lock().unwrap();
                                        if let Err(e) = db_guard.save_contract(&contract) {
                                            eprintln!("⚠️  Error guardando contrato actualizado en BD: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Actualizar métricas
                let duration_ms = start_time.elapsed().as_millis() as u64;
                let timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                
                {
                    let mut metrics = self.contract_sync_metrics.lock().unwrap();
                    metrics.insert(address.to_string(), ContractSyncMetrics {
                        last_sync_timestamp: timestamp,
                        contracts_synced: synced,
                        sync_errors: errors,
                        last_sync_duration_ms: duration_ms,
                    });
                }
                
                if synced > 0 {
                    println!("✅ {} contratos sincronizados desde {} ({}ms, {} errores)", synced, address, duration_ms, errors);
                } else if errors > 0 {
                    println!("⚠️  {} contratos rechazados desde {} por validación", errors, address);
                } else {
                    println!("ℹ️  No hay contratos nuevos para sincronizar desde {}", address);
                }
            }
        } else {
            println!("⚠️  Respuesta inválida al solicitar contratos desde {}", address);
        }

        Ok(())
    }

    /**
     * Envía un nuevo contrato a todos los peers con reintentos
     */
    pub async fn broadcast_contract(&self, contract: &SmartContract) {
        let peers: Vec<String> = {
            let peers_guard = self.peers.lock().unwrap();
            peers_guard.iter().cloned().collect()
        };

        for peer_addr in peers.iter() {
            let mut retries = 3;
            let mut success = false;
            
            while retries > 0 && !success {
                let result = self.send_contract_to_peer(peer_addr, contract).await;
                match result {
                    Ok(_) => {
                        success = true;
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        retries -= 1;
                        if retries > 0 {
                            let delay_ms = 100 * (4 - retries); // Backoff exponencial: 100ms, 200ms, 300ms
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        } else {
                            eprintln!("Error enviando contrato a {} después de 3 intentos: {}", peer_addr, error_msg);
                            // Agregar a cola de pendientes (memoria)
                            let mut pending = self.pending_contract_broadcasts.lock().unwrap();
                            pending.push((peer_addr.clone(), contract.clone()));
                            
                            // Persistir a disco si hay DB disponible
                            if let Some(db) = &self.db {
                                let db_guard = db.lock().unwrap();
                                if let Err(e) = db_guard.save_pending_broadcast(peer_addr, contract) {
                                    eprintln!("⚠️  Error guardando broadcast pendiente en BD: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /**
     * Envía un contrato a un peer específico
     */
    async fn send_contract_to_peer(
        &self,
        address: &str,
        contract: &SmartContract,
    ) -> Result<(), String> {
        let mut stream = TcpStream::connect(address).await
            .map_err(|e| format!("Error conectando: {}", e))?;
        let msg = Message::NewContract(contract.clone());
        let msg_json = serde_json::to_string(&msg)
            .map_err(|e| format!("Error serializando: {}", e))?;
        stream.write_all(msg_json.as_bytes()).await
            .map_err(|e| format!("Error escribiendo: {}", e))?;
        
        // Esperar un poco para que el peer procese el mensaje (similar a bloques)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        Ok(())
    }

    /**
     * Envía una actualización de contrato a todos los peers con reintentos
     */
    pub async fn broadcast_contract_update(&self, contract: &SmartContract) {
        let peers: Vec<String> = {
            let peers_guard = self.peers.lock().unwrap();
            peers_guard.iter().cloned().collect()
        };

        if peers.is_empty() {
            println!("⚠️  No hay peers conectados para broadcast de actualización de contrato: {}", contract.address);
            return;
        }

        println!("📤 Broadcast de actualización de contrato {} a {} peers: {:?}", contract.address, peers.len(), peers);

        for peer_addr in peers.iter() {
            println!("📤 Enviando actualización de contrato {} a peer: {}", contract.address, peer_addr);
            let mut retries = 3;
            let mut success = false;
            
            while retries > 0 && !success {
                let result = self.send_contract_update_to_peer(peer_addr, contract).await;
                match result {
                    Ok(_) => {
                        println!("✅ Actualización de contrato {} enviada exitosamente a {}", contract.address, peer_addr);
                        success = true;
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        retries -= 1;
                        if retries > 0 {
                            let delay_ms = 100 * (4 - retries); // Backoff exponencial: 100ms, 200ms, 300ms
                            println!("⚠️  Error enviando a {} (intento {}): {}, reintentando en {}ms", peer_addr, 4 - retries, error_msg, delay_ms);
                            tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                        } else {
                            eprintln!("❌ Error enviando actualización de contrato a {} después de 3 intentos: {}", peer_addr, error_msg);
                            // Agregar a cola de pendientes (memoria)
                            let mut pending = self.pending_contract_broadcasts.lock().unwrap();
                            pending.push((peer_addr.clone(), contract.clone()));
                            
                            // Persistir a disco si hay DB disponible
                            if let Some(db) = &self.db {
                                let db_guard = db.lock().unwrap();
                                if let Err(e) = db_guard.save_pending_broadcast(peer_addr, contract) {
                                    eprintln!("⚠️  Error guardando broadcast pendiente en BD: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }
        
        println!("✅ Broadcast de actualización de contrato {} completado", contract.address);
    }

    /**
     * Envía una actualización de contrato a un peer específico
     */
    async fn send_contract_update_to_peer(
        &self,
        address: &str,
        contract: &SmartContract,
    ) -> Result<(), String> {
        println!("📤 Conectando a {} para enviar UpdateContract de {}", address, contract.address);
        let mut stream = TcpStream::connect(address).await
            .map_err(|e| format!("Error conectando a {}: {}", address, e))?;
        let msg = Message::UpdateContract(contract.clone());
        let msg_json = serde_json::to_string(&msg)
            .map_err(|e| format!("Error serializando UpdateContract: {}", e))?;
        
        println!("📤 Enviando UpdateContract de {} a {} (tamaño: {} bytes)", contract.address, address, msg_json.len());
        stream.write_all(msg_json.as_bytes()).await
            .map_err(|e| format!("Error escribiendo UpdateContract a {}: {}", address, e))?;
        
        // Esperar un poco para que el peer procese el mensaje (similar a bloques)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        Ok(())
    }
}

