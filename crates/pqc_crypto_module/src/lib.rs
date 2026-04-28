//! `pqc_crypto_module` — FIPS-oriented post-quantum cryptographic module.
//!
//! This crate isolates all cryptographic operations behind a strict boundary
//! with approved-mode enforcement, startup self-tests, and key zeroization.
//!
//! **Important**: This module is FIPS-oriented and aligned with FIPS 202/203/204.
//! It is NOT FIPS certified. It is prepared for future validation by an
//! accredited lab.
//!
//! # Usage
//!
//! ```rust,no_run
//! use pqc_crypto_module::api;
//!
//! // Must be called once at startup
//! api::initialize_approved_mode().expect("self-tests failed");
//!
//! // Now crypto operations are available
//! let kp = api::generate_mldsa_keypair().unwrap();
//! let sig = api::sign_message(&kp.private_key, b"hello").unwrap();
//! api::verify_signature(&kp.public_key, b"hello", &sig).unwrap();
//! let hash = api::sha3_256(b"data").unwrap();
//! ```

pub mod api;
pub mod approved_mode;
pub mod errors;
pub mod hashing;
pub mod mldsa;
pub mod mlkem;
pub mod rng;
pub mod self_tests;
pub mod types;
