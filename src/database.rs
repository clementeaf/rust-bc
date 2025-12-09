use crate::airdrop::NodeTracking;
use crate::blockchain::{Block, Blockchain};
use crate::models::Transaction;
use crate::smart_contracts::SmartContract;
use crate::staking::Validator;
use rusqlite::{params, Connection, Result as SqlResult};
use serde_json;

/**
 * Gestor de persistencia para la blockchain
 */
pub struct BlockchainDB {
    pub(crate) conn: Connection,
}

impl BlockchainDB {
    /**
     * Crea una nueva conexión a la base de datos con optimizaciones
     * @param db_path - Ruta al archivo de base de datos
     * @returns BlockchainDB configurado con WAL mode e índices
     */
    pub fn new(db_path: &str) -> SqlResult<BlockchainDB> {
        #[allow(unused_mut)]
        let mut conn = Connection::open(db_path)?;
        
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA cache_size=-64000;
             PRAGMA temp_store=MEMORY;"
        )?;
        
        let db = BlockchainDB { conn };
        db.init_tables()?;
        db.create_indexes()?;
        Ok(db)
    }

    /**
     * Inicializa las tablas de la base de datos
     */
    fn init_tables(&self) -> SqlResult<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS blocks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                index_num INTEGER NOT NULL,
                timestamp INTEGER NOT NULL,
                previous_hash TEXT NOT NULL,
                hash TEXT NOT NULL UNIQUE,
                nonce INTEGER NOT NULL,
                difficulty INTEGER NOT NULL,
                merkle_root TEXT NOT NULL,
                transactions TEXT NOT NULL
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS wallets (
                address TEXT PRIMARY KEY,
                balance INTEGER NOT NULL,
                public_key TEXT
            )",
            [],
        )?;

        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS contracts (
                address TEXT PRIMARY KEY,
                owner TEXT NOT NULL,
                contract_type TEXT NOT NULL,
                name TEXT NOT NULL,
                symbol TEXT,
                total_supply INTEGER,
                decimals INTEGER,
                state TEXT NOT NULL,
                bytecode TEXT,
                abi TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                update_sequence INTEGER NOT NULL DEFAULT 0,
                integrity_hash TEXT
            )",
            [],
        )?;
        
        // Tabla para contratos pendientes de broadcast
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS pending_contract_broadcasts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                peer_address TEXT NOT NULL,
                contract_address TEXT NOT NULL,
                contract_data TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                retry_count INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;

        // Tabla para tracking de nodos (Airdrop)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS node_tracking (
                node_address TEXT PRIMARY KEY,
                first_block_index INTEGER NOT NULL,
                first_block_timestamp INTEGER NOT NULL,
                blocks_validated INTEGER NOT NULL DEFAULT 0,
                last_block_timestamp INTEGER NOT NULL,
                is_eligible INTEGER NOT NULL DEFAULT 0,
                airdrop_claimed INTEGER NOT NULL DEFAULT 0,
                claim_timestamp INTEGER,
                claim_transaction_id TEXT,
                claim_block_index INTEGER,
                claim_verified INTEGER NOT NULL DEFAULT 0,
                uptime_seconds INTEGER NOT NULL DEFAULT 0,
                eligibility_tier INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;

        // Tabla para claims de airdrop
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS airdrop_claims (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                node_address TEXT NOT NULL,
                claim_timestamp INTEGER NOT NULL,
                airdrop_amount INTEGER NOT NULL,
                transaction_id TEXT NOT NULL,
                transaction_hash TEXT,
                block_index INTEGER,
                tier_id INTEGER NOT NULL,
                verified INTEGER NOT NULL DEFAULT 0,
                verification_timestamp INTEGER,
                retry_count INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;

        // Tabla para validadores (PoS)
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS validators (
                address TEXT PRIMARY KEY,
                staked_amount INTEGER NOT NULL,
                is_active INTEGER NOT NULL,
                total_rewards INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                last_validated_block INTEGER NOT NULL,
                validation_count INTEGER NOT NULL,
                slash_count INTEGER NOT NULL,
                unstaking_requested INTEGER NOT NULL,
                unstaking_timestamp INTEGER
            )",
            [],
        )?;

        // Migración: agregar nuevos campos si no existen (para bases de datos existentes)
        let _ = self.conn.execute(
            "ALTER TABLE contracts ADD COLUMN update_sequence INTEGER NOT NULL DEFAULT 0",
            [],
        );
        let _ = self.conn.execute(
            "ALTER TABLE contracts ADD COLUMN integrity_hash TEXT",
            [],
        );

        Ok(())
    }

    /**
     * Crea índices para optimizar consultas frecuentes
     */
    fn create_indexes(&self) -> SqlResult<()> {
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_blocks_hash ON blocks(hash)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_blocks_index ON blocks(index_num)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_blocks_timestamp ON blocks(timestamp)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_contracts_owner ON contracts(owner)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_node_tracking_address ON node_tracking(node_address)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_node_tracking_eligible ON node_tracking(is_eligible)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_airdrop_claims_address ON airdrop_claims(node_address)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_contracts_type ON contracts(contract_type)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_validators_active ON validators(is_active)",
            [],
        )?;
        Ok(())
    }

    /**
     * Guarda un bloque en la base de datos
     */
    pub fn save_block(&self, block: &Block) -> SqlResult<()> {
        let transactions_json = serde_json::to_string(&block.transactions)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

        self.conn.execute(
            "INSERT OR REPLACE INTO blocks 
             (index_num, timestamp, previous_hash, hash, nonce, difficulty, merkle_root, transactions)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                block.index,
                block.timestamp,
                block.previous_hash,
                block.hash,
                block.nonce,
                block.difficulty,
                block.merkle_root,
                transactions_json
            ],
        )?;
        Ok(())
    }

    /**
     * Carga todos los bloques de la base de datos
     */
    pub fn load_blocks(&self) -> SqlResult<Vec<Block>> {
        let mut stmt = self.conn.prepare(
            "SELECT index_num, timestamp, previous_hash, hash, nonce, difficulty, merkle_root, transactions
             FROM blocks ORDER BY index_num"
        )?;

        let block_iter = stmt.query_map([], |row| {
            let transactions_json: String = row.get(7)?;
            let transactions: Vec<Transaction> = serde_json::from_str(&transactions_json)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

            Ok(Block {
                index: row.get(0)?,
                timestamp: row.get(1)?,
                transactions,
                previous_hash: row.get(2)?,
                hash: row.get(3)?,
                nonce: row.get(4)?,
                difficulty: row.get(5)?,
                merkle_root: row.get(6)?,
            })
        })?;

        let mut blocks = Vec::new();
        for block in block_iter {
            blocks.push(block?);
        }
        Ok(blocks)
    }

    /**
     * Guarda la blockchain completa
     */
    pub fn save_blockchain(&self, blockchain: &Blockchain) -> SqlResult<()> {
        for block in &blockchain.chain {
            self.save_block(block)?;
        }
        Ok(())
    }

    /**
     * Carga la blockchain desde la base de datos
     */
    pub fn load_blockchain(&self, difficulty: u8) -> SqlResult<Blockchain> {
        let blocks = self.load_blocks()?;
        
        if blocks.is_empty() {
            return Ok(Blockchain::new(difficulty));
        }

        Ok(Blockchain {
            chain: blocks,
            difficulty,
            target_block_time: 60,
            difficulty_adjustment_interval: 10,
            max_transactions_per_block: 1000,
            max_block_size_bytes: 1_000_000,
        })
    }

    /**
     * Obtiene un bloque por hash
     */
    #[allow(dead_code)]
    pub fn get_block_by_hash(&self, hash: &str) -> SqlResult<Option<Block>> {
        let mut stmt = self.conn.prepare(
            "SELECT index_num, timestamp, previous_hash, hash, nonce, difficulty, merkle_root, transactions
             FROM blocks WHERE hash = ?1"
        )?;

        let mut rows = stmt.query_map(params![hash], |row| {
            let transactions_json: String = row.get(7)?;
            let transactions: Vec<Transaction> = serde_json::from_str(&transactions_json)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

            Ok(Block {
                index: row.get(0)?,
                timestamp: row.get(1)?,
                transactions,
                previous_hash: row.get(2)?,
                hash: row.get(3)?,
                nonce: row.get(4)?,
                difficulty: row.get(5)?,
                merkle_root: row.get(6)?,
            })
        })?;

        match rows.next() {
            Some(Ok(block)) => Ok(Some(block)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    /**
     * Obtiene el último bloque
     */
    #[allow(dead_code)]
    pub fn get_latest_block(&self) -> SqlResult<Option<Block>> {
        let mut stmt = self.conn.prepare(
            "SELECT index_num, timestamp, previous_hash, hash, nonce, difficulty, merkle_root, transactions
             FROM blocks ORDER BY index_num DESC LIMIT 1"
        )?;

        let mut rows = stmt.query_map([], |row| {
            let transactions_json: String = row.get(7)?;
            let transactions: Vec<Transaction> = serde_json::from_str(&transactions_json)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

            Ok(Block {
                index: row.get(0)?,
                timestamp: row.get(1)?,
                transactions,
                previous_hash: row.get(2)?,
                hash: row.get(3)?,
                nonce: row.get(4)?,
                difficulty: row.get(5)?,
                merkle_root: row.get(6)?,
            })
        })?;

        match rows.next() {
            Some(Ok(block)) => Ok(Some(block)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    /**
     * Obtiene el número total de bloques
     */
    #[allow(dead_code)]
    pub fn get_block_count(&self) -> SqlResult<u64> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM blocks",
            [],
            |row| row.get(0),
        )?;
        Ok(count as u64)
    }

    /**
     * Guarda un smart contract en la base de datos
     */
    pub fn save_contract(&self, contract: &SmartContract) -> SqlResult<()> {
        let state_json = serde_json::to_string(&contract.state)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        
        let bytecode_json = match &contract.bytecode {
            Some(bc) => serde_json::to_string(bc)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?,
            None => String::new(),
        };

        self.conn.execute(
            "INSERT OR REPLACE INTO contracts 
             (address, owner, contract_type, name, symbol, total_supply, decimals, state, bytecode, abi, created_at, updated_at, update_sequence, integrity_hash)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                contract.address,
                contract.owner,
                contract.contract_type,
                contract.name,
                contract.symbol,
                contract.total_supply,
                contract.decimals,
                state_json,
                bytecode_json,
                contract.abi,
                contract.created_at,
                contract.updated_at,
                contract.update_sequence,
                contract.integrity_hash
            ],
        )?;
        Ok(())
    }

    /**
     * Carga todos los contratos de la base de datos
     */
    pub fn load_contracts(&self) -> SqlResult<Vec<SmartContract>> {
        let mut stmt = self.conn.prepare(
            "SELECT address, owner, contract_type, name, symbol, total_supply, decimals, state, bytecode, abi, created_at, updated_at, update_sequence, integrity_hash
             FROM contracts"
        )?;

        let contract_iter = stmt.query_map([], |row| {
            let state_json: String = row.get(7)?;
            let state: crate::smart_contracts::ContractState = serde_json::from_str(&state_json)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

            let bytecode_json: String = row.get(8)?;
            let bytecode: Option<Vec<u8>> = if bytecode_json.is_empty() {
                None
            } else {
                serde_json::from_str(&bytecode_json)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?
            };

            let update_sequence: u64 = row.get(12).unwrap_or(0);
            let integrity_hash: Option<String> = row.get(13).ok();

            let mut contract = SmartContract {
                address: row.get(0)?,
                owner: row.get(1)?,
                contract_type: row.get(2)?,
                name: row.get(3)?,
                symbol: row.get(4)?,
                total_supply: row.get(5)?,
                decimals: row.get(6)?,
                state,
                bytecode,
                abi: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
                update_sequence,
                integrity_hash,
            };
            
            // Si no tiene hash de integridad, calcularlo
            if contract.integrity_hash.is_none() {
                contract.integrity_hash = Some(contract.calculate_hash());
            }
            
            Ok(contract)
        })?;

        let mut contracts = Vec::new();
        for contract in contract_iter {
            contracts.push(contract?);
        }
        Ok(contracts)
    }

    /**
     * Obtiene un contrato por dirección
     */
    pub fn get_contract_by_address(&self, address: &str) -> SqlResult<Option<SmartContract>> {
        let mut stmt = self.conn.prepare(
            "SELECT address, owner, contract_type, name, symbol, total_supply, decimals, state, bytecode, abi, created_at, updated_at, update_sequence, integrity_hash
             FROM contracts WHERE address = ?1"
        )?;

        let mut rows = stmt.query_map(params![address], |row| {
            let state_json: String = row.get(7)?;
            let state: crate::smart_contracts::ContractState = serde_json::from_str(&state_json)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;

            let bytecode_json: String = row.get(8)?;
            let bytecode: Option<Vec<u8>> = if bytecode_json.is_empty() {
                None
            } else {
                serde_json::from_str(&bytecode_json)
                    .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?
            };

            let update_sequence: u64 = row.get(12).unwrap_or(0);
            let integrity_hash: Option<String> = row.get(13).ok();

            let mut contract = SmartContract {
                address: row.get(0)?,
                owner: row.get(1)?,
                contract_type: row.get(2)?,
                name: row.get(3)?,
                symbol: row.get(4)?,
                total_supply: row.get(5)?,
                decimals: row.get(6)?,
                state,
                bytecode,
                abi: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
                update_sequence,
                integrity_hash,
            };
            
            // Si no tiene hash de integridad, calcularlo
            if contract.integrity_hash.is_none() {
                contract.integrity_hash = Some(contract.calculate_hash());
            }
            
            Ok(contract)
        })?;

        match rows.next() {
            Some(Ok(contract)) => Ok(Some(contract)),
            Some(Err(e)) => Err(e),
            None => Ok(None),
        }
    }

    /**
     * Elimina un contrato de la base de datos
     */
    #[allow(dead_code)]
    pub fn delete_contract(&self, address: &str) -> SqlResult<()> {
        self.conn.execute(
            "DELETE FROM contracts WHERE address = ?1",
            params![address],
        )?;
        Ok(())
    }

    /**
     * Guarda un contrato pendiente de broadcast
     */
    pub fn save_pending_broadcast(&self, peer_address: &str, contract: &SmartContract) -> SqlResult<()> {
        let contract_json = serde_json::to_string(contract)
            .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.conn.execute(
            "INSERT INTO pending_contract_broadcasts (peer_address, contract_address, contract_data, created_at, retry_count)
             VALUES (?1, ?2, ?3, ?4, 0)",
            params![peer_address, contract.address, contract_json, now],
        )?;
        
        Ok(())
    }

    /**
     * Carga todos los contratos pendientes de broadcast
     */
    pub fn load_pending_broadcasts(&self) -> SqlResult<Vec<(String, SmartContract)>> {
        let mut stmt = self.conn.prepare(
            "SELECT peer_address, contract_data FROM pending_contract_broadcasts
             WHERE retry_count < 10
             ORDER BY created_at ASC"
        )?;
        
        let rows = stmt.query_map([], |row| {
            let peer_address: String = row.get(0)?;
            let contract_json: String = row.get(1)?;
            
            let contract: SmartContract = serde_json::from_str(&contract_json)
                .map_err(|e| rusqlite::Error::InvalidParameterName(e.to_string()))?;
            
            Ok((peer_address, contract))
        })?;
        
        let mut result = Vec::new();
        for row in rows {
            result.push(row?);
        }
        
        Ok(result)
    }

    /**
     * Elimina un contrato pendiente de broadcast
     */
    pub fn remove_pending_broadcast(&self, peer_address: &str, contract_address: &str) -> SqlResult<()> {
        self.conn.execute(
            "DELETE FROM pending_contract_broadcasts 
             WHERE peer_address = ?1 AND contract_address = ?2",
            params![peer_address, contract_address],
        )?;
        
        Ok(())
    }

    /**
     * Incrementa el contador de reintentos de un broadcast pendiente
     */
    pub fn increment_pending_retry(&self, peer_address: &str, contract_address: &str) -> SqlResult<()> {
        self.conn.execute(
            "UPDATE pending_contract_broadcasts 
             SET retry_count = retry_count + 1
             WHERE peer_address = ?1 AND contract_address = ?2",
            params![peer_address, contract_address],
        )?;
        
        Ok(())
    }

    /**
     * Guarda un validador en la base de datos
     */
    pub fn save_validator(&self, validator: &Validator) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO validators 
             (address, staked_amount, is_active, total_rewards, created_at, 
              last_validated_block, validation_count, slash_count, unstaking_requested, unstaking_timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                validator.address,
                validator.staked_amount,
                if validator.is_active { 1 } else { 0 },
                validator.total_rewards,
                validator.created_at,
                validator.last_validated_block,
                validator.validation_count,
                validator.slash_count,
                if validator.unstaking_requested { 1 } else { 0 },
                validator.unstaking_timestamp,
            ],
        )?;
        Ok(())
    }

    /**
     * Carga todos los validadores desde la base de datos
     */
    pub fn load_validators(&self) -> SqlResult<Vec<Validator>> {
        let mut stmt = self.conn.prepare(
            "SELECT address, staked_amount, is_active, total_rewards, created_at,
                    last_validated_block, validation_count, slash_count, unstaking_requested, unstaking_timestamp
             FROM validators"
        )?;

        let validator_iter = stmt.query_map([], |row| {
            Ok(Validator {
                address: row.get(0)?,
                staked_amount: row.get(1)?,
                is_active: row.get::<_, i32>(2)? != 0,
                total_rewards: row.get(3)?,
                created_at: row.get(4)?,
                last_validated_block: row.get(5)?,
                validation_count: row.get(6)?,
                slash_count: row.get(7)?,
                unstaking_requested: row.get::<_, i32>(8)? != 0,
                unstaking_timestamp: row.get(9)?,
            })
        })?;

        let mut validators = Vec::new();
        for validator in validator_iter {
            validators.push(validator?);
        }
        Ok(validators)
    }

    /**
     * Elimina un validador de la base de datos
     */
    pub fn remove_validator(&self, address: &str) -> SqlResult<()> {
        self.conn.execute(
            "DELETE FROM validators WHERE address = ?1",
            params![address],
        )?;
        Ok(())
    }

    /**
     * Guarda información de tracking de un nodo
     */
    pub fn save_node_tracking(&self, tracking: &NodeTracking) -> SqlResult<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO node_tracking 
             (node_address, first_block_index, first_block_timestamp, blocks_validated,
              last_block_timestamp, is_eligible, airdrop_claimed, claim_timestamp,
              claim_transaction_id, claim_block_index, claim_verified, uptime_seconds, eligibility_tier)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                tracking.node_address,
                tracking.first_block_index,
                tracking.first_block_timestamp,
                tracking.blocks_validated,
                tracking.last_block_timestamp,
                if tracking.is_eligible { 1 } else { 0 },
                if tracking.airdrop_claimed { 1 } else { 0 },
                tracking.claim_timestamp,
                tracking.claim_transaction_id,
                tracking.claim_block_index,
                if tracking.claim_verified { 1 } else { 0 },
                tracking.uptime_seconds,
                tracking.eligibility_tier,
            ],
        )?;
        Ok(())
    }

    /**
     * Carga todos los tracking de nodos desde la base de datos
     */
    pub fn load_node_tracking(&self) -> SqlResult<Vec<NodeTracking>> {
        let mut stmt = self.conn.prepare(
            "SELECT node_address, first_block_index, first_block_timestamp, blocks_validated,
                    last_block_timestamp, is_eligible, airdrop_claimed, claim_timestamp,
                    claim_transaction_id, claim_block_index, claim_verified, uptime_seconds, eligibility_tier
             FROM node_tracking"
        )?;

        let tracking_iter = stmt.query_map([], |row| {
            Ok(NodeTracking {
                node_address: row.get(0)?,
                first_block_index: row.get(1)?,
                first_block_timestamp: row.get(2)?,
                blocks_validated: row.get(3)?,
                last_block_timestamp: row.get(4)?,
                is_eligible: row.get::<_, i32>(5)? != 0,
                airdrop_claimed: row.get::<_, i32>(6)? != 0,
                claim_timestamp: row.get(7)?,
                claim_transaction_id: row.get(8)?,
                claim_block_index: row.get(9)?,
                claim_verified: row.get::<_, i32>(10)? != 0,
                uptime_seconds: row.get(11).unwrap_or(0),
                eligibility_tier: row.get(12).unwrap_or(0),
            })
        })?;

        let mut trackings = Vec::new();
        for tracking in tracking_iter {
            trackings.push(tracking?);
        }
        Ok(trackings)
    }

    /**
     * Guarda un claim de airdrop
     */
    pub fn save_airdrop_claim(&self, tracking: &NodeTracking) -> SqlResult<()> {
        if let Some(claim_timestamp) = tracking.claim_timestamp {
            let transaction_id = tracking.claim_transaction_id.as_deref().unwrap_or("");
            self.conn.execute(
                "INSERT OR REPLACE INTO airdrop_claims 
                 (node_address, claim_timestamp, airdrop_amount, transaction_id, transaction_hash, block_index, tier_id, verified, verification_timestamp, retry_count)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    tracking.node_address,
                    claim_timestamp,
                    0, // Se actualizará cuando se procese la transacción
                    transaction_id,
                    "", // Se actualizará cuando se procese la transacción
                    tracking.claim_block_index,
                    tracking.eligibility_tier,
                    if tracking.claim_verified { 1 } else { 0 },
                    None::<i64>, // verification_timestamp
                    0, // retry_count
                ],
            )?;
        }
        Ok(())
    }

    /**
     * Obtiene todos los claims de airdrop
     */
    pub fn load_airdrop_claims(&self) -> SqlResult<Vec<crate::airdrop::ClaimRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT node_address, claim_timestamp, airdrop_amount, transaction_id, block_index, tier_id, verified, verification_timestamp
             FROM airdrop_claims
             ORDER BY claim_timestamp DESC"
        )?;

        let claim_iter = stmt.query_map([], |row| {
            Ok(crate::airdrop::ClaimRecord {
                node_address: row.get(0)?,
                claim_timestamp: row.get(1)?,
                airdrop_amount: row.get(2)?,
                transaction_id: row.get(3)?,
                block_index: row.get(4)?,
                tier_id: row.get(5).unwrap_or(0),
                verified: row.get::<_, i32>(6)? != 0,
                verification_timestamp: row.get(7)?,
            })
        })?;

        let mut claims = Vec::new();
        for claim in claim_iter {
            claims.push(claim?);
        }
        Ok(claims)
    }
}

