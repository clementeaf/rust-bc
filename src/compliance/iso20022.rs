//! ISO 20022 financial messaging — validation schemas for payment messages.
//!
//! Supports:
//! - **pacs.008**: FI to FI Customer Credit Transfer
//! - **pain.001**: Customer Credit Transfer Initiation
//! - **camt.053**: Bank to Customer Statement
//!
//! Messages are represented as typed Rust structs, validated at ingestion,
//! and storable in chaincode world state.

use serde::{Deserialize, Serialize};

use super::iso3166;
use super::iso4217;

/// ISO 20022 message type identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    /// pacs.008 — FI to FI Customer Credit Transfer
    Pacs008,
    /// pacs.002 — FI to FI Payment Status Report
    Pacs002,
    /// pacs.004 — Payment Return
    Pacs004,
    /// pain.001 — Customer Credit Transfer Initiation
    Pain001,
    /// pain.002 — Customer Payment Status Report
    Pain002,
    /// camt.052 — Bank to Customer Account Report (intraday)
    Camt052,
    /// camt.053 — Bank to Customer Statement
    Camt053,
}

/// Validation errors for ISO 20022 messages.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ValidationError {
    #[error("missing required field: {0}")]
    MissingField(String),
    #[error("invalid currency code: {0}")]
    InvalidCurrency(String),
    #[error("invalid country code: {0}")]
    InvalidCountry(String),
    #[error("invalid IBAN: {0}")]
    InvalidIban(String),
    #[error("amount must be positive")]
    InvalidAmount,
    #[error("invalid BIC: {0}")]
    InvalidBic(String),
}

/// Amount with currency — ISO 20022 `ActiveCurrencyAndAmount`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyAmount {
    pub amount: u64,
    pub currency: String,
}

/// Party identification — simplified ISO 20022 `PartyIdentification`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Party {
    pub name: String,
    pub country: String,
    pub account_iban: Option<String>,
    pub bic: Option<String>,
}

/// pacs.008 — FI to FI Customer Credit Transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pacs008 {
    pub message_id: String,
    pub creation_date: String,
    pub settlement_amount: CurrencyAmount,
    pub debtor: Party,
    pub creditor: Party,
    pub debtor_agent_bic: String,
    pub creditor_agent_bic: String,
    pub remittance_info: Option<String>,
}

/// pain.001 — Customer Credit Transfer Initiation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pain001 {
    pub message_id: String,
    pub creation_date: String,
    pub initiating_party: Party,
    pub payments: Vec<PaymentInstruction>,
}

/// A single payment instruction within pain.001.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentInstruction {
    pub instruction_id: String,
    pub amount: CurrencyAmount,
    pub creditor: Party,
    pub remittance_info: Option<String>,
}

/// camt.053 — Bank to Customer Statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camt053 {
    pub message_id: String,
    pub creation_date: String,
    pub account_iban: String,
    pub opening_balance: CurrencyAmount,
    pub closing_balance: CurrencyAmount,
    pub entries: Vec<StatementEntry>,
}

/// A single entry in a camt.053 statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementEntry {
    pub entry_id: String,
    pub amount: CurrencyAmount,
    pub credit_debit: CreditDebit,
    pub counterparty: String,
    pub value_date: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CreditDebit {
    Credit,
    Debit,
}

/// pacs.002 — FI to FI Payment Status Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pacs002 {
    pub message_id: String,
    pub creation_date: String,
    pub original_message_id: String,
    pub status: PaymentStatus,
    pub reason: Option<String>,
}

/// pacs.004 — Payment Return.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pacs004 {
    pub message_id: String,
    pub creation_date: String,
    pub original_message_id: String,
    pub return_amount: CurrencyAmount,
    pub return_reason: String,
    pub debtor_agent_bic: String,
    pub creditor_agent_bic: String,
}

/// pain.002 — Customer Payment Status Report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pain002 {
    pub message_id: String,
    pub creation_date: String,
    pub original_message_id: String,
    pub statuses: Vec<InstructionStatus>,
}

