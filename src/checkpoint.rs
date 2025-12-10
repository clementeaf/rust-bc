use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{Error, ErrorKind, Result as IoResult};
use std::path::{Path, PathBuf};

/**
 * Representa un checkpoint en la blockchain
 * Los checkpoints son puntos de control inmutables que previenen ataques de reorganización profunda
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub block_index: u64,
    pub block_hash: String,
    pub timestamp: u64,
    pub cumulative_difficulty: u64, // Dificultad acumulada hasta este punto
}

/**
 * Manager para gestionar checkpoints de la blockchain
 * Previene ataques de 51% y long-range attacks
 */
pub struct CheckpointManager {
    checkpoints_dir: PathBuf,
    checkpoint_interval: u64, // Cada cuántos bloques crear checkpoint (default: 2000)
    max_reorg_depth: u64,     // Profundidad máxima permitida de reorganización (default: 2000)
    checkpoints: HashMap<u64, Checkpoint>, // Cache en memoria
}

impl CheckpointManager {
    /**
     * Crea un nuevo CheckpointManager
     * @param checkpoints_dir - Directorio donde guardar checkpoints
     * @param checkpoint_interval - Intervalo para crear checkpoints (default: 2000)
     * @param max_reorg_depth - Profundidad máxima de reorganización permitida (default: 2000)
     */
    pub fn new(
        checkpoints_dir: impl AsRef<Path>,
        checkpoint_interval: Option<u64>,
        max_reorg_depth: Option<u64>,
    ) -> IoResult<Self> {
        let checkpoints_dir = checkpoints_dir.as_ref().to_path_buf();

        // Crear directorio si no existe
        if !checkpoints_dir.exists() {
            fs::create_dir_all(&checkpoints_dir)?;
        }

        let manager = CheckpointManager {
            checkpoints_dir,
            checkpoint_interval: checkpoint_interval.unwrap_or(2000),
            max_reorg_depth: max_reorg_depth.unwrap_or(2000),
            checkpoints: HashMap::new(),
        };

        // Cargar checkpoints existentes
        let manager = manager.load_checkpoints()?;

        Ok(manager)
    }

