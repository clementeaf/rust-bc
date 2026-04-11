// Standard library
use std::collections::HashMap;
use std::time::Instant;

// External crates
use rayon::prelude::*;

// Crate modules
use crate::airdrop::NodeTracking;
use crate::blockchain::Block;
use crate::models::Transaction;
use crate::smart_contracts::SmartContract;
use crate::staking::{StakingManager, Validator};

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
#[derive(Debug, Clone, Default)]
pub struct WalletState {
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
     *
     * OPTIMIZACIONES IMPLEMENTADAS:
     * - Procesamiento paralelo de bloques usando rayon
     * - Procesamiento en batch para reducir overhead
     * - Métricas de tiempo de ejecución
     * - Progreso incremental mejorado
     *
     * @param chain - Cadena de bloques completa
     * @returns ReconstructedState con todo el estado reconstruido
     */
    pub fn from_blockchain(chain: &[Block]) -> Self {
        let start_time = Instant::now();
        let total = chain.len();

        if total == 0 {
            return ReconstructedState::new();
        }

        if total > 100 {
            println!("🔄 Reconstruyendo estado desde {total} bloques...");
        }

        // Para cadenas pequeñas (< 1000 bloques), procesar secuencialmente
        // Para cadenas grandes, usar procesamiento paralelo en chunks
        let state = if total < 1000 {
            Self::from_blockchain_sequential(chain)
        } else {
            Self::from_blockchain_parallel(chain)
        };

        let elapsed = start_time.elapsed();
        if total > 100 {
            println!(
                "✅ Estado reconstruido: {} bloques procesados en {:.2}s",
                total,
                elapsed.as_secs_f64()
            );
        }

        state
    }

    /**
     * Reconstrucción secuencial (para cadenas pequeñas)
     * @param chain - Cadena de bloques completa
     * @returns ReconstructedState reconstruido
     */
    fn from_blockchain_sequential(chain: &[Block]) -> Self {
        let mut state = ReconstructedState::new();
        let total = chain.len();

        for (i, block) in chain.iter().enumerate() {
            state.process_block(block);

            // Mostrar progreso cada 1000 bloques
            if total > 1000 && i > 0 && i % 1000 == 0 {
                let progress = (i as f64 / total as f64) * 100.0;
                println!("   Progreso: {progress:.1}% ({i}/{total})");
            }
        }

        state
    }

    /**
     * Reconstrucción paralela optimizada (para cadenas grandes)
     *
     * ESTRATEGIA DE PARALELISMO:
     * 1. Dividir bloques en chunks de tamaño fijo
     * 2. Procesar cada chunk en paralelo (dentro del chunk, procesar secuencialmente)
     * 3. Mergear resultados de chunks en orden cronológico
     *
     * VENTAJAS:
     * - Mantiene orden cronológico (cada chunk procesa bloques en orden)
     * - Aprovecha múltiples cores para procesar chunks diferentes
     * - Merge final es rápido (solo combina HashMaps)
     *
     * @param chain - Cadena de bloques completa
     * @returns ReconstructedState reconstruido
     */
    fn from_blockchain_parallel(chain: &[Block]) -> Self {
        let total = chain.len();
        // Tamaño de chunk: balance entre paralelismo y overhead
        // Chunks más pequeños = más paralelismo pero más overhead de merge
        // Chunks más grandes = menos paralelismo pero menos overhead
        const CHUNK_SIZE: usize = 500; // Procesar 500 bloques por chunk

        if total <= CHUNK_SIZE {
            // Si cabe en un chunk, procesar secuencialmente
            return Self::from_blockchain_sequential(chain);
        }

        // Dividir en chunks y procesar en paralelo
        let chunk_results: Vec<_> = chain
            .chunks(CHUNK_SIZE)
            .enumerate()
            .par_bridge()
            .map(|(chunk_idx, chunk)| {
                // Procesar chunk secuencialmente (mantiene orden dentro del chunk)
                let mut chunk_state = ReconstructedState::new();
                for block in chunk {
                    chunk_state.process_block(block);
                }

                // Mostrar progreso periódicamente
                let processed = (chunk_idx + 1) * CHUNK_SIZE.min(chunk.len());
                if processed % 2000 == 0 || processed >= total {
                    let progress = (processed as f64 / total as f64) * 100.0;
                    println!(
                        "   Progreso: {:.1}% ({}/{})",
                        progress,
                        processed.min(total),
                        total
                    );
                }

                (chunk_idx, chunk_state)
            })
            .collect();

        // Ordenar chunks por índice para mantener orden cronológico
        let mut sorted_chunks: Vec<_> = chunk_results.into_iter().collect();
        sorted_chunks.sort_by_key(|(idx, _)| *idx);

        // Mergear estados de chunks en orden
        // IMPORTANTE: El merge debe ser correcto porque cada chunk tiene estado acumulativo
        let mut final_state = ReconstructedState::new();
        for (_, chunk_state) in sorted_chunks {
            final_state.merge_ordered(chunk_state);
        }

        final_state
    }

