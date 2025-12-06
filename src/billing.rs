use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/**
 * Niveles de suscripción disponibles
 */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BillingTier {
    Free,
    Basic,
    Pro,
    Enterprise,
}

impl BillingTier {
    /**
     * Obtiene el límite de transacciones por mes para el tier
     */
    pub fn transaction_limit(&self) -> u64 {
        match self {
            BillingTier::Free => 100,
            BillingTier::Basic => 10_000,
            BillingTier::Pro => 100_000,
            BillingTier::Enterprise => u64::MAX,
        }
    }

    /**
     * Obtiene el precio mensual del tier
     */
    #[allow(dead_code)]
    pub fn monthly_price(&self) -> u64 {
        match self {
            BillingTier::Free => 0,
            BillingTier::Basic => 49,
            BillingTier::Pro => 299,
            BillingTier::Enterprise => 0,
        }
    }

    /**
     * Obtiene el límite de wallets para el tier
     */
    #[allow(dead_code)]
    pub fn wallet_limit(&self) -> u64 {
        match self {
            BillingTier::Free => 1,
            BillingTier::Basic => 100,
            BillingTier::Pro => u64::MAX,
            BillingTier::Enterprise => u64::MAX,
        }
    }

    /**
     * Verifica si el tier permite smart contracts
     */
    #[allow(dead_code)]
    pub fn allows_smart_contracts(&self) -> bool {
        matches!(self, BillingTier::Pro | BillingTier::Enterprise)
    }

    /**
     * Obtiene el tier desde un string
     */
    pub fn from_str(s: &str) -> Option<BillingTier> {
        match s.to_lowercase().as_str() {
            "free" => Some(BillingTier::Free),
            "basic" => Some(BillingTier::Basic),
            "pro" => Some(BillingTier::Pro),
            "enterprise" => Some(BillingTier::Enterprise),
            _ => None,
        }
    }
}

/**
 * Estadísticas de uso de una API key
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub transactions_this_month: u64,
    pub wallets_created: u64,
    pub requests_today: u64,
    pub last_reset: u64,
}

impl UsageStats {
    /**
     * Crea nuevas estadísticas de uso
     */
    pub fn new() -> UsageStats {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        UsageStats {
            transactions_this_month: 0,
            wallets_created: 0,
            requests_today: 0,
            last_reset: now,
        }
    }

    /**
     * Verifica si necesita resetear contadores diarios
     */
    pub fn should_reset_daily(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let days_since_reset = (now - self.last_reset) / 86400;
        days_since_reset > 0
    }

    /**
     * Resetea contadores diarios si es necesario
     */
    pub fn reset_if_needed(&mut self) {
        if self.should_reset_daily() {
            self.requests_today = 0;
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            self.last_reset = now;
        }
    }
}

/**
 * Información de una API key
 */
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct APIKeyInfo {
    pub key_hash: String,
    pub tier: BillingTier,
    pub usage: UsageStats,
    pub created_at: u64,
    pub is_active: bool,
    pub rate_limit_per_minute: u32,
}

impl APIKeyInfo {
    /**
     * Crea nueva información de API key
     */
    pub fn new(tier: BillingTier) -> APIKeyInfo {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let rate_limit = match tier {
            BillingTier::Free => 10,
            BillingTier::Basic => 100,
            BillingTier::Pro => 1000,
            BillingTier::Enterprise => 10000,
        };
        APIKeyInfo {
            key_hash: String::new(),
            tier,
            usage: UsageStats::new(),
            created_at: now,
            is_active: true,
            rate_limit_per_minute: rate_limit,
        }
    }

    /**
     * Verifica si puede realizar una transacción
     */
    pub fn can_make_transaction(&self) -> bool {
        if !self.is_active {
            return false;
        }
        let limit = self.tier.transaction_limit();
        if limit == u64::MAX {
            return true;
        }
        self.usage.transactions_this_month < limit
    }

    /**
     * Verifica si puede crear un wallet
     */
    pub fn can_create_wallet(&self) -> bool {
        if !self.is_active {
            return false;
        }
        let limit = self.tier.wallet_limit();
        if limit == u64::MAX {
            return true;
        }
        self.usage.wallets_created < limit
    }

