/// Smart Contract Integration Module
/// 
/// This module provides interfaces for interacting with Ethereum smart contracts,
/// specifically ERC-20 tokens and ERC-721 NFTs. It abstracts away the complexity
/// of ethers-rs and provides a clean API for the oracle system to use.
///
/// Architecture:
/// - SmartContractProvider: Manages connections to Ethereum
/// - ERC20Contract: Token operations (transfer, balance, approve)
/// - ERC721Contract: NFT operations (transfer, mint, burn, ownership)
/// - ContractError: Custom error types for contract operations

use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::{debug, warn, info};

/// Type alias for Ethereum addresses as strings
pub type EthereumAddress = String;

/// Type alias for transaction hashes
pub type TransactionHash = String;

/// Custom error type for smart contract operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContractError {
    InvalidAddress(String),
    InsufficientBalance,
    TransactionFailed(String),
    ContractCallFailed(String),
    ProviderError(String),
    InvalidAmount,
    NotAuthorized,
}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContractError::InvalidAddress(addr) => write!(f, "Invalid address: {}", addr),
            ContractError::InsufficientBalance => write!(f, "Insufficient balance"),
            ContractError::TransactionFailed(msg) => write!(f, "Transaction failed: {}", msg),
            ContractError::ContractCallFailed(msg) => write!(f, "Contract call failed: {}", msg),
            ContractError::ProviderError(msg) => write!(f, "Provider error: {}", msg),
            ContractError::InvalidAmount => write!(f, "Invalid amount"),
            ContractError::NotAuthorized => write!(f, "Not authorized"),
        }
    }
}

impl std::error::Error for ContractError {}

/// Represents a U256 (256-bit unsigned integer) for token amounts
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct U256(pub u128);  // Simplified: in production, use full u256

impl U256 {
    /// Create a new U256 from u128
    pub fn new(value: u128) -> Self {
        U256(value)
    }

    /// Convert to u128
    pub fn to_u128(&self) -> u128 {
        self.0
    }

    /// Check if zero
    pub fn is_zero(&self) -> bool {
        self.0 == 0
    }
}

impl From<u128> for U256 {
    fn from(val: u128) -> Self {
        U256(val)
    }
}

impl From<u64> for U256 {
    fn from(val: u64) -> Self {
        U256(val as u128)
    }
}

/// Configuration for smart contract provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartContractConfig {
    /// RPC endpoint URL (e.g., "https://eth.llamarpc.com" or "http://localhost:8545")
    pub rpc_endpoint: String,
    /// ERC-20 contract address
    pub erc20_address: EthereumAddress,
    /// ERC-721 contract address
    pub erc721_address: EthereumAddress,
    /// Private key for signing transactions (hex string without 0x prefix)
    pub private_key: String,
}

impl SmartContractConfig {
    /// Create a new smart contract configuration
    pub fn new(
        rpc_endpoint: String,
        erc20_address: EthereumAddress,
        erc721_address: EthereumAddress,
        private_key: String,
    ) -> Self {
        SmartContractConfig {
            rpc_endpoint,
            erc20_address,
            erc721_address,
            private_key,
        }
    }

    /// Validate configuration (check addresses are valid Ethereum addresses)
    pub fn validate(&self) -> Result<(), ContractError> {
        Self::validate_address(&self.erc20_address)?;
        Self::validate_address(&self.erc721_address)?;
        
        // Check private key is valid hex
        if self.private_key.len() != 64 {
            warn!("Private key length incorrect: expected 64 chars, got {}", self.private_key.len());
            return Err(ContractError::InvalidAddress(
                "Private key must be 64 hex characters".to_string(),
            ));
        }
        
        debug!("Smart contract configuration validated");
        Ok(())
    }

    /// Validate an Ethereum address format (0x followed by 40 hex chars)
    fn validate_address(address: &str) -> Result<(), ContractError> {
        if !address.starts_with("0x") || address.len() != 42 {
            return Err(ContractError::InvalidAddress(format!(
                "Invalid address format: {}",
                address
            )));
        }
        
        // Check if hex
        if i64::from_str_radix(&address[2..], 16).is_err() {
            return Err(ContractError::InvalidAddress(format!(
                "Address contains invalid hex characters: {}",
                address
            )));
        }
        
        Ok(())
    }
}

