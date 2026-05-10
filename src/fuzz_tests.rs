//! Property-based fuzz tests for critical parsers and validators.
//!
//! Uses proptest to generate adversarial inputs that unit tests miss.
//! These tests exercise deserialization, validation, and state transitions
//! with randomized data to catch panics, overflows, and logic errors.

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    // ── Block deserialization ────────────────────────────────────────────────

    proptest! {
        /// Arbitrary JSON should never panic the block deserializer.
        #[test]
        fn block_deser_never_panics(data in "\\PC{0,500}") {
            let _ = serde_json::from_str::<crate::storage::traits::Block>(&data);
        }

        /// Arbitrary bytes as block fields should not panic.
        #[test]
        fn block_with_arbitrary_fields(
            height in any::<u64>(),
            timestamp in any::<u64>(),
            proposer in "[a-z0-9]{0,64}",
            tx_count in 0usize..20,
        ) {
            let txs: Vec<String> = (0..tx_count).map(|i| format!("tx-{i}")).collect();
            let block = crate::storage::traits::Block {
                height,
                timestamp,
                parent_hash: [0u8; 32],
                merkle_root: [0u8; 32],
                transactions: txs,
                proposer,
                signature: vec![0u8; 64],
                signature_algorithm: Default::default(),
                endorsements: vec![],
                secondary_signature: None,
                secondary_signature_algorithm: None,
                hash_algorithm: Default::default(),
                orderer_signature: None,
            };
            // Serialize and deserialize roundtrip must not panic
            let json = serde_json::to_string(&block).unwrap();
            let back: crate::storage::traits::Block = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(back.height, height);
            prop_assert_eq!(back.timestamp, timestamp);
        }
    }

    // ── ISO 20022 validation ─────────────────────────────────────────────────

    proptest! {
        /// Arbitrary strings in ISO 20022 fields should not panic the validator.
        #[test]
        fn pacs008_never_panics(
            msg_id in "\\PC{0,200}",
            date in "\\PC{0,30}",
            amount in any::<u64>(),
            currency in "[A-Z]{0,5}",
            debtor_name in "\\PC{0,200}",
            creditor_name in "\\PC{0,200}",
            country in "[A-Z]{0,4}",
        ) {
            use crate::compliance::iso20022::*;
            let msg = Pacs008 {
                message_id: msg_id,
                creation_date: date,
                settlement_amount: CurrencyAmount { amount, currency: currency.clone() },
                debtor: Party {
                    name: debtor_name,
                    country: country.clone(),
                    account_iban: None,
                    bic: Some("TESTBIC1".into()),
                },
                creditor: Party {
                    name: creditor_name,
                    country,
                    account_iban: None,
                    bic: Some("TESTBIC2".into()),
                },
                debtor_agent_bic: "TESTBIC1".into(),
                creditor_agent_bic: "TESTBIC2".into(),
                remittance_info: None,
            };
            // Must not panic regardless of input
            let _ = validate_pacs008(&msg);
        }
    }

    // ── Oracle price submission ──────────────────────────────────────────────

    proptest! {
        /// Arbitrary oracle submissions should not panic the registry.
        #[test]
        fn oracle_submit_never_panics(
            oracle_id in "[a-z0-9-]{1,32}",
            symbol in "[A-Z/]{1,10}",
            price in any::<u64>(),
            timestamp in 0u64..50000u64,
            confidence in 0u8..=100u8,
        ) {
            let mut registry = crate::oracle_system::OracleRegistry::new(66, 5000);
            let _ = registry.register_oracle(oracle_id.clone());
            let _ = registry.submit_price_report(
                &oracle_id,
                symbol,
                price,
                timestamp,
                vec![1, 2, 3], // arbitrary sig (test mode skips verification for small timestamps)
                confidence,
            );
        }

        /// Aggregation with arbitrary prices should not panic or overflow.
        #[test]
        fn oracle_aggregate_never_panics(
            prices in prop::collection::vec(1u64..u64::MAX, 1..10),
        ) {
            let mut registry = crate::oracle_system::OracleRegistry::new(66, 5000);
            for (i, price) in prices.iter().enumerate() {
                let id = format!("oracle-{i}");
                let _ = registry.register_oracle(id.clone());
                let _ = registry.submit_price_report(
                    &id,
                    "TEST".into(),
                    *price,
                    1000,
                    vec![1],
                    95,
                );
            }
            let _ = registry.aggregate_reports("TEST", 2000);
        }
    }

    // ── Credential parsing ───────────────────────────────────────────────────

    proptest! {
        /// Arbitrary JSON should never panic the credential deserializer.
        #[test]
        fn credential_deser_never_panics(data in "\\PC{0,500}") {
            let _ = serde_json::from_str::<crate::storage::traits::Credential>(&data);
        }

        /// Credential with extreme field values should serialize/deserialize safely.
        #[test]
        fn credential_roundtrip(
            id in "[a-z0-9-]{1,64}",
            issuer in "[a-z0-9:]{1,64}",
            subject in "[a-z0-9:]{1,64}",
            cred_type in "[A-Za-z]{1,32}",
            issued_at in any::<u64>(),
            expires_at in any::<u64>(),
        ) {
            let cred = crate::storage::traits::Credential {
                id: id.clone(),
                issuer_did: issuer,
                subject_did: subject,
                cred_type,
                claims: serde_json::json!({}),
                issued_at,
                expires_at,
                revoked_at: None,
                signature: "test".into(),
                status: Default::default(),
            };
            let json = serde_json::to_string(&cred).unwrap();
            let back: crate::storage::traits::Credential = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(&back.id, &id);
        }
    }

    // ── Signature consistency ────────────────────────────────────────────────

    proptest! {
        /// Arbitrary signature sizes should never panic the consistency checker.
        #[test]
        fn signature_consistency_never_panics(
            sig_len in 0usize..10000,
            algo_idx in 0u8..2,
        ) {
            use crate::identity::signing::SigningAlgorithm;
            let algo = if algo_idx == 0 {
                SigningAlgorithm::Ed25519
            } else {
                SigningAlgorithm::MlDsa65
            };
            let sig = vec![0xAA; sig_len];
            let _ = crate::identity::pqc_policy::validate_signature_consistency(
                algo, &sig, "fuzz-test",
            );
        }
    }

    // ── Transaction conflict detection ───────────────────────────────────────

    proptest! {
        /// Arbitrary transaction batches should not panic the scheduler.
        #[test]
        fn schedule_batch_never_panics(
            tx_count in 0usize..50,
            key_count in 1usize..10,
        ) {
            use crate::transaction::parallel::{schedule_batch, TxWithRwSet};
            use crate::transaction::rwset::{KVRead, KVWrite, ReadWriteSet};

            let keys: Vec<String> = (0..key_count).map(|i| format!("key-{i}")).collect();

            let txs: Vec<TxWithRwSet> = (0..tx_count).map(|i| {
                let read_key = &keys[i % key_count];
                let write_key = &keys[(i + 1) % key_count];
                TxWithRwSet {
                    index: i,
                    tx_id: format!("tx-{i}"),
                    rwset: ReadWriteSet {
                        reads: vec![KVRead { key: read_key.clone(), version: 1 }],
                        writes: vec![KVWrite { key: write_key.clone(), value: vec![1] }],
                    },
                }
            }).collect();

            let schedule = schedule_batch(&txs);
            // Invariant: all transactions must appear in exactly one wave
            let total_in_waves: usize = schedule.waves.iter().map(|w| w.tx_indices.len()).sum();
            prop_assert_eq!(total_in_waves, tx_count);
        }
    }
}
