// use crate::database::BlockchainDB; // Eliminado - ya no usamos BD
use crate::models::Transaction;
use rusqlite::Result as SqlResult;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/**
 * Información de tracking de un nodo para airdrop
 */
#[derive(Debug, Clone, Serialize)]
pub struct NodeTracking {
    pub node_address: String,
    pub first_block_index: u64,
    pub first_block_timestamp: u64,
    pub blocks_validated: u64,
    pub last_block_timestamp: u64,
    pub is_eligible: bool,
    pub airdrop_claimed: bool,
    pub claim_timestamp: Option<u64>,
    pub claim_transaction_id: Option<String>,
    pub claim_block_index: Option<u64>,
    pub claim_verified: bool,
    pub uptime_seconds: u64,
    pub eligibility_tier: u8,
}

/**
 * Configuración de elegibilidad para airdrop
 */
#[derive(Debug, Clone)]
pub struct EligibilityConfig {
    pub min_blocks_validated: u64,
    pub min_uptime_seconds: u64,
    pub max_eligible_nodes: u64,
    pub require_active: bool,
}

/**
 * Configuración de fases/tiers de airdrop
 */
#[derive(Debug, Clone, Serialize)]
pub struct AirdropTier {
    pub tier_id: u8,
    pub name: String,
    pub min_block_index: u64,
    pub max_block_index: u64,
    pub base_amount: u64,
    pub bonus_per_block: u64,
    pub bonus_per_uptime_day: u64,
}

/**
 * Gestor del sistema de airdrop
 */
pub struct AirdropManager {
    tracking: Arc<Mutex<HashMap<String, NodeTracking>>>,
    eligibility_config: EligibilityConfig,
    airdrop_wallet: String,
    tiers: Vec<AirdropTier>,
    pending_claims: Arc<Mutex<HashMap<String, (String, u64)>>>, // (node_address, (tx_id, timestamp))
    claim_history: Arc<Mutex<Vec<ClaimRecord>>>,
    rate_limits: Arc<Mutex<HashMap<String, Vec<u64>>>>, // (ip_address, timestamps)
}