/// ERC-20 Token Contract Interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ERC20Contract {
    pub address: EthereumAddress,
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
}

impl ERC20Contract {
    /// Create a new ERC-20 contract reference
    pub fn new(
        address: EthereumAddress,
        name: String,
        symbol: String,
        decimals: u8,
    ) -> Self {
        ERC20Contract {
            address,
            name,
            symbol,
            decimals,
        }
    }

    /// Get balance of an account
    pub fn balance_of(&self, account: &str) -> Result<U256, ContractError> {
        SmartContractConfig::validate_address(account)?;
        debug!(account, token = &self.symbol, "Querying balance");
        // In production, this would make an ethers-rs call to the contract
        Ok(U256::new(0))  // Mock implementation
    }

    /// Transfer tokens to recipient
    pub fn transfer(&self, to: &str, amount: U256) -> Result<TransactionHash, ContractError> {
        SmartContractConfig::validate_address(to)?;
        
        if amount.is_zero() {
            warn!("Transfer with zero amount attempted");
            return Err(ContractError::InvalidAmount);
        }
        
        info!(token = &self.symbol, to, amount = amount.0, "Executing token transfer");
        // In production, this would sign and send a transaction via ethers-rs
        Ok(format!("0x{:064x}", 0))  // Mock transaction hash
    }

    /// Approve spender to spend tokens on behalf of owner
    pub fn approve(&self, spender: &str, amount: U256) -> Result<TransactionHash, ContractError> {
        SmartContractConfig::validate_address(spender)?;
        
        if amount.is_zero() {
            warn!("Approval with zero amount attempted");
            return Err(ContractError::InvalidAmount);
        }
        
        info!(token = &self.symbol, spender, amount = amount.0, "Executing approval");
        Ok(format!("0x{:064x}", 1))  // Mock transaction hash
    }

    /// Transfer tokens from one address to another (requires prior approval)
    pub fn transfer_from(
        &self,
        from: &str,
        to: &str,
        amount: U256,
    ) -> Result<TransactionHash, ContractError> {
        SmartContractConfig::validate_address(from)?;
        SmartContractConfig::validate_address(to)?;
        
        if amount.is_zero() {
            warn!("TransferFrom with zero amount attempted");
            return Err(ContractError::InvalidAmount);
        }
        
        info!(token = &self.symbol, from, to, amount = amount.0, "Executing transferFrom");
        Ok(format!("0x{:064x}", 2))  // Mock transaction hash
    }

    /// Get total supply of tokens
    pub fn total_supply(&self) -> Result<U256, ContractError> {
        debug!(token = &self.symbol, "Querying total supply");
        Ok(U256::new(1_000_000 * 10u128.pow(self.decimals as u32)))  // Mock
    }
}

/// ERC-721 NFT Contract Interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ERC721Contract {
    pub address: EthereumAddress,
    pub name: String,
    pub symbol: String,
}

impl ERC721Contract {
    /// Create a new ERC-721 contract reference
    pub fn new(address: EthereumAddress, name: String, symbol: String) -> Self {
        ERC721Contract {
            address,
            name,
            symbol,
        }
    }

    /// Get balance (number of NFTs) of an owner
    pub fn balance_of(&self, owner: &str) -> Result<U256, ContractError> {
        SmartContractConfig::validate_address(owner)?;
        debug!(owner, nft = &self.symbol, "Querying NFT balance");
        Ok(U256::new(0))  // Mock implementation
    }

    /// Get the owner of an NFT token
    pub fn owner_of(&self, token_id: U256) -> Result<EthereumAddress, ContractError> {
        debug!(token_id = token_id.0, nft = &self.symbol, "Querying NFT owner");
        Ok("0x0000000000000000000000000000000000000000".to_string())  // Mock
    }

    /// Transfer NFT from one address to another
    pub fn transfer_from(
        &self,
        from: &str,
        to: &str,
        token_id: U256,
    ) -> Result<TransactionHash, ContractError> {
        SmartContractConfig::validate_address(from)?;
        SmartContractConfig::validate_address(to)?;
        
        info!(nft = &self.symbol, from, to, token_id = token_id.0, "Transferring NFT");
        Ok(format!("0x{:064x}", 3))  // Mock transaction hash
    }