/// Status of a single payment instruction within pain.002.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionStatus {
    pub instruction_id: String,
    pub status: PaymentStatus,
    pub reason: Option<String>,
}

/// Payment status codes (subset of ISO 20022 ExternalPaymentTransactionStatus).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    Accepted,
    Pending,
    Rejected,
    Cancelled,
}

/// camt.052 — Bank to Customer Account Report (intraday).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camt052 {
    pub message_id: String,
    pub creation_date: String,
    pub account_iban: String,
    pub balance: CurrencyAmount,
    pub entries: Vec<StatementEntry>,
}

// ── Validation ───────────────────────────────────────────────────────────────

fn validate_currency(code: &str) -> Result<(), ValidationError> {
    if iso4217::is_valid_currency(code) {
        Ok(())
    } else {
        Err(ValidationError::InvalidCurrency(code.to_string()))
    }
}

fn validate_country(code: &str) -> Result<(), ValidationError> {
    if iso3166::is_valid_country(code) {
        Ok(())
    } else {
        Err(ValidationError::InvalidCountry(code.to_string()))
    }
}

fn validate_amount(a: &CurrencyAmount) -> Result<(), ValidationError> {
    if a.amount == 0 {
        return Err(ValidationError::InvalidAmount);
    }
    validate_currency(&a.currency)
}

fn validate_bic(bic: &str) -> Result<(), ValidationError> {
    // BIC: 8 or 11 alphanumeric characters
    let len = bic.len();
    if (len == 8 || len == 11) && bic.chars().all(|c| c.is_ascii_alphanumeric()) {
        Ok(())
    } else {
        Err(ValidationError::InvalidBic(bic.to_string()))
    }
}

fn validate_iban(iban: &str) -> Result<(), ValidationError> {
    // Simplified: 15-34 alphanumeric, starts with 2 letters
    let len = iban.len();
    if (15..=34).contains(&len)
        && iban[..2].chars().all(|c| c.is_ascii_uppercase())
        && iban.chars().all(|c| c.is_ascii_alphanumeric())
    {
        Ok(())
    } else {
        Err(ValidationError::InvalidIban(iban.to_string()))
    }
}

fn validate_party(party: &Party) -> Result<(), ValidationError> {
    if party.name.is_empty() {
        return Err(ValidationError::MissingField("party.name".into()));
    }
    validate_country(&party.country)?;
    if let Some(ref iban) = party.account_iban {
        validate_iban(iban)?;
    }
    if let Some(ref bic) = party.bic {
        validate_bic(bic)?;
    }
    Ok(())
}

/// Validate a pacs.008 message.
pub fn validate_pacs008(msg: &Pacs008) -> Result<(), ValidationError> {
    if msg.message_id.is_empty() {
        return Err(ValidationError::MissingField("message_id".into()));
    }
    validate_amount(&msg.settlement_amount)?;
    validate_party(&msg.debtor)?;
    validate_party(&msg.creditor)?;
    validate_bic(&msg.debtor_agent_bic)?;
    validate_bic(&msg.creditor_agent_bic)?;
    Ok(())
}

/// Validate a pain.001 message.
pub fn validate_pain001(msg: &Pain001) -> Result<(), ValidationError> {
    if msg.message_id.is_empty() {
        return Err(ValidationError::MissingField("message_id".into()));
    }
    validate_party(&msg.initiating_party)?;
    if msg.payments.is_empty() {
        return Err(ValidationError::MissingField("payments".into()));
    }
    for p in &msg.payments {
        validate_amount(&p.amount)?;
        validate_party(&p.creditor)?;
    }
    Ok(())
}

/// Validate a camt.053 message.
pub fn validate_camt053(msg: &Camt053) -> Result<(), ValidationError> {
    if msg.message_id.is_empty() {
        return Err(ValidationError::MissingField("message_id".into()));
    }
    validate_iban(&msg.account_iban)?;
    validate_amount(&msg.opening_balance)?;
    validate_amount(&msg.closing_balance)?;
    for e in &msg.entries {
        validate_amount(&e.amount)?;
    }
    Ok(())
}

