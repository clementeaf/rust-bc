use crate::blockchain::Block;
use bincode;
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Result as IoResult;

/**
 * Almacenamiento de bloques en archivos (sustituye BD)
 * Formato: block_0000001.dat, block_0000002.dat, etc.
 */
pub struct BlockStorage {
    blocks_dir: PathBuf,
}

impl BlockStorage {
    /**
     * Crea un nuevo BlockStorage
     * @param blocks_dir - Directorio donde se guardan los bloques
     * @returns BlockStorage configurado
     */
    pub fn new(blocks_dir: impl AsRef<Path>) -> IoResult<Self> {
        let blocks_dir = PathBuf::from(blocks_dir.as_ref());
        
        // Crear directorio si no existe
        if !blocks_dir.exists() {
            fs::create_dir_all(&blocks_dir)?;
        }
        
        Ok(BlockStorage { blocks_dir })
    }

    /**
     * Guarda un bloque en un archivo
     * @param block - Bloque a guardar
     * @returns Result indicando éxito o error
     */
    pub fn save_block(&self, block: &Block) -> IoResult<()> {
        let filename = format!("block_{:07}.dat", block.index);
        let path = self.blocks_dir.join(filename);
        
        // Serializar bloque usando bincode (más eficiente que JSON)
        let data = bincode::serialize(block)
            .map_err(|e| std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Error serializando bloque: {}", e)
            ))?;
        
        fs::write(path, data)?;
        Ok(())
    }

    /**
     * Carga un bloque por índice
     * @param index - Índice del bloque
     * @returns Option con el bloque si existe
     */
    pub fn load_block(&self, index: u64) -> IoResult<Option<Block>> {
        let filename = format!("block_{:07}.dat", index);
        let path = self.blocks_dir.join(filename);
        
        if !path.exists() {
            return Ok(None);
        }
        
        let data = fs::read(path)?;
        let block: Block = bincode::deserialize(&data)
            .map_err(|e| std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Error deserializando bloque: {}", e)
            ))?;
        
        Ok(Some(block))
    }

    /**
     * Carga todos los bloques desde archivos
     * @returns Vec con todos los bloques ordenados por índice
     */
    pub fn load_all_blocks(&self) -> IoResult<Vec<Block>> {
        let mut blocks = Vec::new();
        
        // Leer todos los archivos block_*.dat
        let entries = fs::read_dir(&self.blocks_dir)?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if let Some(filename) = path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    if filename_str.starts_with("block_") && filename_str.ends_with(".dat") {
                        let data = fs::read(&path)?;
                        match bincode::deserialize::<Block>(&data) {
                            Ok(block) => blocks.push(block),
                            Err(e) => {
                                eprintln!("⚠️  Error deserializando bloque {}: {}", filename_str, e);
                                continue;
                            }
                        }
                    }
                }
            }
        }
        
        // Ordenar por índice
        blocks.sort_by_key(|b| b.index);
        
        Ok(blocks)
    }

    /**
     * Obtiene el último índice de bloque guardado
     * @returns Option con el último índice, o None si no hay bloques
     */
    pub fn get_latest_block_index(&self) -> IoResult<Option<u64>> {
        let blocks = self.load_all_blocks()?;
        
        if blocks.is_empty() {
            return Ok(None);
        }
        
        Ok(Some(blocks.last().unwrap().index))
    }

    /**
     * Elimina un bloque por índice (útil para limpieza)
     * @param index - Índice del bloque a eliminar
     * @returns Result indicando éxito o error
     */
    pub fn remove_block(&self, index: u64) -> IoResult<()> {
        let filename = format!("block_{:07}.dat", index);
        let path = self.blocks_dir.join(filename);
        
        if path.exists() {
            fs::remove_file(path)?;
        }
        
        Ok(())
    }

    /**
     * Obtiene el número de bloques guardados
     * @returns Número de bloques
     */
    pub fn get_block_count(&self) -> IoResult<usize> {
        let entries = fs::read_dir(&self.blocks_dir)?;
        let count = entries
            .filter_map(|entry| {
                entry.ok().and_then(|e| {
                    e.path()
                        .file_name()
                        .and_then(|n| n.to_str())
                        .filter(|s| s.starts_with("block_") && s.ends_with(".dat"))
                        .map(|_| 1)
                })
            })
            .count();
        
        Ok(count)
    }

    /**
     * Limpia todos los bloques (útil para testing)
     * @returns Result indicando éxito o error
     */
    pub fn clear_all(&self) -> IoResult<()> {
        let entries = fs::read_dir(&self.blocks_dir)?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            if let Some(filename) = path.file_name() {
                if let Some(filename_str) = filename.to_str() {
                    if filename_str.starts_with("block_") && filename_str.ends_with(".dat") {
                        fs::remove_file(path)?;
                    }
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blockchain::Block;
    use crate::models::Transaction;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load_block() {
        let temp_dir = TempDir::new().unwrap();
        let storage = BlockStorage::new(temp_dir.path()).unwrap();
        
        let block = Block::new(
            0,
            vec![],
            "0".to_string(),
            1,
        );
        
        storage.save_block(&block).unwrap();
        let loaded = storage.load_block(0).unwrap().unwrap();
        
        assert_eq!(block.index, loaded.index);
        assert_eq!(block.previous_hash, loaded.previous_hash);
    }

    #[test]
    fn test_load_all_blocks() {
        let temp_dir = TempDir::new().unwrap();
        let storage = BlockStorage::new(temp_dir.path()).unwrap();
        
        for i in 0..5 {
            let block = Block::new(
                i,
                vec![],
                if i == 0 { "0".to_string() } else { format!("hash_{}", i - 1) },
                1,
            );
            storage.save_block(&block).unwrap();
        }
        
        let blocks = storage.load_all_blocks().unwrap();
        assert_eq!(blocks.len(), 5);
        assert_eq!(blocks[0].index, 0);
        assert_eq!(blocks[4].index, 4);
    }
}

