use ed25519_dalek::{Signer, Verifier, Signature, SignatureError, SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

/**
 * Representa una transacción en la blockchain
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub from: String,
    pub to: String,
    pub amount: u64,
    #[serde(default)]
    pub fee: u64, // Fee de transacción (opcional, default: 0)
    pub data: Option<String>,
    pub timestamp: u64,
    pub signature: String,
}

impl Transaction {
    /**
     * Crea una nueva transacción (sin fee)
     * @deprecated Use new_with_fee en su lugar
     */
    #[allow(dead_code)]
    pub fn new(from: String, to: String, amount: u64, data: Option<String>) -> Transaction {
        Self::new_with_fee(from, to, amount, 0, data)
    }

    /**
     * Crea una nueva transacción con fee
     */
    pub fn new_with_fee(from: String, to: String, amount: u64, fee: u64, data: Option<String>) -> Transaction {
        let id = Uuid::new_v4().to_string();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Transaction {
            id,
            from,
            to,
            amount,
            fee,
            data,
            timestamp,
            signature: String::new(),
        }
    }

    /**
     * Calcula el hash de la transacción
     */
    pub fn calculate_hash(&self) -> String {
        let data = format!(
            "{}{}{}{}{}{}{}",
            self.id, self.from, self.to, self.amount, self.fee,
            self.data.as_ref().unwrap_or(&String::new()), self.timestamp
        );
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /**
     * Valida la transacción básica
     */
    pub fn is_valid(&self) -> bool {
        !self.from.is_empty() && !self.to.is_empty() && self.amount > 0
    }

    /**
     * Verifica la firma digital de la transacción
     */
    pub fn verify_signature(&self, public_key_bytes: &[u8]) -> Result<(), SignatureError> {
        if self.signature.is_empty() {
            return Err(SignatureError::new());
        }

        if public_key_bytes.len() != 32 {
            return Err(SignatureError::new());
        }

        let mut pk_array = [0u8; 32];
        pk_array.copy_from_slice(public_key_bytes);
        let public_key = VerifyingKey::from_bytes(&pk_array)
            .map_err(|_| SignatureError::new())?;

        let signature_bytes = hex::decode(&self.signature)
            .map_err(|_| SignatureError::new())?;
        
        if signature_bytes.len() != 64 {
            return Err(SignatureError::new());
        }

        let mut sig_array = [0u8; 64];
        sig_array.copy_from_slice(&signature_bytes);
        let signature = Signature::from_bytes(&sig_array);

        let message = self.calculate_hash();
        public_key.verify(message.as_bytes(), &signature).map_err(|_| SignatureError::new())
    }

    /**
     * Verifica si la transacción tiene una firma válida
     */
    pub fn has_valid_signature(&self, public_key_bytes: &[u8]) -> bool {
        self.verify_signature(public_key_bytes).is_ok()
    }
}

/**
 * Representa un wallet en la blockchain con criptografía
 */
#[derive(Debug, Clone)]
pub struct Wallet {
    pub address: String,
    pub balance: u64,
    pub public_key: VerifyingKey,
    pub signing_key: SigningKey,
}

impl Wallet {
    /**
     * Crea un nuevo wallet con keypair criptográfico
     */
    pub fn new() -> Wallet {
        let signing_key = SigningKey::generate(&mut OsRng);
        let public_key = signing_key.verifying_key();
        let address = hex::encode(public_key.as_bytes());

        Wallet {
            address,
            balance: 0,
            public_key,
            signing_key,
        }
    }

    /**
     * Crea un wallet desde una clave pública (solo lectura)
     */
    #[allow(dead_code)]
    pub fn from_public_key(public_key_bytes: &[u8]) -> Result<Wallet, SignatureError> {
        if public_key_bytes.len() != 32 {
            return Err(SignatureError::new());
        }
        let mut pk_array = [0u8; 32];
        pk_array.copy_from_slice(public_key_bytes);
        let public_key = VerifyingKey::from_bytes(&pk_array)
            .map_err(|_| SignatureError::new())?;
        let address = hex::encode(public_key.as_bytes());

        Ok(Wallet {
            address,
            balance: 0,
            public_key,
            signing_key: SigningKey::from_bytes(&[0u8; 32]),
        })
    }

    /**
     * Firma una transacción con la clave privada del wallet
     */
    pub fn sign_transaction(&self, tx: &mut Transaction) {
        let message = tx.calculate_hash();
        let signature = self.signing_key.sign(message.as_bytes());
        tx.signature = hex::encode(signature.to_bytes());
    }

    /**
     * Obtiene la clave pública como string hexadecimal
     */
    pub fn get_public_key_hex(&self) -> String {
        hex::encode(self.public_key.as_bytes())
    }

    /**
     * Obtiene la clave pública como bytes
     */
    pub fn get_public_key_bytes(&self) -> [u8; 32] {
        self.public_key.to_bytes()
    }

    /**
     * Agrega saldo al wallet
     */
    pub fn add_balance(&mut self, amount: u64) {
        self.balance += amount;
    }

    /**
     * Resta saldo del wallet
     */
    pub fn subtract_balance(&mut self, amount: u64) -> Result<(), String> {
        if self.balance >= amount {
            self.balance -= amount;
            Ok(())
        } else {
            Err("Saldo insuficiente".to_string())
        }
    }
}

impl Serialize for Wallet {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Wallet", 3)?;
        state.serialize_field("address", &self.address)?;
        state.serialize_field("balance", &self.balance)?;
        state.serialize_field("public_key", &self.get_public_key_hex())?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Wallet {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WalletHelper {
            address: String,
            balance: u64,
            public_key: String,
        }

        let helper = WalletHelper::deserialize(deserializer)?;
        let public_key_bytes = hex::decode(&helper.public_key)
            .map_err(serde::de::Error::custom)?;
        
        if public_key_bytes.len() != 32 {
            return Err(serde::de::Error::custom("Invalid public key length"));
        }

        let mut pk_array = [0u8; 32];
        pk_array.copy_from_slice(&public_key_bytes);
        let public_key = VerifyingKey::from_bytes(&pk_array)
            .map_err(serde::de::Error::custom)?;

        Ok(Wallet {
            address: helper.address,
            balance: helper.balance,
            public_key,
            signing_key: SigningKey::from_bytes(&[0u8; 32]),
        })
    }
}

/**
 * Pool de transacciones pendientes (Mempool)
 */
#[derive(Debug, Clone)]
pub struct Mempool {
    pub transactions: Vec<Transaction>,
    pub max_size: usize,
}

impl Mempool {
    /**
     * Crea un nuevo mempool
     */
    pub fn new() -> Mempool {
        Mempool {
            transactions: Vec::new(),
            max_size: 1000,
        }
    }

    /**
     * Crea un mempool con tamaño máximo personalizado
     */
    #[allow(dead_code)]
    pub fn with_max_size(max_size: usize) -> Mempool {
        Mempool {
            transactions: Vec::new(),
            max_size,
        }
    }

    /**
     * Calcula el saldo pendiente (gastado) de un wallet en el mempool
     * @param address - Dirección del wallet
     * @returns Total de amount + fee de todas las transacciones pendientes del wallet
     */
    pub fn calculate_pending_spent(&self, address: &str) -> u64 {
        self.transactions
            .iter()
            .filter(|tx| tx.from == address)
            .map(|tx| tx.amount + tx.fee)
            .sum()
    }

    /**
     * Verifica si hay doble gasto en el mempool
     * @param tx - Transacción a verificar
     * @returns true si hay conflicto con transacciones pendientes
     */
    pub fn has_double_spend(&self, tx: &Transaction) -> bool {
        self.transactions
            .iter()
            .any(|pending_tx| {
                pending_tx.from == tx.from
                    && pending_tx.id != tx.id
                    && pending_tx.amount == tx.amount
                    && pending_tx.timestamp == tx.timestamp
            })
    }

    /**
     * Agrega una transacción al mempool
     */
    pub fn add_transaction(&mut self, tx: Transaction) -> Result<(), String> {
        if self.transactions.len() >= self.max_size {
            return Err("Mempool lleno".to_string());
        }

        if self.transactions.iter().any(|t| t.id == tx.id) {
            return Err("Transacción ya existe en mempool".to_string());
        }

        self.transactions.push(tx);
        Ok(())
    }

    /**
     * Obtiene transacciones del mempool para incluir en un bloque
     * Ordena por fee (mayor a menor) para priorizar transacciones con fees más altos
     * @param max - Número máximo de transacciones a obtener
     * @returns Vector de transacciones removidas del mempool (ordenadas por fee)
     */
    pub fn get_transactions_for_block(&mut self, max: usize) -> Vec<Transaction> {
        // Ordenar por fee descendente (mayor fee primero)
        self.transactions.sort_by(|a, b| b.fee.cmp(&a.fee));
        
        let count = max.min(self.transactions.len());
        self.transactions.drain(..count).collect()
    }

    /**
     * Obtiene todas las transacciones del mempool sin removerlas
     */
    pub fn get_all_transactions(&self) -> &[Transaction] {
        &self.transactions
    }

    /**
     * Remueve una transacción del mempool por ID
     */
    pub fn remove_transaction(&mut self, tx_id: &str) -> bool {
        if let Some(pos) = self.transactions.iter().position(|t| t.id == tx_id) {
            self.transactions.remove(pos);
            true
        } else {
            false
        }
    }

    /**
     * Limpia todas las transacciones del mempool
     */
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.transactions.clear();
    }

    /**
     * Obtiene el número de transacciones en el mempool
     */
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    /**
     * Verifica si el mempool está vacío
     */
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.transactions.is_empty()
    }
}

