use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use aztibase_core::{BlsKeypair, Keypair, address_from_pubkey};
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bls_public_key: Option<String>,
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
    let mut validators = Vec::with_capacity(n_validators);
    let mut validator_keys = Vec::with_capacity(n_validators);

    for i in 0..n_validators {
        let kp = Keypair::generate();
        let bls_kp = BlsKeypair::generate();
        let addr = address_from_pubkey(kp.public_key().as_bytes());
        let hex_addr = hex_encode(&addr);
        let bls_pub_hex = hex_encode(bls_kp.public_key().as_bytes());
        validators.push(ValidatorEntry {
            name: format!("validator-{}", i + 1),
            address: hex_addr.clone(),
            stake: 1_000_000,
            bls_public_key: Some(bls_pub_hex),
        });
        validator_keys.push((hex_addr, kp, bls_kp));
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

pub fn genesis_hash(config: &GenesisConfig) -> [u8; 32] {
    let serialized = toml::to_string_pretty(config).expect("genesis config serializable");
    blake3::hash(serialized.as_bytes()).into()
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
                bls_public_key: None,
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
