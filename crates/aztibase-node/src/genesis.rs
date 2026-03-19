use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use aztibase_core::{BlsKeypair, Keypair, address_from_pubkey};
use aztibase_execution::AccountState;

type Address = [u8; 32];

const CHAIN_ID: u64 = 0xA27B;
const GENESIS_SUPPLY: u128 = 400_000_000;
const DEFAULT_MIN_VALIDATOR_STAKE: u128 = 10_000;

mod serde_u128_as_string {
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &u128, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u128, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<u128>().map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenesisConfig {
    pub chain_id: u64,
    pub timestamp: u64,
    pub validators: Vec<ValidatorEntry>,
    pub accounts: BTreeMap<String, AccountEntry>,
    /// Foundation-approved validator addresses (hex). Only used in Permissioned mode.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub approved_validators: Vec<String>,
    /// Emergency key pubkey (hex, Ed25519). Dead after sunset epoch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emergency_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ValidatorEntry {
    pub name: String,
    pub address: String,
    #[serde(with = "serde_u128_as_string")]
    pub stake: u128,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bls_public_key: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AccountEntry {
    #[serde(with = "serde_u128_as_string")]
    pub balance: u128,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bls_public_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bls_secret_key: Option<String>,
}

pub struct GeneratedGenesis {
    pub config: GenesisConfig,
    pub validator_keys: Vec<(String, Keypair, BlsKeypair)>,
    pub funded_keys: Vec<(String, Keypair)>,
}

pub fn generate_genesis(n_validators: usize, n_funded: usize, timestamp: u64) -> GeneratedGenesis {
    use aztibase_execution::tokenomics::GENESIS_MINT;

    let allocations: [(&str, u128); 8] = [
        ("team", GENESIS_MINT * 15 / 100),      // 60M
        ("investors", GENESIS_MINT * 10 / 100), // 40M
        ("ecosystem", GENESIS_MINT * 25 / 100), // 100M
        ("community", GENESIS_MINT * 20 / 100), // 80M
        ("treasury", GENESIS_MINT * 15 / 100),  // 60M
        ("validators", GENESIS_MINT * 5 / 100), // 20M
        ("advisors", GENESIS_MINT * 5 / 100),   // 20M
        ("reserve", GENESIS_MINT * 5 / 100),    // 20M
    ];

    let validator_pool = allocations[5].1;
    let stake_per_validator = validator_pool / n_validators as u128;

    let mut validators = Vec::with_capacity(n_validators);
    let mut validator_keys = Vec::with_capacity(n_validators);

    for i in 0..n_validators {
        let kp = Keypair::generate();
        let bls_kp = BlsKeypair::generate();
        let addr = address_from_pubkey(kp.public_key().as_bytes());
        let hex_addr = hex_encode(&addr);
        let bls_pub_hex = hex_encode(bls_kp.public_key().as_bytes());
        let ed25519_pub_hex = hex_encode(kp.public_key().as_bytes());
        validators.push(ValidatorEntry {
            name: format!("validator-{}", i + 1),
            address: hex_addr.clone(),
            stake: stake_per_validator,
            public_key: Some(ed25519_pub_hex),
            bls_public_key: Some(bls_pub_hex),
        });
        validator_keys.push((hex_addr, kp, bls_kp));
    }

    let mut accounts = BTreeMap::new();
    let mut funded_keys = Vec::with_capacity(n_funded);

    for &(name, amount) in &allocations {
        if name == "validators" {
            continue;
        }
        let seed = blake3::hash(format!("aztibase-genesis-{name}-{timestamp}").as_bytes());
        let kp = Keypair::from_secret_bytes(seed.as_bytes());
        let addr = address_from_pubkey(kp.public_key().as_bytes());
        let hex_addr = hex_encode(&addr);
        accounts.insert(hex_addr.clone(), AccountEntry { balance: amount });
        funded_keys.push((hex_addr, kp));
    }

    // Extra faucet accounts funded from the community allocation (not additional supply)
    let faucet_amount = 1_000_000u128;
    let community_key_idx = funded_keys.iter().position(|(_, _)| true).unwrap_or(0);
    for _ in 0..n_funded {
        let kp = Keypair::generate();
        let addr = address_from_pubkey(kp.public_key().as_bytes());
        let hex_addr = hex_encode(&addr);
        accounts.insert(
            hex_addr.clone(),
            AccountEntry {
                balance: faucet_amount,
            },
        );
        funded_keys.push((hex_addr, kp));

        // Deduct from first allocation account so total stays within GENESIS_SUPPLY
        if let Some((first_key, _)) = funded_keys.get(community_key_idx)
            && let Some(acct) = accounts.get_mut(first_key)
        {
            acct.balance = acct.balance.saturating_sub(faucet_amount);
        }
    }

    let config = GenesisConfig {
        chain_id: CHAIN_ID,
        timestamp,
        validators,
        accounts,
        approved_validators: Vec::new(),
        emergency_key: None,
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

    for (hex_addr, kp, bls_kp) in &genesis.validator_keys {
        write_keyfile(&keys_dir, hex_addr, kp, Some(bls_kp))?;
    }

    for (hex_addr, kp) in &genesis.funded_keys {
        write_keyfile(&keys_dir, hex_addr, kp, None)?;
    }

    Ok(())
}

fn write_keyfile(
    keys_dir: &Path,
    filename: &str,
    kp: &Keypair,
    bls_kp: Option<&BlsKeypair>,
) -> Result<()> {
    let addr = address_from_pubkey(kp.public_key().as_bytes());
    let keyfile = KeyFile {
        public_key: hex_encode(kp.public_key().as_bytes()),
        secret_key: hex_encode(&kp.secret_bytes()),
        address: hex_encode(&addr),
        bls_public_key: bls_kp.map(|b| hex_encode(b.public_key().as_bytes())),
        bls_secret_key: bls_kp.map(|b| hex_encode(&b.secret_bytes())),
    };
    let path = keys_dir.join(format!("{filename}.json"));
    let json = serde_json::to_string_pretty(&keyfile).context("Failed to serialize key file")?;
    std::fs::write(&path, json)?;
    Ok(())
}

pub fn write_node_configs(genesis: &GeneratedGenesis, output_dir: &Path) -> Result<()> {
    use crate::config::NodeConfig;

    let genesis_path = output_dir.join("genesis.toml");
    let keys_dir = output_dir.join("keys");

    for (i, (hex_addr, _, _)) in genesis.validator_keys.iter().enumerate() {
        let base_p2p = 30333 + (i as u16) * 10;
        let mut cfg = NodeConfig {
            genesis_path: Some(genesis_path.clone()),
            validator_key: Some(keys_dir.join(format!("{hex_addr}.json"))),
            data_dir: output_dir.join(format!("node-{}", i + 1)),
            ..NodeConfig::default()
        };
        cfg.network.listen_addresses = vec![
            format!("/ip4/127.0.0.1/tcp/{base_p2p}"),
            format!("/ip4/127.0.0.1/udp/{base_p2p}/quic-v1"),
        ];
        cfg.rpc.listen_addr = format!("127.0.0.1:{}", 9944 + i as u16);

        let toml_str = toml::to_string_pretty(&cfg).context("Failed to serialize node config")?;
        let config_path = output_dir.join(format!("node-{}.toml", i + 1));
        std::fs::write(&config_path, toml_str)
            .with_context(|| format!("Failed to write {}", config_path.display()))?;
    }

    Ok(())
}

/// Write Docker-compatible testnet layout matching docker-compose.yml volume mounts.
///
/// Output structure:
///   output_dir/
///     genesis/genesis.toml
///     node1/node1.toml
///     node1/keys/validator1.json
///     node2/node2.toml
///     node2/keys/validator2.json
///     node3/node3.toml
///     node3/keys/validator3.json
pub fn write_docker_configs(genesis: &GeneratedGenesis, output_dir: &Path) -> Result<()> {
    use crate::config::NodeConfig;

    let genesis_dir = output_dir.join("genesis");
    std::fs::create_dir_all(&genesis_dir)?;

    let toml_str =
        toml::to_string_pretty(&genesis.config).context("Failed to serialize genesis config")?;
    std::fs::write(genesis_dir.join("genesis.toml"), toml_str)?;

    let n = genesis.validator_keys.len();
    for (i, (_hex_addr, kp, bls_kp)) in genesis.validator_keys.iter().enumerate() {
        let node_name = format!("node{}", i + 1);
        let validator_name = format!("validator{}", i + 1);
        let node_dir = output_dir.join(&node_name);
        let keys_dir = node_dir.join("keys");
        std::fs::create_dir_all(&keys_dir)?;

        write_keyfile(&keys_dir, &validator_name, kp, Some(bls_kp))?;

        let boot_nodes: Vec<String> = (0..n)
            .filter(|&j| j != i)
            .map(|j| format!("/dns4/validator{}/tcp/30333", j + 1))
            .collect();

        let mut cfg = NodeConfig {
            genesis_path: Some("/data/genesis/genesis.toml".into()),
            validator_key: Some(format!("/data/keys/{validator_name}.json").into()),
            data_dir: "/data".into(),
            ..NodeConfig::default()
        };
        cfg.network.listen_addresses = vec![
            "/ip4/0.0.0.0/tcp/30333".into(),
            "/ip4/0.0.0.0/udp/30333/quic-v1".into(),
        ];
        cfg.network.boot_nodes = boot_nodes;
        cfg.rpc.listen_addr = "0.0.0.0:9944".into();
        cfg.metrics.enabled = true;

        let toml_str = toml::to_string_pretty(&cfg).context("Failed to serialize node config")?;
        std::fs::write(node_dir.join(format!("{node_name}.toml")), toml_str)?;
    }

    for (hex_addr, kp) in &genesis.funded_keys {
        let keys_dir = genesis_dir.join("keys");
        std::fs::create_dir_all(&keys_dir)?;
        write_keyfile(&keys_dir, hex_addr, kp, None)?;
    }

    Ok(())
}

pub fn genesis_hash(config: &GenesisConfig) -> Result<[u8; 32]> {
    let serialized =
        toml::to_string_pretty(config).context("failed to serialize genesis config")?;
    Ok(blake3::hash(serialized.as_bytes()).into())
}

pub fn load_keyfile(path: &Path) -> Result<(Keypair, Address)> {
    let (kp, addr, _) = load_keyfile_full(path)?;
    Ok((kp, addr))
}

pub fn load_keyfile_full(path: &Path) -> Result<(Keypair, Address, Option<BlsKeypair>)> {
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

    let bls_kp = if let Some(ref bls_hex) = kf.bls_secret_key {
        let bls_bytes = hex_decode(bls_hex).context("Invalid BLS secret key hex")?;
        let bls_secret: [u8; 32] = bls_bytes
            .as_slice()
            .try_into()
            .map_err(|_| anyhow::anyhow!("BLS secret key must be 32 bytes"))?;
        Some(
            BlsKeypair::from_secret_bytes(&bls_secret)
                .context("Failed to reconstruct BLS keypair")?,
        )
    } else {
        None
    };

    Ok((kp, addr, bls_kp))
}

#[derive(Debug, thiserror::Error)]
pub enum GenesisValidationError {
    #[error("no validators in genesis config")]
    NoValidators,
    #[error("duplicate validator address: {0}")]
    DuplicateValidator(String),
    #[error("duplicate account address: {0}")]
    DuplicateAccount(String),
    #[error("validator {name} has invalid address: {address}")]
    InvalidValidatorAddress { name: String, address: String },
    #[error("account has invalid address: {0}")]
    InvalidAccountAddress(String),
    #[error("validator {name} stake {stake} below minimum {min}")]
    StakeBelowMinimum {
        name: String,
        stake: u128,
        min: u128,
    },
    #[error("total genesis supply {total} exceeds cap {cap}")]
    SupplyExceeded { total: u128, cap: u128 },
    #[error("validator {name} has invalid BLS key length: {len} (expected 48 bytes)")]
    InvalidBlsKeyLength { name: String, len: usize },
    #[error("validator {name} has invalid BLS key hex")]
    InvalidBlsKeyHex { name: String },
    #[error("validator address also appears in accounts: {0}")]
    ValidatorAccountOverlap(String),
}

pub fn validate_genesis(config: &GenesisConfig) -> Result<(), Vec<GenesisValidationError>> {
    validate_genesis_with_min_stake(config, DEFAULT_MIN_VALIDATOR_STAKE)
}

pub fn validate_genesis_with_min_stake(
    config: &GenesisConfig,
    min_stake: u128,
) -> Result<(), Vec<GenesisValidationError>> {
    let mut errors = Vec::new();

    if config.validators.is_empty() {
        errors.push(GenesisValidationError::NoValidators);
    }

    let mut validator_addrs: HashSet<String> = HashSet::new();
    for entry in &config.validators {
        if !validator_addrs.insert(entry.address.clone()) {
            errors.push(GenesisValidationError::DuplicateValidator(
                entry.address.clone(),
            ));
        }

        match hex_decode(&entry.address) {
            Some(bytes) if bytes.len() == 32 => {}
            _ => {
                errors.push(GenesisValidationError::InvalidValidatorAddress {
                    name: entry.name.clone(),
                    address: entry.address.clone(),
                });
            }
        }

        if entry.stake < min_stake {
            errors.push(GenesisValidationError::StakeBelowMinimum {
                name: entry.name.clone(),
                stake: entry.stake,
                min: min_stake,
            });
        }

        if let Some(ref bls_hex) = entry.bls_public_key {
            match hex_decode(bls_hex) {
                Some(bytes) if bytes.len() == 48 => {}
                Some(bytes) => {
                    errors.push(GenesisValidationError::InvalidBlsKeyLength {
                        name: entry.name.clone(),
                        len: bytes.len(),
                    });
                }
                None => {
                    errors.push(GenesisValidationError::InvalidBlsKeyHex {
                        name: entry.name.clone(),
                    });
                }
            }
        }
    }

    let mut account_addrs: HashSet<String> = HashSet::new();
    for hex_addr in config.accounts.keys() {
        if !account_addrs.insert(hex_addr.clone()) {
            errors.push(GenesisValidationError::DuplicateAccount(hex_addr.clone()));
        }

        match hex_decode(hex_addr) {
            Some(bytes) if bytes.len() == 32 => {}
            _ => {
                errors.push(GenesisValidationError::InvalidAccountAddress(
                    hex_addr.clone(),
                ));
            }
        }

        if validator_addrs.contains(hex_addr) {
            errors.push(GenesisValidationError::ValidatorAccountOverlap(
                hex_addr.clone(),
            ));
        }
    }

    let total_supply: u128 = config
        .validators
        .iter()
        .map(|v| v.stake)
        .chain(config.accounts.values().map(|a| a.balance))
        .sum();

    if total_supply > GENESIS_SUPPLY {
        errors.push(GenesisValidationError::SupplyExceeded {
            total: total_supply,
            cap: GENESIS_SUPPLY,
        });
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn testnet_genesis() -> GenesisConfig {
    let mut validators = Vec::with_capacity(3);
    let mut accounts = BTreeMap::new();

    for i in 0..3u8 {
        let seed = blake3::hash(format!("aztibase-testnet-validator-{i}").as_bytes());
        let kp = Keypair::from_secret_bytes(seed.as_bytes());
        let addr = address_from_pubkey(kp.public_key().as_bytes());

        let bls_ikm = blake3::hash(format!("aztibase-testnet-bls-{i}").as_bytes());
        let bls_kp = BlsKeypair::from_ikm(bls_ikm.as_bytes());
        let bls_pub_hex = hex_encode(bls_kp.public_key().as_bytes());

        validators.push(ValidatorEntry {
            name: format!("testnet-{}", i + 1),
            address: hex_encode(&addr),
            stake: 1_000_000,
            public_key: Some(hex_encode(kp.public_key().as_bytes())),
            bls_public_key: Some(bls_pub_hex),
        });
    }

    let faucet_seed = blake3::hash(b"aztibase-testnet-faucet");
    let faucet_kp = Keypair::from_secret_bytes(faucet_seed.as_bytes());
    let faucet_addr = address_from_pubkey(faucet_kp.public_key().as_bytes());
    accounts.insert(
        hex_encode(&faucet_addr),
        AccountEntry {
            balance: 100_000_000,
        },
    );

    GenesisConfig {
        chain_id: CHAIN_ID,
        timestamp: 1_710_000_000_000,
        validators,
        accounts,
        approved_validators: Vec::new(),
        emergency_key: None,
    }
}

/// Mainnet genesis with real tokenomics allocations.
/// 400M AZTB total supply across 8 categories.
/// Validator keys and allocation addresses are placeholders — replace at ceremony.
#[allow(dead_code)] // Activated during mainnet genesis ceremony
pub fn mainnet_genesis(n_validators: usize) -> GeneratedGenesis {
    const TOTAL_SUPPLY: u128 = 400_000_000;

    // Allocation percentages (must sum to 100)
    let allocations: [(&str, u128); 8] = [
        ("team", TOTAL_SUPPLY * 15 / 100),      // 60M
        ("investors", TOTAL_SUPPLY * 10 / 100), // 40M
        ("ecosystem", TOTAL_SUPPLY * 25 / 100), // 100M
        ("community", TOTAL_SUPPLY * 20 / 100), // 80M
        ("treasury", TOTAL_SUPPLY * 15 / 100),  // 60M
        ("validators", TOTAL_SUPPLY * 5 / 100), // 20M
        ("advisors", TOTAL_SUPPLY * 5 / 100),   // 20M
        ("reserve", TOTAL_SUPPLY * 5 / 100),    // 20M
    ];

    let validator_pool = allocations[5].1;
    let stake_per_validator = validator_pool / n_validators as u128;

    let mut validators = Vec::with_capacity(n_validators);
    let mut validator_keys = Vec::with_capacity(n_validators);

    for i in 0..n_validators {
        let kp = Keypair::generate();
        let bls_kp = BlsKeypair::generate();
        let addr = address_from_pubkey(kp.public_key().as_bytes());
        let hex_addr = hex_encode(&addr);
        let bls_pub_hex = hex_encode(bls_kp.public_key().as_bytes());
        validators.push(ValidatorEntry {
            name: format!("mainnet-validator-{}", i + 1),
            address: hex_addr.clone(),
            stake: stake_per_validator,
            public_key: Some(hex_encode(kp.public_key().as_bytes())),
            bls_public_key: Some(bls_pub_hex),
        });
        validator_keys.push((hex_addr, kp, bls_kp));
    }

    let mut accounts = BTreeMap::new();
    let mut funded_keys = Vec::new();

    // Create allocation accounts (skip validators — they get stake directly)
    for &(name, amount) in &allocations {
        if name == "validators" {
            continue;
        }
        let seed = blake3::hash(format!("aztibase-mainnet-{name}").as_bytes());
        let kp = Keypair::from_secret_bytes(seed.as_bytes());
        let addr = address_from_pubkey(kp.public_key().as_bytes());
        let hex_addr = hex_encode(&addr);
        accounts.insert(hex_addr.clone(), AccountEntry { balance: amount });
        funded_keys.push((hex_addr, kp));
    }

    let config = GenesisConfig {
        chain_id: CHAIN_ID,
        timestamp: 0, // Set at ceremony
        validators,
        accounts,
        approved_validators: Vec::new(),
        emergency_key: None,
    };

    GeneratedGenesis {
        config,
        validator_keys,
        funded_keys,
    }
}

pub fn testnet_boot_nodes() -> Vec<String> {
    vec![
        "/dns4/testnet1.aztibase.com/tcp/30333".into(),
        "/dns4/testnet2.aztibase.com/tcp/30334".into(),
        "/dns4/testnet3.aztibase.com/tcp/30335".into(),
    ]
}

pub fn load_genesis(path: &Path) -> Result<GenesisConfig> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read genesis file: {}", path.display()))?;
    let config: GenesisConfig =
        toml::from_str(&contents).context("Failed to parse genesis config")?;
    Ok(config)
}

fn save_genesis(path: &Path, config: &GenesisConfig) -> Result<()> {
    let toml_str = toml::to_string_pretty(config).context("Failed to serialize genesis config")?;
    std::fs::write(path, toml_str)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

pub fn init_genesis(output_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create {}", output_dir.display()))?;

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    let config = GenesisConfig {
        chain_id: CHAIN_ID,
        timestamp,
        validators: Vec::new(),
        accounts: BTreeMap::new(),
        approved_validators: Vec::new(),
        emergency_key: None,
    };

    let genesis_path = output_dir.join("genesis.toml");
    save_genesis(&genesis_path, &config)?;
    let hash = genesis_hash(&config)?;
    println!("Genesis scaffold created: {}", genesis_path.display());
    println!("Chain ID: 0x{:X} ({})", config.chain_id, config.chain_id);
    println!("Genesis hash: {}", hex_encode(&hash));
    println!("\nNext: add validators with `aztibase genesis add-validator`");
    Ok(())
}

pub fn add_validator_to_genesis(
    genesis_path: &Path,
    name: &str,
    key_path: Option<&Path>,
    address: Option<&str>,
    public_key: Option<&str>,
    bls_public_key: Option<&str>,
    stake: u128,
) -> Result<()> {
    let mut config = load_genesis(genesis_path)?;

    let (addr_hex, pk_hex, bls_hex) = if let Some(kf_path) = key_path {
        let contents = std::fs::read_to_string(kf_path)
            .with_context(|| format!("Failed to read {}", kf_path.display()))?;
        let kf: KeyFile = serde_json::from_str(&contents).context("Failed to parse keyfile")?;
        let bls = kf.bls_public_key.ok_or_else(|| {
            anyhow::anyhow!(
                "Keyfile has no BLS public key. Generate with: aztibase wallet generate --validator"
            )
        })?;
        (kf.address, Some(kf.public_key), bls)
    } else {
        let addr =
            address.ok_or_else(|| anyhow::anyhow!("Either --key or --address is required"))?;
        let bls = bls_public_key
            .ok_or_else(|| anyhow::anyhow!("--bls-public-key is required when using --address"))?;
        (
            addr.strip_prefix("0x").unwrap_or(addr).to_string(),
            public_key.map(|s| s.strip_prefix("0x").unwrap_or(s).to_string()),
            bls.strip_prefix("0x").unwrap_or(bls).to_string(),
        )
    };

    if config.validators.iter().any(|v| v.address == addr_hex) {
        anyhow::bail!("Validator with address {addr_hex} already exists in genesis");
    }
    if config.accounts.contains_key(&addr_hex) {
        anyhow::bail!("Address {addr_hex} already exists as an account in genesis");
    }

    config.validators.push(ValidatorEntry {
        name: name.to_string(),
        address: addr_hex.clone(),
        stake,
        public_key: pk_hex,
        bls_public_key: Some(bls_hex),
    });

    save_genesis(genesis_path, &config)?;
    println!("Added validator '{name}' (address: 0x{addr_hex}, stake: {stake})");
    println!("Total validators: {}", config.validators.len());
    Ok(())
}

pub fn add_account_to_genesis(genesis_path: &Path, address: &str, balance: u128) -> Result<()> {
    let mut config = load_genesis(genesis_path)?;
    let addr_hex = address.strip_prefix("0x").unwrap_or(address).to_string();

    if config.validators.iter().any(|v| v.address == addr_hex) {
        anyhow::bail!("Address {addr_hex} already exists as a validator in genesis");
    }

    config
        .accounts
        .insert(addr_hex.clone(), AccountEntry { balance });

    save_genesis(genesis_path, &config)?;
    println!("Added account 0x{addr_hex} with balance {balance}");
    println!("Total accounts: {}", config.accounts.len());
    Ok(())
}

pub fn show_genesis(genesis_path: &Path) -> Result<()> {
    let config = load_genesis(genesis_path)?;
    let hash = genesis_hash(&config)?;

    let validator_total: u128 = config.validators.iter().map(|v| v.stake).sum();
    let account_total: u128 = config.accounts.values().map(|a| a.balance).sum();
    let total_supply = validator_total + account_total;

    println!("Genesis: {}", genesis_path.display());
    println!("Chain ID: 0x{:X} ({})", config.chain_id, config.chain_id);
    println!("Timestamp: {}", config.timestamp);
    println!("Genesis hash: {}", hex_encode(&hash));
    println!();
    println!("Validators ({}):", config.validators.len());
    for v in &config.validators {
        println!("  {} — 0x{}… stake: {}", v.name, &v.address[..16], v.stake);
    }
    println!();
    println!("Accounts: {}", config.accounts.len());
    println!("Total supply: {total_supply} AZTB (cap: {GENESIS_SUPPLY})");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_genesis_config() -> GenesisConfig {
        GenesisConfig {
            chain_id: CHAIN_ID,
            timestamp: 1000,
            validators: vec![ValidatorEntry {
                name: "v1".into(),
                address: hex_encode(&[0x01; 32]),
                stake: 100_000,
                public_key: None,
                bls_public_key: Some(hex_encode(&[0xAA; 48])),
            }],
            accounts: BTreeMap::from([(hex_encode(&[0x02; 32]), AccountEntry { balance: 50_000 })]),
            approved_validators: Vec::new(),
            emergency_key: None,
        }
    }

    #[test]
    fn validate_valid_genesis_passes() {
        assert!(validate_genesis(&valid_genesis_config()).is_ok());
    }

    #[test]
    fn validate_rejects_no_validators() {
        let mut cfg = valid_genesis_config();
        cfg.validators.clear();
        let errs = validate_genesis(&cfg).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, GenesisValidationError::NoValidators))
        );
    }

    #[test]
    fn validate_rejects_duplicate_validator_address() {
        let mut cfg = valid_genesis_config();
        let dup = cfg.validators[0].clone();
        cfg.validators.push(ValidatorEntry {
            name: "v2".into(),
            ..dup
        });
        let errs = validate_genesis(&cfg).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, GenesisValidationError::DuplicateValidator(_)))
        );
    }

    #[test]
    fn validate_rejects_duplicate_account_address() {
        let mut cfg = valid_genesis_config();
        let addr = hex_encode(&[0x03; 32]);
        cfg.accounts
            .insert(addr.clone(), AccountEntry { balance: 100 });
        // BTreeMap deduplicates keys, so test a different way:
        // accounts map uses String keys so can't have actual dups.
        // Instead test validator-account overlap:
        let val_addr = cfg.validators[0].address.clone();
        cfg.accounts.insert(val_addr, AccountEntry { balance: 100 });
        let errs = validate_genesis(&cfg).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, GenesisValidationError::ValidatorAccountOverlap(_)))
        );
    }

    #[test]
    fn validate_rejects_zero_stake() {
        let mut cfg = valid_genesis_config();
        cfg.validators[0].stake = 0;
        let errs = validate_genesis(&cfg).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, GenesisValidationError::StakeBelowMinimum { .. }))
        );
    }

    #[test]
    fn validate_rejects_supply_exceeded() {
        let mut cfg = valid_genesis_config();
        cfg.validators[0].stake = GENESIS_SUPPLY;
        cfg.accounts
            .insert(hex_encode(&[0x02; 32]), AccountEntry { balance: 1 });
        let errs = validate_genesis(&cfg).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, GenesisValidationError::SupplyExceeded { .. }))
        );
    }

    #[test]
    fn validate_rejects_invalid_address() {
        let mut cfg = valid_genesis_config();
        cfg.validators[0].address = "not_hex".into();
        let errs = validate_genesis(&cfg).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, GenesisValidationError::InvalidValidatorAddress { .. }))
        );
    }

    #[test]
    fn validate_rejects_short_address() {
        let mut cfg = valid_genesis_config();
        cfg.validators[0].address = hex_encode(&[0x01; 16]); // 16 bytes, need 32
        let errs = validate_genesis(&cfg).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, GenesisValidationError::InvalidValidatorAddress { .. }))
        );
    }

    #[test]
    fn validate_rejects_bad_bls_key_length() {
        let mut cfg = valid_genesis_config();
        cfg.validators[0].bls_public_key = Some(hex_encode(&[0xBB; 32])); // 32 bytes, need 48
        let errs = validate_genesis(&cfg).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, GenesisValidationError::InvalidBlsKeyLength { .. }))
        );
    }

    #[test]
    fn validate_rejects_bad_bls_key_hex() {
        let mut cfg = valid_genesis_config();
        cfg.validators[0].bls_public_key = Some("not_valid_hex_zzz".into());
        let errs = validate_genesis(&cfg).unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, GenesisValidationError::InvalidBlsKeyHex { .. }))
        );
    }

    #[test]
    fn validate_collects_multiple_errors() {
        let cfg = GenesisConfig {
            chain_id: CHAIN_ID,
            timestamp: 1000,
            validators: vec![],
            accounts: BTreeMap::from([("bad_hex".into(), AccountEntry { balance: 100 })]),
            approved_validators: Vec::new(),
            emergency_key: None,
        };
        let errs = validate_genesis(&cfg).unwrap_err();
        assert!(errs.len() >= 2); // NoValidators + InvalidAccountAddress
    }

    #[test]
    fn validate_generated_genesis_passes() {
        let generated = generate_genesis(3, 2, 1000);
        assert!(validate_genesis(&generated.config).is_ok());
    }

    #[test]
    fn genesis_applies_balances() {
        let mut config = GenesisConfig {
            chain_id: CHAIN_ID,
            timestamp: 1000,
            validators: vec![],
            accounts: BTreeMap::new(),
            approved_validators: Vec::new(),
            emergency_key: None,
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
                public_key: Some(hex_encode(kp.public_key().as_bytes())),
                bls_public_key: None,
            }],
            accounts: BTreeMap::new(),
            approved_validators: Vec::new(),
            emergency_key: None,
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
        // 7 allocation accounts + 2 faucet accounts = 9
        assert_eq!(generated.config.accounts.len(), 9);
        assert_eq!(generated.validator_keys.len(), 3);
        // 7 allocation keys + 2 faucet keys = 9
        assert_eq!(generated.funded_keys.len(), 9);

        let toml_str = toml::to_string_pretty(&generated.config).unwrap();
        let parsed: GenesisConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.chain_id, CHAIN_ID);
        assert_eq!(parsed.validators.len(), 3);

        // Total supply should not exceed GENESIS_SUPPLY
        let total: u128 = generated
            .config
            .validators
            .iter()
            .map(|v| v.stake)
            .sum::<u128>()
            + generated
                .config
                .accounts
                .values()
                .map(|a| a.balance)
                .sum::<u128>();
        assert!(
            total <= GENESIS_SUPPLY,
            "total {total} exceeds cap {GENESIS_SUPPLY}"
        );
    }

    #[test]
    fn genesis_toml_roundtrip() {
        let generated = generate_genesis(1, 1, 42);
        let toml_str = toml::to_string_pretty(&generated.config).unwrap();
        let parsed: GenesisConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.chain_id, generated.config.chain_id);
        assert_eq!(parsed.timestamp, generated.config.timestamp);
        assert_eq!(parsed.validators.len(), 1);
        // 7 allocations + 1 faucet = 8
        assert_eq!(parsed.accounts.len(), 8);
    }

    #[test]
    fn load_keyfile_with_bls() {
        let dir =
            std::env::temp_dir().join(format!("aztibase_blsload_test_{}", std::process::id()));
        let generated = generate_genesis(2, 0, 500);
        write_genesis(&generated, &dir).unwrap();

        let (hex_addr, _, orig_bls) = &generated.validator_keys[0];
        let key_path = dir.join("keys").join(format!("{hex_addr}.json"));
        let (_, _, bls_opt) = load_keyfile_full(&key_path).unwrap();
        let loaded_bls = bls_opt.expect("BLS key should be present in validator key file");
        assert_eq!(
            loaded_bls.public_key().as_bytes(),
            orig_bls.public_key().as_bytes()
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn genesis_generates_bls_keys() {
        let generated = generate_genesis(3, 1, 1000);

        // Every validator entry should have a BLS public key
        for entry in &generated.config.validators {
            assert!(entry.bls_public_key.is_some());
            let bls_hex = entry.bls_public_key.as_ref().unwrap();
            let bls_bytes = hex_decode(bls_hex).unwrap();
            assert_eq!(bls_bytes.len(), 48, "BLS public key should be 48 bytes");
        }

        // Every validator key tuple should include a BlsKeypair
        assert_eq!(generated.validator_keys.len(), 3);
        for (_, _, bls_kp) in &generated.validator_keys {
            assert_eq!(bls_kp.public_key().as_bytes().len(), 48);
        }
    }

    #[test]
    fn genesis_writes_node_configs() {
        let dir =
            std::env::temp_dir().join(format!("aztibase_nodeconf_test_{}", std::process::id()));
        let generated = generate_genesis(3, 1, 1000);
        write_genesis(&generated, &dir).unwrap();
        write_node_configs(&generated, &dir).unwrap();

        for i in 1..=3 {
            let config_path = dir.join(format!("node-{i}.toml"));
            assert!(config_path.exists(), "node-{i}.toml should exist");

            let contents = std::fs::read_to_string(&config_path).unwrap();
            let cfg: crate::config::NodeConfig = toml::from_str(&contents).unwrap();
            assert!(cfg.genesis_path.is_some());
            assert!(cfg.validator_key.is_some());
            assert_eq!(cfg.data_dir, dir.join(format!("node-{i}")));
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn testnet_genesis_is_valid() {
        let cfg = testnet_genesis();
        assert_eq!(cfg.chain_id, CHAIN_ID);
        assert_eq!(cfg.validators.len(), 3);
        assert_eq!(cfg.accounts.len(), 1);
        assert!(validate_genesis(&cfg).is_ok());
    }

    #[test]
    fn testnet_genesis_is_deterministic() {
        let a = testnet_genesis();
        let b = testnet_genesis();
        assert_eq!(genesis_hash(&a).unwrap(), genesis_hash(&b).unwrap());
        for i in 0..3 {
            assert_eq!(a.validators[i].address, b.validators[i].address);
            assert_eq!(
                a.validators[i].bls_public_key,
                b.validators[i].bls_public_key
            );
        }
    }

    #[test]
    fn testnet_boot_nodes_are_valid_multiaddrs() {
        for addr in testnet_boot_nodes() {
            assert!(addr.starts_with("/dns4/testnet"));
            assert!(addr.contains("/tcp/"));
        }
    }

    #[test]
    fn config_file_loads_correctly() {
        let dir =
            std::env::temp_dir().join(format!("aztibase_cfgload_test_{}", std::process::id()));
        let generated = generate_genesis(2, 0, 500);
        write_genesis(&generated, &dir).unwrap();
        write_node_configs(&generated, &dir).unwrap();

        let cfg = crate::config::NodeConfig::load(&dir.join("node-1.toml")).unwrap();
        assert!(cfg.genesis_path.is_some());
        assert!(cfg.validator_key.is_some());
        assert!(cfg.rpc.listen_addr.contains("9944"));

        let _ = std::fs::remove_dir_all(&dir);
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
        // 7 allocations + 1 faucet = 8
        assert_eq!(loaded.accounts.len(), 8);

        let keys_dir = dir.join("keys");
        let key_files: Vec<_> = std::fs::read_dir(&keys_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .collect();
        // 2 validators + 8 funded accounts = 10
        assert_eq!(key_files.len(), 10);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn init_genesis_creates_scaffold() {
        let dir =
            std::env::temp_dir().join(format!("aztibase_init_genesis_{}", std::process::id()));
        init_genesis(&dir).unwrap();

        let genesis_path = dir.join("genesis.toml");
        assert!(genesis_path.exists());

        let config = load_genesis(&genesis_path).unwrap();
        assert_eq!(config.chain_id, CHAIN_ID);
        assert!(config.validators.is_empty());
        assert!(config.accounts.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn add_validator_from_keyfile() {
        let dir = std::env::temp_dir().join(format!("aztibase_addval_kf_{}", std::process::id()));
        init_genesis(&dir).unwrap();
        let genesis_path = dir.join("genesis.toml");

        let keys_dir = dir.join("keys");
        std::fs::create_dir_all(&keys_dir).unwrap();
        let kp = Keypair::generate();
        let bls_kp = BlsKeypair::generate();
        let addr = address_from_pubkey(kp.public_key().as_bytes());
        let keyfile = KeyFile {
            public_key: hex_encode(kp.public_key().as_bytes()),
            secret_key: hex_encode(&kp.secret_bytes()),
            address: hex_encode(&addr),
            bls_public_key: Some(hex_encode(bls_kp.public_key().as_bytes())),
            bls_secret_key: Some(hex_encode(&bls_kp.secret_bytes())),
        };
        let kf_path = keys_dir.join("val.json");
        std::fs::write(&kf_path, serde_json::to_string_pretty(&keyfile).unwrap()).unwrap();

        add_validator_to_genesis(
            &genesis_path,
            "alice",
            Some(&kf_path),
            None,
            None,
            None,
            500_000,
        )
        .unwrap();

        let config = load_genesis(&genesis_path).unwrap();
        assert_eq!(config.validators.len(), 1);
        assert_eq!(config.validators[0].name, "alice");
        assert_eq!(config.validators[0].stake, 500_000);
        assert!(config.validators[0].bls_public_key.is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn add_validator_from_public_info() {
        let dir = std::env::temp_dir().join(format!("aztibase_addval_pub_{}", std::process::id()));
        init_genesis(&dir).unwrap();
        let genesis_path = dir.join("genesis.toml");

        let addr = hex_encode(&[0x42; 32]);
        let pk = hex_encode(&[0x11; 32]);
        let bls = hex_encode(&[0xAA; 48]);

        add_validator_to_genesis(
            &genesis_path,
            "bob",
            None,
            Some(&addr),
            Some(&pk),
            Some(&bls),
            1_000_000,
        )
        .unwrap();

        let config = load_genesis(&genesis_path).unwrap();
        assert_eq!(config.validators.len(), 1);
        assert_eq!(config.validators[0].name, "bob");
        assert_eq!(config.validators[0].address, addr);
        assert_eq!(
            config.validators[0].public_key.as_deref(),
            Some(pk.as_str())
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn add_validator_rejects_duplicate() {
        let dir = std::env::temp_dir().join(format!("aztibase_addval_dup_{}", std::process::id()));
        init_genesis(&dir).unwrap();
        let genesis_path = dir.join("genesis.toml");

        let addr = hex_encode(&[0x42; 32]);
        let bls = hex_encode(&[0xAA; 48]);

        add_validator_to_genesis(
            &genesis_path,
            "v1",
            None,
            Some(&addr),
            None,
            Some(&bls),
            100_000,
        )
        .unwrap();
        let result = add_validator_to_genesis(
            &genesis_path,
            "v2",
            None,
            Some(&addr),
            None,
            Some(&bls),
            100_000,
        );
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn add_account_works() {
        let dir = std::env::temp_dir().join(format!("aztibase_addacct_{}", std::process::id()));
        init_genesis(&dir).unwrap();
        let genesis_path = dir.join("genesis.toml");

        let addr = hex_encode(&[0x55; 32]);
        add_account_to_genesis(&genesis_path, &addr, 5_000_000).unwrap();

        let config = load_genesis(&genesis_path).unwrap();
        assert_eq!(config.accounts.len(), 1);
        assert_eq!(config.accounts[&addr].balance, 5_000_000);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn add_account_rejects_validator_overlap() {
        let dir =
            std::env::temp_dir().join(format!("aztibase_addacct_overlap_{}", std::process::id()));
        init_genesis(&dir).unwrap();
        let genesis_path = dir.join("genesis.toml");

        let addr = hex_encode(&[0x42; 32]);
        let bls = hex_encode(&[0xAA; 48]);
        add_validator_to_genesis(
            &genesis_path,
            "v1",
            None,
            Some(&addr),
            None,
            Some(&bls),
            100_000,
        )
        .unwrap();

        let result = add_account_to_genesis(&genesis_path, &addr, 1_000);
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn show_genesis_runs_without_error() {
        let dir = std::env::temp_dir().join(format!("aztibase_showgen_{}", std::process::id()));
        let generated = generate_genesis(2, 1, 1000);
        write_genesis(&generated, &dir).unwrap();
        show_genesis(&dir.join("genesis.toml")).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn full_ceremony_flow() {
        let dir = std::env::temp_dir().join(format!("aztibase_ceremony_{}", std::process::id()));
        init_genesis(&dir).unwrap();
        let genesis_path = dir.join("genesis.toml");

        for i in 0..3 {
            let addr = hex_encode(&[i + 1; 32]);
            let pk = hex_encode(&[i + 0x10; 32]);
            let bls = hex_encode(&[i + 0xA0; 48]);
            add_validator_to_genesis(
                &genesis_path,
                &format!("validator-{}", i + 1),
                None,
                Some(&addr),
                Some(&pk),
                Some(&bls),
                1_000_000,
            )
            .unwrap();
        }

        let faucet = hex_encode(&[0xFF; 32]);
        add_account_to_genesis(&genesis_path, &faucet, 10_000_000).unwrap();

        let config = load_genesis(&genesis_path).unwrap();
        assert_eq!(config.validators.len(), 3);
        assert_eq!(config.accounts.len(), 1);

        assert!(validate_genesis(&config).is_ok());
        show_genesis(&genesis_path).unwrap();

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mainnet_genesis_valid() {
        let result = mainnet_genesis(4);
        let cfg = &result.config;

        assert_eq!(cfg.chain_id, CHAIN_ID);
        assert_eq!(cfg.validators.len(), 4);
        assert_eq!(cfg.accounts.len(), 7); // 8 categories minus validators

        let validator_total: u128 = cfg.validators.iter().map(|v| v.stake).sum();
        let account_total: u128 = cfg.accounts.values().map(|a| a.balance).sum();
        assert_eq!(validator_total + account_total, 400_000_000);

        // Each validator gets equal share of 5% pool (20M / 4 = 5M)
        for v in &cfg.validators {
            assert_eq!(v.stake, 5_000_000);
            assert!(v.bls_public_key.is_some());
        }

        // No faucet account
        let has_faucet = cfg.accounts.keys().any(|k| k.contains("faucet"));
        assert!(!has_faucet);
    }
}