impl AirdropManager {
    /**
     * Crea un nuevo gestor de airdrop
     * @param max_eligible_nodes - Número máximo de nodos elegibles (ej: 500)
     * @param airdrop_amount_per_node - Cantidad base de tokens por nodo
     * @param airdrop_wallet - Dirección del wallet que distribuirá los tokens
     */
    pub fn new(
        max_eligible_nodes: u64,
        airdrop_amount_per_node: u64,
        airdrop_wallet: String,
    ) -> AirdropManager {
        let eligibility_config = EligibilityConfig {
            min_blocks_validated: 10,
            min_uptime_seconds: 7 * 24 * 3600, // 7 días
            max_eligible_nodes,
            require_active: true,
        };

        // Crear tiers por defecto
        let tiers = vec![
            AirdropTier {
                tier_id: 1,
                name: "Early Adopter".to_string(),
                min_block_index: 1,
                max_block_index: 100,
                base_amount: airdrop_amount_per_node * 2,
                bonus_per_block: 10,
                bonus_per_uptime_day: 50,
            },
            AirdropTier {
                tier_id: 2,
                name: "Active Participant".to_string(),
                min_block_index: 101,
                max_block_index: 300,
                base_amount: airdrop_amount_per_node,
                bonus_per_block: 5,
                bonus_per_uptime_day: 25,
            },
            AirdropTier {
                tier_id: 3,
                name: "Community Member".to_string(),
                min_block_index: 301,
                max_block_index: max_eligible_nodes,
                base_amount: airdrop_amount_per_node / 2,
                bonus_per_block: 2,
                bonus_per_uptime_day: 10,
            },
        ];

        AirdropManager {
            tracking: Arc::new(Mutex::new(HashMap::new())),
            eligibility_config,
            airdrop_wallet,
            tiers,
            pending_claims: Arc::new(Mutex::new(HashMap::new())),
            claim_history: Arc::new(Mutex::new(Vec::new())),
            rate_limits: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /**
     * Registra que un nodo ha minado/validado un bloque
     * @param node_address - Dirección del nodo/minero
     * @param block_index - Índice del bloque
     * @param block_timestamp - Timestamp del bloque
     */
    pub fn record_block_validation(
        &self,
        node_address: &str,
        block_index: u64,
        block_timestamp: u64,
    ) {
        let mut tracking = self.tracking.lock().unwrap_or_else(|e| e.into_inner());
        let current_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let entry = tracking.entry(node_address.to_string()).or_insert_with(|| {
            NodeTracking {
                node_address: node_address.to_string(),
                first_block_index: block_index,
                first_block_timestamp: block_timestamp,
                blocks_validated: 0,
                last_block_timestamp: block_timestamp,
                is_eligible: false,
                airdrop_claimed: false,
                claim_timestamp: None,
                claim_transaction_id: None,
                claim_block_index: None,
                claim_verified: false,
                uptime_seconds: 0,
                eligibility_tier: 0,
            }
        });

        entry.blocks_validated += 1;
        entry.last_block_timestamp = block_timestamp;
        
        // Calcular uptime real (tiempo desde primer bloque hasta ahora)
        entry.uptime_seconds = current_time.saturating_sub(entry.first_block_timestamp);
        
        // Verificar elegibilidad con criterios robustos
        entry.is_eligible = self.check_eligibility_criteria(entry);
        
        // Determinar tier de elegibilidad
        entry.eligibility_tier = self.determine_tier(entry.first_block_index);
    }

    /**
     * Verifica si un nodo cumple los criterios de elegibilidad
     */
    fn check_eligibility_criteria(&self, tracking: &NodeTracking) -> bool {
        // Debe estar entre los primeros N nodos
        if tracking.first_block_index > self.eligibility_config.max_eligible_nodes {
            return false;
        }

        // Debe haber validado mínimo de bloques
        if tracking.blocks_validated < self.eligibility_config.min_blocks_validated {
            return false;
        }

        // Debe tener uptime mínimo
        if tracking.uptime_seconds < self.eligibility_config.min_uptime_seconds {
            return false;
        }

        // Si requiere estar activo, verificar que no esté offline (última actividad reciente)
        if self.eligibility_config.require_active {
            let current_time = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let time_since_last_activity = current_time.saturating_sub(tracking.last_block_timestamp);
            // Considerar offline si no ha tenido actividad en 24 horas
            if time_since_last_activity > 24 * 3600 {
                return false;
            }
        }

        true
    }

    /**
     * Determina el tier de elegibilidad basado en el índice del primer bloque
     */
    fn determine_tier(&self, first_block_index: u64) -> u8 {
        for tier in &self.tiers {
            if first_block_index >= tier.min_block_index && first_block_index <= tier.max_block_index {
                return tier.tier_id;
            }
        }
        0
    }

    /**
     * Calcula la cantidad de airdrop para un nodo basado en su tier y participación
     */
    pub fn calculate_airdrop_amount(&self, tracking: &NodeTracking) -> u64 {
        let tier = self.tiers.iter()
            .find(|t| t.tier_id == tracking.eligibility_tier);
        
        if let Some(t) = tier {
            let mut amount = t.base_amount;
            
            // Bonus por bloques validados (máximo 100 bloques)
            let bonus_blocks = tracking.blocks_validated.min(100);
            amount += bonus_blocks * t.bonus_per_block;
            
            // Bonus por uptime (máximo 30 días)
            let uptime_days = (tracking.uptime_seconds / (24 * 3600)).min(30);
            amount += uptime_days * t.bonus_per_uptime_day;
            
            amount
        } else {
            0
        }
    }

    /**
     * Obtiene información de tracking de un nodo
     * @param node_address - Dirección del nodo
     * @returns Información de tracking o None si no existe
     */
    pub fn get_node_tracking(&self, node_address: &str) -> Option<NodeTracking> {
        let tracking = self.tracking.lock().unwrap_or_else(|e| e.into_inner());
        tracking.get(node_address).cloned()
    }

    /**
     * Obtiene todos los nodos elegibles que aún no han reclamado
     * @returns Lista de nodos elegibles
     */
    pub fn get_eligible_nodes(&self) -> Vec<NodeTracking> {
        let tracking = self.tracking.lock().unwrap_or_else(|e| e.into_inner());
        tracking
            .values()
            .filter(|node| node.is_eligible && !node.airdrop_claimed)
            .cloned()
            .collect()
    }

    /**
     * Verifica si un nodo es elegible para airdrop
     * @param node_address - Dirección del nodo
     * @returns true si es elegible y no ha reclamado
     */
    pub fn is_eligible(&self, node_address: &str) -> bool {
        if let Some(tracking) = self.get_node_tracking(node_address) {
            tracking.is_eligible && !tracking.airdrop_claimed
        } else {
            false
        }
    }

    /**
     * Marca un nodo como que ha reclamado su airdrop
     * @param node_address - Dirección del nodo
     * @param transaction_id - ID de la transacción de claim
     * @returns true si se marcó exitosamente
     */
    pub fn mark_as_claimed(&self, node_address: &str, transaction_id: String) -> bool {
        let mut tracking = self.tracking.lock().unwrap_or_else(|e| e.into_inner());
        
        if let Some(entry) = tracking.get_mut(node_address) {
            if entry.is_eligible && !entry.airdrop_claimed {
                entry.airdrop_claimed = true;
                entry.claim_timestamp = Some(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_secs(),
                );
                entry.claim_transaction_id = Some(transaction_id);
                entry.claim_verified = false;
                return true;
            }
        }
        false
    }

    /**
     * Verifica si una transacción de airdrop fue procesada
     * @param node_address - Dirección del nodo
     * @param transaction_id - ID de la transacción
     * @param block_index - Índice del bloque donde se incluyó
     * @returns true si se verificó exitosamente
     */
    pub fn verify_claim_transaction(
        &self,
        node_address: &str,
        transaction_id: &str,
        block_index: u64,
    ) -> bool {
        let mut tracking = self.tracking.lock().unwrap_or_else(|e| e.into_inner());
        
        if let Some(entry) = tracking.get_mut(node_address) {
            if entry.claim_transaction_id.as_ref() == Some(&transaction_id.to_string()) {
                entry.claim_verified = true;
                entry.claim_block_index = Some(block_index);
                return true;
            }
        }
        false
    }

    /**
     * Hace rollback de un claim si la transacción falló
     * @param node_address - Dirección del nodo
     * @param transaction_id - ID de la transacción fallida
     * @returns true si se hizo rollback exitosamente
     */
    pub fn rollback_claim(&self, node_address: &str, transaction_id: &str) -> bool {
        let mut tracking = self.tracking.lock().unwrap_or_else(|e| e.into_inner());
        
        if let Some(entry) = tracking.get_mut(node_address) {
            if entry.claim_transaction_id.as_ref() == Some(&transaction_id.to_string()) {
                entry.airdrop_claimed = false;
                entry.claim_timestamp = None;
                entry.claim_transaction_id = None;
                entry.claim_block_index = None;
                entry.claim_verified = false;
                
                // Remover de pending claims
                let mut pending = self.pending_claims.lock().unwrap_or_else(|e| e.into_inner());
                pending.remove(node_address);
                
                return true;
            }
        }
        false
    }

    /**
     * Agrega un claim pendiente para verificación
     * @param node_address - Dirección del nodo
     * @param transaction_id - ID de la transacción
     */
    pub fn add_pending_claim(&self, node_address: &str, transaction_id: String) {
        let mut pending = self.pending_claims.lock().unwrap_or_else(|e| e.into_inner());
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        pending.insert(node_address.to_string(), (transaction_id, timestamp));
    }

    /**
     * Verifica rate limiting para un request
     * @param identifier - Identificador del request (IP o address)
     * @param max_per_minute - Máximo de requests por minuto
     * @returns true si está dentro del límite
     */
    pub fn check_rate_limit(&self, identifier: &str, max_per_minute: u64) -> bool {
        let mut rate_limits = self.rate_limits.lock().unwrap_or_else(|e| e.into_inner());
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let timestamps = rate_limits.entry(identifier.to_string())
            .or_insert_with(Vec::new);
        
        // Limpiar timestamps antiguos (más de 1 minuto)
        timestamps.retain(|&t| now.saturating_sub(t) < 60);
        
        // Verificar límite
        if timestamps.len() >= max_per_minute as usize {
            return false;
        }
        
        // Agregar timestamp actual
        timestamps.push(now);
        true
    }

    /**
     * Obtiene la cantidad base de airdrop por nodo
     */
    pub fn get_base_airdrop_amount(&self) -> u64 {
        // Retornar la cantidad base del tier medio
        if let Some(tier) = self.tiers.get(1) {
            tier.base_amount
        } else if let Some(tier) = self.tiers.first() {
            tier.base_amount
        } else {
            1000
        }
    }

    /**
     * Obtiene la dirección del wallet de airdrop
     */
    pub fn get_airdrop_wallet(&self) -> &str {
        &self.airdrop_wallet
    }

    /**
     * Obtiene estadísticas del airdrop
     */
    pub fn get_statistics(&self) -> AirdropStatistics {
        let tracking = self.tracking.lock().unwrap_or_else(|e| e.into_inner());
        
        let total_nodes = tracking.len() as u64;
        let eligible_nodes = tracking.values().filter(|n| n.is_eligible).count() as u64;
        let claimed_nodes = tracking.values().filter(|n| n.airdrop_claimed).count() as u64;
        let verified_claims = tracking.values().filter(|n| n.claim_verified).count() as u64;
        let pending_verification = claimed_nodes - verified_claims;
        
        // Calcular total distribuido basado en claims verificados
        let mut total_distributed = 0u64;
        for node in tracking.values() {
            if node.claim_verified {
                total_distributed += self.calculate_airdrop_amount(node);
            }
        }
        
        AirdropStatistics {
            total_nodes,
            eligible_nodes,
            claimed_nodes,
            pending_claims: eligible_nodes - claimed_nodes,
            pending_verification,
            airdrop_amount_per_node: self.get_base_airdrop_amount(),
            total_distributed,
            max_eligible_nodes: self.eligibility_config.max_eligible_nodes,
            verified_claims,
            tiers_count: self.tiers.len() as u64,
        }
    }

    /**
     * Carga tracking desde la base de datos
     */
    #[allow(dead_code)]
    pub fn load_from_db(&self, _db: &dyn std::any::Any) -> SqlResult<()> {
        // Base de datos eliminada - el tracking se reconstruye desde blockchain
        Ok(())
    }

    /**
     * Guarda tracking en la base de datos
     */
    #[allow(dead_code)]
    pub fn save_to_db(&self, _db: &dyn std::any::Any, _node_address: &str) -> SqlResult<()> {
        // Base de datos eliminada - el tracking se reconstruye desde blockchain
        Ok(())
    }

    /**
     * Guarda claim en la base de datos
     */
    #[allow(dead_code)]
    pub fn save_claim_to_db(&self, _db: &dyn std::any::Any, _node_address: &str) -> SqlResult<()> {
        // Base de datos eliminada - el tracking se reconstruye desde blockchain
        Ok(())
    }

    /**
     * Obtiene información de elegibilidad para un nodo
     */
    pub fn get_eligibility_info(&self, node_address: &str) -> Option<EligibilityInfo> {
        if let Some(tracking) = self.get_node_tracking(node_address) {
            let uptime_days = tracking.uptime_seconds / (24 * 3600);
            let estimated_amount = self.calculate_airdrop_amount(&tracking);
            
            let requirements = EligibilityRequirements {
                min_blocks_validated: self.eligibility_config.min_blocks_validated,
                min_uptime_days: self.eligibility_config.min_uptime_seconds / (24 * 3600),
                max_eligible_nodes: self.eligibility_config.max_eligible_nodes,
                current_blocks: tracking.blocks_validated,
                current_uptime_days: uptime_days,
                meets_blocks_requirement: tracking.blocks_validated >= self.eligibility_config.min_blocks_validated,
                meets_uptime_requirement: tracking.uptime_seconds >= self.eligibility_config.min_uptime_seconds,
                meets_position_requirement: tracking.first_block_index <= self.eligibility_config.max_eligible_nodes,
            };
            
            Some(EligibilityInfo {
                is_eligible: tracking.is_eligible && !tracking.airdrop_claimed,
                node_address: tracking.node_address,
                tier: tracking.eligibility_tier,
                estimated_amount,
                blocks_validated: tracking.blocks_validated,
                uptime_days,
                requirements,
            })
        } else {
            None
        }
    }

    /**
     * Obtiene historial completo de claims
     */
    pub fn get_claim_history(&self, limit: Option<u64>, node_address: Option<&str>) -> Vec<ClaimRecord> {
        let history = self.claim_history.lock().unwrap_or_else(|e| e.into_inner());
        let mut records: Vec<ClaimRecord> = history.iter()
            .filter(|r| {
                if let Some(addr) = node_address {
                    r.node_address == addr
                } else {
                    true
                }
            })
            .cloned()
            .collect();
        
        records.sort_by(|a, b| b.claim_timestamp.cmp(&a.claim_timestamp));
        
        if let Some(l) = limit {
            records.truncate(l as usize);
        }
        
        records
    }

    /**
     * Agrega un registro al historial de claims
     */
    pub fn add_claim_to_history(&self, record: ClaimRecord) {
        let mut history = self.claim_history.lock().unwrap_or_else(|e| e.into_inner());
        history.push(record);
    }

    /**
     * Obtiene los tiers disponibles
     */
    pub fn get_tiers(&self) -> Vec<AirdropTier> {
        self.tiers.clone()
    }

    /**
     * Obtiene claims pendientes de verificación
     */
    pub fn get_pending_claims(&self) -> HashMap<String, (String, u64)> {
        let pending = self.pending_claims.lock().unwrap_or_else(|e| e.into_inner());
        pending.clone()
    }

    /**
     * Verifica y actualiza claims pendientes basado en transacciones en un bloque
     */
    pub fn verify_pending_claims_in_block(&self, transactions: &[Transaction], block_index: u64, airdrop_wallet: &str) {
        let mut verified_nodes = Vec::new();
        
        // Buscar transacciones de airdrop en el bloque
        for tx in transactions {
            if tx.from == airdrop_wallet {
                let pending = self.pending_claims.lock().unwrap_or_else(|e| e.into_inner());
                for (node_address, (pending_tx_id, _)) in pending.iter() {
                    if pending_tx_id == &tx.id {
                        verified_nodes.push((node_address.clone(), pending_tx_id.clone()));
                        break;
                    }
                }
            }
        }
        
        // Verificar y actualizar tracking
        for (node_address, tx_id) in verified_nodes {
            self.verify_claim_transaction(&node_address, &tx_id, block_index);
        }
    }
}

/**
 * Registro de claim de airdrop
 */
#[derive(Debug, Clone, Serialize)]
pub struct ClaimRecord {
    pub node_address: String,
    pub claim_timestamp: u64,
    pub airdrop_amount: u64,
    pub transaction_id: String,
    pub block_index: Option<u64>,
    pub tier_id: u8,
    pub verified: bool,
    pub verification_timestamp: Option<u64>,
}

/**
 * Estadísticas del sistema de airdrop
 */
#[derive(Debug, Clone, Serialize)]
pub struct AirdropStatistics {
    pub total_nodes: u64,
    pub eligible_nodes: u64,
    pub claimed_nodes: u64,
    pub pending_claims: u64,
    pub pending_verification: u64,
    pub verified_claims: u64,
    pub airdrop_amount_per_node: u64,
    pub total_distributed: u64,
    pub max_eligible_nodes: u64,
    pub tiers_count: u64,
}

/**
 * Información de elegibilidad de un nodo
 */
#[derive(Debug, Clone, Serialize)]
pub struct EligibilityInfo {
    pub is_eligible: bool,
    pub node_address: String,
    pub tier: u8,
    pub estimated_amount: u64,
    pub blocks_validated: u64,
    pub uptime_days: u64,
    pub requirements: EligibilityRequirements,
}

/**
 * Requisitos de elegibilidad
 */
#[derive(Debug, Clone, Serialize)]
pub struct EligibilityRequirements {
    pub min_blocks_validated: u64,
    pub min_uptime_days: u64,
    pub max_eligible_nodes: u64,
    pub current_blocks: u64,
    pub current_uptime_days: u64,
    pub meets_blocks_requirement: bool,
    pub meets_uptime_requirement: bool,
    pub meets_position_requirement: bool,
}

