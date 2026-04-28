#![no_main]
use libfuzzer_sys::fuzz_target;
use rust_bc::network::gossip::AliveMessage;

fuzz_target!(|data: &[u8]| {
    // Attempt to deserialize random bytes as an AliveMessage.
    // Must not panic regardless of input.
    let _ = serde_json::from_slice::<AliveMessage>(data);
});
