use crate::block_storage::BlockStorage;
use crate::state_snapshot::StateSnapshotManager;
use std::io::Result as IoResult;

/**
 * Manager para pruning de bloques antiguos
 * Mantiene solo los bloques necesarios para reconstrucción desde snapshot
 */
pub struct PruningManager {
    block_storage: BlockStorage,
    snapshot_manager: StateSnapshotManager,
    keep_blocks_after_snapshot: u64, // Bloques a mantener después del último snapshot
    snapshot_interval: u64,          // Cada cuántos bloques crear snapshot
}

impl PruningManager {
    /**
     * Crea un nuevo PruningManager
     * @param block_storage - Storage de bloques
     * @param snapshot_manager - Manager de snapshots
     * @param keep_blocks_after_snapshot - Bloques a mantener después del snapshot (default: 1000)
     * @param snapshot_interval - Intervalo para crear snapshots (default: 1000)
     */
    pub fn new(
        block_storage: BlockStorage,
        snapshot_manager: StateSnapshotManager,
        keep_blocks_after_snapshot: Option<u64>,
        snapshot_interval: Option<u64>,
    ) -> Self {
        PruningManager {
            block_storage,
            snapshot_manager,
            keep_blocks_after_snapshot: keep_blocks_after_snapshot.unwrap_or(1000),
            snapshot_interval: snapshot_interval.unwrap_or(1000),
        }
    }

    /**
     * Verifica si se debe crear un snapshot basado en el índice del bloque
     * @param block_index - Índice del último bloque minado
     * @returns true si se debe crear snapshot
     */
    pub fn should_create_snapshot(&self, block_index: u64) -> bool {
        block_index > 0 && block_index.is_multiple_of(self.snapshot_interval)
    }

    /**
     * Ejecuta pruning de bloques antiguos
     * Elimina bloques anteriores al último snapshot, manteniendo solo los necesarios
     * @param latest_block_index - Índice del último bloque
     * @returns Número de bloques eliminados
     */
    pub fn prune_old_blocks(&self, _latest_block_index: u64) -> IoResult<usize> {
        // Obtener el snapshot más reciente
        let latest_snapshot = self.snapshot_manager.load_latest_snapshot()?;

        if let Some(snapshot) = latest_snapshot {
            let snapshot_index = snapshot.block_index;

            // Calcular el índice más antiguo a mantener
            // Mantenemos bloques desde (snapshot_index - keep_blocks) hasta latest_block_index
            let keep_from_index = if snapshot_index > self.keep_blocks_after_snapshot {
                snapshot_index.saturating_sub(self.keep_blocks_after_snapshot)
            } else {
                0 // Mantener desde génesis si el snapshot es muy reciente
            };

            // Eliminar bloques anteriores a keep_from_index
            let mut pruned_count = 0;
            for index in 0..keep_from_index {
                if let Err(e) = self.block_storage.remove_block(index) {
                    eprintln!(
                        "⚠️  Error eliminando bloque {} durante pruning: {}",
                        index, e
                    );
                } else {
                    pruned_count += 1;
                }
            }

            if pruned_count > 0 {
                println!(
                    "🗑️  Pruning completado: {} bloques eliminados (manteniendo desde bloque {})",
                    pruned_count, keep_from_index
                );
            }

            Ok(pruned_count)
        } else {
            // Sin snapshot, no hacer pruning aún
            Ok(0)
        }
    }

    /**
     * Limpia snapshots antiguos, manteniendo solo los más recientes
     * @param keep_count - Número de snapshots a mantener (default: 10)
     * @returns Número de snapshots eliminados
     */
    #[allow(dead_code)]
    pub fn cleanup_old_snapshots(&self, keep_count: Option<usize>) -> IoResult<usize> {
        self.snapshot_manager
            .cleanup_old_snapshots(keep_count.unwrap_or(10))
    }

    /**
     * Obtiene estadísticas de pruning
     * @returns Tupla con (bloques totales, bloques después de pruning estimado, snapshots disponibles)
     */
    #[allow(dead_code)]
    pub fn get_pruning_stats(&self) -> IoResult<(usize, usize, usize)> {
        let total_blocks = self.block_storage.get_block_count()?;
        let snapshots = self.snapshot_manager.list_snapshots()?;

        // Estimar bloques después de pruning
        let estimated_blocks_after_pruning = if let Some(latest_snapshot_index) = snapshots.last() {
            let keep_from = if *latest_snapshot_index > self.keep_blocks_after_snapshot {
                latest_snapshot_index.saturating_sub(self.keep_blocks_after_snapshot)
            } else {
                0
            };
            total_blocks.saturating_sub(keep_from as usize)
        } else {
            total_blocks
        };

        Ok((
            total_blocks,
            estimated_blocks_after_pruning,
            snapshots.len(),
        ))
    }
}
