use ed25519_dalek::{Signature, Signer, SigningKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

const TX_DOMAIN: &[u8] = b"AZTB_TX_V1";
const ENVELOPE_MAGIC: u8 = 0xAA;
const PREFIX_TRANSFER: u8 = 0x01;

type Address = [u8; 32];

/// Minimal Transfer representation matching aztibase-execution's TxKind::Transfer.
/// Serialized via postcard to produce a wire-compatible payload.
#[derive(Serialize, Deserialize)]
struct TransferFields {
    from: Address,
    to: Address,
    value: u128,
    nonce: u64,
    gas_price: u64,
}

/// Postcard encodes enums with a variant index. TxKind::Transfer is variant 0.
/// The full TxKind enum serialization is: [varint(variant_index)][fields...].
/// We replicate only variant 0 (Transfer) here.
fn encode_transfer(from: Address, to: Address, value: u128, nonce: u64, gas_price: u64) -> Vec<u8> {
    let fields = TransferFields {
        from,
        to,
        value,
        nonce,
        gas_price,
    };
    let enum_payload = postcard::to_allocvec(&(0u32, fields)).expect("serialization cannot fail");
    let mut buf = Vec::with_capacity(1 + enum_payload.len());
    buf.push(PREFIX_TRANSFER);
    buf.extend_from_slice(&enum_payload);
    buf
}

fn signing_message(payload: &[u8]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(TX_DOMAIN.len() + payload.len());
    msg.extend_from_slice(TX_DOMAIN);
    msg.extend_from_slice(payload);
    msg
}

fn sign_payload(payload: &[u8], signing_key: &SigningKey) -> Vec<u8> {
    let msg = signing_message(payload);
    let sig: Signature = signing_key.sign(&msg);
    let pubkey = signing_key.verifying_key();

    let payload_len = payload.len() as u32;
    let total = 1 + 4 + payload.len() + 32 + 64;
    let mut buf = Vec::with_capacity(total);
    buf.push(ENVELOPE_MAGIC);
    buf.extend_from_slice(&payload_len.to_le_bytes());
    buf.extend_from_slice(payload);
    buf.extend_from_slice(pubkey.as_bytes());
    buf.extend_from_slice(&sig.to_bytes());
    buf
}

fn address_from_pubkey(pubkey: &[u8; 32]) -> Address {
    *blake3::hash(pubkey).as_bytes()
}

fn parse_hex_address(hex: &str) -> Result<Address, String> {
    let clean = hex.strip_prefix("0x").unwrap_or(hex);
    if clean.len() != 64 {
        return Err(format!("address must be 64 hex chars, got {}", clean.len()));
    }
    let mut out = [0u8; 32];
    for (i, chunk) in clean.as_bytes().chunks(2).enumerate() {
        let hi = hex_nibble(chunk[0]).ok_or("invalid hex char")?;
        let lo = hex_nibble(chunk[1]).ok_or("invalid hex char")?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

/// Generate a new Ed25519 keypair. Returns JSON: `{"secret": "hex", "public": "hex", "address": "hex"}`.
#[wasm_bindgen(js_name = "generateKeypair")]
pub fn js_generate_keypair() -> String {
    let sk = SigningKey::generate(&mut OsRng);
    let pk = sk.verifying_key();
    let addr = address_from_pubkey(pk.as_bytes());
    serde_json::json!({
        "secret": hex_encode(&sk.to_bytes()),
        "public": hex_encode(pk.as_bytes()),
        "address": hex_encode(&addr),
    })
    .to_string()
}

/// Derive the address from a secret key hex string.
/// Returns hex address on success, or `"error: ..."` on failure.
#[wasm_bindgen(js_name = "addressFromSecret")]
pub fn js_address_from_secret(secret_hex: &str) -> String {
    let sk = match parse_secret_key(secret_hex) {
        Ok(sk) => sk,
        Err(e) => return format!("error: {e}"),
    };
    let pk = sk.verifying_key();
    let addr = address_from_pubkey(pk.as_bytes());
    hex_encode(&addr)
}

/// Sign a Transfer transaction and return the raw signed envelope as a hex string.
///
/// Parameters:
/// - `secret_hex`: 64-char hex Ed25519 secret key
/// - `to_hex`: 64-char hex recipient address
/// - `value_str`: transfer amount as decimal string (u128)
/// - `nonce`: sender nonce
/// - `gas_price`: gas price
#[wasm_bindgen(js_name = "signTransfer")]
pub fn js_sign_transfer(
    secret_hex: &str,
    to_hex: &str,
    value_str: &str,
    nonce: u64,
    gas_price: u64,
) -> String {
    let sk = match parse_secret_key(secret_hex) {
        Ok(sk) => sk,
        Err(e) => return format!("error: {e}"),
    };
    let to = match parse_hex_address(to_hex) {
        Ok(a) => a,
        Err(e) => return format!("error: {e}"),
    };
    let value: u128 = match value_str.parse() {
        Ok(v) => v,
        Err(_) => return "error: invalid value (expected decimal u128)".to_string(),
    };

    let pk = sk.verifying_key();
    let from = address_from_pubkey(pk.as_bytes());
    let payload = encode_transfer(from, to, value, nonce, gas_price);
    let envelope = sign_payload(&payload, &sk);
    hex_encode(&envelope)
}

/// Build a JSON-RPC request body to send a raw transaction.
/// Returns a JSON string ready to POST to the node's RPC endpoint.
#[wasm_bindgen(js_name = "buildSendTxRequest")]
pub fn js_build_send_tx_request(signed_tx_hex: &str, id: u32) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "aztb_sendRawTransaction",
        "params": [signed_tx_hex],
        "id": id,
    })
    .to_string()
}

