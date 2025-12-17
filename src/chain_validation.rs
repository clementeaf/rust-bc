/**
 * Chain Validation and Fork Resolution Module
 * 
 * Handles:
 * - Full chain history validation
 * - Fork detection and resolution using cumulative difficulty
 * - Chain reorganization (reorg) safety
 * - Longest chain rule enforcement
 */

use crate::blockchain::{Block, Blockchain};
use std::collections::HashMap;

/**
 * Validación de cadena completa
 */
pub struct ChainValidator;

impl ChainValidator {
    /**
     * Valida una cadena completa
     * @returns (is_valid, errors)
     */
    pub fn validate_full_chain(chain: &[Block]) -> (bool, Vec<String>) {
        let mut errors = Vec::new();

        if chain.is_empty() {
            errors.push("Cadena vacía".to_string());
            return (false, errors);
        }

        // Validar bloque génesis
        if chain[0].index != 0 {
            errors.push("El primer bloque debe tener índice 0".to_string());
        }

        if chain[0].previous_hash != "0" {
            errors.push("El bloque génesis debe tener previous_hash = \"0\"".to_string());
        }

        // Validar continuidad e integridad de cada bloque
        for (i, block) in chain.iter().enumerate() {
            // Validar índice secuencial
            if block.index as usize != i {
                errors.push(format!(
                    "Bloque {} tiene índice incorrecto: {} (esperado: {})",
                    i, block.index, i
                ));
            }

            // Validar que cada bloque tenga hash válido
            if !block.is_valid() {
                errors.push(format!("Bloque {} tiene hash inválido", i));
            }

            // Validar referencias entre bloques (solo para bloques no-génesis)
            if i > 0 {
                if block.previous_hash != chain[i - 1].hash {
                    errors.push(format!(
                        "Bloque {} referencia bloque anterior incorrecto",
                        i
                    ));
                }

                // Validar que timestamp sea monótonamente creciente (con margen)
                if block.timestamp < chain[i - 1].timestamp {
                    errors.push(format!(
                        "Bloque {} tiene timestamp más antiguo que su antecesor",
                        i
                    ));
                }
            }

            // Validar transacciones del bloque
            for (tx_idx, tx) in block.transactions.iter().enumerate() {
                if tx.amount == 0 && tx.fee == 0 && tx.from != "genesis" && tx.from != "0" {
                    errors.push(format!(
                        "Bloque {}, transacción {} tiene monto y fee = 0",
                        i, tx_idx
                    ));
                }
            }
        }

        let is_valid = errors.is_empty();
        (is_valid, errors)
    }

    /**
     * Valida un bloque individual en el contexto de la cadena
     */
    pub fn validate_block_for_chain(
        new_block: &Block,
        previous_block: &Block,
        blockchain: &Blockchain,
    ) -> Result<(), String> {
        // Validar referencia al bloque anterior
        if new_block.previous_hash != previous_block.hash {
            return Err("Bloque no referencia correctamente al bloque anterior".to_string());
        }

        // Validar índice
        if new_block.index != previous_block.index + 1 {
            return Err(format!(
                "Índice incorrecto: {} (esperado: {})",
                new_block.index,
                previous_block.index + 1
            ));
        }

        // Validar timestamp (monotonía con margen de 60 segundos)
        if new_block.timestamp < previous_block.timestamp {
            return Err("Timestamp del nuevo bloque es más antiguo que el anterior".to_string());
        }

        // Validar dificultad (no debe cambiar más de 1 nivel sin adjustment_interval)
        let blocks_since_adjustment = (new_block.index % blockchain.difficulty_adjustment_interval);
        if blocks_since_adjustment != 0 && new_block.difficulty != blockchain.difficulty {
            return Err("Dificultad cambió fuera del intervalo de ajuste".to_string());
        }

        // Validar que el bloque en sí sea válido
        if !new_block.is_valid() {
            return Err("Bloque no cumple validación PoW".to_string());
        }

        Ok(())
    }
}

/**
 * Fork resolution usando cumulative difficulty
 */
pub struct ForkResolver;

