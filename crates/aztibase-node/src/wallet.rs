use std::path::Path;

use anyhow::{Context, Result};
use zeroize::Zeroize;

use aztibase_core::{Keypair, address_from_pubkey};
use aztibase_execution::{SignedTx, TxKind};

use crate::genesis::{KeyFile, hex_decode, hex_encode, load_keyfile};

const ARGON2_MEM_COST_KIB: u32 = 262_144; // 256 MB
const ARGON2_TIME_COST: u32 = 3;
const ARGON2_PARALLELISM: u32 = 1;
const SALT_LEN: usize = 32;
const NONCE_LEN: usize = 12;

pub fn generate_key(output_path: &Path) -> Result<()> {
    let kp = Keypair::generate();
    let addr = address_from_pubkey(kp.public_key().as_bytes());

    let keyfile = KeyFile {
        public_key: hex_encode(kp.public_key().as_bytes()),
        secret_key: hex_encode(&kp.secret_bytes()),
        address: hex_encode(&addr),
        bls_public_key: None,
        bls_secret_key: None,
    };

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(&keyfile).context("Failed to serialize key file")?;
    std::fs::write(output_path, json)
        .with_context(|| format!("Failed to write {}", output_path.display()))?;

    println!("Address: {}", keyfile.address);
    println!("Public key: {}", keyfile.public_key);
    println!("Key file: {}", output_path.display());
    Ok(())
}

pub fn generate_mnemonic() -> Result<(bip39::Mnemonic, Keypair)> {
    let mut entropy = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut entropy);
    let mnemonic =
        bip39::Mnemonic::from_entropy(&entropy).map_err(|e| anyhow::anyhow!("BIP-39: {e}"))?;
    entropy.zeroize();
    let kp = keypair_from_mnemonic(&mnemonic)?;
    Ok((mnemonic, kp))
}

pub fn recover_from_mnemonic(phrase: &str) -> Result<Keypair> {
    let mnemonic = bip39::Mnemonic::parse_in_normalized(bip39::Language::English, phrase)
        .map_err(|e| anyhow::anyhow!("Invalid mnemonic: {e}"))?;
    keypair_from_mnemonic(&mnemonic)
}

fn keypair_from_mnemonic(mnemonic: &bip39::Mnemonic) -> Result<Keypair> {
    use blake3::derive_key;
    let mut seed = mnemonic.to_seed("");
    let mut derived = derive_key("aztibase m/44'/aztb'/0'/0/0", &seed);
    seed.zeroize();
    let kp = Keypair::from_secret_bytes(&derived);
    derived.zeroize();
    Ok(kp)
}

pub fn generate_key_with_mnemonic(output_path: &Path, passphrase: &str) -> Result<String> {
    let (mnemonic, kp) = generate_mnemonic()?;
    let addr = address_from_pubkey(kp.public_key().as_bytes());
    let phrase = mnemonic.to_string();

    let encrypted = encrypt_keyfile(&kp.secret_bytes(), passphrase)?;
    let enc_kf = EncryptedKeyFile {
        public_key: hex_encode(kp.public_key().as_bytes()),
        address: hex_encode(&addr),
        encrypted,
    };

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let json = serde_json::to_string_pretty(&enc_kf).context("Failed to serialize")?;
    std::fs::write(output_path, json)
        .with_context(|| format!("Failed to write {}", output_path.display()))?;

    println!("Address: {}", enc_kf.address);
    println!("Public key: {}", enc_kf.public_key);
    println!("Key file (encrypted): {}", output_path.display());

    Ok(phrase)
}

