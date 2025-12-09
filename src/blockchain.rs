use crate::models::{Transaction, WalletManager};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/**
 * Representa un bloque en la blockchain con transacciones
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub index: u64,
    pub timestamp: u64,
    pub transactions: Vec<Transaction>,
    pub previous_hash: String,
    pub hash: String,
    pub nonce: u64,
    pub difficulty: u8,
    pub merkle_root: String,
}

impl Block {
    /**
     * Crea un nuevo bloque con transacciones
     */
    pub fn new(
        index: u64,
        transactions: Vec<Transaction>,
        previous_hash: String,
        difficulty: u8,
    ) -> Block {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let merkle_root = Self::calculate_merkle_root(&transactions);

        Block {
            index,
            timestamp,
            transactions,
            previous_hash,
            hash: String::new(),
            nonce: 0,
            difficulty,
            merkle_root,
        }
    }

    /**
     * Calcula el Merkle root de las transacciones
     */
    fn calculate_merkle_root(transactions: &[Transaction]) -> String {
        if transactions.is_empty() {
            return String::new();
        }

        let mut hashes: Vec<String> = transactions
            .iter()
            .map(|tx| tx.calculate_hash())
            .collect();

        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            for i in (0..hashes.len()).step_by(2) {
                if i + 1 < hashes.len() {
                    let combined = format!("{}{}", hashes[i], hashes[i + 1]);
                    let mut hasher = Sha256::new();
                    hasher.update(combined.as_bytes());
                    next_level.push(format!("{:x}", hasher.finalize()));
                } else {
                    next_level.push(hashes[i].clone());
                }
            }
            hashes = next_level;
        }

        hashes.first().cloned().unwrap_or_default()
    }

    /**
     * Calcula el hash del bloque
     */
    pub fn calculate_hash(&self) -> String {
        let data = format!(
            "{}{}{}{}{}{}",
            self.index,
            self.timestamp,
            self.merkle_root,
            self.previous_hash,
            self.nonce,
            self.transactions.len()
        );
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /**
     * Realiza el Proof of Work minando el bloque
     */
    pub fn mine(&mut self) -> String {
        let target = "0".repeat(self.difficulty as usize);

        loop {
            self.hash = self.calculate_hash();
            if self.hash.starts_with(&target) {
                break;
            }
            self.nonce += 1;
        }

        self.hash.clone()
    }

    /**
     * Verifica que el hash del bloque sea válido según la dificultad
     */
    pub fn is_valid(&self) -> bool {
        let target = "0".repeat(self.difficulty as usize);
        let calculated_merkle = Self::calculate_merkle_root(&self.transactions);
        
        self.hash == self.calculate_hash()
            && self.hash.starts_with(&target)
            && self.merkle_root == calculated_merkle
    }
}

/**
 * Representa la blockchain completa con gestión de wallets
 */
#[derive(Debug, Clone)]
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub difficulty: u8,
    pub target_block_time: u64,
    pub difficulty_adjustment_interval: u64,
    pub max_transactions_per_block: usize,
    pub max_block_size_bytes: usize,
}

impl Blockchain {
    /**
     * Crea una nueva blockchain con el bloque génesis
     */
    pub fn new(difficulty: u8) -> Blockchain {
        let mut blockchain = Blockchain {
            chain: Vec::new(),
            difficulty,
            target_block_time: 60,
            difficulty_adjustment_interval: 10,
            max_transactions_per_block: 1000,
            max_block_size_bytes: 1_000_000,
        };
        blockchain.create_genesis_block();
        blockchain
    }

    /**
     * Ajusta la dificultad dinámicamente basado en el tiempo de los últimos bloques
     * @returns true si la dificultad fue ajustada
     */
    pub fn adjust_difficulty(&mut self) -> bool {
        if self.chain.len() < 2 {
            return false;
        }

        let adjustment_interval = self.difficulty_adjustment_interval as usize;
        if self.chain.len() % adjustment_interval != 0 {
            return false;
        }

        let recent_blocks = &self.chain[self.chain.len().saturating_sub(adjustment_interval)..];
        if recent_blocks.len() < 2 {
            return false;
        }

        let time_span = recent_blocks[recent_blocks.len() - 1].timestamp
            .saturating_sub(recent_blocks[0].timestamp);
        let expected_time = self.target_block_time * adjustment_interval as u64;
        
        let ratio = if time_span > 0 {
            expected_time as f64 / time_span as f64
        } else {
            1.0
        };

        let old_difficulty = self.difficulty;
        
        if ratio < 0.8 {
            self.difficulty = (self.difficulty as u16 + 1).min(255) as u8;
        } else if ratio > 1.2 {
            self.difficulty = (self.difficulty as u16).saturating_sub(1) as u8;
        }

        if self.difficulty < 1 {
            self.difficulty = 1;
        }
        if self.difficulty > 20 {
            self.difficulty = 20;
        }

        if old_difficulty != self.difficulty {
            println!("📊 Dificultad ajustada: {} -> {} (ratio: {:.2})", old_difficulty, self.difficulty, ratio);
            true
        } else {
            false
        }
    }

