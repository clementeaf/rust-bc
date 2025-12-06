use crate::blockchain::{Block, Blockchain};
use crate::database::BlockchainDB;
use crate::models::{Transaction, WalletManager};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
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
    },
}

/**
 * Nodo en la red P2P
 */
#[derive(Clone)]
pub struct Node {
    #[allow(dead_code)]
    pub address: SocketAddr,
    pub peers: Arc<Mutex<HashSet<String>>>,
    pub blockchain: Arc<Mutex<Blockchain>>,
    pub wallet_manager: Option<Arc<Mutex<WalletManager>>>,
    pub db: Option<Arc<Mutex<BlockchainDB>>>,
    pub listening: bool,
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
            listening: false,
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
     * Inicia el servidor P2P
     */
    pub async fn start_server(&mut self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        let addr = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(&addr).await?;
        self.listening = true;

        println!("🌐 Servidor P2P iniciado en {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    println!("📡 Nueva conexión desde: {}", peer_addr);
                    let peers = self.peers.clone();
                    let blockchain = self.blockchain.clone();
                    let wallet_manager = self.wallet_manager.clone();
                    let db = self.db.clone();

                    tokio::spawn(async move {
                        if let Err(e) = Self::handle_connection(stream, peers, blockchain, wallet_manager, db).await {
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
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut buffer = [0; 4096];

        loop {
            let n = stream.read(&mut buffer).await?;
            if n == 0 {
                break;
            }

            let message_str = String::from_utf8_lossy(&buffer[..n]);
            if let Ok(message) = serde_json::from_str::<Message>(&message_str) {
                let response = Self::process_message(message, &peers, &blockchain, wallet_manager.clone(), db.clone()).await?;
                
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
            
            Message::Version { block_count: their_count, latest_hash: their_hash, .. } => {
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
                }))
            }
            
            Message::Pong => Ok(None),
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
            Message::Version {
                version: "1.0.0".to_string(),
                block_count: blockchain.chain.len(),
                latest_hash: latest.hash.clone(),
            }
        };

        let msg_json = serde_json::to_string(&version_msg)?;
        stream.write_all(msg_json.as_bytes()).await?;

        let mut buffer = [0; 4096];
        let n = stream.read(&mut buffer).await?;
        let response_str = String::from_utf8_lossy(&buffer[..n]);
        
        if let Ok(Message::Version { block_count: their_count, latest_hash: their_hash, .. }) = serde_json::from_str(&response_str) {
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
        }

        {
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
        let version_msg = Message::Version {
            version: "1.0.0".to_string(),
            block_count: my_count,
            latest_hash: my_latest.clone(),
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
}

