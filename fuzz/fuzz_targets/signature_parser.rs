#![no_main]
use libfuzzer_sys::fuzz_target;
use rust_bc::identity::pqc_policy::validate_signature_consistency;
use rust_bc::identity::signing::SigningAlgorithm;

fuzz_target!(|data: &[u8]| {
    // Feed random bytes as signatures to the consistency checker.
    // Must not panic regardless of input.
    let _ = validate_signature_consistency(SigningAlgorithm::Ed25519, data, "fuzz");
    let _ = validate_signature_consistency(SigningAlgorithm::MlDsa65, data, "fuzz");
});
