use std::path::Path;

use anyhow::{Context, Result};

use aztibase_core::{Keypair, address_from_pubkey};
use aztibase_execution::{SignedTx, TxKind};

use crate::genesis::{KeyFile, hex_decode, hex_encode, load_keyfile};

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

    let signed = SignedTx::new(tx.encode(), &kp);
    Ok(signed.encode())
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
}