    /**
     * Mergea otro estado en este estado manteniendo orden cronológico
     *
     * ESTRATEGIA:
     * - Wallets: Sumar balances (cada chunk procesó transacciones independientes)
     * - Validators: Sumar stakes y actualizar estado
     * - Airdrop: Combinar tracking (sumar bloques validados)
     * - Contracts: Extender (último estado gana)
     *
     * @param other - Estado del siguiente chunk a mergear
     */
    fn merge_ordered(&mut self, other: Self) {
        // Mergear wallets: sumar balances porque cada chunk procesó transacciones independientes
        for (addr, wallet_state) in other.wallets {
            let wallet = self.wallets.entry(addr).or_default();
            wallet.balance += wallet_state.balance;
        }

        // Mergear validadores: sumar stakes y actualizar estado activo
        for (addr, validator) in other.validators {
            let existing = self
                .validators
                .entry(addr.clone())
                .or_insert_with(|| Validator {
                    address: addr.clone(),
                    staked_amount: 0,
                    is_active: false,
                    total_rewards: 0,
                    created_at: validator.created_at,
                    last_validated_block: 0,
                    validation_count: 0,
                    slash_count: 0,
                    unstaking_requested: false,
                    unstaking_timestamp: None,
                });
            existing.staked_amount += validator.staked_amount;
            if existing.staked_amount >= 1000 {
                existing.is_active = true;
            }
            // Mantener el timestamp más antiguo (primera creación)
            if validator.created_at < existing.created_at {
                existing.created_at = validator.created_at;
            }
        }

        // Mergear airdrop tracking: combinar información de nodos
        for (addr, tracking) in other.airdrop_tracking {
            let existing = self
                .airdrop_tracking
                .entry(addr.clone())
                .or_insert_with(|| NodeTracking {
                    node_address: addr,
                    first_block_index: tracking.first_block_index,
                    first_block_timestamp: tracking.first_block_timestamp,
                    blocks_validated: 0,
                    last_block_timestamp: tracking.last_block_timestamp,
                    is_eligible: false,
                    airdrop_claimed: false,
                    claim_timestamp: None,
                    claim_transaction_id: None,
                    claim_block_index: None,
                    claim_verified: false,
                    uptime_seconds: tracking.uptime_seconds,
                    eligibility_tier: 0,
                });
            // Sumar bloques validados
            existing.blocks_validated += tracking.blocks_validated;
            // Mantener el índice más antiguo (primer bloque)
            if tracking.first_block_index < existing.first_block_index {
                existing.first_block_index = tracking.first_block_index;
                existing.first_block_timestamp = tracking.first_block_timestamp;
            }
            // Mantener el timestamp más reciente (último bloque)
            if tracking.last_block_timestamp > existing.last_block_timestamp {
                existing.last_block_timestamp = tracking.last_block_timestamp;
            }
            // Actualizar uptime
            if existing.first_block_timestamp > 0 {
                existing.uptime_seconds =
                    existing.last_block_timestamp - existing.first_block_timestamp;
            }
        }

        // Mergear contratos: extender (último estado gana)
        self.contracts.extend(other.contracts);
    }

    /**
     * Procesa un bloque y actualiza el estado
     *
     * OPTIMIZACIÓN: Procesa transacciones en batch para reducir overhead
     *
     * @param block - Bloque a procesar
     */
    fn process_block(&mut self, block: &Block) {
        // Procesar transacciones en batch
        // Agrupar por tipo para optimizar procesamiento
        let mut coinbase_txs = Vec::new();
        let mut staking_txs = Vec::new();
        let mut unstaking_txs = Vec::new();
        let mut normal_txs = Vec::new();

        for tx in &block.transactions {
            if tx.from == "0" {
                coinbase_txs.push(tx);
            } else if tx.from == "STAKING" {
                unstaking_txs.push(tx);
            } else if tx.to == "STAKING" {
                staking_txs.push(tx);
            } else {
                normal_txs.push(tx);
            }
        }

        // Procesar cada tipo de transacción
        for tx in coinbase_txs {
            self.process_transaction(tx, block);
        }
        for tx in unstaking_txs {
            self.process_transaction(tx, block);
        }
        for tx in staking_txs {
            self.process_transaction(tx, block);
        }
        for tx in normal_txs {
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
            let wallet = self.wallets.entry(tx.to.clone()).or_default();
            wallet.balance += tx.amount;
        } else if tx.from == "STAKING" {
            // Unstaking transaction
            let wallet = self.wallets.entry(tx.to.clone()).or_default();
            wallet.balance += tx.amount;
        } else if tx.to == "STAKING" {
            // Staking transaction
            let wallet = self.wallets.entry(tx.from.clone()).or_default();
            wallet.balance = wallet.balance.saturating_sub(tx.amount + tx.fee);

            // Reconstruir validador desde transacción de staking
            self.reconstruct_validator_from_staking(&tx.from, tx.amount, block);
        } else {
            // Transferencia normal
            let from_wallet = self.wallets.entry(tx.from.clone()).or_default();
            from_wallet.balance = from_wallet.balance.saturating_sub(tx.amount + tx.fee);

            let to_wallet = self.wallets.entry(tx.to.clone()).or_default();
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
        let validator = self
            .validators
            .entry(address.to_string())
            .or_insert_with(|| Validator {
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
                let tracking = self
                    .airdrop_tracking
                    .entry(tx.to.clone())
                    .or_insert_with(|| NodeTracking {
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
     * Carga validadores en el StakingManager
     * @param staking_manager - StakingManager a poblar
     */
    #[allow(dead_code)]
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
