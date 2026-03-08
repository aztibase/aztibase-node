#![no_main]
use libfuzzer_sys::fuzz_target;

use aztibase_consensus::SignerBitmap;

fuzz_target!(|data: &[u8]| {
    if data.len() < 2 {
        return;
    }
    let len = u16::from_le_bytes([data[0], data[1]]) as usize;
    if len == 0 || len > 1024 {
        return;
    }
    let mut bm = SignerBitmap::new(len);
    for (i, byte) in data[2..].iter().enumerate() {
        let idx = (*byte as usize) % len;
        bm.set(idx, true);
        let _ = bm.get(idx);
        if i % 3 == 0 {
            bm.set(idx, false);
        }
    }
    let _ = bm.count_set();
});
