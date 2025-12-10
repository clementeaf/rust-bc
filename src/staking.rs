use crate::models::WalletManager;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/**
 * Representa un validador en el sistema PoS
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    pub address: String,
    pub staked_amount: u64,
    pub is_active: bool,
    pub total_rewards: u64,
    pub created_at: u64,
    pub last_validated_block: u64,
    pub validation_count: u64,
    pub slash_count: u64,
    pub unstaking_requested: bool,
    pub unstaking_timestamp: Option<u64>,
}

impl Validator {
    /**
     * Crea un nuevo validador
     * @param address - Dirección del validador
     * @param staked_amount - Cantidad de tokens staked
     * @returns Nuevo validador
     */
    pub fn new(address: String, staked_amount: u64) -> Validator {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Validator {
            address,
            staked_amount,
            is_active: true,
            total_rewards: 0,
            created_at: timestamp,
            last_validated_block: 0,
            validation_count: 0,
            slash_count: 0,
            unstaking_requested: false,
            unstaking_timestamp: None,
        }
    }

    /**
     * Verifica si el validador tiene el stake mínimo requerido
     * @param min_stake - Stake mínimo requerido (32 o 1000 NOTA)
     * @returns true si cumple con el mínimo
     */
    pub fn has_minimum_stake(&self, min_stake: u64) -> bool {
        self.staked_amount >= min_stake
    }

    /**
     * Agrega recompensa al validador
     * @param reward - Cantidad de recompensa
     */
    pub fn add_reward(&mut self, reward: u64) {
        self.total_rewards += reward;
    }

    /**
     * Incrementa el contador de validaciones
     */
    pub fn increment_validation(&mut self, block_index: u64) {
        self.validation_count += 1;
        self.last_validated_block = block_index;
    }

    /**
     * Aplica slashing (penalización)
     * @param slash_amount - Cantidad a slashear
     * @returns Cantidad realmente slasheada
     */
    pub fn slash(&mut self, slash_amount: u64) -> u64 {
        self.slash_count += 1;
        let actual_slash = slash_amount.min(self.staked_amount);
        self.staked_amount -= actual_slash;

        // Si el stake cae por debajo del mínimo después de slashing, desactivar
        if self.staked_amount < 1000 {
            // min_stake típico
            self.is_active = false;
        }

        actual_slash
    }
}

/**
 * Gestor del sistema de staking
 */
pub struct StakingManager {
    validators: Arc<Mutex<HashMap<String, Validator>>>,
    min_stake: u64,
    unstaking_period: u64, // Período de lock en segundos (ej: 7 días = 604800)
    slash_percentage: u8,  // Porcentaje de slashing (ej: 5%)
}

impl StakingManager {
    /**
     * Crea un nuevo gestor de staking
     * @param min_stake - Stake mínimo requerido para ser validador (default: 1000)
     * @param unstaking_period - Período de lock para unstaking en segundos (default: 7 días)
     * @param slash_percentage - Porcentaje de slashing (default: 5%)
     * @returns Nuevo gestor de staking
     */
    pub fn new(
        min_stake: Option<u64>,
        unstaking_period: Option<u64>,
        slash_percentage: Option<u8>,
    ) -> StakingManager {
        StakingManager {
            validators: Arc::new(Mutex::new(HashMap::new())),
            min_stake: min_stake.unwrap_or(1000),
            unstaking_period: unstaking_period.unwrap_or(604800), // 7 días por defecto
            slash_percentage: slash_percentage.unwrap_or(5),
        }
    }

    /**
     * Stakes tokens para convertirse en validador
     * @param address - Dirección del validador
     * @param amount - Cantidad de tokens a stakear
     * @param wallet_manager - Gestor de wallets para validar balance
     * @returns Resultado de la operación
     */
    pub fn stake(
        &self,
        address: &str,
        amount: u64,
        wallet_manager: &WalletManager,
    ) -> Result<(), String> {
        if amount < self.min_stake {
            return Err(format!(
                "Stake mínimo requerido: {} tokens (intentaste stakear: {})",
                self.min_stake, amount
            ));
        }

        // Verificar que el wallet existe
        wallet_manager
            .get_wallet(address)
            .ok_or_else(|| "Wallet no encontrado".to_string())?;

        // Nota: El balance se verifica en la blockchain, no en el wallet manager
        // Aquí solo verificamos que el wallet existe

        let mut validators = self.validators.lock().unwrap();

        // Si ya es validador, agregar al stake existente
        if let Some(validator) = validators.get_mut(address) {
            if validator.unstaking_requested {
                return Err("No puedes stakear mientras tienes un unstaking pendiente".to_string());
            }
            validator.staked_amount += amount;
        } else {
            // Crear nuevo validador
            let validator = Validator::new(address.to_string(), amount);
            validators.insert(address.to_string(), validator);
        }

        Ok(())
    }

    /**
     * Solicita unstaking (retiro de tokens)
     * @param address - Dirección del validador
     * @param amount - Cantidad a retirar (opcional, si es None retira todo)
     * @returns Resultado de la operación
     */
    pub fn request_unstake(&self, address: &str, amount: Option<u64>) -> Result<u64, String> {
        let mut validators = self.validators.lock().unwrap();
        let validator = validators
            .get_mut(address)
            .ok_or_else(|| "No eres un validador".to_string())?;

        if validator.unstaking_requested {
            return Err("Ya tienes un unstaking pendiente".to_string());
        }

        let unstake_amount = amount.unwrap_or(validator.staked_amount);
        if unstake_amount > validator.staked_amount {
            return Err("No puedes retirar más de lo que tienes staked".to_string());
        }

        // Si retira todo y queda por debajo del mínimo, desactivar
        if validator.staked_amount - unstake_amount < self.min_stake {
            validator.is_active = false;
        }

        validator.unstaking_requested = true;
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        validator.unstaking_timestamp = Some(timestamp);

        Ok(unstake_amount)
    }

