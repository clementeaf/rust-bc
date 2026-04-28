#![no_main]
use libfuzzer_sys::fuzz_target;
use rust_bc::storage::traits::Block;

fuzz_target!(|data: &[u8]| {
    // Attempt to deserialize random bytes as a Block.
    // Must not panic regardless of input.
    let _ = serde_json::from_slice::<Block>(data);
});