    /// Safely transfer NFT with onERC721Received callback
    pub fn safe_transfer_from(
        &self,
        from: &str,
        to: &str,
        token_id: U256,
    ) -> Result<TransactionHash, ContractError> {
        SmartContractConfig::validate_address(from)?;
        SmartContractConfig::validate_address(to)?;
        
        info!(nft = &self.symbol, from, to, token_id = token_id.0, "Safely transferring NFT");
        Ok(format!("0x{:064x}", 4))  // Mock transaction hash
    }

    /// Approve an address to spend a specific NFT
    pub fn approve(&self, to: &str, token_id: U256) -> Result<TransactionHash, ContractError> {
        SmartContractConfig::validate_address(to)?;
        
        info!(nft = &self.symbol, to, token_id = token_id.0, "Approving NFT transfer");
        Ok(format!("0x{:064x}", 5))  // Mock transaction hash
    }

    /// Mint a new NFT (requires appropriate permissions)
    pub fn mint(&self, to: &str, token_id: U256) -> Result<TransactionHash, ContractError> {
        SmartContractConfig::validate_address(to)?;
        
        info!(nft = &self.symbol, to, token_id = token_id.0, "Minting new NFT");
        Ok(format!("0x{:064x}", 6))  // Mock transaction hash
    }

    /// Burn (destroy) an NFT
    pub fn burn(&self, token_id: U256) -> Result<TransactionHash, ContractError> {
        info!(nft = &self.symbol, token_id = token_id.0, "Burning NFT");
        Ok(format!("0x{:064x}", 7))  // Mock transaction hash
    }

    /// Get total supply of NFTs
    pub fn total_supply(&self) -> Result<U256, ContractError> {
        debug!(nft = &self.symbol, "Querying total NFT supply");
        Ok(U256::new(10000))  // Mock
    }

    /// Get metadata URI for a token
    pub fn token_uri(&self, token_id: U256) -> Result<String, ContractError> {
        debug!(token_id = token_id.0, nft = &self.symbol, "Querying token URI");
        Ok(format!(
            "https://metadata.example.com/nft/{}/{}",
            &self.symbol, token_id.0
        ))
    }
}

/// Smart Contract Provider for managing contract interactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartContractProvider {
    pub config: SmartContractConfig,
    pub erc20: ERC20Contract,
    pub erc721: ERC721Contract,
}

impl SmartContractProvider {
    /// Create a new smart contract provider
    pub fn new(config: SmartContractConfig) -> Result<Self, ContractError> {
        config.validate()?;
        
        let erc20 = ERC20Contract::new(
            config.erc20_address.clone(),
            "TokenName".to_string(),
            "TKN".to_string(),
            18,
        );
        
        let erc721 = ERC721Contract::new(
            config.erc721_address.clone(),
            "NFTName".to_string(),
            "NFT".to_string(),
        );
        
        info!("Smart contract provider initialized with {} and {}", erc20.symbol, erc721.symbol);
        
        Ok(SmartContractProvider {
            config,
            erc20,
            erc721,
        })
    }

    /// Update contract metadata (name, symbol, decimals)
    pub fn update_erc20_metadata(
        &mut self,
        name: String,
        symbol: String,
        decimals: u8,
    ) {
        self.erc20.name = name;
        self.erc20.symbol = symbol;
        self.erc20.decimals = decimals;
        debug!("ERC-20 metadata updated");
    }