impl Default for Mempool {
    fn default() -> Self {
        Self::new()
    }
}

/**
 * Sistema de gestión de wallets
 */
#[derive(Debug)]
pub struct WalletManager {
    pub wallets: HashMap<String, Wallet>,
}

impl WalletManager {
    /**
     * Crea un nuevo gestor de wallets
     */
    pub fn new() -> WalletManager {
        WalletManager {
            wallets: HashMap::new(),
        }
    }

    /**
     * Crea un nuevo wallet con keypair criptográfico
     */
    pub fn create_wallet(&mut self) -> &Wallet {
        let wallet = Wallet::new();
        let address = wallet.address.clone();
        self.wallets.insert(address.clone(), wallet);
        self.wallets.get(&address).unwrap()
    }

    /**
     * Crea un wallet con dirección específica (legacy, genera nuevo keypair)
     */
    #[allow(dead_code)]
    pub fn create_wallet_with_address(&mut self, _address: String) -> &Wallet {
        self.create_wallet()
    }

    /**
     * Obtiene un wallet y permite firmar transacciones
     */
    pub fn get_wallet_for_signing(&self, address: &str) -> Option<&Wallet> {
        self.wallets.get(address)
    }

    /**
     * Obtiene o crea un wallet con la dirección especificada
     */
    #[allow(dead_code)]
    pub fn get_or_create_wallet(&mut self, address: &str) -> &mut Wallet {
        if !self.wallets.contains_key(address) {
            let mut wallet = Wallet::new();
            wallet.address = address.to_string();
            self.wallets.insert(address.to_string(), wallet);
        }
        self.wallets.get_mut(address).unwrap()
    }