/// Validate a pacs.002 message.
pub fn validate_pacs002(msg: &Pacs002) -> Result<(), ValidationError> {
    if msg.message_id.is_empty() {
        return Err(ValidationError::MissingField("message_id".into()));
    }
    if msg.original_message_id.is_empty() {
        return Err(ValidationError::MissingField("original_message_id".into()));
    }
    Ok(())
}

/// Validate a pacs.004 message.
pub fn validate_pacs004(msg: &Pacs004) -> Result<(), ValidationError> {
    if msg.message_id.is_empty() {
        return Err(ValidationError::MissingField("message_id".into()));
    }
    if msg.original_message_id.is_empty() {
        return Err(ValidationError::MissingField("original_message_id".into()));
    }
    validate_amount(&msg.return_amount)?;
    validate_bic(&msg.debtor_agent_bic)?;
    validate_bic(&msg.creditor_agent_bic)?;
    Ok(())
}

/// Validate a pain.002 message.
pub fn validate_pain002(msg: &Pain002) -> Result<(), ValidationError> {
    if msg.message_id.is_empty() {
        return Err(ValidationError::MissingField("message_id".into()));
    }
    if msg.original_message_id.is_empty() {
        return Err(ValidationError::MissingField("original_message_id".into()));
    }
    Ok(())
}

