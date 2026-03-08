#![no_main]
use libfuzzer_sys::fuzz_target;

use aztibase_core::{hash, PublicKey};

fuzz_target!(|data: &[u8]| {
    let h = hash(data);
    assert_ne!(h.len(), 0);

    if data.len() >= 32 {
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&data[..32]);
        let _ = PublicKey::from_bytes(&key_bytes);
    }
});