    /**
     * Obtiene un wallet por dirección
     */
    pub fn get_wallet(&self, address: &str) -> Option<&Wallet> {
        self.wallets.get(address)
    }

    /**
     * Obtiene un wallet mutable por dirección
     */
    #[allow(dead_code)]
    pub fn get_wallet_mut(&mut self, address: &str) -> Option<&mut Wallet> {
        self.wallets.get_mut(address)
    }

    /**
     * Obtiene el balance de un wallet
     * NOTA: Este método devuelve el balance almacenado en el wallet.
     * Para obtener el balance real desde la blockchain, usar blockchain.calculate_balance()
     */
    #[allow(dead_code)]
    pub fn get_balance(&self, address: &str) -> u64 {
        self.wallets
            .get(address)
            .or_else(|| self.wallets.values().find(|w| w.address == address))
            .map(|w| w.balance)
            .unwrap_or(0)
    }

    /**
     * Busca un wallet por dirección (puede estar con dirección como clave o en el campo address)
     */
    #[allow(dead_code)]
    pub fn find_wallet_by_address(&self, address: &str) -> Option<&Wallet> {
        self.wallets.get(address)
            .or_else(|| self.wallets.values().find(|w| w.address == address))
    }

    /**
     * Busca un wallet mutable por dirección
     */
    #[allow(dead_code)]
    pub fn find_wallet_by_address_mut(&mut self, address: &str) -> Option<&mut Wallet> {
        if self.wallets.contains_key(address) {
            return self.wallets.get_mut(address);
        }
        
        let key_to_update = self.wallets
            .iter()
            .find(|(_, w)| w.address == address)
            .map(|(k, _)| k.clone());
        
        if let Some(key) = key_to_update {
            return self.wallets.get_mut(&key);
        }
        
        None
    }

