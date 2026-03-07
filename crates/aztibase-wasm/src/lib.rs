mod proof;
mod sync;

use wasm_bindgen::prelude::*;

pub use proof::{verify_merkle_proof, verify_verkle_proof};
pub use sync::{SyncFinalityCert, SyncHeader, verify_header_chain};

fn hash(data: &[u8]) -> [u8; 32] {
    *blake3::hash(data).as_bytes()
}

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

#[wasm_bindgen(js_name = "verifyHeaderChain")]
pub fn js_verify_header_chain(headers_json: &str, cert_json: &str, expected_start: u64) -> JsValue {
    let headers: Vec<SyncHeader> = match serde_json::from_str(headers_json) {
        Ok(h) => h,
        Err(e) => return JsValue::from_str(&format!("error: invalid headers JSON: {e}")),
    };
    let cert: SyncFinalityCert = match serde_json::from_str(cert_json) {
        Ok(c) => c,
        Err(e) => return JsValue::from_str(&format!("error: invalid cert JSON: {e}")),
    };

    match verify_header_chain(&headers, &cert, expected_start) {
        Ok(()) => JsValue::TRUE,
        Err(e) => JsValue::from_str(&format!("error: {e}")),
    }
}

#[wasm_bindgen(js_name = "verifyMerkleProof")]
pub fn js_verify_merkle_proof(proof_json: &str, root_hex: &str, leaf_hex: &str) -> JsValue {
    let proof: proof::JsMerkleProof = match serde_json::from_str(proof_json) {
        Ok(p) => p,
        Err(e) => return JsValue::from_str(&format!("error: invalid proof JSON: {e}")),
    };

    let root = match parse_hash(root_hex) {
        Some(r) => r,
        None => return JsValue::from_str("error: invalid root hex (expected 64 hex chars)"),
    };
    let leaf = match parse_hash(leaf_hex) {
        Some(l) => l,
        None => return JsValue::from_str("error: invalid leaf hex (expected 64 hex chars)"),
    };

    let mp = proof.into_merkle_proof();
    JsValue::from_bool(verify_merkle_proof(&root, &leaf, &mp))
}

#[wasm_bindgen(js_name = "verifyVerkleProof")]
pub fn js_verify_verkle_proof(proof_json: &str, root_hex: &str) -> JsValue {
    let proof: proof::JsVerkleProof = match serde_json::from_str(proof_json) {
        Ok(p) => p,
        Err(e) => return JsValue::from_str(&format!("error: invalid proof JSON: {e}")),
    };

    let root = match parse_hash(root_hex) {
        Some(r) => r,
        None => return JsValue::from_str("error: invalid root hex (expected 64 hex chars)"),
    };

    let vp = proof.into_verkle_proof();
    JsValue::from_bool(verify_verkle_proof(&root, &vp))
}

#[wasm_bindgen(js_name = "verifyLightClientProof")]
pub fn js_verify_light_client_proof(proof_json: &str, leaf_hex: &str) -> JsValue {
    let lcp: proof::JsLightClientProof = match serde_json::from_str(proof_json) {
        Ok(p) => p,
        Err(e) => return JsValue::from_str(&format!("error: invalid proof JSON: {e}")),
    };

    let leaf = match parse_hash(leaf_hex) {
        Some(l) => l,
        None => return JsValue::from_str("error: invalid leaf hex (expected 64 hex chars)"),
    };

    JsValue::from_bool(proof::verify_light_client_proof(&lcp, &leaf))
}

#[wasm_bindgen(js_name = "blake3Hash")]
pub fn js_blake3_hash(data: &[u8]) -> String {
    hex_encode(&hash(data))
}

#[wasm_bindgen(js_name = "latestSyncedRound")]
pub fn js_latest_synced_round(state_json: &str) -> JsValue {
    let state: sync::JsSyncState = match serde_json::from_str(state_json) {
        Ok(s) => s,
        Err(e) => return JsValue::from_str(&format!("error: {e}")),
    };
    JsValue::from_f64(state.last_synced_round as f64)
}

#[wasm_bindgen(js_name = "buildHeaderRequest")]
pub fn js_build_header_request(from_round: u64, count: u64) -> String {
    let capped = count.min(sync::MAX_HEADERS_PER_REQUEST);
    serde_json::json!({
        "type": "RequestHeaders",
        "version": 1,
        "from_round": from_round,
        "count": capped,
    })
    .to_string()
}

fn parse_hash(hex_str: &str) -> Option<[u8; 32]> {
    let clean = hex_str.strip_prefix("0x").unwrap_or(hex_str);
    if clean.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, chunk) in clean.as_bytes().chunks(2).enumerate() {
        let hi = hex_nibble(chunk[0])?;
        let lo = hex_nibble(chunk[1])?;
        out[i] = (hi << 4) | lo;
    }
    Some(out)
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_deterministic() {
        let h1 = hash(b"aztibase");
        let h2 = hash(b"aztibase");
        assert_eq!(h1, h2);
    }

    #[test]
    fn parse_hash_valid() {
        let h = hash(b"test");
        let hex = hex_encode(&h);
        let parsed = parse_hash(&hex).unwrap();
        assert_eq!(parsed, h);
    }

    #[test]
    fn parse_hash_with_prefix() {
        let h = hash(b"test");
        let hex = format!("0x{}", hex_encode(&h));
        let parsed = parse_hash(&hex).unwrap();
        assert_eq!(parsed, h);
    }

    #[test]
    fn parse_hash_invalid_length() {
        assert!(parse_hash("abcd").is_none());
    }
}
