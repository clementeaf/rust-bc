use crate::airdrop::NodeTracking;
use crate::blockchain::Block;
use crate::models::Transaction;
use crate::smart_contracts::{ContractManager, SmartContract};
use crate::staking::{StakingManager, Validator};
use std::collections::HashMap;

/**
 * Estado reconstruido desde la blockchain
 */
pub struct ReconstructedState {
    pub wallets: HashMap<String, WalletState>,
    pub contracts: HashMap<String, SmartContract>,
    pub validators: HashMap<String, Validator>,
    pub airdrop_tracking: HashMap<String, NodeTracking>,
}

/**
 * Estado de un wallet reconstruido
 */
#[derive(Debug, Clone)]
pub struct WalletState {
    pub address: String,
    pub balance: u64,
}

impl ReconstructedState {
    /**
     * Crea un nuevo estado vacío
     * @returns ReconstructedState vacío
     */
    pub fn new() -> Self {
        ReconstructedState {
            wallets: HashMap::new(),
            contracts: HashMap::new(),
            validators: HashMap::new(),
            airdrop_tracking: HashMap::new(),
        }
    }

    /**
     * Reconstruye el estado completo desde la blockchain
     * @param chain - Cadena de bloques completa
     * @returns ReconstructedState con todo el estado reconstruido
     */
    pub fn from_blockchain(chain: &[Block]) -> Self {
        let mut state = ReconstructedState::new();
        
        // Procesar cada bloque desde génesis
        for block in chain {
            state.process_block(block);
        }
        
        state
    }

    /**
     * Procesa un bloque y actualiza el estado
     * @param block - Bloque a procesar
     */
    fn process_block(&mut self, block: &Block) {
        for tx in &block.transactions {
            self.process_transaction(tx, block);
        }
        
        // Reconstruir tracking de airdrop desde bloques minados
        self.reconstruct_airdrop_from_block(block);
    }

    /**
     * Procesa una transacción y actualiza el estado
     * @param tx - Transacción a procesar
     * @param block - Bloque que contiene la transacción
     */
    fn process_transaction(&mut self, tx: &Transaction, block: &Block) {
        // Procesar transacciones normales (wallets)
        if tx.from == "0" {
            // Coinbase transaction
            let wallet = self.wallets.entry(tx.to.clone()).or_insert_with(|| WalletState {
                address: tx.to.clone(),
                balance: 0,
            });
            wallet.balance += tx.amount;
        } else if tx.from == "STAKING" {
            // Unstaking transaction
            let wallet = self.wallets.entry(tx.to.clone()).or_insert_with(|| WalletState {
                address: tx.to.clone(),
                balance: 0,
            });
            wallet.balance += tx.amount;
        } else if tx.to == "STAKING" {
            // Staking transaction
            let wallet = self.wallets.entry(tx.from.clone()).or_insert_with(|| WalletState {
                address: tx.from.clone(),
                balance: 0,
            });
            wallet.balance = wallet.balance.saturating_sub(tx.amount + tx.fee);
            
            // Reconstruir validador desde transacción de staking
            self.reconstruct_validator_from_staking(&tx.from, tx.amount, block);
        } else {
            // Transferencia normal
            let from_wallet = self.wallets.entry(tx.from.clone()).or_insert_with(|| WalletState {
                address: tx.from.clone(),
                balance: 0,
            });
            from_wallet.balance = from_wallet.balance.saturating_sub(tx.amount + tx.fee);
            
            let to_wallet = self.wallets.entry(tx.to.clone()).or_insert_with(|| WalletState {
                address: tx.to.clone(),
                balance: 0,
            });
            to_wallet.balance += tx.amount;
        }
        
        // Reconstruir contratos desde transacciones
        // Nota: Los contratos se despliegan directamente, no a través de transacciones
        // Por ahora, los contratos se reconstruyen desde el ContractManager en memoria
        // Esto se manejará en la fase de migración
    }

    /**
     * Reconstruye un validador desde una transacción de staking
     * @param address - Dirección del validador
     * @param amount - Cantidad stakeada
     * @param block - Bloque que contiene la transacción
     */
    fn reconstruct_validator_from_staking(&mut self, address: &str, amount: u64, block: &Block) {
        let validator = self.validators.entry(address.to_string()).or_insert_with(|| Validator {
            address: address.to_string(),
            staked_amount: 0,
            is_active: false,
            total_rewards: 0,
            created_at: block.timestamp,
            last_validated_block: 0,
            validation_count: 0,
            slash_count: 0,
            unstaking_requested: false,
            unstaking_timestamp: None,
        });
        
        validator.staked_amount += amount;
        if validator.staked_amount >= 1000 {
            validator.is_active = true;
        }
    }

    /**
     * Reconstruye tracking de airdrop desde un bloque minado
     * @param block - Bloque minado
     */
    fn reconstruct_airdrop_from_block(&mut self, block: &Block) {
        // Buscar transacción coinbase para identificar el minero
        for tx in &block.transactions {
            if tx.from == "0" {
                // Esta es una coinbase transaction
                // El minero es tx.to
                let tracking = self.airdrop_tracking.entry(tx.to.clone()).or_insert_with(|| NodeTracking {
                    node_address: tx.to.clone(),
                    first_block_index: block.index,
                    first_block_timestamp: block.timestamp,
                    blocks_validated: 0,
                    last_block_timestamp: block.timestamp,
                    is_eligible: false,
                    airdrop_claimed: false,
                    claim_timestamp: None,
                    claim_transaction_id: None,
                    claim_block_index: None,
                    claim_verified: false,
                    uptime_seconds: 0,
                    eligibility_tier: 0,
                });
                
                tracking.blocks_validated += 1;
                tracking.last_block_timestamp = block.timestamp;
                
                // Calcular uptime
                if tracking.first_block_timestamp > 0 {
                    tracking.uptime_seconds = block.timestamp - tracking.first_block_timestamp;
                }
            }
        }
    }

    /**
     * Reconstruye contratos desde el ContractManager en memoria
     * Nota: Los contratos se despliegan directamente, no a través de transacciones
     * Por ahora, esto se maneja manteniendo el ContractManager en memoria
     * En el futuro, podríamos incluir el estado de contratos en los bloques
     * @param contract_manager - ContractManager con contratos en memoria
     */
    pub fn reconstruct_contracts_from_manager(&mut self, contract_manager: &ContractManager) {
        for contract in contract_manager.get_all_contracts() {
            self.contracts.insert(contract.address.clone(), contract.clone());
        }
    }

    /**
     * Carga validadores en el StakingManager
     * @param staking_manager - StakingManager a poblar
     */
    pub fn load_validators_into_staking(&self, staking_manager: &mut StakingManager) {
        for validator in self.validators.values() {
            staking_manager.load_validators(vec![validator.clone()]);
        }
    }

    /**
     * Obtiene el tracking de airdrop reconstruido
     * @returns HashMap con el tracking de todos los nodos
     */
    pub fn get_airdrop_tracking(&self) -> &HashMap<String, NodeTracking> {
        &self.airdrop_tracking
    }
}

impl Default for ReconstructedState {
    fn default() -> Self {
        Self::new()
    }
}

