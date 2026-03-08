#![no_main]
use libfuzzer_sys::fuzz_target;

use aztibase_core::Transaction;

fuzz_target!(|data: &[u8]| {
    let _ = postcard::from_bytes::<Transaction>(data);
});