    /**
     * Procesa una transacción actualizando saldos
     * Nota: Los fees se suman al minero cuando se procesa el bloque
     */
    pub fn process_transaction(&mut self, tx: &Transaction) -> Result<(), String> {
        if !tx.is_valid() {
            return Err("Transacción inválida".to_string());
        }

        let from_wallet = self.wallets.entry(tx.from.clone()).or_insert_with(|| {
            Wallet::new()
        });

        // Restar amount + fee del wallet origen
        let total = tx.amount + tx.fee;
        from_wallet.subtract_balance(total)?;

        let to_wallet = self.wallets.entry(tx.to.clone()).or_insert_with(|| {
            Wallet::new()
        });

        // Solo agregar amount al destinatario (el fee va al minero)
        to_wallet.add_balance(tx.amount);
        Ok(())
    }

    /**
     * Obtiene todos los wallets
     */
    #[allow(dead_code)]
    pub fn get_all_wallets(&self) -> Vec<&Wallet> {
        self.wallets.values().collect()
    }

    /**
     * Procesa una transacción coinbase creando el wallet si no existe
     * @param tx - Transacción coinbase (from == "0")
     * @returns Result indicando éxito o error
     */
    pub fn process_coinbase_transaction(&mut self, tx: &Transaction) -> Result<(), String> {
        if tx.from != "0" {
            return Err("No es una transacción coinbase".to_string());
        }

        if tx.amount == 0 {
            return Err("Cantidad de coinbase debe ser mayor a 0".to_string());
        }

        if tx.to.is_empty() {
            return Err("Dirección destinataria de coinbase no puede estar vacía".to_string());
        }

        let wallet = self.wallets.entry(tx.to.clone()).or_insert_with(|| {
            let mut new_wallet = Wallet::new();
            new_wallet.address = tx.to.clone();
            new_wallet
        });

        wallet.add_balance(tx.amount);
        Ok(())
    }

    /**
     * Sincroniza todos los wallets desde la cadena de bloques
     * Recalcula balances desde todas las transacciones históricas
     * @param chain - Referencia a la cadena de bloques para calcular balances
     */
    pub fn sync_from_blockchain(&mut self, chain: &[crate::blockchain::Block]) {
        use std::collections::HashSet;

        let mut addresses = HashSet::new();

        for block in chain {
            for tx in &block.transactions {
                if !tx.from.is_empty() && tx.from != "0" {
                    addresses.insert(tx.from.clone());
                }
                if !tx.to.is_empty() {
                    addresses.insert(tx.to.clone());
                }
            }
        }

        let mut balance_map: HashMap<String, u64> = HashMap::new();

        for block in chain {
            for tx in &block.transactions {
                if tx.from == "0" {
                    *balance_map.entry(tx.to.clone()).or_insert(0) += tx.amount;
                } else {
                    *balance_map.entry(tx.from.clone()).or_insert(0) = 
                        balance_map.get(&tx.from).unwrap_or(&0).saturating_sub(tx.amount);
                    *balance_map.entry(tx.to.clone()).or_insert(0) += tx.amount;
                }
            }
        }

        for (address, balance) in balance_map {
            if balance > 0 || self.wallets.contains_key(&address) {
                let wallet = self.wallets.entry(address.clone()).or_insert_with(|| {
                    let mut new_wallet = Wallet::new();
                    new_wallet.address = address.clone();
                    new_wallet
                });
                wallet.balance = balance;
            }
        }
    }
}

impl Default for WalletManager {
    fn default() -> Self {
        Self::new()
    }
}