    /**
     * Crea el bloque génesis (fijo y compartido por todos los nodos)
     */
    pub fn create_genesis_block(&mut self) {
        if !self.chain.is_empty() {
            return;
        }

        let genesis_tx = Transaction::new_with_fee(
            "0".to_string(),
            "genesis".to_string(),
            0,
            0,
            Some("Genesis Block - Rust Blockchain".to_string()),
        );

        let timestamp = 1700000000u64;
        let previous_hash = "0".to_string();
        let merkle_root = Self::calculate_merkle_root_static(&vec![genesis_tx.clone()]);
        
        let data = format!(
            "{}{}{}{}{}{}",
            0u64,
            timestamp,
            merkle_root,
            previous_hash,
            0u64,
            1usize
        );
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let hash = format!("{:x}", hasher.finalize());
        
        let target = "0".repeat(self.difficulty as usize);
        let mut nonce = 0u64;
        let mut final_hash = hash.clone();
        
        while !final_hash.starts_with(&target) {
            let data = format!(
                "{}{}{}{}{}{}",
                0u64,
                timestamp,
                merkle_root,
                previous_hash,
                nonce,
                1usize
            );
            let mut hasher = Sha256::new();
            hasher.update(data.as_bytes());
            final_hash = format!("{:x}", hasher.finalize());
            nonce += 1;
            
            if nonce > 10000000 {
                nonce = 12345;
                let data = format!(
                    "{}{}{}{}{}{}",
                    0u64, timestamp, merkle_root, previous_hash, nonce, 1usize
                );
                let mut hasher = Sha256::new();
                hasher.update(data.as_bytes());
                final_hash = format!("{:x}", hasher.finalize());
                break;
            }
        }

        let genesis = Block {
            index: 0,
            timestamp,
            transactions: vec![genesis_tx],
            previous_hash,
            hash: final_hash,
            nonce,
            difficulty: self.difficulty,
            merkle_root,
        };
        
        self.chain.push(genesis);
    }

    /**
     * Calcula el Merkle root (método estático para uso en génesis)
     */
    fn calculate_merkle_root_static(transactions: &[Transaction]) -> String {
        if transactions.is_empty() {
            return String::new();
        }

        let mut hashes: Vec<String> = transactions
            .iter()
            .map(|tx| tx.calculate_hash())
            .collect();

        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            for i in (0..hashes.len()).step_by(2) {
                if i + 1 < hashes.len() {
                    let combined = format!("{}{}", hashes[i], hashes[i + 1]);
                    let mut hasher = Sha256::new();
                    hasher.update(combined.as_bytes());
                    next_level.push(format!("{:x}", hasher.finalize()));
                } else {
                    next_level.push(hashes[i].clone());
                }
            }
            hashes = next_level;
        }

