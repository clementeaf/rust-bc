//! ERC-3643 (T-REX) — Security Token framework for regulated assets.
//!
//! Implements the core concepts of ERC-3643 adapted for Cerulean's
//! permissioned model:
//! - **Identity Registry**: links DIDs to verified investor identities
//! - **Compliance Module**: rules that must be satisfied for transfers
//! - **Token with forced transfers**: issuer can freeze, force-transfer, recover
//!
//! Reference: https://erc3643.info/

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;

/// Investor claim types (KYC/AML verification status).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClaimType {
    /// Identity verified (KYC level 1)
    IdentityVerified,
    /// Accredited investor status
    AccreditedInvestor,
    /// Country of residence verified
    CountryVerified,
    /// AML screening passed
    AmlCleared,
}

/// A verified claim attached to an investor identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityClaim {
    pub claim_type: ClaimType,
    pub issuer_did: String,
    pub issued_at: u64,
    pub expires_at: Option<u64>,
    pub country: Option<String>,
}

impl IdentityClaim {
    pub fn is_expired(&self, now: u64) -> bool {
        self.expires_at.is_some_and(|exp| now > exp)
    }
}

/// Investor identity in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvestorIdentity {
    pub did: String,
    pub claims: Vec<IdentityClaim>,
    pub frozen: bool,
}

/// Identity registry — maps DIDs to verified investor identities.
pub struct IdentityRegistry {
    identities: Mutex<HashMap<String, InvestorIdentity>>,
}

impl IdentityRegistry {
    pub fn new() -> Self {
        Self {
            identities: Mutex::new(HashMap::new()),
        }
    }

    pub fn register(&self, did: &str, claims: Vec<IdentityClaim>) {
        let mut map = self.identities.lock().unwrap();
        map.insert(
            did.to_string(),
            InvestorIdentity {
                did: did.to_string(),
                claims,
                frozen: false,
            },
        );
    }

    pub fn get(&self, did: &str) -> Option<InvestorIdentity> {
        self.identities.lock().unwrap().get(did).cloned()
    }

    pub fn has_valid_claim(&self, did: &str, claim_type: ClaimType, now: u64) -> bool {
        self.identities.lock().unwrap().get(did).is_some_and(|id| {
            id.claims
                .iter()
                .any(|c| c.claim_type == claim_type && !c.is_expired(now))
        })
    }

    pub fn freeze(&self, did: &str) -> bool {
        let mut map = self.identities.lock().unwrap();
        if let Some(id) = map.get_mut(did) {
            id.frozen = true;
            true
        } else {
            false
        }
    }

    pub fn unfreeze(&self, did: &str) -> bool {
        let mut map = self.identities.lock().unwrap();
        if let Some(id) = map.get_mut(did) {
            id.frozen = false;
            true
        } else {
            false
        }
    }

    pub fn is_frozen(&self, did: &str) -> bool {
        self.identities
            .lock()
            .unwrap()
            .get(did)
            .is_some_and(|id| id.frozen)
    }
}

impl Default for IdentityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A compliance rule that must pass for a transfer to be allowed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComplianceRule {
    /// Both parties must have a specific claim type.
    RequireClaim(ClaimType),
    /// Maximum number of holders allowed.
    MaxHolders(usize),
    /// Transfer only allowed between parties in these countries.
    AllowedCountries(Vec<String>),
    /// Minimum holding period in seconds before transfer.
    MinHoldingPeriod(u64),
    /// Maximum percentage of supply one holder can own (0-100).
    MaxOwnershipPercent(u8),
}

/// Result of compliance check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComplianceResult {
    Allowed,
    Denied(String),
}

/// Compliance module — evaluates rules against a transfer.
pub struct ComplianceModule {
    rules: Vec<ComplianceRule>,
}

impl ComplianceModule {
    pub fn new(rules: Vec<ComplianceRule>) -> Self {
        Self { rules }
    }

