use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/**
 * Tipos de funciones de contrato soportadas
 */
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContractFunction {
    // ERC-20: Funciones requeridas
    Transfer {
        to: String,
        amount: u64,
    },
    TransferFrom {
        from: String,
        to: String,
        amount: u64,
    },
    Approve {
        spender: String,
        amount: u64,
    },
    // Funciones adicionales
    Mint {
        to: String,
        amount: u64,
    },
    Burn {
        from: String,
        amount: u64,
    },
    // NFT: Funciones básicas (ERC-721 simplificado)
    MintNFT {
        to: String,
        token_id: u64,
        token_uri: String,
    },
    TransferNFT {
        from: String,
        to: String,
        token_id: u64,
    },
    ApproveNFT {
        to: String,
        token_id: u64,
    },
    TransferFromNFT {
        from: String,
        to: String,
        token_id: u64,
    },
    // NFT: Funciones avanzadas
    BurnNFT {
        owner: String,
        token_id: u64,
    },
    SetNFTMetadata {
        token_id: u64,
        metadata: NFTMetadata,
    },
    Custom {
        name: String,
        params: Vec<String>,
    },
}

/**
 * Metadata estructurada para NFTs
 */
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NFTMetadata {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub image: String,
    #[serde(default)]
    pub external_url: String,
    #[serde(default)]
    pub attributes: Vec<Attribute>,
}

/**
 * Atributo de un NFT (para traits/rarity)
 */
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Attribute {
    pub trait_type: String,
    pub value: String,
}

impl NFTMetadata {
    pub fn new(name: String) -> Self {
        NFTMetadata {
            name,
            description: String::new(),
            image: String::new(),
            external_url: String::new(),
            attributes: Vec::new(),
        }
    }
}