impl ForkResolver {
    /**
     * Compara dos cadenas y retorna cuál tiene mayor dificultad acumulada
     * @returns 1 si chain_a es mejor, -1 si chain_b es mejor, 0 si son iguales
     */
    pub fn compare_chains(chain_a: &Blockchain, chain_b: &Blockchain) -> i32 {
        let diff_a = Self::calculate_cumulative_difficulty(&chain_a.chain);
        let diff_b = Self::calculate_cumulative_difficulty(&chain_b.chain);

        if diff_a > diff_b {
            1
        } else if diff_a < diff_b {
            -1
        } else {
            0
        }
    }

    /**
     * Calcula dificultad acumulada de una cadena
     */
    fn calculate_cumulative_difficulty(chain: &[Block]) -> u64 {
        chain
            .iter()
            .map(|block| 2u64.pow(block.difficulty as u32))
            .sum()
    }

    /**
     * Encuentra el punto de divergencia entre dos cadenas
     * @returns (fork_point_index, chain_a_suffix, chain_b_suffix)
     */
    pub fn find_fork_point(
        chain_a: &[Block],
        chain_b: &[Block],
    ) -> (usize, Vec<Block>, Vec<Block>) {
        let mut fork_point = 0;

        for i in 0..chain_a.len().min(chain_b.len()) {
            if chain_a[i].hash == chain_b[i].hash {
                fork_point = i;
            } else {
                break;
            }
        }

        let chain_a_suffix = chain_a[fork_point..].to_vec();
        let chain_b_suffix = chain_b[fork_point..].to_vec();

        (fork_point, chain_a_suffix, chain_b_suffix)
    }

    /**
     * Determina si un reorg es seguro (límite máximo de bloques reorganizados)
     */
    pub fn is_reorg_safe(
        fork_point: usize,
        current_chain_length: usize,
        max_reorg_depth: usize,
    ) -> bool {
        let reorg_depth = current_chain_length - fork_point;
        reorg_depth <= max_reorg_depth
    }
}

/**
 * Protección contra el ataque 51%
 */
pub struct AttackProtection;

impl AttackProtection {
    // Checkpoints hardcodeados (bloque_index, bloque_hash)
    const HARDCODED_CHECKPOINTS: &'static [(u64, &'static str)] = &[];

    /**
     * Valida que un bloque no viole checkpoints
     */
    pub fn is_checkpoint_safe(block_index: u64, block_hash: &str) -> bool {
        for (checkpoint_index, checkpoint_hash) in Self::HARDCODED_CHECKPOINTS {
            if block_index == *checkpoint_index && block_hash != *checkpoint_hash {
                return false; // Violación de checkpoint
            }
        }
        true
    }

    /**
     * Limita cambios de dificultad para prevenir ataques
     */
    pub fn validate_difficulty_adjustment(
        old_difficulty: u8,
        new_difficulty: u8,
        min_difficulty: u8,
        max_single_adjustment: u8,
    ) -> Result<(), String> {
        // Validar piso de dificultad
        if new_difficulty < min_difficulty {
            return Err(format!(
                "Dificultad por debajo del mínimo: {} < {}",
                new_difficulty, min_difficulty
            ));
        }

        // Validar ajuste máximo permitido
        let adjustment = (old_difficulty as i16 - new_difficulty as i16).abs();
        if adjustment > max_single_adjustment as i16 {
            return Err(format!(
                "Ajuste de dificultad demasiado grande: {}",
                adjustment
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_difficulty_adjustment_limits() {
        // Ajuste normal (+ 1)
        assert!(
            AttackProtection::validate_difficulty_adjustment(5, 6, 1, 2).is_ok(),
            "Ajuste +1 debe ser válido"
        );

        // Ajuste excesivo (+ 3)
        assert!(
            AttackProtection::validate_difficulty_adjustment(5, 8, 1, 2).is_err(),
            "Ajuste +3 debe ser rechazado con límite de 2"
        );

        // Difictultad debajo del mínimo
        assert!(
            AttackProtection::validate_difficulty_adjustment(2, 0, 1, 2).is_err(),
            "Dificultad bajo el mínimo debe ser rechazada"
        );
    }
}
