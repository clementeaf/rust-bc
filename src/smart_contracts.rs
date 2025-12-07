use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

/**
 * Tipos de funciones de contrato soportadas
 */
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContractFunction {
    Transfer {
        from: String,
        to: String,
        amount: u64,
    },
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
}

impl ContractState {
    pub fn new() -> Self {
        ContractState {
            balances: HashMap::new(),
            metadata: HashMap::new(),
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
     */
    pub fn execute(&mut self, function: ContractFunction) -> Result<String, String> {
        match function {
            ContractFunction::Transfer { from, to, amount } => {
                self.transfer(&from, &to, amount)
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
     * Transfiere tokens entre direcciones
     */
    fn transfer(&mut self, from: &str, to: &str, amount: u64) -> Result<String, String> {
        if amount == 0 {
            return Err("Amount must be greater than 0".to_string());
        }

        let from_balance = *self.state.balances.get(from).unwrap_or(&0);
        if from_balance < amount {
            return Err("Insufficient balance".to_string());
        }

        let to_balance = *self.state.balances.get(to).unwrap_or(&0);

        self.state.balances.insert(from.to_string(), from_balance - amount);
        self.state.balances.insert(to.to_string(), to_balance + amount);
        let (secs, _) = Self::get_timestamp_nanos();
        self.updated_at = secs;
        self.update_sequence += 1;
        self.update_integrity_hash();

        Ok(format!("Transferred {} from {} to {}", amount, from, to))
    }

    /**
     * Mina nuevos tokens
     */
    fn mint(&mut self, to: &str, amount: u64) -> Result<String, String> {
        if amount == 0 {
            return Err("Amount must be greater than 0".to_string());
        }

        // Verificar límite de supply si existe
        if let Some(max_supply) = self.total_supply {
            let current_supply: u64 = self.state.balances.values().sum();
            if current_supply + amount > max_supply {
                return Err("Minting would exceed total supply".to_string());
            }
        }

        let current_balance = *self.state.balances.get(to).unwrap_or(&0);
        self.state.balances.insert(to.to_string(), current_balance + amount);
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
        if amount == 0 {
            return Err("Amount must be greater than 0".to_string());
        }

        let from_balance = *self.state.balances.get(from).unwrap_or(&0);
        if from_balance < amount {
            return Err("Insufficient balance to burn".to_string());
        }

        self.state.balances.insert(from.to_string(), from_balance - amount);
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
     * Obtiene el balance de una dirección
     */
    pub fn get_balance(&self, address: &str) -> u64 {
        *self.state.balances.get(address).unwrap_or(&0)
    }

    /**
     * Obtiene el supply total actual
     */
    pub fn get_current_supply(&self) -> u64 {
        self.state.balances.values().sum()
    }

    /**
     * Calcula el hash de integridad del contrato
     */
    pub fn calculate_hash(&self) -> String {
        use serde_json;
        let mut hasher = Sha256::new();
        
        // Serializar campos críticos para el hash
        let data = format!(
            "{}{}{}{}{:?}{:?}{:?}{}{}{}{}",
            self.address,
            self.owner,
            self.contract_type,
            self.name,
            self.symbol,
            self.total_supply,
            self.decimals,
            serde_json::to_string(&self.state).unwrap_or_default(),
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
    ) -> Result<String, String> {
        let contract = self
            .get_contract_mut(contract_address)
            .ok_or_else(|| "Contract not found".to_string())?;

        contract.execute(function)
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