/**
 * Estado de un smart contract
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractState {
    pub balances: HashMap<String, u64>,
    pub metadata: HashMap<String, String>,
    // ERC-20: Sistema de approvals (owner -> spender -> amount)
    #[serde(default)]
    pub allowances: HashMap<String, HashMap<String, u64>>, // owner -> (spender -> amount)
    // NFT: Estructuras para tokens no fungibles
    #[serde(default)]
    pub token_owners: HashMap<u64, String>, // token_id -> owner address
    #[serde(default)]
    pub token_uris: HashMap<u64, String>, // token_id -> URI/metadata (legacy, para compatibilidad)
    #[serde(default)]
    pub token_approvals: HashMap<u64, String>, // token_id -> approved address
    #[serde(default)]
    pub nft_balances: HashMap<String, u64>, // owner -> count of NFTs owned
    // NFT: Mejoras Fase 1
    #[serde(default)]
    pub nft_metadata: HashMap<u64, NFTMetadata>, // token_id -> metadata estructurada
    #[serde(default)]
    pub owner_to_tokens: HashMap<String, HashSet<u64>>, // owner -> set of token_ids (índice inverso para O(1))
    #[serde(default)]
    pub token_index: Vec<u64>, // Lista ordenada de todos los token_ids (para enumeración)
}

impl ContractState {
    pub fn new() -> Self {
        ContractState {
            balances: HashMap::new(),
            metadata: HashMap::new(),
            allowances: HashMap::new(),
            token_owners: HashMap::new(),
            token_uris: HashMap::new(),
            token_approvals: HashMap::new(),
            nft_balances: HashMap::new(),
            nft_metadata: HashMap::new(),
            owner_to_tokens: HashMap::new(),
            token_index: Vec::new(),
        }
    }
}

/**
 * Smart Contract
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartContract {
    pub address: String,
    pub owner: String,
    pub contract_type: String, // "token", "nft", "custom"
    pub name: String,
    pub symbol: Option<String>,
    pub total_supply: Option<u64>,
    pub decimals: Option<u8>,
    pub state: ContractState,
    pub bytecode: Option<Vec<u8>>,
    pub abi: Option<String>, // JSON string
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub update_sequence: u64, // Número de secuencia para resolver race conditions
    #[serde(default)]
    pub integrity_hash: Option<String>, // Hash de integridad del contrato
}

impl SmartContract {
    /**
     * Crea un nuevo smart contract
     */
    pub fn new(
        owner: String,
        contract_type: String,
        name: String,
        symbol: Option<String>,
        total_supply: Option<u64>,
        decimals: Option<u8>,
    ) -> SmartContract {
        let address = Self::generate_address(&owner, &name);
        let (timestamp, _) = Self::get_timestamp_nanos();

        let mut contract = SmartContract {
            address: address.clone(),
            owner,
            contract_type,
            name,
            symbol,
            total_supply,
            decimals,
            state: ContractState::new(),
            bytecode: None,
            abi: None,
            created_at: timestamp,
            updated_at: timestamp,
            update_sequence: 0,
            integrity_hash: None,
        };
        
        // Calcular hash de integridad inicial
        contract.integrity_hash = Some(contract.calculate_hash());
        contract
    }

    /**
     * Genera una dirección única para el contrato
     */
    fn generate_address(owner: &str, name: &str) -> String {
        let data = format!("{}{}{}", owner, name, Uuid::new_v4());
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let hash = hasher.finalize();
        format!("contract_{:x}", hash)
    }

    /**
     * Ejecuta una función del contrato
     * @param function - Función a ejecutar
     * @param caller - Dirección que llama la función (para ERC-20)
     */
    pub fn execute(&mut self, function: ContractFunction, caller: Option<&str>) -> Result<String, String> {
        match function {
            ContractFunction::Transfer { to, amount } => {
                let from = caller.ok_or("Caller address required for transfer")?;
                self.transfer(from, &to, amount)
            }
            ContractFunction::TransferFrom { from, to, amount } => {
                let spender = caller.ok_or("Caller address required for transferFrom")?;
                self.transfer_from(&from, &to, amount, spender)
            }
            ContractFunction::Approve { spender, amount } => {
                let owner = caller.ok_or("Caller address required for approve")?;
                self.approve(owner, &spender, amount)
            }
            ContractFunction::Mint { to, amount } => {
                self.mint(&to, amount)
            }
            ContractFunction::Burn { from, amount } => {
                self.burn(&from, amount)
            }
            // NFT: Funciones básicas
            ContractFunction::MintNFT { to, token_id, token_uri } => {
                self.mint_nft(&to, token_id, &token_uri)
            }
            ContractFunction::TransferNFT { from, to, token_id } => {
                let caller = caller.ok_or("Caller address required for transferNFT")?;
                self.transfer_nft(&from, &to, token_id, caller)
            }
            ContractFunction::ApproveNFT { to, token_id } => {
                let owner = caller.ok_or("Caller address required for approveNFT")?;
                self.approve_nft(owner, &to, token_id)
            }
            ContractFunction::TransferFromNFT { from, to, token_id } => {
                let spender = caller.ok_or("Caller address required for transferFromNFT")?;
                self.transfer_from_nft(&from, &to, token_id, spender)
            }
            ContractFunction::BurnNFT { owner, token_id } => {
                let caller = caller.ok_or("Caller address required for burnNFT")?;
                self.burn_nft(&owner, token_id, caller)
            }
            ContractFunction::SetNFTMetadata { token_id, metadata } => {
                self.set_nft_metadata(token_id, metadata)
                    .map(|_| format!("Metadata set for token {}", token_id))
            }
            ContractFunction::Custom { name, params } => {
                self.execute_custom(&name, &params)
            }
        }
    }

    /**
     * Valida una dirección de wallet
     */
    fn validate_address(address: &str) -> Result<(), String> {
        if address.is_empty() {
            return Err("Address cannot be empty".to_string());
        }
        
        // Protección contra zero address
        if address == "0" || (address.len() == 1 && address.chars().all(|c| c == '0')) {
            return Err("Zero address is not allowed".to_string());
        }
        
        if address.len() < 32 {
            return Err("Address format invalid (too short)".to_string());
        }
        if address.len() > 128 {
            return Err("Address format invalid (too long)".to_string());
        }
        // Validar que sea hexadecimal (opcional, pero buena práctica)
        if !address.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err("Address contains invalid characters".to_string());
        }
        Ok(())
    }

    /**
     * Valida un token_id para NFTs
     */
    fn validate_token_id(token_id: u64) -> Result<(), String> {
        // Token ID 0 está reservado (usado para eventos de mint)
        if token_id == 0 {
            return Err("Token ID 0 is reserved and cannot be used".to_string());
        }
        
        // Límite máximo para prevenir problemas de serialización y DoS
        const MAX_TOKEN_ID: u64 = 1_000_000_000; // 1 billón
        if token_id > MAX_TOKEN_ID {
            return Err(format!("Token ID exceeds maximum allowed: {}", MAX_TOKEN_ID));
        }
        
        Ok(())
    }

    /**
     * Verifica que el contrato sea del tipo correcto
     */
    fn ensure_contract_type(&self, expected_type: &str) -> Result<(), String> {
        if self.contract_type != expected_type {
            return Err(format!("This function is only available for {} contracts, but contract is {}", 
                expected_type, self.contract_type));
        }
        Ok(())
    }

    /**
     * ERC-20: Transfiere tokens desde el caller a otra dirección
     */
    fn transfer(&mut self, from: &str, to: &str, amount: u64) -> Result<String, String> {
        // Validaciones de entrada
        Self::validate_address(from)?;
        Self::validate_address(to)?;
        
        if amount == 0 {
            return Err("Amount must be greater than 0".to_string());
        }

        // Límite máximo de amount para prevenir DoS (1 billón de tokens)
        const MAX_AMOUNT: u64 = 1_000_000_000_000;
        if amount > MAX_AMOUNT {
            return Err(format!("Amount exceeds maximum allowed: {}", MAX_AMOUNT));
        }

        if from == to {
            return Err("Cannot transfer to self".to_string());
        }

        let from_balance = *self.state.balances.get(from).unwrap_or(&0);
        if from_balance < amount {
            return Err("Insufficient balance".to_string());
        }

        let to_balance = *self.state.balances.get(to).unwrap_or(&0);
        
        // Protección contra overflow usando checked_add
        let new_to_balance = to_balance.checked_add(amount)
            .ok_or_else(|| "Balance overflow: recipient balance would exceed maximum".to_string())?;
        
        // Protección contra underflow usando checked_sub (ya validamos arriba, pero por seguridad)
        let new_from_balance = from_balance.checked_sub(amount)
            .ok_or_else(|| "Balance underflow: insufficient balance".to_string())?;

        self.state.balances.insert(from.to_string(), new_from_balance);
        self.state.balances.insert(to.to_string(), new_to_balance);
        
        // Emit Transfer event (tracked in metadata)
        self.emit_transfer_event(from, to, amount);
        
        let (secs, _) = Self::get_timestamp_nanos();
        self.updated_at = secs;
        self.update_sequence += 1;
        self.update_integrity_hash();

        Ok(format!("Transferred {} from {} to {}", amount, from, to))
    }

    /**
     * ERC-20: Transfiere tokens desde una dirección a otra usando allowance
     */
    fn transfer_from(&mut self, from: &str, to: &str, amount: u64, spender: &str) -> Result<String, String> {
        // Validaciones de entrada
        Self::validate_address(from)?;
        Self::validate_address(to)?;
        Self::validate_address(spender)?;
        
        if amount == 0 {
            return Err("Amount must be greater than 0".to_string());
        }

        // Límite máximo de amount
        const MAX_AMOUNT: u64 = 1_000_000_000_000;
        if amount > MAX_AMOUNT {
            return Err(format!("Amount exceeds maximum allowed: {}", MAX_AMOUNT));
        }

        if from == to {
            return Err("Cannot transfer to self".to_string());
        }

        // Verificar allowance
        let allowance = self.allowance(from, spender);
        if allowance < amount {
            return Err("Insufficient allowance".to_string());
        }

        // Verificar balance
        let from_balance = *self.state.balances.get(from).unwrap_or(&0);
        if from_balance < amount {
            return Err("Insufficient balance".to_string());
        }

        // Realizar transferencia con protección contra overflow
        let to_balance = *self.state.balances.get(to).unwrap_or(&0);
        
        let new_to_balance = to_balance.checked_add(amount)
            .ok_or_else(|| "Balance overflow: recipient balance would exceed maximum".to_string())?;
        
        let new_from_balance = from_balance.checked_sub(amount)
            .ok_or_else(|| "Balance underflow: insufficient balance".to_string())?;
        
        self.state.balances.insert(from.to_string(), new_from_balance);
        self.state.balances.insert(to.to_string(), new_to_balance);

        // Reducir allowance
        self.decrease_allowance(from, spender, amount);

        // Emit Transfer event
        self.emit_transfer_event(from, to, amount);
        
        let (secs, _) = Self::get_timestamp_nanos();
        self.updated_at = secs;
        self.update_sequence += 1;
        self.update_integrity_hash();

        Ok(format!("Transferred {} from {} to {} via {}", amount, from, to, spender))
    }

    /**
     * ERC-20: Aprueba que otra dirección gaste tokens en nombre del owner
     */
    fn approve(&mut self, owner: &str, spender: &str, amount: u64) -> Result<String, String> {
        // Validaciones de entrada
        Self::validate_address(owner)?;
        Self::validate_address(spender)?;
        
        if owner == spender {
            return Err("Cannot approve self".to_string());
        }

        // Límite máximo de allowance
        const MAX_AMOUNT: u64 = 1_000_000_000_000;
        if amount > MAX_AMOUNT {
            return Err(format!("Allowance amount exceeds maximum allowed: {}", MAX_AMOUNT));
        }

        // Establecer allowance
        let owner_allowances = self.state.allowances.entry(owner.to_string())
            .or_insert_with(HashMap::new);
        owner_allowances.insert(spender.to_string(), amount);

        // Emit Approval event
        self.emit_approval_event(owner, spender, amount);
        
        let (secs, _) = Self::get_timestamp_nanos();
        self.updated_at = secs;
        self.update_sequence += 1;
        self.update_integrity_hash();

        Ok(format!("Approved {} to spend {} tokens from {}", spender, amount, owner))
    }

    /**
     * ERC-20: Obtiene la cantidad aprobada que spender puede gastar de owner
     */
    pub fn allowance(&self, owner: &str, spender: &str) -> u64 {
        self.state.allowances
            .get(owner)
            .and_then(|allowances| allowances.get(spender).copied())
            .unwrap_or(0)
    }

    /**
     * Reduce el allowance después de una transferFrom
     */
    fn decrease_allowance(&mut self, owner: &str, spender: &str, amount: u64) {
        if let Some(owner_allowances) = self.state.allowances.get_mut(owner) {
            if let Some(current_allowance) = owner_allowances.get_mut(spender) {
                if *current_allowance >= amount {
                    // Usar checked_sub para seguridad adicional
                    if let Some(new_allowance) = current_allowance.checked_sub(amount) {
                        *current_allowance = new_allowance;
                    } else {
                        // Si hay underflow (no debería pasar), establecer a 0
                        *current_allowance = 0;
                    }
                }
            }
        }
    }

    /**
     * Emite evento Transfer (tracked en metadata)
     * Limita el número de eventos para prevenir crecimiento ilimitado
     */
    fn emit_transfer_event(&mut self, from: &str, to: &str, value: u64) {
        const MAX_EVENTS: usize = 1000; // Límite de eventos en metadata
        
        // Limpiar eventos antiguos si hay demasiados
        if self.state.metadata.len() >= MAX_EVENTS {
            let event_keys: Vec<String> = self.state.metadata.keys()
                .filter(|k| k.starts_with("event_"))
                .cloned()
                .collect();
            
            // Mantener solo los últimos 500 eventos
            if event_keys.len() > 500 {
                let to_remove = event_keys.len() - 500;
                for key in event_keys.iter().take(to_remove) {
                    self.state.metadata.remove(key);
                }
            }
        }
        
        let event_key = format!("event_transfer_{}", self.update_sequence);
        let event_value = format!("from:{}|to:{}|value:{}", from, to, value);
        self.state.metadata.insert(event_key, event_value);
    }

    /**
     * Emite evento Approval (tracked en metadata)
     * Limita el número de eventos para prevenir crecimiento ilimitado
     */
    fn emit_approval_event(&mut self, owner: &str, spender: &str, value: u64) {
        const MAX_EVENTS: usize = 1000; // Límite de eventos en metadata
        
        // Limpiar eventos antiguos si hay demasiados
        if self.state.metadata.len() >= MAX_EVENTS {
            let event_keys: Vec<String> = self.state.metadata.keys()
                .filter(|k| k.starts_with("event_"))
                .cloned()
                .collect();
            
            // Mantener solo los últimos 500 eventos
            if event_keys.len() > 500 {
                let to_remove = event_keys.len() - 500;
                for key in event_keys.iter().take(to_remove) {
                    self.state.metadata.remove(key);
                }
            }
        }
        
        let event_key = format!("event_approval_{}", self.update_sequence);
        let event_value = format!("owner:{}|spender:{}|value:{}", owner, spender, value);
        self.state.metadata.insert(event_key, event_value);
    }

    /**
     * Mina nuevos tokens
     */
    fn mint(&mut self, to: &str, amount: u64) -> Result<String, String> {
        // Validación de dirección
        Self::validate_address(to)?;
        
        if amount == 0 {
            return Err("Amount must be greater than 0".to_string());
        }

        // Límite máximo de amount
        const MAX_AMOUNT: u64 = 1_000_000_000_000;
        if amount > MAX_AMOUNT {
            return Err(format!("Mint amount exceeds maximum allowed: {}", MAX_AMOUNT));
        }

        // Verificar límite de supply si existe
        if let Some(max_supply) = self.total_supply {
            let current_supply: u64 = self.state.balances.values().sum();
            let new_supply = current_supply.checked_add(amount)
                .ok_or_else(|| "Supply overflow: minting would cause overflow".to_string())?;
            
            if new_supply > max_supply {
                return Err("Minting would exceed total supply".to_string());
            }
        }

        let current_balance = *self.state.balances.get(to).unwrap_or(&0);
        let new_balance = current_balance.checked_add(amount)
            .ok_or_else(|| "Balance overflow: recipient balance would exceed maximum".to_string())?;
        
        self.state.balances.insert(to.to_string(), new_balance);
        let (secs, _) = Self::get_timestamp_nanos();
        self.updated_at = secs;
        self.update_sequence += 1;
        self.update_integrity_hash();

        Ok(format!("Minted {} to {}", amount, to))
    }

    /**
     * Quema tokens
     */
    fn burn(&mut self, from: &str, amount: u64) -> Result<String, String> {
        // Validación de dirección
        Self::validate_address(from)?;
        
        if amount == 0 {
            return Err("Amount must be greater than 0".to_string());
        }

        // Límite máximo de amount
        const MAX_AMOUNT: u64 = 1_000_000_000_000;
        if amount > MAX_AMOUNT {
            return Err(format!("Burn amount exceeds maximum allowed: {}", MAX_AMOUNT));
        }

        let from_balance = *self.state.balances.get(from).unwrap_or(&0);
        if from_balance < amount {
            return Err("Insufficient balance to burn".to_string());
        }

        let new_balance = from_balance.checked_sub(amount)
            .ok_or_else(|| "Balance underflow: insufficient balance".to_string())?;
        
        self.state.balances.insert(from.to_string(), new_balance);
        let (secs, _) = Self::get_timestamp_nanos();
        self.updated_at = secs;
        self.update_sequence += 1;
        self.update_integrity_hash();

        Ok(format!("Burned {} from {}", amount, from))
    }

    /**
     * Ejecuta una función personalizada
     */
    fn execute_custom(&mut self, name: &str, _params: &[String]) -> Result<String, String> {
        // Por ahora, solo registramos la ejecución
        let (secs, _) = Self::get_timestamp_nanos();
        self.state.metadata.insert(
            format!("last_execution_{}", name),
            secs.to_string(),
        );
        self.updated_at = secs;
        self.update_sequence += 1;
        self.update_integrity_hash();

        Ok(format!("Executed custom function: {}", name))
    }

    /**
     * ERC-20: Obtiene el balance de una dirección
     */
    pub fn get_balance(&self, address: &str) -> u64 {
        *self.state.balances.get(address).unwrap_or(&0)
    }

    /**
     * NFT: Mina un nuevo token no fungible
     * @param to - Dirección que recibirá el NFT
     * @param token_id - ID único del token
     * @param token_uri - URI/metadata del token
     */
    fn mint_nft(&mut self, to: &str, token_id: u64, token_uri: &str) -> Result<String, String> {
        // Verificar tipo de contrato
        self.ensure_contract_type("nft")?;
        
        // Validación de dirección
        Self::validate_address(to)?;
        
        // Validar token_id
        Self::validate_token_id(token_id)?;
        
        // Verificar que el token_id no exista
        if self.state.token_owners.contains_key(&token_id) {
            return Err(format!("Token ID {} already exists", token_id));
        }

        // Límites de DoS: tokens por contrato
        const MAX_TOKENS_PER_CONTRACT: usize = 10_000_000; // 10 millones
        if self.state.token_index.len() >= MAX_TOKENS_PER_CONTRACT {
            return Err(format!("Maximum tokens per contract reached: {}", MAX_TOKENS_PER_CONTRACT));
        }

        // Límites de DoS: tokens por owner
        const MAX_TOKENS_PER_OWNER: usize = 1_000_000; // 1 millón
        let owner_token_count = self.state.owner_to_tokens
            .get(to)
            .map(|tokens| tokens.len())
            .unwrap_or(0);
        if owner_token_count >= MAX_TOKENS_PER_OWNER {
            return Err(format!("Maximum tokens per owner reached: {}", MAX_TOKENS_PER_OWNER));
        }

        // Verificar límite de URI
        if !token_uri.is_empty() && token_uri.len() > 2048 {
            return Err("Token URI exceeds maximum length (2048 characters)".to_string());
        }

        // Asignar el token al owner
        self.state.token_owners.insert(token_id, to.to_string());
        if !token_uri.is_empty() {
            self.state.token_uris.insert(token_id, token_uri.to_string());
        }
        
        // Actualizar balance de NFTs del owner
        let current_balance = *self.state.nft_balances.get(to).unwrap_or(&0);
        self.state.nft_balances.insert(to.to_string(), current_balance + 1);

        // Mantener índice inverso (owner -> tokens) para búsquedas O(1)
        self.state.owner_to_tokens
            .entry(to.to_string())
            .or_insert_with(HashSet::new)
            .insert(token_id);

        // Agregar a índice de tokens para enumeración
        self.state.token_index.push(token_id);

        // Emit Transfer event (from zero address to owner)
        self.emit_nft_transfer_event("0", to, token_id);
        
        let (secs, _) = Self::get_timestamp_nanos();
        self.updated_at = secs;
        self.update_sequence += 1;
        self.update_integrity_hash();

        Ok(format!("Minted NFT {} to {}", token_id, to))
    }

    /**
     * NFT: Transfiere un token no fungible
     * @param from - Dirección actual del owner
     * @param to - Dirección que recibirá el token
     * @param token_id - ID del token a transferir
     * @param caller - Dirección que ejecuta la transferencia
     */
    fn transfer_nft(&mut self, from: &str, to: &str, token_id: u64, caller: &str) -> Result<String, String> {
        // Verificar tipo de contrato
        self.ensure_contract_type("nft")?;
        
        // Validación de direcciones
        Self::validate_address(from)?;
        Self::validate_address(to)?;
        Self::validate_address(caller)?;

        if from == to {
            return Err("Cannot transfer NFT to self".to_string());
        }

        // Verificar que el token existe
        let current_owner = self.state.token_owners.get(&token_id)
            .ok_or_else(|| format!("Token ID {} does not exist", token_id))?;

        // Verificar permisos: el caller debe ser el owner o estar aprobado
        if current_owner != from {
            return Err(format!("Token {} is not owned by {}", token_id, from));
        }

        if caller != from {
            // Verificar si el caller está aprobado para este token
            let approved = self.state.token_approvals.get(&token_id);
            if approved.map(|a| a.as_str()) != Some(caller) {
                return Err(format!("Caller {} is not authorized to transfer token {}", caller, token_id));
            }
            // Limpiar approval después de transferir
            self.state.token_approvals.remove(&token_id);
        }

        // Transferir el token
        self.state.token_owners.insert(token_id, to.to_string());

        // Actualizar balances de NFTs
        let from_balance = *self.state.nft_balances.get(from).unwrap_or(&0);
        if from_balance > 0 {
            self.state.nft_balances.insert(from.to_string(), from_balance - 1);
        }
        let to_balance = *self.state.nft_balances.get(to).unwrap_or(&0);
        self.state.nft_balances.insert(to.to_string(), to_balance + 1);

        // Actualizar índice inverso (owner -> tokens)
        if let Some(from_tokens) = self.state.owner_to_tokens.get_mut(from) {
            from_tokens.remove(&token_id);
            if from_tokens.is_empty() {
                self.state.owner_to_tokens.remove(from);
            }
        }
        self.state.owner_to_tokens
            .entry(to.to_string())
            .or_insert_with(HashSet::new)
            .insert(token_id);

        // Emit Transfer event
        self.emit_nft_transfer_event(from, to, token_id);
        
        let (secs, _) = Self::get_timestamp_nanos();
        self.updated_at = secs;
        self.update_sequence += 1;
        self.update_integrity_hash();

        Ok(format!("Transferred NFT {} from {} to {}", token_id, from, to))
    }

    /**
     * NFT: Aprueba que otra dirección transfiera un token
     * @param owner - Owner del token
     * @param to - Dirección aprobada para transferir
     * @param token_id - ID del token
     */
    fn approve_nft(&mut self, owner: &str, to: &str, token_id: u64) -> Result<String, String> {
        // Verificar tipo de contrato
        self.ensure_contract_type("nft")?;
        
        // Validación de direcciones
        Self::validate_address(owner)?;
        Self::validate_address(to)?;

        if owner == to {
            return Err("Cannot approve self".to_string());
        }

        // Verificar que el token existe y pertenece al owner
        let current_owner = self.state.token_owners.get(&token_id)
            .ok_or_else(|| format!("Token ID {} does not exist", token_id))?;

        if current_owner != owner {
            return Err(format!("Token {} is not owned by {}", token_id, owner));
        }

        // Aprobar
        self.state.token_approvals.insert(token_id, to.to_string());

        // Emit Approval event
        self.emit_nft_approval_event(owner, to, token_id);
        
        let (secs, _) = Self::get_timestamp_nanos();
        self.updated_at = secs;
        self.update_sequence += 1;
        self.update_integrity_hash();

        Ok(format!("Approved {} to transfer NFT {}", to, token_id))
    }

    /**
     * NFT: Transfiere un token usando approval (transferFrom)
     * @param from - Owner actual del token
     * @param to - Dirección que recibirá el token
     * @param token_id - ID del token
     * @param spender - Dirección que ejecuta la transferencia (debe estar aprobada)
     */
    fn transfer_from_nft(&mut self, from: &str, to: &str, token_id: u64, spender: &str) -> Result<String, String> {
        // Verificar tipo de contrato
        self.ensure_contract_type("nft")?;
        
        // Validación de direcciones
        Self::validate_address(from)?;
        Self::validate_address(to)?;
        Self::validate_address(spender)?;

        if from == to {
            return Err("Cannot transfer NFT to self".to_string());
        }

        // Verificar que el token existe y pertenece a 'from'
        let current_owner = self.state.token_owners.get(&token_id)
            .ok_or_else(|| format!("Token ID {} does not exist", token_id))?;

        if current_owner != from {
            return Err(format!("Token {} is not owned by {}", token_id, from));
        }

        // Verificar que el spender está aprobado
        let approved = self.state.token_approvals.get(&token_id)
            .ok_or_else(|| format!("Token {} is not approved for transfer", token_id))?;

        if approved != spender {
            return Err(format!("Spender {} is not approved to transfer token {}", spender, token_id));
        }

        // Transferir el token
        self.state.token_owners.insert(token_id, to.to_string());
        
        // Limpiar approval
        self.state.token_approvals.remove(&token_id);

        // Actualizar balances de NFTs
        let from_balance = *self.state.nft_balances.get(from).unwrap_or(&0);
        if from_balance > 0 {
            self.state.nft_balances.insert(from.to_string(), from_balance - 1);
        }
        let to_balance = *self.state.nft_balances.get(to).unwrap_or(&0);
        self.state.nft_balances.insert(to.to_string(), to_balance + 1);

        // Actualizar índice inverso (owner -> tokens)
        if let Some(from_tokens) = self.state.owner_to_tokens.get_mut(from) {
            from_tokens.remove(&token_id);
            if from_tokens.is_empty() {
                self.state.owner_to_tokens.remove(from);
            }
        }
        self.state.owner_to_tokens
            .entry(to.to_string())
            .or_insert_with(HashSet::new)
            .insert(token_id);

        // Emit Transfer event
        self.emit_nft_transfer_event(from, to, token_id);
        
        let (secs, _) = Self::get_timestamp_nanos();
        self.updated_at = secs;
        self.update_sequence += 1;
        self.update_integrity_hash();

        Ok(format!("Transferred NFT {} from {} to {} via {}", token_id, from, to, spender))
    }

    /**
     * NFT: Obtiene el owner de un token
     * @param token_id - ID del token
     */
    pub fn owner_of(&self, token_id: u64) -> Option<String> {
        self.state.token_owners.get(&token_id).cloned()
    }

    /**
     * NFT: Obtiene el balance de NFTs de una dirección
     * @param address - Dirección a consultar
     */
    pub fn balance_of_nft(&self, address: &str) -> u64 {
        *self.state.nft_balances.get(address).unwrap_or(&0)
    }

    /**
     * NFT: Obtiene la URI/metadata de un token
     * @param token_id - ID del token
     */
    pub fn token_uri(&self, token_id: u64) -> Option<String> {
        self.state.token_uris.get(&token_id).cloned()
    }

    /**
     * NFT: Obtiene el total de NFTs minteados
     */
    pub fn total_supply_nft(&self) -> u64 {
        self.state.token_owners.len() as u64
    }

    /**
     * NFT: Obtiene la dirección aprobada para un token
     * @param token_id - ID del token
     */
    pub fn get_approved(&self, token_id: u64) -> Option<String> {
        self.state.token_approvals.get(&token_id).cloned()
    }

    /**
     * NFT: Lista todos los tokens de un owner (enumeración)
     * @param owner - Dirección del owner
     * @returns Vector de token_ids ordenados
     */
    pub fn tokens_of_owner(&self, owner: &str) -> Vec<u64> {
        self.state.owner_to_tokens
            .get(owner)
            .map(|tokens| {
                let mut token_list: Vec<u64> = tokens.iter().cloned().collect();
                token_list.sort();
                token_list
            })
            .unwrap_or_default()
    }

    /**
     * NFT: Obtiene un token por índice (enumeración)
     * @param index - Índice del token (0-based)
     * @returns token_id si existe
     */
    pub fn token_by_index(&self, index: usize) -> Option<u64> {
        self.state.token_index.get(index).copied()
    }

    /**
     * NFT: Obtiene el total de tokens (para enumeración)
     * @returns Total de tokens minteados
     */
    pub fn total_supply_enumerable(&self) -> u64 {
        self.state.token_index.len() as u64
    }

    /**
     * NFT: Obtiene metadata estructurada de un token
     * @param token_id - ID del token
     * @returns Metadata si existe
     */
    pub fn get_nft_metadata(&self, token_id: u64) -> Option<&NFTMetadata> {
        self.state.nft_metadata.get(&token_id)
    }

    /**
     * Verifica la consistencia de índices y balances de NFTs
     * Útil para debugging y detección de corrupción
     * @returns Ok(()) si todo es consistente, Err con detalles si hay inconsistencias
     */
    pub fn verify_nft_integrity(&self) -> Result<(), String> {
        // Verificar que todos los tokens en token_owners están en token_index
        for (token_id, _) in &self.state.token_owners {
            if !self.state.token_index.contains(token_id) {
                return Err(format!("Token {} in owners but not in index", token_id));
            }
        }

        // Verificar que todos los tokens en token_index tienen owner
        for token_id in &self.state.token_index {
            if !self.state.token_owners.contains_key(token_id) {
                return Err(format!("Token {} in index but has no owner", token_id));
            }
        }

        // Verificar que balances coinciden con owner_to_tokens
        for (owner, balance) in &self.state.nft_balances {
            let actual_count = self.state.owner_to_tokens
                .get(owner)
                .map(|tokens| tokens.len())
                .unwrap_or(0) as u64;
            if *balance != actual_count {
                return Err(format!("Balance mismatch for owner {}: balance={}, actual tokens={}", 
                    owner, balance, actual_count));
            }
        }

        // Verificar que owner_to_tokens coincide con token_owners
        for (owner, tokens) in &self.state.owner_to_tokens {
            for token_id in tokens {
                if let Some(token_owner) = self.state.token_owners.get(token_id) {
                    if token_owner != owner {
                        return Err(format!("Token {} owned by {} but in owner_to_tokens for {}", 
                            token_id, token_owner, owner));
                    }
                } else {
                    return Err(format!("Token {} in owner_to_tokens for {} but has no owner", 
                        token_id, owner));
                }
            }
        }

        // Verificar que total supply coincide
        let total_by_owners = self.state.token_owners.len() as u64;
        let total_by_index = self.state.token_index.len() as u64;
        if total_by_owners != total_by_index {
            return Err(format!("Total supply mismatch: token_owners={}, token_index={}", 
                total_by_owners, total_by_index));
        }

        Ok(())
    }

    /**
     * NFT: Establece metadata estructurada para un token
     * @param token_id - ID del token
     * @param metadata - Metadata estructurada
     */
    pub fn set_nft_metadata(&mut self, token_id: u64, metadata: NFTMetadata) -> Result<(), String> {
        // Verificar tipo de contrato
        self.ensure_contract_type("nft")?;
        
        // Validar token_id
        Self::validate_token_id(token_id)?;
        
        // Validar que el token existe
        if !self.state.token_owners.contains_key(&token_id) {
            return Err(format!("Token ID {} does not exist", token_id));
        }

        // Validar límites de tamaño
        if metadata.name.len() > 256 {
            return Err("Metadata name exceeds maximum length (256 characters)".to_string());
        }
        if metadata.description.len() > 2048 {
            return Err("Metadata description exceeds maximum length (2048 characters)".to_string());
        }
        if metadata.image.len() > 512 {
            return Err("Metadata image URL exceeds maximum length (512 characters)".to_string());
        }
        if metadata.external_url.len() > 512 {
            return Err("Metadata external_url exceeds maximum length (512 characters)".to_string());
        }
        if metadata.attributes.len() > 50 {
            return Err("Metadata attributes exceed maximum count (50)".to_string());
        }

        // Validar tamaño de cada atributo
        for attr in &metadata.attributes {
            if attr.trait_type.len() > 64 {
                return Err("Attribute trait_type exceeds maximum length (64 characters)".to_string());
            }
            if attr.value.len() > 256 {
                return Err("Attribute value exceeds maximum length (256 characters)".to_string());
            }
        }

        self.state.nft_metadata.insert(token_id, metadata);
        let (secs, _) = Self::get_timestamp_nanos();
        self.updated_at = secs;
        self.update_sequence += 1;
        self.update_integrity_hash();
        Ok(())
    }

    /**
     * NFT: Quema/destruye un token
     * @param owner - Owner del token
     * @param token_id - ID del token a quemar
     * @param caller - Dirección que ejecuta el burn
     */
    fn burn_nft(&mut self, owner: &str, token_id: u64, caller: &str) -> Result<String, String> {
        // Verificar tipo de contrato
        self.ensure_contract_type("nft")?;
        
        // Validación de direcciones
        Self::validate_address(owner)?;
        Self::validate_address(caller)?;

        // Verificar que el token existe y pertenece al owner
        let current_owner = self.state.token_owners.get(&token_id)
            .ok_or_else(|| format!("Token ID {} does not exist", token_id))?;

        if current_owner != owner {
            return Err(format!("Token {} is not owned by {}", token_id, owner));
        }

        // Verificar permisos: el caller debe ser el owner
        if caller != owner {
            return Err(format!("Caller {} is not authorized to burn token {}", caller, token_id));
        }

        // Eliminar el token
        self.state.token_owners.remove(&token_id);
        self.state.token_uris.remove(&token_id);
        self.state.token_approvals.remove(&token_id);
        self.state.nft_metadata.remove(&token_id);

        // Actualizar balance
        let owner_balance = *self.state.nft_balances.get(owner).unwrap_or(&0);
        if owner_balance > 0 {
            self.state.nft_balances.insert(owner.to_string(), owner_balance - 1);
        }

        // Actualizar índice inverso
        if let Some(owner_tokens) = self.state.owner_to_tokens.get_mut(owner) {
            owner_tokens.remove(&token_id);
            if owner_tokens.is_empty() {
                self.state.owner_to_tokens.remove(owner);
            }
        }

        // Remover del índice de tokens
        if let Some(pos) = self.state.token_index.iter().position(|&x| x == token_id) {
            self.state.token_index.remove(pos);
        }

        // Emit Transfer event (to zero address = burn)
        self.emit_nft_transfer_event(owner, "0", token_id);
        
        let (secs, _) = Self::get_timestamp_nanos();
        self.updated_at = secs;
        self.update_sequence += 1;
        self.update_integrity_hash();

        Ok(format!("Burned NFT {}", token_id))
    }

    /**
     * Emite evento Transfer para NFT
     */
    fn emit_nft_transfer_event(&mut self, from: &str, to: &str, token_id: u64) {
        const MAX_EVENTS: usize = 1000;
        
        if self.state.metadata.len() >= MAX_EVENTS {
            let event_keys: Vec<String> = self.state.metadata.keys()
                .filter(|k| k.starts_with("event_"))
                .cloned()
                .collect();
            
            if event_keys.len() > 500 {
                let to_remove = event_keys.len() - 500;
                for key in event_keys.iter().take(to_remove) {
                    self.state.metadata.remove(key);
                }
            }
        }
        
        let event_key = format!("event_nft_transfer_{}", self.update_sequence);
        let event_value = format!("from:{}|to:{}|token_id:{}", from, to, token_id);
        self.state.metadata.insert(event_key, event_value);
    }

    /**
     * Emite evento Approval para NFT
     */
    fn emit_nft_approval_event(&mut self, owner: &str, approved: &str, token_id: u64) {
        const MAX_EVENTS: usize = 1000;
        
        if self.state.metadata.len() >= MAX_EVENTS {
            let event_keys: Vec<String> = self.state.metadata.keys()
                .filter(|k| k.starts_with("event_"))
                .cloned()
                .collect();
            
            if event_keys.len() > 500 {
                let to_remove = event_keys.len() - 500;
                for key in event_keys.iter().take(to_remove) {
                    self.state.metadata.remove(key);
                }
            }
        }
        
        let event_key = format!("event_nft_approval_{}", self.update_sequence);
        let event_value = format!("owner:{}|approved:{}|token_id:{}", owner, approved, token_id);
        self.state.metadata.insert(event_key, event_value);
    }

    /**
     * ERC-20: Obtiene el supply total
     */
    pub fn total_supply(&self) -> u64 {
        self.total_supply.unwrap_or_else(|| self.get_current_supply())
    }

    /**
     * Obtiene el supply total actual (suma de balances)
     */
    pub fn get_current_supply(&self) -> u64 {
        self.state.balances.values().sum()
    }

    /**
     * ERC-20: Obtiene el nombre del token
     */
    pub fn name(&self) -> &str {
        &self.name
    }

    /**
     * ERC-20: Obtiene el símbolo del token
     */
    pub fn symbol(&self) -> Option<&str> {
        self.symbol.as_deref()
    }

    /**
     * ERC-20: Obtiene los decimales del token
     */
    pub fn decimals(&self) -> Option<u8> {
        self.decimals
    }

    /**
     * Calcula el hash de integridad del contrato
     * Optimizado: solo serializa campos críticos, no metadata completa
     */
    pub fn calculate_hash(&self) -> String {
        eprintln!("[HASH] Iniciando calculate_hash()");
        use serde_json;
        let mut hasher = Sha256::new();
        
        // Serializar solo campos críticos (balances, allowances, NFT data, no metadata completa)
        // Esto mejora performance al evitar serializar eventos históricos
        eprintln!("[HASH] Serializando balances...");
        let balances_json = serde_json::to_string(&self.state.balances).unwrap_or_default();
        eprintln!("[HASH] Serializando allowances...");
        let allowances_json = serde_json::to_string(&self.state.allowances).unwrap_or_default();
        eprintln!("[HASH] Serializando token_owners...");
        let token_owners_json = serde_json::to_string(&self.state.token_owners).unwrap_or_default();
        eprintln!("[HASH] Serializando token_uris...");
        let token_uris_json = serde_json::to_string(&self.state.token_uris).unwrap_or_default();
        eprintln!("[HASH] Serializando token_approvals...");
        let token_approvals_json = serde_json::to_string(&self.state.token_approvals).unwrap_or_default();
        eprintln!("[HASH] Serializando nft_balances...");
        let nft_balances_json = serde_json::to_string(&self.state.nft_balances).unwrap_or_default();
        eprintln!("[HASH] Serializando nft_metadata...");
        let nft_metadata_json = serde_json::to_string(&self.state.nft_metadata).unwrap_or_default();
        eprintln!("[HASH] Serializando owner_to_tokens...");
        let owner_to_tokens_json = serde_json::to_string(&self.state.owner_to_tokens).unwrap_or_default();
        eprintln!("[HASH] Serializando token_index...");
        let token_index_json = serde_json::to_string(&self.state.token_index).unwrap_or_default();
        eprintln!("[HASH] Creando string de datos...");
        
        let data = format!(
            "{}{}{}{}{:?}{:?}{:?}{}{}{}{}{}{}{}{}{}{}{}{}",
            self.address,
            self.owner,
            self.contract_type,
            self.name,
            self.symbol,
            self.total_supply,
            self.decimals,
            balances_json,
            allowances_json,
            token_owners_json,
            token_uris_json,
            token_approvals_json,
            nft_balances_json,
            nft_metadata_json,
            owner_to_tokens_json,
            token_index_json,
            self.created_at,
            self.updated_at,
            self.update_sequence
        );
        
        eprintln!("[HASH] Calculando hash SHA256...");
        hasher.update(data.as_bytes());
        let hash = hasher.finalize();
        eprintln!("[HASH] Hash calculado exitosamente");
        format!("{:x}", hash)
    }

    /**
     * Valida el hash de integridad del contrato
     */
    pub fn validate_integrity(&self) -> bool {
        if let Some(stored_hash) = &self.integrity_hash {
            let calculated_hash = self.calculate_hash();
            stored_hash == &calculated_hash
        } else {
            // Si no tiene hash, calcularlo y actualizarlo
            false
        }
    }

    /**
     * Actualiza el hash de integridad después de una modificación
     */
    fn update_integrity_hash(&mut self) {
        self.integrity_hash = Some(self.calculate_hash());
    }

    /**
     * Obtiene timestamp con nanosegundos para mayor precisión
     */
    fn get_timestamp_nanos() -> (u64, u64) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();
        (now.as_secs(), now.subsec_nanos() as u64)
    }

    /**
     * Valida que el owner del contrato no haya cambiado ilegalmente
     */
    pub fn validate_owner(&self, expected_owner: &str) -> bool {
        self.owner == expected_owner
    }
}

