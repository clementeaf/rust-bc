use crate::blockchain::{Block, Blockchain};
use crate::models::Transaction;
use crate::smart_contracts::SmartContract;
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
                updated_at INTEGER NOT NULL
            )",
            [],
        )?;

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
            "CREATE INDEX IF NOT EXISTS idx_contracts_type ON contracts(contract_type)",
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
             (address, owner, contract_type, name, symbol, total_supply, decimals, state, bytecode, abi, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
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
                contract.updated_at
            ],
        )?;
        Ok(())
    }

    /**
     * Carga todos los contratos de la base de datos
     */
    pub fn load_contracts(&self) -> SqlResult<Vec<SmartContract>> {
        let mut stmt = self.conn.prepare(
            "SELECT address, owner, contract_type, name, symbol, total_supply, decimals, state, bytecode, abi, created_at, updated_at
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

            Ok(SmartContract {
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
            })
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
            "SELECT address, owner, contract_type, name, symbol, total_supply, decimals, state, bytecode, abi, created_at, updated_at
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

            Ok(SmartContract {
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
            })
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
}

