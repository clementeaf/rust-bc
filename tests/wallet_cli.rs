//! Wallet CLI correctness tests — library-level validation of keygen,
//! address derivation, signing, and transfer submission.

use rust_bc::account::address::{address_from_pubkey, is_valid_address};
use rust_bc::account::{AccountStore, MemoryAccountStore};
use rust_bc::identity::signing::{SigningProvider, SoftwareSigningProvider};
use rust_bc::transaction::mempool::{Mempool, MempoolConfig};
use rust_bc::transaction::native::{
    execute_transfer, execute_transfer_checked, verify_tx_signature, NativeTransaction,
    NativeTxError,
};

// ── Key Generation ─────────────────────────────────────────────────────────

#[test]
fn ed25519_keygen_produces_valid_format() {
    let provider = SoftwareSigningProvider::generate();
    let pk = provider.public_key();
    assert_eq!(pk.len(), 32, "Ed25519 pubkey must be 32 bytes");

    let addr = address_from_pubkey(&pk);
    assert!(is_valid_address(&addr), "derived address must be valid hex");
}

#[test]
fn ed25519_keygen_is_unique() {
    let a = SoftwareSigningProvider::generate();
    let b = SoftwareSigningProvider::generate();
    assert_ne!(a.public_key(), b.public_key());
}

#[test]
fn mldsa_keygen_produces_valid_format() {
    use pqc_crypto_module::legacy::mldsa_raw::mldsa65;
    use pqcrypto_traits::sign::PublicKey;

    let (pk, _sk) = mldsa65::keypair();
    let pk_bytes = pk.as_bytes();
    assert_eq!(pk_bytes.len(), 1952, "ML-DSA-65 pubkey must be 1952 bytes");

    let addr = address_from_pubkey(pk_bytes);
    assert!(is_valid_address(&addr));
}

// ── Address Derivation ─────────────────────────────────────────────────────

#[test]
fn address_derivation_matches_hash_of_pubkey() {
    let provider = SoftwareSigningProvider::generate();
    let pk = provider.public_key();

    let addr1 = address_from_pubkey(&pk);
    let addr2 = address_from_pubkey(&pk);
    assert_eq!(addr1, addr2, "derivation must be deterministic");

    // Verify it's actually sha256(pk)[0..20] hex
    use pqc_crypto_module::legacy::legacy_sha256;
    let hash = legacy_sha256(&pk).unwrap();
    let expected = hex::encode(&hash[..20]);
    assert_eq!(addr1, expected);
}

#[test]
fn different_keys_produce_different_addresses() {
    let a = SoftwareSigningProvider::generate();
    let b = SoftwareSigningProvider::generate();
    assert_ne!(
        address_from_pubkey(&a.public_key()),
        address_from_pubkey(&b.public_key())
    );
}

// ── Signing ────────────────────────────────────────────────────────────────

#[test]
fn sign_transfer_produces_valid_signature() {
    let provider = SoftwareSigningProvider::generate();
    let pk = provider.public_key();

    let mut tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
    let payload = tx.signing_payload();
    tx.signature = provider.sign(&payload).unwrap();
    tx.signature_algorithm = "ed25519".to_string();

    assert_eq!(tx.signature.len(), 64);
    assert!(verify_tx_signature(&tx, &pk).unwrap());
}

#[test]
fn signed_transfer_accepted_by_execute() {
    let store = MemoryAccountStore::with_genesis(&[("alice", 10_000)]);
    let provider = SoftwareSigningProvider::generate();

    let mut tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
    let payload = tx.signing_payload();
    tx.signature = provider.sign(&payload).unwrap();

    // execute_transfer doesn't check sig (that's mempool/API layer), but verify passes
    assert!(verify_tx_signature(&tx, &provider.public_key()).unwrap());
    execute_transfer(&store, &tx, "miner").unwrap();

    let alice = store.get_account("alice").unwrap();
    assert_eq!(alice.balance, 10_000 - 100 - 5);
}

#[test]
fn signed_transfer_accepted_by_mempool() {
    let pool = Mempool::new(MempoolConfig {
        max_size: 100,
        max_per_sender: 10,
        min_fee: 1,
    });
    let provider = SoftwareSigningProvider::generate();

    let mut tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
    let payload = tx.signing_payload();
    tx.signature = provider.sign(&payload).unwrap();

    assert!(pool.add(tx).unwrap());
    assert_eq!(pool.len(), 1);
}

// ── Invalid Signature ──────────────────────────────────────────────────────

#[test]
fn invalid_signature_rejected_by_verify() {
    let mut tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
    tx.signature = vec![0xDE; 64]; // garbage

    let provider = SoftwareSigningProvider::generate();
    assert!(!verify_tx_signature(&tx, &provider.public_key()).unwrap());
}

#[test]
fn tampered_payload_invalidates_signature() {
    let provider = SoftwareSigningProvider::generate();

    let mut tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
    let payload = tx.signing_payload();
    tx.signature = provider.sign(&payload).unwrap();

    // Tamper with amount
    tx.fee = 999;

    // Signature no longer matches payload
    assert!(!verify_tx_signature(&tx, &provider.public_key()).unwrap());
}

#[test]
fn empty_signature_rejected() {
    let tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
    let err = verify_tx_signature(&tx, &[0u8; 32]).unwrap_err();
    assert!(matches!(err, NativeTxError::MissingSignature));
}

// ── Chain ID ───────────────────────────────────────────────────────────────

#[test]
fn wrong_chain_id_rejected() {
    let store = MemoryAccountStore::with_genesis(&[("alice", 10_000)]);
    let tx = NativeTransaction::new_transfer_with_chain("alice", "bob", 100, 0, 5, 9999);

    let err = execute_transfer_checked(&store, &tx, "v", 1).unwrap_err();
    assert!(matches!(err, NativeTxError::ChainIdMismatch { .. }));
}

#[test]
fn correct_chain_id_accepted() {
    let store = MemoryAccountStore::with_genesis(&[("alice", 10_000)]);
    let tx = NativeTransaction::new_transfer_with_chain("alice", "bob", 100, 0, 5, 9999);
    execute_transfer_checked(&store, &tx, "v", 9999).unwrap();
}

#[test]
fn chain_id_zero_accepted_by_any_network() {
    let store = MemoryAccountStore::with_genesis(&[("alice", 10_000)]);
    let tx = NativeTransaction::new_transfer("alice", "bob", 100, 0, 5);
    assert_eq!(tx.chain_id, 0);
    execute_transfer_checked(&store, &tx, "v", 9999).unwrap();
}

// ── Serde Roundtrip ────────────────────────────────────────────────────────

#[test]
fn signed_tx_survives_json_roundtrip() {
    let provider = SoftwareSigningProvider::generate();
    let mut tx = NativeTransaction::new_transfer_with_chain("alice", "bob", 500, 3, 10, 9999);
    let payload = tx.signing_payload();
    tx.signature = provider.sign(&payload).unwrap();
    tx.signature_algorithm = "ed25519".to_string();

    let json = serde_json::to_string(&tx).unwrap();
    let restored: NativeTransaction = serde_json::from_str(&json).unwrap();

    assert_eq!(tx, restored);
    assert!(verify_tx_signature(&restored, &provider.public_key()).unwrap());
}