    /// Update ERC-721 metadata
    pub fn update_erc721_metadata(&mut self, name: String, symbol: String) {
        self.erc721.name = name;
        self.erc721.symbol = symbol;
        debug!("ERC-721 metadata updated");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u256_creation() {
        let val = U256::new(1000);
        assert_eq!(val.to_u128(), 1000);
        assert!(!val.is_zero());
    }

    #[test]
    fn test_u256_zero() {
        let val = U256::new(0);
        assert!(val.is_zero());
    }

    #[test]
    fn test_u256_from_u64() {
        let val = U256::from(500u64);
        assert_eq!(val.to_u128(), 500);
    }

    #[test]
    fn test_valid_ethereum_address() {
        let result =
            SmartContractConfig::validate_address("0x742d35Cc6634C0532925a3b844Bc9e7595f42bE0");
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_ethereum_address_format() {
        let result = SmartContractConfig::validate_address("0x742d35");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_ethereum_address_no_prefix() {
        let result = SmartContractConfig::validate_address("742d35Cc6634C0532925a3b844Bc9e7595f42bE0");
        assert!(result.is_err());
    }

    #[test]
    fn test_config_validation_valid() {
        let config = SmartContractConfig::new(
            "http://localhost:8545".to_string(),
            "0x742d35Cc6634C0532925a3b844Bc9e7595f42bE0".to_string(),
            "0x742d35Cc6634C0532925a3b844Bc9e7595f42bE1".to_string(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        );
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_invalid_address() {
        let config = SmartContractConfig::new(
            "http://localhost:8545".to_string(),
            "invalid_address".to_string(),
            "0x742d35Cc6634C0532925a3b844Bc9e7595f42bE1".to_string(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        );
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_erc20_transfer_zero_amount() {
        let erc20 = ERC20Contract::new(
            "0x742d35Cc6634C0532925a3b844Bc9e7595f42bE0".to_string(),
            "Token".to_string(),
            "TKN".to_string(),
            18,
        );
        let result = erc20.transfer(
            "0x742d35Cc6634C0532925a3b844Bc9e7595f42bE1",
            U256::new(0),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_erc20_transfer_valid() {
        let erc20 = ERC20Contract::new(
            "0x742d35Cc6634C0532925a3b844Bc9e7595f42bE0".to_string(),
            "Token".to_string(),
            "TKN".to_string(),
            18,
        );
        let result = erc20.transfer(
            "0x742d35Cc6634C0532925a3b844Bc9e7595f42bE1",
            U256::new(1000),
        );
        assert!(result.is_ok());
        assert!(!result.unwrap().is_empty());
    }

    #[test]
    fn test_erc721_mint() {
        let erc721 = ERC721Contract::new(
            "0x742d35Cc6634C0532925a3b844Bc9e7595f42bE0".to_string(),
            "NFT".to_string(),
            "NFT".to_string(),
        );
        let result = erc721.mint(
            "0x742d35Cc6634C0532925a3b844Bc9e7595f42bE1",
            U256::new(1),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_erc721_burn() {
        let erc721 = ERC721Contract::new(
            "0x742d35Cc6634C0532925a3b844Bc9e7595f42bE0".to_string(),
            "NFT".to_string(),
            "NFT".to_string(),
        );
        let result = erc721.burn(U256::new(1));
        assert!(result.is_ok());
    }

    #[test]
    fn test_smart_contract_provider_creation() {
        let config = SmartContractConfig::new(
            "http://localhost:8545".to_string(),
            "0x742d35Cc6634C0532925a3b844Bc9e7595f42bE0".to_string(),
            "0x742d35Cc6634C0532925a3b844Bc9e7595f42bE1".to_string(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        );
        let provider = SmartContractProvider::new(config);
        assert!(provider.is_ok());
    }

    #[test]
    fn test_smart_contract_provider_invalid_config() {
        let config = SmartContractConfig::new(
            "http://localhost:8545".to_string(),
            "invalid".to_string(),
            "0x742d35Cc6634C0532925a3b844Bc9e7595f42bE1".to_string(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        );
        let provider = SmartContractProvider::new(config);
        assert!(provider.is_err());
    }

    #[test]
    fn test_contract_error_display() {
        let err = ContractError::InvalidAddress("bad_address".to_string());
        assert_eq!(
            err.to_string(),
            "Invalid address: bad_address"
        );
    }

    #[test]
    fn test_erc20_total_supply() {
        let erc20 = ERC20Contract::new(
            "0x742d35Cc6634C0532925a3b844Bc9e7595f42bE0".to_string(),
            "Token".to_string(),
            "TKN".to_string(),
            18,
        );
        let supply = erc20.total_supply().unwrap();
        assert_eq!(supply.to_u128(), 1_000_000 * 10u128.pow(18));
    }
}
