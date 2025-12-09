use crate::database::BlockchainDB;
use rusqlite::{params, Result as SqlResult};
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
}

/**
 * Gestor del sistema de airdrop
 */
pub struct AirdropManager {
    tracking: Arc<Mutex<HashMap<String, NodeTracking>>>,
    max_eligible_nodes: u64,
    airdrop_amount_per_node: u64,
    airdrop_wallet: String,
}

impl AirdropManager {
    /**
     * Crea un nuevo gestor de airdrop
     * @param max_eligible_nodes - Número máximo de nodos elegibles (ej: 500)
     * @param airdrop_amount_per_node - Cantidad de tokens por nodo
     * @param airdrop_wallet - Dirección del wallet que distribuirá los tokens
     */
    pub fn new(
        max_eligible_nodes: u64,
        airdrop_amount_per_node: u64,
        airdrop_wallet: String,
    ) -> AirdropManager {
        AirdropManager {
            tracking: Arc::new(Mutex::new(HashMap::new())),
            max_eligible_nodes,
            airdrop_amount_per_node,
            airdrop_wallet,
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
            }
        });

        entry.blocks_validated += 1;
        entry.last_block_timestamp = block_timestamp;
        
        // Verificar elegibilidad: debe estar entre los primeros N nodos
        entry.is_eligible = entry.first_block_index <= self.max_eligible_nodes;
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
     * @returns true si se marcó exitosamente
     */
    pub fn mark_as_claimed(&self, node_address: &str) -> bool {
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
                return true;
            }
        }
        false
    }

    /**
     * Obtiene la cantidad de airdrop por nodo
     */
    pub fn get_airdrop_amount(&self) -> u64 {
        self.airdrop_amount_per_node
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
        let total_distributed = claimed_nodes * self.airdrop_amount_per_node;
        
        AirdropStatistics {
            total_nodes,
            eligible_nodes,
            claimed_nodes,
            pending_claims: eligible_nodes - claimed_nodes,
            airdrop_amount_per_node: self.airdrop_amount_per_node,
            total_distributed,
            max_eligible_nodes: self.max_eligible_nodes,
        }
    }

    /**
     * Carga tracking desde la base de datos
     */
    pub fn load_from_db(&self, db: &BlockchainDB) -> SqlResult<()> {
        match db.load_node_tracking() {
            Ok(nodes) => {
                let mut tracking = self.tracking.lock().unwrap_or_else(|e| e.into_inner());
                for node in nodes {
                    tracking.insert(node.node_address.clone(), node);
                }
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /**
     * Guarda tracking en la base de datos
     */
    pub fn save_to_db(&self, db: &BlockchainDB, node_address: &str) -> SqlResult<()> {
        if let Some(tracking) = self.get_node_tracking(node_address) {
            db.save_node_tracking(&tracking)
        } else {
            Ok(())
        }
    }

    /**
     * Guarda claim en la base de datos
     */
    pub fn save_claim_to_db(&self, db: &BlockchainDB, node_address: &str) -> SqlResult<()> {
        if let Some(tracking) = self.get_node_tracking(node_address) {
            db.save_airdrop_claim(&tracking)
        } else {
            Ok(())
        }
    }
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
    pub airdrop_amount_per_node: u64,
    pub total_distributed: u64,
    pub max_eligible_nodes: u64,
}