    /**
     * Incrementa el contador de transacciones
     */
    pub fn increment_transactions(&mut self) {
        self.usage.transactions_this_month += 1;
    }

    /**
     * Incrementa el contador de wallets
     */
    pub fn increment_wallets(&mut self) {
        self.usage.wallets_created += 1;
    }

    /**
     * Incrementa el contador de requests
     */
    pub fn increment_requests(&mut self) {
        self.usage.reset_if_needed();
        self.usage.requests_today += 1;
    }
}

/**
 * Gestor de billing y API keys
 */
pub struct BillingManager {
    keys: Arc<Mutex<HashMap<String, APIKeyInfo>>>,
}

impl BillingManager {
    /**
     * Crea un nuevo gestor de billing
     */
    pub fn new() -> BillingManager {
        BillingManager {
            keys: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /**
     * Genera un hash seguro de una API key
     */
    fn hash_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /**
     * Crea una nueva API key para un tier
     */
    pub fn create_api_key(&self, tier: BillingTier) -> Result<String, String> {
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 10;
        
        loop {
            let key = self.generate_secure_key();
            let key_hash = Self::hash_key(&key);
            
            let mut keys = self.keys.lock().unwrap_or_else(|e| e.into_inner());
            
            if !keys.contains_key(&key_hash) {
                let mut key_info = APIKeyInfo::new(tier);
                key_info.key_hash = key_hash.clone();
                keys.insert(key_hash, key_info);
                return Ok(key);
            }
            
            attempts += 1;
            if attempts >= MAX_ATTEMPTS {
                return Err("Error generando API key única después de múltiples intentos".to_string());
            }
        }
    }

    /**
     * Genera una API key segura usando UUID y hash
     */
    fn generate_secure_key(&self) -> String {
        use uuid::Uuid;
        let uuid = Uuid::new_v4();
        let uuid_str = uuid.to_string().replace("-", "");
        format!("bc_{}", &uuid_str[..32])
    }

    /**
     * Valida una API key y retorna su información
     */
    pub fn validate_key(&self, key: &str) -> Result<APIKeyInfo, String> {
        if key.is_empty() {
            return Err("API key vacía".to_string());
        }

        if !key.starts_with("bc_") || key.len() < 35 {
            return Err("Formato de API key inválido".to_string());
        }

        let key_hash = Self::hash_key(key);
        let mut keys = self.keys.lock().unwrap_or_else(|e| e.into_inner());
        
        match keys.get_mut(&key_hash) {
            Some(key_info) => {
                if !key_info.is_active {
                    return Err("API key desactivada".to_string());
                }
                key_info.usage.reset_if_needed();
                Ok(key_info.clone())
            }
            None => Err("API key no encontrada".to_string()),
        }
    }

    /**
     * Verifica si una API key puede realizar una transacción
     */
    pub fn can_make_transaction(&self, key: &str) -> Result<bool, String> {
        let key_info = self.validate_key(key)?;
        Ok(key_info.can_make_transaction())
    }

    /**
     * Verifica el límite de transacciones sin incrementar (operación atómica)
     * Útil para verificar antes de procesar una transacción costosa
     */
    pub fn check_transaction_limit(&self, key: &str) -> Result<(), String> {
        if key.is_empty() {
            return Err("API key vacía".to_string());
        }

        if !key.starts_with("bc_") || key.len() < 35 {
            return Err("Formato de API key inválido".to_string());
        }

        let key_hash = Self::hash_key(key);
        let mut keys = self.keys.lock().unwrap_or_else(|e| e.into_inner());
        
        match keys.get_mut(&key_hash) {
            Some(key_info) => {
                if !key_info.is_active {
                    return Err("API key desactivada".to_string());
                }
                
                key_info.usage.reset_if_needed();
                
                // Verificar límite sin incrementar (operación atómica)
                let limit = key_info.tier.transaction_limit();
                if limit != u64::MAX && key_info.usage.transactions_this_month >= limit {
                    return Err("Límite de transacciones alcanzado para tu tier".to_string());
                }
                
                Ok(())
            }
            None => Err("API key no encontrada".to_string()),
        }
    }

    /**
     * Verifica si una API key puede crear un wallet
     */
    pub fn can_create_wallet(&self, key: &str) -> Result<bool, String> {
        let key_info = self.validate_key(key)?;
        Ok(key_info.can_create_wallet())
    }

    /**
     * Intenta registrar una transacción verificando el límite de forma atómica
     * Retorna Ok(()) si se pudo registrar, Err si se excedió el límite o hubo un error
     */
    pub fn try_record_transaction(&self, key: &str) -> Result<(), String> {
        if key.is_empty() {
            return Err("API key vacía".to_string());
        }

        if !key.starts_with("bc_") || key.len() < 35 {
            return Err("Formato de API key inválido".to_string());
        }

        let key_hash = Self::hash_key(key);
        let mut keys = self.keys.lock().unwrap_or_else(|e| e.into_inner());
        
        match keys.get_mut(&key_hash) {
            Some(key_info) => {
                if !key_info.is_active {
                    return Err("API key desactivada".to_string());
                }
                
                key_info.usage.reset_if_needed();
                
                // Verificar límite antes de incrementar (operación atómica)
                let limit = key_info.tier.transaction_limit();
                if limit != u64::MAX && key_info.usage.transactions_this_month >= limit {
                    return Err("Límite de transacciones alcanzado para tu tier".to_string());
                }
                
                // Incrementar contador (dentro del mismo lock, operación atómica)
                key_info.increment_transactions();
                Ok(())
            }
            None => Err("API key no encontrada".to_string()),
        }
    }

    /**
     * Registra una transacción realizada (sin verificar límite)
     * Usar solo cuando ya se verificó el límite previamente
     */
    pub fn record_transaction(&self, key: &str) -> Result<(), String> {
        let key_hash = Self::hash_key(key);
        let mut keys = self.keys.lock().unwrap_or_else(|e| e.into_inner());
        
        match keys.get_mut(&key_hash) {
            Some(key_info) => {
                key_info.increment_transactions();
                Ok(())
            }
            None => Err("API key no encontrada".to_string()),
        }
    }

    /**
     * Registra la creación de un wallet
     */
    pub fn record_wallet_creation(&self, key: &str) -> Result<(), String> {
        let key_hash = Self::hash_key(key);
        let mut keys = self.keys.lock().unwrap_or_else(|e| e.into_inner());
        
        match keys.get_mut(&key_hash) {
            Some(key_info) => {
                key_info.increment_wallets();
                Ok(())
            }
            None => Err("API key no encontrada".to_string()),
        }
    }

    /**
     * Registra un request
     */
    pub fn record_request(&self, key: &str) -> Result<(), String> {
        let key_hash = Self::hash_key(key);
        let mut keys = self.keys.lock().unwrap_or_else(|e| e.into_inner());
        
        match keys.get_mut(&key_hash) {
            Some(key_info) => {
                key_info.increment_requests();
                Ok(())
            }
            None => Err("API key no encontrada".to_string()),
        }
    }

    /**
     * Obtiene información de uso de una API key
     */
    pub fn get_usage(&self, key: &str) -> Result<UsageStats, String> {
        let key_info = self.validate_key(key)?;
        Ok(key_info.usage.clone())
    }

    /**
     * Obtiene información completa de una API key
     */
    pub fn get_key_info(&self, key: &str) -> Result<APIKeyInfo, String> {
        self.validate_key(key)
    }

    /**
     * Desactiva una API key
     */
    pub fn deactivate_key(&self, key: &str) -> Result<(), String> {
        let key_hash = Self::hash_key(key);
        let mut keys = self.keys.lock().unwrap_or_else(|e| e.into_inner());
        
        match keys.get_mut(&key_hash) {
            Some(key_info) => {
                key_info.is_active = false;
                Ok(())
            }
            None => Err("API key no encontrada".to_string()),
        }
    }

    /**
     * Obtiene el rate limit de una API key
     */
    pub fn get_rate_limit(&self, key: &str) -> Result<u32, String> {
        let key_info = self.validate_key(key)?;
        Ok(key_info.rate_limit_per_minute)
    }
}

impl Default for BillingManager {
    fn default() -> Self {
        Self::new()
    }
}