    /// Check if a transfer is compliant.
    #[allow(clippy::too_many_arguments)]
    pub fn check_transfer(
        &self,
        from: &str,
        to: &str,
        _amount: u64,
        registry: &IdentityRegistry,
        now: u64,
        current_holders: usize,
        recipient_balance: u64,
        total_supply: u64,
    ) -> ComplianceResult {
        // Frozen check
        if registry.is_frozen(from) {
            return ComplianceResult::Denied("sender is frozen".into());
        }
        if registry.is_frozen(to) {
            return ComplianceResult::Denied("recipient is frozen".into());
        }

        for rule in &self.rules {
            match rule {
                ComplianceRule::RequireClaim(claim_type) => {
                    if !registry.has_valid_claim(from, *claim_type, now) {
                        return ComplianceResult::Denied(format!(
                            "sender missing claim: {claim_type:?}"
                        ));
                    }
                    if !registry.has_valid_claim(to, *claim_type, now) {
                        return ComplianceResult::Denied(format!(
                            "recipient missing claim: {claim_type:?}"
                        ));
                    }
                }
                ComplianceRule::MaxHolders(max) => {
                    // If recipient has zero balance, this adds a new holder
                    if recipient_balance == 0 && current_holders >= *max {
                        return ComplianceResult::Denied(format!(
                            "max holders ({max}) would be exceeded"
                        ));
                    }
                }
                ComplianceRule::AllowedCountries(countries) => {
                    let check_country = |did: &str| -> Result<(), String> {
                        let id = registry.get(did).ok_or("identity not found")?;
                        let country = id
                            .claims
                            .iter()
                            .find(|c| c.claim_type == ClaimType::CountryVerified)
                            .and_then(|c| c.country.as_deref())
                            .ok_or("no country claim")?;
                        if countries.iter().any(|c| c == country) {
                            Ok(())
                        } else {
                            Err(format!("country {country} not in allowed list"))
                        }
                    };
                    if let Err(reason) = check_country(from) {
                        return ComplianceResult::Denied(format!("sender: {reason}"));
                    }
                    if let Err(reason) = check_country(to) {
                        return ComplianceResult::Denied(format!("recipient: {reason}"));
                    }
                }
                ComplianceRule::MaxOwnershipPercent(max_pct) => {
                    let new_balance = recipient_balance + _amount;
                    if let Some(pct) = new_balance.saturating_mul(100).checked_div(total_supply) {
                        if pct > *max_pct as u64 {
                            return ComplianceResult::Denied(format!(
                                "recipient would own {pct}%, max is {max_pct}%"
                            ));
                        }
                    }
                }
                ComplianceRule::MinHoldingPeriod(_) => {
                    // Would need acquisition timestamp per holder — defer to phase 2
                }
            }
        }

        ComplianceResult::Allowed
    }
}

/// Security token with issuer controls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityToken {
    pub name: String,
    pub symbol: String,
    pub total_supply: u64,
    pub issuer_did: String,
    pub balances: HashMap<String, u64>,
}

impl SecurityToken {
    pub fn new(name: &str, symbol: &str, issuer_did: &str, initial_supply: u64) -> Self {
        let mut balances = HashMap::new();
        balances.insert(issuer_did.to_string(), initial_supply);
        Self {
            name: name.to_string(),
            symbol: symbol.to_string(),
            total_supply: initial_supply,
            issuer_did: issuer_did.to_string(),
            balances,
        }
    }

    pub fn balance_of(&self, did: &str) -> u64 {
        *self.balances.get(did).unwrap_or(&0)
    }

    pub fn holder_count(&self) -> usize {
        self.balances.values().filter(|&&b| b > 0).count()
    }

    /// Transfer tokens with compliance check.
    pub fn transfer(
        &mut self,
        from: &str,
        to: &str,
        amount: u64,
        compliance: &ComplianceModule,
        registry: &IdentityRegistry,
        now: u64,
    ) -> ComplianceResult {
        if self.balance_of(from) < amount {
            return ComplianceResult::Denied("insufficient balance".into());
        }

        let result = compliance.check_transfer(
            from,
            to,
            amount,
            registry,
            now,
            self.holder_count(),
            self.balance_of(to),
            self.total_supply,
        );

        if result == ComplianceResult::Allowed {
            *self.balances.entry(from.to_string()).or_insert(0) -= amount;
            *self.balances.entry(to.to_string()).or_insert(0) += amount;
        }

        result
    }