        hashes.first().cloned().unwrap_or_default()
    }

    /**
     * Obtiene el último bloque de la cadena
     */
    pub fn get_latest_block(&self) -> &Block {
        self.chain.last().unwrap()
    }

    /**
     * Calcula el tamaño aproximado de un bloque en bytes
     */
    fn calculate_block_size(transactions: &[Transaction]) -> usize {
        let base_size = 200;
        let tx_size: usize = transactions.iter().map(|tx| {
            tx.from.len() + tx.to.len() + tx.signature.len() + 50
        }).sum();
        base_size + tx_size
    }

    /**
     * Agrega un nuevo bloque a la cadena
     */
    pub fn add_block(
        &mut self,
        transactions: Vec<Transaction>,
        wallet_manager: &WalletManager,
    ) -> Result<String, String> {
        if transactions.is_empty() {
            return Err("Un bloque debe tener al menos una transacción".to_string());
        }

        let block_size = Self::calculate_block_size(&transactions);
        if block_size > self.max_block_size_bytes {
            return Err(format!("Bloque excede tamaño máximo: {} bytes", block_size));
        }

        if transactions.len() > self.max_transactions_per_block {
            return Err(format!("Bloque excede máximo de transacciones: {}", transactions.len()));
        }

        let mut coinbase_count = 0;
        for tx in &transactions {
            if tx.from == "0" {
                coinbase_count += 1;
                if let Err(e) = self.validate_coinbase_transaction(tx) {
                    return Err(format!("Transacción coinbase inválida: {}", e));
                }
            } else if tx.from == "STAKING" {
                // Transacciones de unstaking: permitidas sin validación adicional
                // (se validan en el contexto de staking)
            } else {
                if let Err(e) = self.validate_transaction(tx, wallet_manager) {
                    return Err(format!("Transacción inválida: {}", e));
                }
            }
        }

        if coinbase_count > 1 {
            return Err("Solo puede haber una transacción coinbase por bloque".to_string());
        }

        self.adjust_difficulty();

        let previous_hash = self.get_latest_block().hash.clone();
        let index = self.chain.len() as u64;
        let mut new_block = Block::new(index, transactions, previous_hash, self.difficulty);
        let hash = new_block.mine();
        self.chain.push(new_block);
        Ok(hash)
    }

    /**
     * Calcula el total de fees de un conjunto de transacciones
     */
    fn calculate_total_fees(transactions: &[Transaction]) -> u64 {
        transactions.iter().map(|tx| tx.fee).sum()
    }

    /**
     * Mina un nuevo bloque con recompensa automática para el minero
     * Los fees de las transacciones se suman a la recompensa del minero
     * @param miner_address - Dirección del minero que recibirá la recompensa
     * @param transactions - Transacciones a incluir en el bloque (opcional)
     * @param wallet_manager - Gestor de wallets para validación
     * @returns Hash del bloque minado
     */
    pub fn mine_block_with_reward(
        &mut self,
        miner_address: &str,
        transactions: Vec<Transaction>,
        wallet_manager: &WalletManager,
    ) -> Result<String, String> {
        let base_reward = self.calculate_mining_reward();
        let total_fees = Self::calculate_total_fees(&transactions);
        let total_reward = base_reward + total_fees;
        
        let coinbase = Self::create_coinbase_transaction(miner_address, Some(total_reward));
        
        let mut all_transactions = vec![coinbase];
        all_transactions.extend(transactions);
        
        self.add_block(all_transactions, wallet_manager)
    }

    /**
     * Verifica la validez de toda la cadena
     * También detecta bloques duplicados (forks)
     */
    pub fn is_chain_valid(&self) -> bool {
        if self.chain.is_empty() {
            return false;
        }

        if !self.chain[0].is_valid() {
            return false;
        }

        let mut seen_indices = std::collections::HashSet::new();
        seen_indices.insert(0);

        for i in 1..self.chain.len() {
            let current = &self.chain[i];
            let previous = &self.chain[i - 1];

            if !current.is_valid() {
                return false;
            }

            if current.previous_hash != previous.hash {
                return false;
            }

            if current.index != previous.index + 1 {
                return false;
            }

            if seen_indices.contains(&current.index) {
                return false;
            }
            seen_indices.insert(current.index);
        }
        true
    }

    /**
     * Resuelve conflictos usando la regla de la cadena más larga
     * Retorna true si se reemplazó la cadena actual
     */
    pub fn resolve_conflict(
        &mut self,
        other_chain: &[Block],
        wallet_manager: &WalletManager,
    ) -> bool {
        if other_chain.len() <= self.chain.len() {
            return false;
        }

        if !Self::is_valid_chain_static(other_chain) {
            return false;
        }

        for block in other_chain {
            for tx in &block.transactions {
                if tx.from != "0" {
                    if let Err(_) = self.validate_transaction(tx, wallet_manager) {
                        return false;
                    }
                }
            }
        }

        self.chain = other_chain.to_vec();
        true
    }

    /**
     * Verifica si una cadena es válida (método estático)
     */
    fn is_valid_chain_static(chain: &[Block]) -> bool {
        if chain.is_empty() {
            return false;
        }

        if !chain[0].is_valid() {
            return false;
        }

        for i in 1..chain.len() {
            let current = &chain[i];
            let previous = &chain[i - 1];

            if !current.is_valid() {
                return false;
            }

            if current.previous_hash != previous.hash {
                return false;
            }

            if current.index != previous.index + 1 {
                return false;
            }
        }

        true
    }

    /**
     * Encuentra el punto común más reciente entre dos cadenas
     * Retorna el índice del último bloque común
     */
    #[allow(dead_code)]
    pub fn find_common_ancestor(&self, other_chain: &[Block]) -> Option<usize> {
        let min_len = self.chain.len().min(other_chain.len());
        
        for i in (0..min_len).rev() {
            if self.chain[i].hash == other_chain[i].hash {
                return Some(i);
            }
        }
        
        None
    }

    /**
     * Obtiene un bloque por su hash
     */
    pub fn get_block_by_hash(&self, hash: &str) -> Option<&Block> {
        self.chain.iter().find(|b| b.hash == hash)
    }

    /**
     * Obtiene un bloque por índice
     */
    pub fn get_block_by_index(&self, index: u64) -> Option<&Block> {
        self.chain.get(index as usize)
    }

    /**
     * Obtiene todas las transacciones de un wallet
     */
    pub fn get_transactions_for_wallet(&self, address: &str) -> Vec<&Transaction> {
        self.chain
            .iter()
            .flat_map(|block| &block.transactions)
            .filter(|tx| tx.from == address || tx.to == address)
            .collect()
    }

    /**
     * Calcula el balance de un wallet basándose en todas las transacciones
     */
    pub fn calculate_balance(&self, address: &str) -> u64 {
        let mut balance = 0u64;
        
        for block in &self.chain {
            for tx in &block.transactions {
                if tx.from == "0" && tx.to == address {
                    balance += tx.amount;
                } else if tx.from == address {
                    balance = balance.saturating_sub(tx.amount + tx.fee);
                } else if tx.to == address {
                    balance += tx.amount;
                }
            }
        }
        
        balance
    }

    /**
     * Valida una transacción coinbase
     */
    pub fn validate_coinbase_transaction(&self, tx: &Transaction) -> Result<(), String> {
        if tx.from != "0" {
            return Err("No es una transacción coinbase".to_string());
        }

        if tx.to.is_empty() {
            return Err("Dirección destinataria de coinbase no puede estar vacía".to_string());
        }

        if tx.amount == 0 {
            return Err("Cantidad de coinbase debe ser mayor a 0".to_string());
        }

        if tx.amount > 1_000_000_000 {
            return Err("Cantidad de coinbase excede el límite máximo".to_string());
        }

        if !tx.signature.is_empty() {
            return Err("Transacciones coinbase no deben tener firma".to_string());
        }

        if tx.to.len() < 32 {
            return Err("Dirección destinataria de coinbase debe tener formato válido".to_string());
        }

        Ok(())
    }

    /**
     * Valida una transacción verificando su firma digital
     */
    pub fn validate_transaction(
        &self,
        tx: &Transaction,
        wallet_manager: &WalletManager,
    ) -> Result<(), String> {
        if !tx.is_valid() {
            return Err("Transacción inválida: campos básicos incorrectos".to_string());
        }

        // Transacciones desde "STAKING" son del sistema (unstaking) y no requieren firma
        if tx.from == "STAKING" {
            // Verificar que el balance de STAKING es suficiente
            // El balance de STAKING es la suma de todos los stakes activos
            // Por ahora, permitimos estas transacciones sin validar balance
            // (se validará en el contexto de staking)
            return Ok(());
        }

        // Transacciones coinbase (from == "0") tampoco requieren firma
        if tx.from == "0" {
            return Ok(());
        }

        let wallet = wallet_manager
            .get_wallet(&tx.from)
            .ok_or_else(|| "Wallet no encontrado".to_string())?;

        let public_key_bytes = wallet.get_public_key_bytes();
        if !tx.has_valid_signature(&public_key_bytes) {
            return Err("Firma digital inválida".to_string());
        }

        let balance = self.calculate_balance(&tx.from);
        let total_required = tx.amount + tx.fee;
        if balance < total_required {
            return Err("Saldo insuficiente (incluyendo fee)".to_string());
        }

        if self.is_double_spend(tx) {
            return Err("Doble gasto detectado".to_string());
        }

        Ok(())
    }

    /**
     * Verifica si una transacción es doble gasto
     */
    fn is_double_spend(&self, tx: &Transaction) -> bool {
        self.chain
            .iter()
            .flat_map(|block| &block.transactions)
            .any(|existing_tx| {
                existing_tx.from == tx.from
                    && existing_tx.id != tx.id
                    && existing_tx.amount == tx.amount
                    && existing_tx.timestamp == tx.timestamp
            })
    }

    /**
     * Crea una transacción coinbase para recompensar al minero
     */
    pub fn create_coinbase_transaction(miner_address: &str, reward: Option<u64>) -> Transaction {
        let base_reward = reward.unwrap_or(50);
        Transaction::new_with_fee(
            "0".to_string(),
            miner_address.to_string(),
            base_reward,
            0,
            Some("Coinbase - Mining Reward".to_string()),
        )
    }

    /**
     * Calcula la recompensa de minería para el bloque actual
     */
    pub fn calculate_mining_reward(&self) -> u64 {
        let base_reward = 50u64;
        let halving_interval = 210000u64;
        
        let halvings = self.chain.len() as u64 / halving_interval;
        base_reward >> halvings.min(64)
    }
}
