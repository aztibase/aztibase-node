use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use aztibase_core::{Keypair, address_from_pubkey};
use aztibase_execution::AccountState;

type Address = [u8; 32];

const CHAIN_ID: u64 = 0xA27B;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenesisConfig {
    pub chain_id: u64,
    pub timestamp: u64,
    pub validators: Vec<ValidatorEntry>,
    pub accounts: BTreeMap<String, AccountEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorEntry {
    pub name: String,
    pub address: String,
    pub stake: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountEntry {
    pub balance: u64,
}

pub fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

pub fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(s.len() / 2);
    for i in (0..s.len()).step_by(2) {
        let byte = u8::from_str_radix(&s[i..i + 2], 16).ok()?;
        bytes.push(byte);
    }
    Some(bytes)
}

fn address_from_hex(hex: &str) -> Option<Address> {
    let bytes = hex_decode(hex)?;
    bytes.as_slice().try_into().ok()
}

pub fn apply_genesis(config: &GenesisConfig, state: &mut AccountState) {
    for entry in &config.validators {
        if let Some(addr) = address_from_hex(&entry.address) {
            state.set_balance(&addr, entry.stake);
        }
    }
    for (hex_addr, entry) in &config.accounts {
        if let Some(addr) = address_from_hex(hex_addr) {
            state.set_balance(&addr, entry.balance);
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyFile {
    pub public_key: String,
    pub secret_key: String,
    pub address: String,
}

pub struct GeneratedGenesis {
    pub config: GenesisConfig,
    pub validator_keys: Vec<(String, Keypair)>,
    pub funded_keys: Vec<(String, Keypair)>,
}

pub fn generate_genesis(n_validators: usize, n_funded: usize, timestamp: u64) -> GeneratedGenesis {
    let mut validators = Vec::with_capacity(n_validators);
    let mut validator_keys = Vec::with_capacity(n_validators);

    for i in 0..n_validators {
        let kp = Keypair::generate();
        let addr = address_from_pubkey(kp.public_key().as_bytes());
        let hex_addr = hex_encode(&addr);
        validators.push(ValidatorEntry {
            name: format!("validator-{}", i + 1),
            address: hex_addr.clone(),
            stake: 1_000_000,
        });
        validator_keys.push((hex_addr, kp));
    }

    let mut accounts = BTreeMap::new();
    let mut funded_keys = Vec::with_capacity(n_funded);

    for _ in 0..n_funded {
        let kp = Keypair::generate();
        let addr = address_from_pubkey(kp.public_key().as_bytes());
        let hex_addr = hex_encode(&addr);
        accounts.insert(
            hex_addr.clone(),
            AccountEntry {
                balance: 10_000_000,
            },
        );
        funded_keys.push((hex_addr, kp));
    }

    let config = GenesisConfig {
        chain_id: CHAIN_ID,
        timestamp,
        validators,
        accounts,
    };

    GeneratedGenesis {
        config,
        validator_keys,
        funded_keys,
    }
}

pub fn write_genesis(genesis: &GeneratedGenesis, output_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create output dir: {}", output_dir.display()))?;

    let toml_str =
        toml::to_string_pretty(&genesis.config).context("Failed to serialize genesis config")?;
    let genesis_path = output_dir.join("genesis.toml");
    std::fs::write(&genesis_path, toml_str)
        .with_context(|| format!("Failed to write {}", genesis_path.display()))?;

    let keys_dir = output_dir.join("keys");
    std::fs::create_dir_all(&keys_dir)?;

    for (hex_addr, kp) in &genesis.validator_keys {
        write_keyfile(&keys_dir, hex_addr, kp)?;
    }

    for (hex_addr, kp) in &genesis.funded_keys {
        write_keyfile(&keys_dir, hex_addr, kp)?;
    }

    Ok(())
}

fn write_keyfile(keys_dir: &Path, hex_addr: &str, kp: &Keypair) -> Result<()> {
    let keyfile = KeyFile {
        public_key: hex_encode(kp.public_key().as_bytes()),
        secret_key: hex_encode(&kp.secret_bytes()),
        address: hex_addr.to_string(),
    };
    let path = keys_dir.join(format!("{hex_addr}.json"));
    let json = serde_json::to_string_pretty(&keyfile).context("Failed to serialize key file")?;
    std::fs::write(&path, json)?;
    Ok(())
}

pub fn load_keyfile(path: &Path) -> Result<(Keypair, Address)> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read key file: {}", path.display()))?;
    let kf: KeyFile = serde_json::from_str(&contents).context("Failed to parse key file")?;
    let secret_bytes = hex_decode(&kf.secret_key).context("Invalid secret key hex")?;
    let secret: [u8; 32] = secret_bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("Secret key must be 32 bytes"))?;
    let kp = Keypair::from_secret_bytes(&secret);
    let addr = address_from_pubkey(kp.public_key().as_bytes());
    Ok((kp, addr))
}

pub fn load_genesis(path: &Path) -> Result<GenesisConfig> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read genesis file: {}", path.display()))?;
    let config: GenesisConfig =
        toml::from_str(&contents).context("Failed to parse genesis config")?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_applies_balances() {
        let mut config = GenesisConfig {
            chain_id: CHAIN_ID,
            timestamp: 1000,
            validators: vec![],
            accounts: BTreeMap::new(),
        };
        let addr = [0xAA; 32];
        config
            .accounts
            .insert(hex_encode(&addr), AccountEntry { balance: 5000 });

        let mut state = AccountState::new();
        apply_genesis(&config, &mut state);
        assert_eq!(state.balance(&addr), 5000);
    }

    #[test]
    fn genesis_applies_validators() {
        let kp = Keypair::generate();
        let addr = address_from_pubkey(kp.public_key().as_bytes());
        let config = GenesisConfig {
            chain_id: CHAIN_ID,
            timestamp: 1000,
            validators: vec![ValidatorEntry {
                name: "v1".into(),
                address: hex_encode(&addr),
                stake: 1_000_000,
            }],
            accounts: BTreeMap::new(),
        };

        let mut state = AccountState::new();
        apply_genesis(&config, &mut state);
        assert_eq!(state.balance(&addr), 1_000_000);
    }

    #[test]
    fn generate_genesis_creates_valid_config() {
        let generated = generate_genesis(3, 2, 1000);
        assert_eq!(generated.config.chain_id, CHAIN_ID);
        assert_eq!(generated.config.validators.len(), 3);
        assert_eq!(generated.config.accounts.len(), 2);
        assert_eq!(generated.validator_keys.len(), 3);
        assert_eq!(generated.funded_keys.len(), 2);

        let toml_str = toml::to_string_pretty(&generated.config).unwrap();
        let parsed: GenesisConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.chain_id, CHAIN_ID);
        assert_eq!(parsed.validators.len(), 3);
    }

    #[test]
    fn genesis_toml_roundtrip() {
        let generated = generate_genesis(1, 1, 42);
        let toml_str = toml::to_string_pretty(&generated.config).unwrap();
        let parsed: GenesisConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.chain_id, generated.config.chain_id);
        assert_eq!(parsed.timestamp, generated.config.timestamp);
        assert_eq!(parsed.validators.len(), 1);
        assert_eq!(parsed.accounts.len(), 1);
    }

    #[test]
    fn write_and_load_genesis() {
        let dir =
            std::env::temp_dir().join(format!("aztibase_genesis_test_{}", std::process::id()));
        let generated = generate_genesis(2, 1, 999);
        write_genesis(&generated, &dir).unwrap();

        let loaded = load_genesis(&dir.join("genesis.toml")).unwrap();
        assert_eq!(loaded.chain_id, CHAIN_ID);
        assert_eq!(loaded.validators.len(), 2);
        assert_eq!(loaded.accounts.len(), 1);

        let keys_dir = dir.join("keys");
        let key_files: Vec<_> = std::fs::read_dir(&keys_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        assert_eq!(key_files.len(), 3);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
