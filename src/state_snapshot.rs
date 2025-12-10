use crate::blockchain::Block;
use crate::smart_contracts::SmartContract;
use crate::staking::Validator;
use crate::state_reconstructor::WalletState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{ErrorKind, Result as IoResult};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/**
 * Snapshot del estado completo de la blockchain
 * Permite reconstruir el estado rápidamente sin procesar todos los bloques
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub block_index: u64,
    pub block_hash: String,
    pub timestamp: u64,
    pub wallets: HashMap<String, WalletSnapshot>,
    pub contracts: HashMap<String, SmartContract>,
    pub validators: HashMap<String, Validator>,
    // Nota: airdrop_tracking se reconstruye desde bloques, no se guarda en snapshot
    // pub airdrop_tracking: HashMap<String, NodeTracking>,
}

/**
 * Estado de un wallet en el snapshot
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSnapshot {
    pub balance: u64,
}

/**
 * Manager para crear y cargar snapshots
 */
pub struct StateSnapshotManager {
    snapshots_dir: PathBuf,
}

impl StateSnapshotManager {
    /**
     * Crea un nuevo StateSnapshotManager
     * @param snapshots_dir - Directorio donde guardar los snapshots
     */
    pub fn new(snapshots_dir: impl AsRef<Path>) -> IoResult<Self> {
        let snapshots_dir = PathBuf::from(snapshots_dir.as_ref());
        if !snapshots_dir.exists() {
            fs::create_dir_all(&snapshots_dir)?;
        }
        Ok(StateSnapshotManager { snapshots_dir })
    }

    /**
     * Guarda un snapshot del estado
     * @param snapshot - Snapshot a guardar
     * @param block_index - Índice del bloque (usado para el nombre del archivo)
     */
    pub fn save_snapshot(&self, snapshot: &StateSnapshot, block_index: u64) -> IoResult<()> {
        let filename = format!("snapshot_{:07}.json", block_index);
        let path = self.snapshots_dir.join(filename);

        let json = serde_json::to_string_pretty(snapshot).map_err(|e| {
            std::io::Error::new(
                ErrorKind::InvalidData,
                format!("Error serializando snapshot: {}", e),
            )
        })?;

        fs::write(path, json)?;
        Ok(())
    }

    /**
     * Carga el snapshot más reciente
     * @returns Snapshot más reciente o None si no hay snapshots
     */
    pub fn load_latest_snapshot(&self) -> IoResult<Option<StateSnapshot>> {
        if !self.snapshots_dir.exists() {
            return Ok(None);
        }

        let mut latest_index: Option<u64> = None;
        let mut latest_path: Option<PathBuf> = None;

        for entry in fs::read_dir(&self.snapshots_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(filename) = path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    if filename_str.starts_with("snapshot_") && filename_str.ends_with(".json") {
                        if let Some(index_str) = filename_str
                            .strip_prefix("snapshot_")
                            .and_then(|s| s.strip_suffix(".json"))
                        {
                            if let Ok(index) = index_str.parse::<u64>() {
                                match latest_index {
                                    Some(current_max) if index > current_max => {
                                        latest_index = Some(index);
                                        latest_path = Some(path);
                                    }
                                    None => {
                                        latest_index = Some(index);
                                        latest_path = Some(path);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some(path) = latest_path {
            let json = fs::read_to_string(&path)?;
            let snapshot = serde_json::from_str::<StateSnapshot>(&json).map_err(|e| {
                std::io::Error::new(
                    ErrorKind::InvalidData,
                    format!("Error deserializando snapshot: {}", e),
                )
            })?;
            Ok(Some(snapshot))
        } else {
            Ok(None)
        }
    }

    /**
     * Lista todos los snapshots disponibles
     * @returns Vector de índices de bloques con snapshots
     */
    pub fn list_snapshots(&self) -> IoResult<Vec<u64>> {
        let mut snapshots = Vec::new();

        if !self.snapshots_dir.exists() {
            return Ok(snapshots);
        }

        for entry in fs::read_dir(&self.snapshots_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(filename) = path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    if filename_str.starts_with("snapshot_") && filename_str.ends_with(".json") {
                        if let Some(index_str) = filename_str
                            .strip_prefix("snapshot_")
                            .and_then(|s| s.strip_suffix(".json"))
                        {
                            if let Ok(index) = index_str.parse::<u64>() {
                                snapshots.push(index);
                            }
                        }
                    }
                }
            }
        }

        snapshots.sort();
        Ok(snapshots)
    }

    /**
     * Elimina snapshots antiguos, manteniendo solo los más recientes
     * @param keep_count - Número de snapshots más recientes a mantener
     */
    pub fn cleanup_old_snapshots(&self, keep_count: usize) -> IoResult<usize> {
        let snapshots = self.list_snapshots()?;

        if snapshots.len() <= keep_count {
            return Ok(0);
        }

        let to_remove = snapshots.len() - keep_count;
        let snapshots_to_remove = &snapshots[..to_remove];

        let mut removed = 0;
        for index in snapshots_to_remove {
            let filename = format!("snapshot_{:07}.json", index);
            let path = self.snapshots_dir.join(filename);
            if let Err(e) = fs::remove_file(&path) {
                eprintln!("⚠️  Error eliminando snapshot {}: {}", index, e);
            } else {
                removed += 1;
            }
        }

        Ok(removed)
    }
}

/**
 * Crea un snapshot desde el estado reconstruido
 */
impl StateSnapshot {
    /**
     * Crea un snapshot desde un bloque y estado reconstruido
     * @param block - Bloque de referencia
     * @param wallets - Estado de wallets
     * @param contracts - Contratos
     * @param validators - Validadores
     * @param airdrop_tracking - Tracking de airdrop
     */
    pub fn from_state(
        block: &Block,
        wallets: HashMap<String, WalletState>,
        contracts: HashMap<String, SmartContract>,
        validators: HashMap<String, Validator>,
    ) -> Self {
        let wallet_snapshots: HashMap<String, WalletSnapshot> = wallets
            .into_iter()
            .map(|(addr, state)| {
                (
                    addr.clone(),
                    WalletSnapshot {
                        balance: state.balance,
                    },
                )
            })
            .collect();

        StateSnapshot {
            block_index: block.index,
            block_hash: block.hash.clone(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            wallets: wallet_snapshots,
            contracts,
            validators,
        }
    }
}