    /**
     * Completa el unstaking después del período de lock
     * @param address - Dirección del validador
     * @returns Cantidad retirada
     */
    pub fn complete_unstake(&self, address: &str) -> Result<u64, String> {
        let mut validators = self.validators.lock().unwrap();
        let validator = validators
            .get_mut(address)
            .ok_or_else(|| "No eres un validador".to_string())?;

        if !validator.unstaking_requested {
            return Err("No tienes un unstaking pendiente".to_string());
        }

        let unstaking_timestamp = validator
            .unstaking_timestamp
            .ok_or_else(|| "Timestamp de unstaking no encontrado".to_string())?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now < unstaking_timestamp + self.unstaking_period {
            return Err(format!(
                "Período de lock aún no ha terminado. Espera {} segundos más",
                (unstaking_timestamp + self.unstaking_period) - now
            ));
        }

        // Calcular cantidad a retirar (todo lo que tiene staked)
        let unstake_amount = validator.staked_amount;

        // Si queda por debajo del mínimo, remover validador
        if unstake_amount >= validator.staked_amount {
            validators.remove(address);
        } else {
            validator.staked_amount -= unstake_amount;
            validator.unstaking_requested = false;
            validator.unstaking_timestamp = None;
        }

        Ok(unstake_amount)
    }

    /**
     * Selecciona un validador aleatorio ponderado por stake
     * @param block_hash - Hash del bloque anterior para aleatoriedad determinística
     * @returns Dirección del validador seleccionado o None
     */
    pub fn select_validator(&self, block_hash: &str) -> Option<String> {
        let validators = self.validators.lock().unwrap();

        // Filtrar solo validadores activos
        let active_validators: Vec<(&String, &Validator)> = validators
            .iter()
            .filter(|(_, v)| {
                v.is_active && !v.unstaking_requested && v.has_minimum_stake(self.min_stake)
            })
            .collect();

        if active_validators.is_empty() {
            return None;
        }

        // Calcular stake total
        let total_stake: u64 = active_validators.iter().map(|(_, v)| v.staked_amount).sum();

        if total_stake == 0 {
            return None;
        }

        // Usar hash del bloque anterior para aleatoriedad determinística
        let mut hasher = Sha256::new();
        hasher.update(block_hash.as_bytes());
        let hash_bytes = hasher.finalize();

        // Convertir primeros 8 bytes a u64
        let mut random_value = 0u64;
        for i in 0..8 {
            random_value = (random_value << 8) | (hash_bytes[i] as u64);
        }
        let random_stake = random_value % total_stake;

        // Seleccionar validador ponderado
        let mut cumulative_stake = 0u64;
        for (address, validator) in &active_validators {
            cumulative_stake += validator.staked_amount;
            if random_stake < cumulative_stake {
                return Some((*address).clone());
            }
        }

        // Fallback: retornar el primero (no debería llegar aquí)
        active_validators.first().map(|(addr, _)| (*addr).clone())
    }

    /**
     * Obtiene un validador por dirección
     * @param address - Dirección del validador
     * @returns Validator o None
     */
    pub fn get_validator(&self, address: &str) -> Option<Validator> {
        let validators = self.validators.lock().unwrap();
        validators.get(address).cloned()
    }

    /**
     * Obtiene todos los validadores activos
     * @returns Lista de validadores activos
     */
    pub fn get_active_validators(&self) -> Vec<Validator> {
        let validators = self.validators.lock().unwrap();
        validators
            .values()
            .filter(|v| {
                v.is_active && !v.unstaking_requested && v.has_minimum_stake(self.min_stake)
            })
            .cloned()
            .collect()
    }

    /**
     * Carga validadores desde la base de datos
     * @param validators_from_db - Lista de validadores desde la base de datos
     */
    pub fn load_validators(&self, validators_from_db: Vec<Validator>) {
        let mut validators = self.validators.lock().unwrap();
        for validator in validators_from_db {
            validators.insert(validator.address.clone(), validator);
        }
    }

    /**
     * Registra una validación exitosa
     * @param address - Dirección del validador
     * @param block_index - Índice del bloque validado
     * @param reward - Recompensa por validar
     */
    pub fn record_validation(&self, address: &str, block_index: u64, reward: u64) {
        let mut validators = self.validators.lock().unwrap();
        if let Some(validator) = validators.get_mut(address) {
            validator.increment_validation(block_index);
            validator.add_reward(reward);
        }
    }

    /**
     * Detecta y aplica slashing por doble firma
     * Si un validador firma dos bloques en el mismo índice, se aplica slashing
     * @param validator_address - Dirección del validador
     * @param block_index - Índice del bloque
     * @param block_hash - Hash del bloque firmado
     * @returns true si se detectó y aplicó slashing
     */
    pub fn detect_and_slash_double_sign(
        &self,
        validator_address: &str,
        block_index: u64,
        _block_hash: &str,
    ) -> bool {
        let mut validators = self.validators.lock().unwrap();

        if let Some(validator) = validators.get_mut(validator_address) {
            // Verificar si ya validó este índice de bloque (doble firma)
            if validator.last_validated_block == block_index {
                // Doble firma detectada: aplicar slashing
                let slash_amount = (validator.staked_amount * self.slash_percentage as u64) / 100;
                let slashed = validator.slash(slash_amount);

                eprintln!(
                    "⚡ SLASHING aplicado a {}: {} tokens slasheados por doble firma en bloque {}",
                    validator_address, slashed, block_index
                );

                return true;
            }
        }

        false
    }
}