pub fn load_encrypted_keyfile(path: &Path, passphrase: &str) -> Result<Keypair> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let enc_kf: EncryptedKeyFile =
        serde_json::from_str(&contents).context("Failed to parse encrypted key file")?;
    let mut secret = decrypt_keyfile(&enc_kf.encrypted, passphrase)?;
    let kp = Keypair::from_secret_bytes(&secret);
    secret.zeroize();
    Ok(kp)
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EncryptedKeyFile {
    pub public_key: String,
    pub address: String,
    pub encrypted: EncryptedPayload,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct EncryptedPayload {
    pub salt: String,
    pub nonce: String,
    pub ciphertext: String,
    pub argon2_mem_kib: u32,
    pub argon2_time: u32,
    pub argon2_parallelism: u32,
}

fn encrypt_keyfile(secret_key: &[u8; 32], passphrase: &str) -> Result<EncryptedPayload> {
    use argon2::Argon2;
    use chacha20poly1305::{
        ChaCha20Poly1305, KeyInit,
        aead::{Aead, OsRng},
    };
    use rand::RngCore;

    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);

    let params = argon2::Params::new(
        ARGON2_MEM_COST_KIB,
        ARGON2_TIME_COST,
        ARGON2_PARALLELISM,
        Some(32),
    )
    .map_err(|e| anyhow::anyhow!("Invalid Argon2 params: {e}"))?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let mut key = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), &salt, &mut key)
        .map_err(|e| anyhow::anyhow!("Argon2id hash failed: {e}"))?;

    let cipher = ChaCha20Poly1305::new((&key).into());
    key.zeroize();

    let nonce = chacha20poly1305::Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, secret_key.as_ref())
        .map_err(|e| anyhow::anyhow!("Encryption failed: {e}"))?;

    Ok(EncryptedPayload {
        salt: hex_encode(&salt),
        nonce: hex_encode(&nonce_bytes),
        ciphertext: hex_encode(&ciphertext),
        argon2_mem_kib: ARGON2_MEM_COST_KIB,
        argon2_time: ARGON2_TIME_COST,
        argon2_parallelism: ARGON2_PARALLELISM,
    })
}

fn decrypt_keyfile(payload: &EncryptedPayload, passphrase: &str) -> Result<[u8; 32]> {
    use argon2::Argon2;
    use chacha20poly1305::{ChaCha20Poly1305, KeyInit, aead::Aead};

    let salt = hex_decode(&payload.salt).context("Invalid salt hex")?;
    let nonce_bytes = hex_decode(&payload.nonce).context("Invalid nonce hex")?;
    let ciphertext = hex_decode(&payload.ciphertext).context("Invalid ciphertext hex")?;

    if nonce_bytes.len() != NONCE_LEN {
        anyhow::bail!("Nonce must be {} bytes", NONCE_LEN);
    }

    let params = argon2::Params::new(
        payload.argon2_mem_kib,
        payload.argon2_time,
        payload.argon2_parallelism,
        Some(32),
    )
    .map_err(|e| anyhow::anyhow!("Invalid Argon2 params: {e}"))?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let mut key = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), &salt, &mut key)
        .map_err(|e| anyhow::anyhow!("Argon2id hash failed: {e}"))?;

    let cipher = ChaCha20Poly1305::new((&key).into());
    key.zeroize();

    let nonce = chacha20poly1305::Nonce::from_slice(&nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| anyhow::anyhow!("Decryption failed: wrong passphrase or corrupted data"))?;

    let secret: [u8; 32] = plaintext
        .try_into()
        .map_err(|_| anyhow::anyhow!("Decrypted key must be 32 bytes"))?;

    Ok(secret)
}

pub fn show_key(path: &Path) -> Result<()> {
    let (kp, addr) = load_keyfile(path)?;
    println!("Address: {}", hex_encode(&addr));
    println!("Public key: {}", hex_encode(kp.public_key().as_bytes()));
    Ok(())
}

pub fn sign_transfer(
    keyfile_path: &Path,
    to_hex: &str,
    value: u64,
    nonce: u64,
    gas_price: u64,
) -> Result<Vec<u8>> {
    let (kp, sender) = load_keyfile(keyfile_path)?;
    build_signed_transfer(&kp, sender, to_hex, value, nonce, gas_price)
}