    /**
     * Carga todos los checkpoints desde archivos
     */
    fn load_checkpoints(mut self) -> IoResult<Self> {
        if !self.checkpoints_dir.exists() {
            return Ok(self);
        }

        for entry in fs::read_dir(&self.checkpoints_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(filename) = path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    if filename_str.starts_with("checkpoint_") && filename_str.ends_with(".json") {
                        if let Some(index_str) = filename_str
                            .strip_prefix("checkpoint_")
                            .and_then(|s| s.strip_suffix(".json"))
                        {
                            if let Ok(index) = index_str.parse::<u64>() {
                                match fs::read_to_string(&path) {
                                    Ok(json) => match serde_json::from_str::<Checkpoint>(&json) {
                                        Ok(checkpoint) => {
                                            self.checkpoints.insert(index, checkpoint);
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "⚠️  Error deserializando checkpoint {}: {}",
                                                index, e
                                            );
                                        }
                                    },
                                    Err(e) => {
                                        eprintln!("⚠️  Error leyendo checkpoint {}: {}", index, e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(self)
    }

    /**
     * Verifica si se debe crear un checkpoint para el bloque dado
     * @param block_index - Índice del bloque
     * @returns true si se debe crear checkpoint
     */
    pub fn should_create_checkpoint(&self, block_index: u64) -> bool {
        block_index > 0 && block_index.is_multiple_of(self.checkpoint_interval)
    }

    /**
     * Crea y guarda un checkpoint para un bloque
     * @param block_index - Índice del bloque
     * @param block_hash - Hash del bloque
     * @param timestamp - Timestamp del bloque
     * @param cumulative_difficulty - Dificultad acumulada hasta este bloque
     * @returns Result indicando éxito o error
     */
    pub fn create_checkpoint(
        &mut self,
        block_index: u64,
        block_hash: String,
        timestamp: u64,
        cumulative_difficulty: u64,
    ) -> IoResult<()> {
        // Guardar hash para el mensaje antes de moverlo
        let hash_display = block_hash[..std::cmp::min(16, block_hash.len())].to_string();

        let checkpoint = Checkpoint {
            block_index,
            block_hash,
            timestamp,
            cumulative_difficulty,
        };

        // Guardar en archivo
        let filename = format!("checkpoint_{:07}.json", block_index);
        let path = self.checkpoints_dir.join(filename);

        let json = serde_json::to_string_pretty(&checkpoint).map_err(|e| {
            Error::new(
                ErrorKind::InvalidData,
                format!("Error serializando checkpoint: {}", e),
            )
        })?;

        fs::write(&path, json)?;

        // Guardar en cache
        self.checkpoints.insert(block_index, checkpoint);

        println!(
            "✅ Checkpoint creado en bloque {} (hash: {}...)",
            block_index, hash_display
        );

        Ok(())
    }

    /**
     * Obtiene el checkpoint más reciente antes o en el índice dado
     * @param block_index - Índice del bloque
     * @returns Option con el checkpoint si existe
     */
    pub fn get_latest_checkpoint_before(&self, block_index: u64) -> Option<&Checkpoint> {
        // Buscar el checkpoint más reciente que sea <= block_index
        let mut latest: Option<&Checkpoint> = None;
        let mut latest_index = 0u64;

        for (index, checkpoint) in &self.checkpoints {
            if *index <= block_index && *index > latest_index {
                latest_index = *index;
                latest = Some(checkpoint);
            }
        }

        latest
    }

    /**
     * Obtiene el checkpoint exacto para un índice
     * @param block_index - Índice del bloque
     * @returns Option con el checkpoint si existe
     */
    pub fn get_checkpoint(&self, block_index: u64) -> Option<&Checkpoint> {
        self.checkpoints.get(&block_index)
    }

    /**
     * Valida que un bloque cumple con los checkpoints requeridos
     * @param block_index - Índice del bloque
     * @param block_hash - Hash del bloque
     * @param previous_hash - Hash del bloque anterior
     * @returns Result indicando si es válido o error
     */
    pub fn validate_block_against_checkpoints(
        &self,
        block_index: u64,
        block_hash: &str,
        _previous_hash: &str,
    ) -> Result<(), String> {
        // Si hay un checkpoint exacto en este índice, validar que el hash coincida
        if let Some(checkpoint) = self.get_checkpoint(block_index) {
            if checkpoint.block_hash != block_hash {
                return Err(format!(
                    "Bloque {} no coincide con checkpoint: esperado {}, recibido {}",
                    block_index, checkpoint.block_hash, block_hash
                ));
            }
        }

        // Validar que no estamos intentando reorganizar más allá del límite permitido
        if let Some(latest_checkpoint) = self.get_latest_checkpoint_before(block_index) {
            let distance_from_checkpoint =
                block_index.saturating_sub(latest_checkpoint.block_index);

            if distance_from_checkpoint > self.max_reorg_depth {
                return Err(format!(
                    "Reorganización demasiado profunda: {} bloques desde checkpoint en {} (máximo permitido: {})",
                    distance_from_checkpoint, latest_checkpoint.block_index, self.max_reorg_depth
                ));
            }
        }

        Ok(())
    }

    /**
     * Valida una cadena completa contra checkpoints
     * @param chain_blocks - Vector de (index, hash) de la cadena
     * @returns Result indicando si es válida o error
     */
    #[allow(dead_code)]
    pub fn validate_chain_against_checkpoints(
        &self,
        chain_blocks: &[(u64, String)],
    ) -> Result<(), String> {
        for (index, hash) in chain_blocks {
            // Verificar si hay checkpoint en este índice
            if let Some(checkpoint) = self.get_checkpoint(*index) {
                if checkpoint.block_hash != *hash {
                    return Err(format!(
                        "Cadena inválida: bloque {} no coincide con checkpoint (esperado: {}, recibido: {})",
                        index, checkpoint.block_hash, hash
                    ));
                }
            }
        }

        // Verificar que no hay reorganizaciones demasiado profundas
        if let Some((first_index, _)) = chain_blocks.first() {
            if let Some(latest_checkpoint) = self.get_latest_checkpoint_before(*first_index) {
                let distance = first_index.saturating_sub(latest_checkpoint.block_index);

                if distance > self.max_reorg_depth {
                    return Err(format!(
                        "Cadena rechazada: reorganización de {} bloques desde checkpoint (máximo: {})",
                        distance, self.max_reorg_depth
                    ));
                }
            }
        }

        Ok(())
    }

    /**
     * Obtiene todos los checkpoints (útil para sincronización)
     * @returns Vector de checkpoints ordenados por índice
     */
    #[allow(dead_code)]
    pub fn get_all_checkpoints(&self) -> Vec<&Checkpoint> {
        let mut checkpoints: Vec<&Checkpoint> = self.checkpoints.values().collect();
        checkpoints.sort_by_key(|c| c.block_index);
        checkpoints
    }

    /**
     * Obtiene el número de checkpoints
     * @returns Número de checkpoints
     */
    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }

    /**
     * Obtiene el índice del checkpoint más reciente
     * @returns Option con el índice del checkpoint más reciente
     */
    #[allow(dead_code)]
    pub fn get_latest_checkpoint_index(&self) -> Option<u64> {
        self.checkpoints.keys().max().copied()
    }
}