/**
 * Gestor de smart contracts
 */
pub struct ContractManager {
    contracts: HashMap<String, SmartContract>,
}

impl ContractManager {
    pub fn new() -> Self {
        ContractManager {
            contracts: HashMap::new(),
        }
    }

    /**
     * Despliega un nuevo contrato
     */
    pub fn deploy_contract(&mut self, contract: SmartContract) -> Result<String, String> {
        if self.contracts.contains_key(&contract.address) {
            return Err("Contract address already exists".to_string());
        }

        let address = contract.address.clone();
        self.contracts.insert(address.clone(), contract);
        Ok(address)
    }

    /**
     * Obtiene un contrato por dirección
     */
    pub fn get_contract(&self, address: &str) -> Option<&SmartContract> {
        self.contracts.get(address)
    }

    /**
     * Obtiene un contrato mutable por dirección
     */
    pub fn get_contract_mut(&mut self, address: &str) -> Option<&mut SmartContract> {
        self.contracts.get_mut(address)
    }

    /**
     * Ejecuta una función en un contrato
     */
    pub fn execute_contract_function(
        &mut self,
        contract_address: &str,
        function: ContractFunction,
        caller: Option<&str>,
    ) -> Result<String, String> {
        let contract = self
            .get_contract_mut(contract_address)
            .ok_or_else(|| "Contract not found".to_string())?;

        contract.execute(function, caller)
    }

    /**
     * Obtiene todos los contratos
     */
    pub fn get_all_contracts(&self) -> Vec<&SmartContract> {
        self.contracts.values().collect()
    }

    /**
     * Obtiene contratos por owner
     */
    pub fn get_contracts_by_owner(&self, owner: &str) -> Vec<&SmartContract> {
        self.contracts
            .values()
            .filter(|c| c.owner == owner)
            .collect()
    }

    /**
     * Obtiene contratos por tipo
     */
    pub fn get_contracts_by_type(&self, contract_type: &str) -> Vec<&SmartContract> {
        self.contracts
            .values()
            .filter(|c| c.contract_type == contract_type)
            .collect()
    }
}

impl Default for ContractManager {
    fn default() -> Self {
        Self::new()
    }
}