pub fn sign_transfer_encrypted(
    keyfile_path: &Path,
    passphrase: &str,
    to_hex: &str,
    value: u64,
    nonce: u64,
    gas_price: u64,
) -> Result<Vec<u8>> {
    let kp = load_encrypted_keyfile(keyfile_path, passphrase)?;
    let sender = address_from_pubkey(kp.public_key().as_bytes());
    build_signed_transfer(&kp, sender, to_hex, value, nonce, gas_price)
}

fn build_signed_transfer(
    kp: &Keypair,
    sender: [u8; 32],
    to_hex: &str,
    value: u64,
    nonce: u64,
    gas_price: u64,
) -> Result<Vec<u8>> {
    let to_bytes = hex_decode(to_hex).context("Invalid recipient address hex")?;
    let to: [u8; 32] = to_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("Recipient address must be 32 bytes"))?;

    let tx = TxKind::Transfer {
        from: sender,
        to,
        value,
        nonce,
        gas_price,
    };

    let signed = SignedTx::new(tx.encode(), kp);
    Ok(signed.encode())
}

pub async fn broadcast_transaction(rpc_url: &str, tx_hex: &str) -> Result<String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "aztb_sendTransaction",
        "params": [format!("0x{tx_hex}")],
        "id": 1
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .context("Failed to connect to RPC endpoint")?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .context("Failed to read RPC response body")?;

    if !status.is_success() {
        anyhow::bail!("RPC returned HTTP {status}: {text}");
    }

    let parsed: serde_json::Value = serde_json::from_str(&text).context("Invalid JSON from RPC")?;

    if let Some(err) = parsed.get("error") {
        let msg = err
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        anyhow::bail!("RPC error: {msg}");
    }

    let result = parsed
        .get("result")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use aztibase_execution::verify_and_route;

    #[test]
    fn generate_and_show_roundtrip() {
        let dir = std::env::temp_dir().join(format!("aztibase_wallet_test_{}", std::process::id()));
        let keyfile_path = dir.join("test.json");

        generate_key(&keyfile_path).unwrap();
        assert!(keyfile_path.exists());

        let (kp, addr) = load_keyfile(&keyfile_path).unwrap();
        assert_eq!(address_from_pubkey(kp.public_key().as_bytes()), addr);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sign_transfer_produces_valid_envelope() {
        let dir = std::env::temp_dir().join(format!("aztibase_wallet_sign_{}", std::process::id()));
        let keyfile_path = dir.join("sender.json");
        generate_key(&keyfile_path).unwrap();

        let to = [0xBB; 32];
        let envelope = sign_transfer(&keyfile_path, &hex_encode(&to), 500, 0, 1).unwrap();

        let routed = verify_and_route(&envelope).unwrap();
        match routed {
            TxKind::Transfer {
                to: t,
                value,
                nonce,
                gas_price,
                ..
            } => {
                assert_eq!(t, to);
                assert_eq!(value, 500);
                assert_eq!(nonce, 0);
                assert_eq!(gas_price, 1);
            }
            _ => panic!("expected Transfer"),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mnemonic_roundtrip() {
        let (mnemonic, kp1) = generate_mnemonic().unwrap();
        let phrase = mnemonic.to_string();
        let words: Vec<&str> = phrase.split_whitespace().collect();
        assert_eq!(words.len(), 24);

        let kp2 = recover_from_mnemonic(&phrase).unwrap();
        assert_eq!(kp1.public_key().as_bytes(), kp2.public_key().as_bytes());
    }

    #[test]
    fn mnemonic_12_word_recovery() {
        let entropy = [0xABu8; 16]; // 128-bit = 12 words
        let m = bip39::Mnemonic::from_entropy(&entropy).unwrap();
        let phrase = m.to_string();
        let kp1 = recover_from_mnemonic(&phrase).unwrap();
        let kp2 = recover_from_mnemonic(&phrase).unwrap();
        assert_eq!(kp1.public_key().as_bytes(), kp2.public_key().as_bytes());
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let kp = Keypair::generate();
        let secret = kp.secret_bytes();
        let payload = encrypt_keyfile(&secret, "test-passphrase").unwrap();
        let recovered = decrypt_keyfile(&payload, "test-passphrase").unwrap();
        assert_eq!(secret, recovered);
    }

    #[test]
    fn wrong_passphrase_rejected() {
        let kp = Keypair::generate();
        let secret = kp.secret_bytes();
        let payload = encrypt_keyfile(&secret, "correct-password").unwrap();
        let result = decrypt_keyfile(&payload, "wrong-password");
        assert!(result.is_err());
    }

    #[test]
    fn generate_mnemonic_encrypted_keyfile_roundtrip() {
        let dir =
            std::env::temp_dir().join(format!("aztibase_mnemonic_enc_{}", std::process::id()));
        let keyfile_path = dir.join("encrypted.json");
        let passphrase = "strong-test-passphrase";

        let phrase = generate_key_with_mnemonic(&keyfile_path, passphrase).unwrap();
        assert!(!phrase.is_empty());
        assert_eq!(phrase.split_whitespace().count(), 24);

        let loaded_kp = load_encrypted_keyfile(&keyfile_path, passphrase).unwrap();
        let recovered_kp = recover_from_mnemonic(&phrase).unwrap();
        assert_eq!(
            loaded_kp.public_key().as_bytes(),
            recovered_kp.public_key().as_bytes()
        );

        assert!(load_encrypted_keyfile(&keyfile_path, "wrong").is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sign_transfer_encrypted_produces_valid_envelope() {
        let dir = std::env::temp_dir().join(format!("aztibase_enc_sign_{}", std::process::id()));
        let keyfile_path = dir.join("enc.json");
        let passphrase = "test-pass-42";

        let _phrase = generate_key_with_mnemonic(&keyfile_path, passphrase).unwrap();
        let kp = load_encrypted_keyfile(&keyfile_path, passphrase).unwrap();
        let sender = address_from_pubkey(kp.public_key().as_bytes());

        let to = [0xCC; 32];
        let envelope =
            sign_transfer_encrypted(&keyfile_path, passphrase, &hex_encode(&to), 1000, 0, 2)
                .unwrap();

        let routed = verify_and_route(&envelope).unwrap();
        match routed {
            TxKind::Transfer {
                from,
                to: t,
                value,
                nonce,
                gas_price,
            } => {
                assert_eq!(from, sender);
                assert_eq!(t, to);
                assert_eq!(value, 1000);
                assert_eq!(nonce, 0);
                assert_eq!(gas_price, 2);
            }
            _ => panic!("expected Transfer"),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn broadcast_to_mock_rpc() {
        use axum::{Router, routing::post};

        async fn mock_rpc(
            body: axum::extract::Json<serde_json::Value>,
        ) -> axum::response::Json<serde_json::Value> {
            let method = body.get("method").and_then(|v| v.as_str()).unwrap_or("");
            assert_eq!(method, "aztb_sendTransaction");
            let tx_hex = body["params"][0].as_str().unwrap();
            assert!(tx_hex.starts_with("0x"));
            axum::response::Json(serde_json::json!({
                "jsonrpc": "2.0",
                "result": "0xdeadbeef",
                "id": 1
            }))
        }

        let app = Router::new().route("/", post(mock_rpc));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let url = format!("http://{addr}/");
        let result = broadcast_transaction(&url, "aabbccdd").await.unwrap();
        assert_eq!(result, "0xdeadbeef");
    }

    #[tokio::test]
    async fn broadcast_rpc_error_propagates() {
        use axum::{Router, routing::post};

        async fn mock_error() -> axum::response::Json<serde_json::Value> {
            axum::response::Json(serde_json::json!({
                "jsonrpc": "2.0",
                "error": {"code": -32000, "message": "mempool full"},
                "id": 1
            }))
        }

        let app = Router::new().route("/", post(mock_error));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let url = format!("http://{addr}/");
        let result = broadcast_transaction(&url, "aabbccdd").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("mempool full"));
    }
}
