use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/**
 * Caché de balances para optimizar consultas frecuentes
 */
pub struct BalanceCache {
    balances: Arc<Mutex<HashMap<String, CachedBalance>>>,
    last_block_index: Arc<Mutex<u64>>,
}

struct CachedBalance {
    balance: u64,
    #[allow(dead_code)]
    block_index: u64,
}

impl BalanceCache {
    /**
     * Crea un nuevo caché de balances
     * @returns BalanceCache inicializado
     */
    pub fn new() -> BalanceCache {
        BalanceCache {
            balances: Arc::new(Mutex::new(HashMap::new())),
            last_block_index: Arc::new(Mutex::new(0)),
        }
    }

    /**
     * Obtiene el balance desde el caché si está actualizado
     * @param address - Dirección del wallet
     * @param current_block_index - Índice del último bloque en la blockchain
     * @returns Some(balance) si está en caché y actualizado, None si necesita recalcular
     */
    pub fn get(&self, address: &str, current_block_index: u64) -> Option<u64> {
        let balances = self.balances.lock().unwrap_or_else(|e| e.into_inner());
        let last_index = self.last_block_index.lock().unwrap_or_else(|e| e.into_inner());
        
        if *last_index != current_block_index {
            return None;
        }
        
        balances.get(address).map(|cached| cached.balance)
    }

    /**
     * Guarda un balance en el caché
     * @param address - Dirección del wallet
     * @param balance - Balance a cachear
     * @param block_index - Índice del bloque actual
     */
    pub fn set(&self, address: String, balance: u64, block_index: u64) {
        let mut balances = self.balances.lock().unwrap_or_else(|e| e.into_inner());
        let mut last_index = self.last_block_index.lock().unwrap_or_else(|e| e.into_inner());
        
        balances.insert(address, CachedBalance {
            balance,
            block_index,
        });
        
        *last_index = block_index;
    }

    /**
     * Invalida todo el caché cuando la blockchain cambia
     * @param new_block_index - Índice del nuevo bloque
     */
    pub fn invalidate(&self, new_block_index: u64) {
        let mut balances = self.balances.lock().unwrap_or_else(|e| e.into_inner());
        let mut last_index = self.last_block_index.lock().unwrap_or_else(|e| e.into_inner());
        
        if *last_index != new_block_index {
            balances.clear();
            *last_index = new_block_index;
        }
    }

    /**
     * Invalida el balance de una dirección específica
     * @param address - Dirección a invalidar
     */
    #[allow(dead_code)]
    pub fn invalidate_address(&self, address: &str) {
        let mut balances = self.balances.lock().unwrap_or_else(|e| e.into_inner());
        balances.remove(address);
    }

    /**
     * Limpia todo el caché
     */
    #[allow(dead_code)]
    pub fn clear(&self) {
        let mut balances = self.balances.lock().unwrap_or_else(|e| e.into_inner());
        balances.clear();
    }

    /**
     * Obtiene estadísticas del caché
     * @returns (tamaño del caché, último índice de bloque)
     */
    pub fn stats(&self) -> (usize, u64) {
        let balances = self.balances.lock().unwrap_or_else(|e| e.into_inner());
        let last_index = self.last_block_index.lock().unwrap_or_else(|e| e.into_inner());
        (balances.len(), *last_index)
    }
}

impl Default for BalanceCache {
    fn default() -> Self {
        Self::new()
    }
}

