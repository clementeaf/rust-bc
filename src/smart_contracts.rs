use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
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
    Custom {
        name: String,
        params: Vec<String>,
    },
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
}

impl ContractState {
    pub fn new() -> Self {
        ContractState {
            balances: HashMap::new(),
            metadata: HashMap::new(),
            allowances: HashMap::new(),
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
        use serde_json;
        let mut hasher = Sha256::new();
        
        // Serializar solo campos críticos (balances y allowances, no metadata completa)
        // Esto mejora performance al evitar serializar eventos históricos
        let balances_json = serde_json::to_string(&self.state.balances).unwrap_or_default();
        let allowances_json = serde_json::to_string(&self.state.allowances).unwrap_or_default();
        
        let data = format!(
            "{}{}{}{}{:?}{:?}{:?}{}{}{}{}{}",
            self.address,
            self.owner,
            self.contract_type,
            self.name,
            self.symbol,
            self.total_supply,
            self.decimals,
            balances_json,
            allowances_json,
            self.created_at,
            self.updated_at,
            self.update_sequence
        );
        
        hasher.update(data.as_bytes());
        let hash = hasher.finalize();
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