/// Validate a camt.052 message.
pub fn validate_camt052(msg: &Camt052) -> Result<(), ValidationError> {
    if msg.message_id.is_empty() {
        return Err(ValidationError::MissingField("message_id".into()));
    }
    validate_iban(&msg.account_iban)?;
    validate_amount(&msg.balance)?;
    for e in &msg.entries {
        validate_amount(&e.amount)?;
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_party(country: &str) -> Party {
        Party {
            name: "Acme Corp".into(),
            country: country.into(),
            account_iban: Some("CL9300000000123456789012".into()),
            bic: Some("BCHICLRM".into()),
        }
    }

    fn sample_amount(currency: &str) -> CurrencyAmount {
        CurrencyAmount {
            amount: 100000,
            currency: currency.into(),
        }
    }

    // ── pacs.008 ──

    #[test]
    fn pacs008_valid() {
        let msg = Pacs008 {
            message_id: "MSG001".into(),
            creation_date: "2026-05-08".into(),
            settlement_amount: sample_amount("CLP"),
            debtor: sample_party("CL"),
            creditor: sample_party("AR"),
            debtor_agent_bic: "BCHICLRM".into(),
            creditor_agent_bic: "NACNARBAXXX".into(),
            remittance_info: None,
        };
        assert!(validate_pacs008(&msg).is_ok());
    }

    #[test]
    fn pacs008_missing_id() {
        let msg = Pacs008 {
            message_id: "".into(),
            creation_date: "2026-05-08".into(),
            settlement_amount: sample_amount("CLP"),
            debtor: sample_party("CL"),
            creditor: sample_party("CL"),
            debtor_agent_bic: "BCHICLRM".into(),
            creditor_agent_bic: "BCHICLRM".into(),
            remittance_info: None,
        };
        assert_eq!(
            validate_pacs008(&msg),
            Err(ValidationError::MissingField("message_id".into()))
        );
    }

    #[test]
    fn pacs008_invalid_currency() {
        let msg = Pacs008 {
            message_id: "MSG001".into(),
            creation_date: "2026-05-08".into(),
            settlement_amount: sample_amount("ZZZ"),
            debtor: sample_party("CL"),
            creditor: sample_party("CL"),
            debtor_agent_bic: "BCHICLRM".into(),
            creditor_agent_bic: "BCHICLRM".into(),
            remittance_info: None,
        };
        assert!(matches!(
            validate_pacs008(&msg),
            Err(ValidationError::InvalidCurrency(_))
        ));
    }

    #[test]
    fn pacs008_invalid_country() {
        let msg = Pacs008 {
            message_id: "MSG001".into(),
            creation_date: "2026-05-08".into(),
            settlement_amount: sample_amount("CLP"),
            debtor: sample_party("XX"),
            creditor: sample_party("CL"),
            debtor_agent_bic: "BCHICLRM".into(),
            creditor_agent_bic: "BCHICLRM".into(),
            remittance_info: None,
        };
        assert!(matches!(
            validate_pacs008(&msg),
            Err(ValidationError::InvalidCountry(_))
        ));
    }

    #[test]
    fn pacs008_invalid_bic() {
        let msg = Pacs008 {
            message_id: "MSG001".into(),
            creation_date: "2026-05-08".into(),
            settlement_amount: sample_amount("CLP"),
            debtor: sample_party("CL"),
            creditor: sample_party("CL"),
            debtor_agent_bic: "X".into(),
            creditor_agent_bic: "BCHICLRM".into(),
            remittance_info: None,
        };
        assert!(matches!(
            validate_pacs008(&msg),
            Err(ValidationError::InvalidBic(_))
        ));
    }

    // ── pain.001 ──

    #[test]
    fn pain001_valid() {
        let msg = Pain001 {
            message_id: "PAY001".into(),
            creation_date: "2026-05-08".into(),
            initiating_party: sample_party("CL"),
            payments: vec![PaymentInstruction {
                instruction_id: "INST001".into(),
                amount: sample_amount("USD"),
                creditor: sample_party("US"),
                remittance_info: Some("Invoice 123".into()),
            }],
        };
        assert!(validate_pain001(&msg).is_ok());
    }

    #[test]
    fn pain001_empty_payments() {
        let msg = Pain001 {
            message_id: "PAY001".into(),
            creation_date: "2026-05-08".into(),
            initiating_party: sample_party("CL"),
            payments: vec![],
        };
        assert_eq!(
            validate_pain001(&msg),
            Err(ValidationError::MissingField("payments".into()))
        );
    }

    // ── camt.053 ──

    #[test]
    fn camt053_valid() {
        let msg = Camt053 {
            message_id: "STMT001".into(),
            creation_date: "2026-05-08".into(),
            account_iban: "CL9300000000123456789012".into(),
            opening_balance: sample_amount("CLP"),
            closing_balance: sample_amount("CLP"),
            entries: vec![StatementEntry {
                entry_id: "E001".into(),
                amount: sample_amount("CLP"),
                credit_debit: CreditDebit::Credit,
                counterparty: "Proveedor SA".into(),
                value_date: "2026-05-07".into(),
            }],
        };
        assert!(validate_camt053(&msg).is_ok());
    }

    #[test]
    fn camt053_invalid_iban() {
        let msg = Camt053 {
            message_id: "STMT001".into(),
            creation_date: "2026-05-08".into(),
            account_iban: "bad".into(),
            opening_balance: sample_amount("CLP"),
            closing_balance: sample_amount("CLP"),
            entries: vec![],
        };
        assert!(matches!(
            validate_camt053(&msg),
            Err(ValidationError::InvalidIban(_))
        ));
    }

    #[test]
    fn zero_amount_rejected() {
        let a = CurrencyAmount {
            amount: 0,
            currency: "USD".into(),
        };
        assert_eq!(validate_amount(&a), Err(ValidationError::InvalidAmount));
    }

    // ── IBAN / BIC ──

    #[test]
    fn valid_iban() {
        assert!(validate_iban("CL9300000000123456789012").is_ok());
        assert!(validate_iban("DE89370400440532013000").is_ok());
    }

    #[test]
    fn invalid_iban_too_short() {
        assert!(validate_iban("CL93000").is_err());
    }

    #[test]
    fn valid_bic_8_chars() {
        assert!(validate_bic("BCHICLRM").is_ok());
    }

    #[test]
    fn valid_bic_11_chars() {
        assert!(validate_bic("NACNARBAXXX").is_ok());
    }

    #[test]
    fn invalid_bic_wrong_length() {
        assert!(validate_bic("BCHI").is_err());
    }

    // ── serde roundtrip ──

    #[test]
    fn pacs008_serde_roundtrip() {
        let msg = Pacs008 {
            message_id: "MSG001".into(),
            creation_date: "2026-05-08".into(),
            settlement_amount: sample_amount("CLP"),
            debtor: sample_party("CL"),
            creditor: sample_party("AR"),
            debtor_agent_bic: "BCHICLRM".into(),
            creditor_agent_bic: "NACNARBAXXX".into(),
            remittance_info: Some("test".into()),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let restored: Pacs008 = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.message_id, "MSG001");
    }

    // ── pacs.002 ──

    #[test]
    fn pacs002_valid() {
        let msg = Pacs002 {
            message_id: "STS001".into(),
            creation_date: "2026-05-08".into(),
            original_message_id: "MSG001".into(),
            status: PaymentStatus::Accepted,
            reason: None,
        };
        assert!(validate_pacs002(&msg).is_ok());
    }

    #[test]
    fn pacs002_missing_original() {
        let msg = Pacs002 {
            message_id: "STS001".into(),
            creation_date: "2026-05-08".into(),
            original_message_id: "".into(),
            status: PaymentStatus::Rejected,
            reason: Some("insufficient funds".into()),
        };
        assert!(validate_pacs002(&msg).is_err());
    }

    // ── pacs.004 ──

    #[test]
    fn pacs004_valid() {
        let msg = Pacs004 {
            message_id: "RET001".into(),
            creation_date: "2026-05-08".into(),
            original_message_id: "MSG001".into(),
            return_amount: sample_amount("CLP"),
            return_reason: "duplicate payment".into(),
            debtor_agent_bic: "BCHICLRM".into(),
            creditor_agent_bic: "NACNARBAXXX".into(),
        };
        assert!(validate_pacs004(&msg).is_ok());
    }

    #[test]
    fn pacs004_invalid_bic() {
        let msg = Pacs004 {
            message_id: "RET001".into(),
            creation_date: "2026-05-08".into(),
            original_message_id: "MSG001".into(),
            return_amount: sample_amount("CLP"),
            return_reason: "error".into(),
            debtor_agent_bic: "BAD".into(),
            creditor_agent_bic: "NACNARBAXXX".into(),
        };
        assert!(matches!(
            validate_pacs004(&msg),
            Err(ValidationError::InvalidBic(_))
        ));
    }

    // ── pain.002 ──

    #[test]
    fn pain002_valid() {
        let msg = Pain002 {
            message_id: "PSTS001".into(),
            creation_date: "2026-05-08".into(),
            original_message_id: "PAY001".into(),
            statuses: vec![InstructionStatus {
                instruction_id: "INST001".into(),
                status: PaymentStatus::Accepted,
                reason: None,
            }],
        };
        assert!(validate_pain002(&msg).is_ok());
    }

    // ── camt.052 ──

    #[test]
    fn camt052_valid() {
        let msg = Camt052 {
            message_id: "RPT001".into(),
            creation_date: "2026-05-08".into(),
            account_iban: "CL9300000000123456789012".into(),
            balance: sample_amount("CLP"),
            entries: vec![],
        };
        assert!(validate_camt052(&msg).is_ok());
    }

    #[test]
    fn camt052_invalid_iban() {
        let msg = Camt052 {
            message_id: "RPT001".into(),
            creation_date: "2026-05-08".into(),
            account_iban: "bad".into(),
            balance: sample_amount("CLP"),
            entries: vec![],
        };
        assert!(matches!(
            validate_camt052(&msg),
            Err(ValidationError::InvalidIban(_))
        ));
    }

    #[test]
    fn payment_status_serde_roundtrip() {
        let statuses = vec![
            PaymentStatus::Accepted,
            PaymentStatus::Pending,
            PaymentStatus::Rejected,
            PaymentStatus::Cancelled,
        ];
        for s in statuses {
            let json = serde_json::to_string(&s).unwrap();
            let restored: PaymentStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(restored, s);
        }
    }
}