    /// Force transfer by issuer (recovery, legal order).
    pub fn force_transfer(
        &mut self,
        caller: &str,
        from: &str,
        to: &str,
        amount: u64,
    ) -> Result<(), String> {
        if caller != self.issuer_did {
            return Err("only issuer can force transfer".into());
        }
        if self.balance_of(from) < amount {
            return Err("insufficient balance".into());
        }
        *self.balances.entry(from.to_string()).or_insert(0) -= amount;
        *self.balances.entry(to.to_string()).or_insert(0) += amount;
        Ok(())
    }

    /// Mint new tokens (issuer only).
    pub fn mint(&mut self, caller: &str, to: &str, amount: u64) -> Result<(), String> {
        if caller != self.issuer_did {
            return Err("only issuer can mint".into());
        }
        self.total_supply = self
            .total_supply
            .checked_add(amount)
            .ok_or("overflow: total supply would exceed u64::MAX")?;
        *self.balances.entry(to.to_string()).or_insert(0) = self
            .balances
            .get(to)
            .unwrap_or(&0)
            .checked_add(amount)
            .ok_or("overflow: balance would exceed u64::MAX")?;
        Ok(())
    }

    /// Burn tokens (holder burns own tokens).
    pub fn burn(&mut self, holder: &str, amount: u64) -> Result<(), String> {
        if self.balance_of(holder) < amount {
            return Err("insufficient balance".into());
        }
        *self.balances.entry(holder.to_string()).or_insert(0) -= amount;
        self.total_supply -= amount;
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn kyc_claim(country: &str) -> Vec<IdentityClaim> {
        vec![
            IdentityClaim {
                claim_type: ClaimType::IdentityVerified,
                issuer_did: "did:cerulean:kyc-provider".into(),
                issued_at: 1000,
                expires_at: Some(999_999),
                country: None,
            },
            IdentityClaim {
                claim_type: ClaimType::CountryVerified,
                issuer_did: "did:cerulean:kyc-provider".into(),
                issued_at: 1000,
                expires_at: None,
                country: Some(country.into()),
            },
        ]
    }

    fn setup() -> (SecurityToken, IdentityRegistry, ComplianceModule) {
        let token = SecurityToken::new("Test Bond", "TBOND", "did:cerulean:issuer", 1_000_000);
        let registry = IdentityRegistry::new();
        registry.register("did:cerulean:issuer", kyc_claim("CL"));
        registry.register("did:cerulean:alice", kyc_claim("CL"));
        registry.register("did:cerulean:bob", kyc_claim("AR"));

        let compliance = ComplianceModule::new(vec![
            ComplianceRule::RequireClaim(ClaimType::IdentityVerified),
            ComplianceRule::AllowedCountries(vec!["CL".into(), "AR".into()]),
            ComplianceRule::MaxHolders(10),
            ComplianceRule::MaxOwnershipPercent(50),
        ]);

        (token, registry, compliance)
    }

    #[test]
    fn transfer_between_verified_investors() {
        let (mut token, registry, compliance) = setup();
        // Issuer sends to alice
        let result = token.transfer(
            "did:cerulean:issuer",
            "did:cerulean:alice",
            1000,
            &compliance,
            &registry,
            2000,
        );
        assert_eq!(result, ComplianceResult::Allowed);
        assert_eq!(token.balance_of("did:cerulean:alice"), 1000);
    }

    #[test]
    fn transfer_denied_unverified_recipient() {
        let (mut token, registry, compliance) = setup();
        // Charlie not registered
        let result = token.transfer(
            "did:cerulean:issuer",
            "did:cerulean:charlie",
            1000,
            &compliance,
            &registry,
            2000,
        );
        assert!(matches!(result, ComplianceResult::Denied(_)));
    }

    #[test]
    fn transfer_denied_frozen_sender() {
        let (mut token, registry, compliance) = setup();
        registry.freeze("did:cerulean:issuer");
        let result = token.transfer(
            "did:cerulean:issuer",
            "did:cerulean:alice",
            1000,
            &compliance,
            &registry,
            2000,
        );
        assert_eq!(result, ComplianceResult::Denied("sender is frozen".into()));
    }

    #[test]
    fn transfer_denied_disallowed_country() {
        let (mut token, registry, compliance) = setup();
        registry.register("did:cerulean:dave", kyc_claim("US")); // US not in allowed list
        let result = token.transfer(
            "did:cerulean:issuer",
            "did:cerulean:dave",
            1000,
            &compliance,
            &registry,
            2000,
        );
        assert!(matches!(result, ComplianceResult::Denied(_)));
    }

    #[test]
    fn transfer_denied_max_ownership() {
        let (mut token, registry, compliance) = setup();
        // Try to send 600K (60%) — max is 50%
        let result = token.transfer(
            "did:cerulean:issuer",
            "did:cerulean:alice",
            600_000,
            &compliance,
            &registry,
            2000,
        );
        assert!(matches!(result, ComplianceResult::Denied(_)));
    }

    #[test]
    fn transfer_denied_insufficient_balance() {
        let (mut token, registry, compliance) = setup();
        let result = token.transfer(
            "did:cerulean:alice",
            "did:cerulean:bob",
            1000,
            &compliance,
            &registry,
            2000,
        );
        assert_eq!(
            result,
            ComplianceResult::Denied("insufficient balance".into())
        );
    }

    #[test]
    fn force_transfer_by_issuer() {
        let (mut token, _, _) = setup();
        token
            .force_transfer(
                "did:cerulean:issuer",
                "did:cerulean:issuer",
                "did:cerulean:alice",
                500,
            )
            .unwrap();
        assert_eq!(token.balance_of("did:cerulean:alice"), 500);
    }

    #[test]
    fn force_transfer_denied_non_issuer() {
        let (mut token, _, _) = setup();
        let err = token
            .force_transfer(
                "did:cerulean:alice",
                "did:cerulean:issuer",
                "did:cerulean:alice",
                500,
            )
            .unwrap_err();
        assert!(err.contains("only issuer"));
    }

    #[test]
    fn mint_by_issuer() {
        let (mut token, _, _) = setup();
        token
            .mint("did:cerulean:issuer", "did:cerulean:alice", 5000)
            .unwrap();
        assert_eq!(token.balance_of("did:cerulean:alice"), 5000);
        assert_eq!(token.total_supply, 1_005_000);
    }

    #[test]
    fn mint_denied_non_issuer() {
        let (mut token, _, _) = setup();
        assert!(token
            .mint("did:cerulean:alice", "did:cerulean:alice", 5000)
            .is_err());
    }

    #[test]
    fn burn_reduces_supply() {
        let (mut token, _, _) = setup();
        token.burn("did:cerulean:issuer", 1000).unwrap();
        assert_eq!(token.total_supply, 999_000);
    }

    #[test]
    fn freeze_and_unfreeze() {
        let registry = IdentityRegistry::new();
        registry.register("did:cerulean:alice", vec![]);
        assert!(!registry.is_frozen("did:cerulean:alice"));
        registry.freeze("did:cerulean:alice");
        assert!(registry.is_frozen("did:cerulean:alice"));
        registry.unfreeze("did:cerulean:alice");
        assert!(!registry.is_frozen("did:cerulean:alice"));
    }

    #[test]
    fn expired_claim_rejected() {
        let registry = IdentityRegistry::new();
        registry.register(
            "did:cerulean:expired",
            vec![IdentityClaim {
                claim_type: ClaimType::IdentityVerified,
                issuer_did: "did:cerulean:kyc".into(),
                issued_at: 100,
                expires_at: Some(500),
                country: None,
            }],
        );
        assert!(!registry.has_valid_claim(
            "did:cerulean:expired",
            ClaimType::IdentityVerified,
            1000
        ));
        assert!(registry.has_valid_claim("did:cerulean:expired", ClaimType::IdentityVerified, 400));
    }

    #[test]
    fn holder_count_tracks_nonzero() {
        let (mut token, registry, compliance) = setup();
        assert_eq!(token.holder_count(), 1); // only issuer
        token.transfer(
            "did:cerulean:issuer",
            "did:cerulean:alice",
            100,
            &compliance,
            &registry,
            2000,
        );
        assert_eq!(token.holder_count(), 2);
    }

    #[test]
    fn security_token_serde_roundtrip() {
        let token = SecurityToken::new("Bond", "BND", "did:cerulean:issuer", 1000);
        let json = serde_json::to_string(&token).unwrap();
        let restored: SecurityToken = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.symbol, "BND");
        assert_eq!(restored.total_supply, 1000);
    }
}