/// Build a JSON-RPC request for `aztb_getNonce`.
#[wasm_bindgen(js_name = "buildGetNonceRequest")]
pub fn js_build_get_nonce_request(address_hex: &str, id: u32) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "aztb_getNonce",
        "params": [address_hex],
        "id": id,
    })
    .to_string()
}

/// Build a JSON-RPC request for `aztb_getBalance`.
#[wasm_bindgen(js_name = "buildGetBalanceRequest")]
pub fn js_build_get_balance_request(address_hex: &str, id: u32) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "aztb_getBalance",
        "params": [address_hex],
        "id": id,
    })
    .to_string()
}

/// Build a JSON-RPC request for `aztb_estimateGas`.
#[wasm_bindgen(js_name = "buildEstimateGasRequest")]
pub fn js_build_estimate_gas_request(tx_type: &str, id: u32) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "aztb_estimateGas",
        "params": [tx_type],
        "id": id,
    })
    .to_string()
}

fn parse_secret_key(hex: &str) -> Result<SigningKey, String> {
    let clean = hex.strip_prefix("0x").unwrap_or(hex);
    if clean.len() != 64 {
        return Err(format!(
            "secret key must be 64 hex chars, got {}",
            clean.len()
        ));
    }
    let mut bytes = [0u8; 32];
    for (i, chunk) in clean.as_bytes().chunks(2).enumerate() {
        let hi = hex_nibble(chunk[0]).ok_or("invalid hex char in secret key")?;
        let lo = hex_nibble(chunk[1]).ok_or("invalid hex char in secret key")?;
        bytes[i] = (hi << 4) | lo;
    }
    Ok(SigningKey::from_bytes(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_transfer_roundtrip() {
        let sk = SigningKey::generate(&mut OsRng);
        let pk = sk.verifying_key();
        let from = address_from_pubkey(pk.as_bytes());
        let to = [2u8; 32];
        let payload = encode_transfer(from, to, 1000, 0, 1);
        let envelope = sign_payload(&payload, &sk);

        assert_eq!(envelope[0], ENVELOPE_MAGIC);
        let payload_len =
            u32::from_le_bytes([envelope[1], envelope[2], envelope[3], envelope[4]]) as usize;
        assert_eq!(payload_len, payload.len());
        let pk_start = 5 + payload_len;
        assert_eq!(&envelope[pk_start..pk_start + 32], pk.as_bytes());

        let msg = signing_message(&payload);
        let sig_start = pk_start + 32;
        let sig_bytes: [u8; 64] = envelope[sig_start..sig_start + 64].try_into().unwrap();
        let sig = Signature::from_bytes(&sig_bytes);
        assert!(pk.verify_strict(&msg, &sig).is_ok());
    }

    #[test]
    fn js_sign_transfer_produces_valid_hex() {
        let sk = SigningKey::generate(&mut OsRng);
        let secret_hex = hex_encode(&sk.to_bytes());
        let to_hex = hex_encode(&[2u8; 32]);
        let result = js_sign_transfer(&secret_hex, &to_hex, "500", 0, 1);
        assert!(!result.starts_with("error:"));
        assert!(result.len() > 200);
    }

    #[test]
    fn js_sign_transfer_invalid_secret_rejected() {
        let result = js_sign_transfer("abcd", &hex_encode(&[2u8; 32]), "100", 0, 1);
        assert!(result.starts_with("error:"));
    }

    #[test]
    fn js_sign_transfer_invalid_value_rejected() {
        let sk = SigningKey::generate(&mut OsRng);
        let secret_hex = hex_encode(&sk.to_bytes());
        let to_hex = hex_encode(&[2u8; 32]);
        let result = js_sign_transfer(&secret_hex, &to_hex, "not_a_number", 0, 1);
        assert!(result.starts_with("error:"));
    }

    #[test]
    fn generate_keypair_returns_valid_json() {
        let json_str = js_generate_keypair();
        let val: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert!(val["secret"].is_string());
        assert!(val["public"].is_string());
        assert!(val["address"].is_string());
        assert_eq!(val["secret"].as_str().unwrap().len(), 64);
        assert_eq!(val["public"].as_str().unwrap().len(), 64);
        assert_eq!(val["address"].as_str().unwrap().len(), 64);
    }

    #[test]
    fn address_from_secret_roundtrip() {
        let sk = SigningKey::generate(&mut OsRng);
        let pk = sk.verifying_key();
        let expected = address_from_pubkey(pk.as_bytes());

        let addr_hex = js_address_from_secret(&hex_encode(&sk.to_bytes()));
        assert!(!addr_hex.starts_with("error:"));
        assert_eq!(addr_hex, hex_encode(&expected));
    }

    #[test]
    fn build_rpc_requests_valid_json() {
        let addr = hex_encode(&[1u8; 32]);
        let nonce_req = js_build_get_nonce_request(&addr, 1);
        let bal_req = js_build_get_balance_request(&addr, 2);
        let gas_req = js_build_estimate_gas_request("Transfer", 3);
        let send_req = js_build_send_tx_request("aabb", 4);

        for req in [&nonce_req, &bal_req, &gas_req, &send_req] {
            let v: serde_json::Value = serde_json::from_str(req).unwrap();
            assert_eq!(v["jsonrpc"], "2.0");
            assert!(v["method"].is_string());
        }
    }

    #[test]
    fn envelope_wire_compatible() {
        let sk = SigningKey::generate(&mut OsRng);
        let pk = sk.verifying_key();
        let from = address_from_pubkey(pk.as_bytes());
        let to = [3u8; 32];
        let payload = encode_transfer(from, to, 42, 7, 2);

        assert_eq!(payload[0], PREFIX_TRANSFER);

        let envelope = sign_payload(&payload, &sk);
        assert_eq!(envelope[0], ENVELOPE_MAGIC);
        let total = 1 + 4 + payload.len() + 32 + 64;
        assert_eq!(envelope.len(), total);
    }
}
