use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkProfile {
    #[default]
    Dev,
    Testnet,
    Mainnet,
}

impl std::fmt::Display for NetworkProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dev => write!(f, "dev"),
            Self::Testnet => write!(f, "testnet"),
            Self::Mainnet => write!(f, "mainnet"),
        }
    }
}

impl NetworkProfile {
    pub fn is_mainnet(self) -> bool {
        matches!(self, Self::Mainnet)
    }

    pub fn faucet_enabled(self) -> bool {
        !self.is_mainnet()
    }

    pub fn permissive_cors(self) -> bool {
        !self.is_mainnet()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NodeConfig {
    pub data_dir: PathBuf,
    pub genesis_path: Option<PathBuf>,
    pub validator_key: Option<PathBuf>,
    pub profile: NetworkProfile,
    pub network: NetworkConfig,
    pub rpc: RpcConfig,
    pub log: LogConfig,
    pub ai: AiConfig,
    pub metrics: MetricsConfig,
    pub archive: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MetricsConfig {
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NetworkConfig {
    pub listen_addresses: Vec<String>,
    pub boot_nodes: Vec<String>,
    pub idle_timeout_secs: u64,
    pub stun_servers: Vec<String>,
    pub enable_webrtc: bool,
    pub webrtc_listen_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RpcConfig {
    pub listen_addr: String,
    pub enabled: bool,
    pub rate_limit_per_ip: u32,
    pub max_body_bytes: usize,
    pub cors_allowed_origins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LogConfig {
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AiConfig {
    pub enabled: bool,
    pub models: Vec<ModelEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub model_id: String,
    pub path: PathBuf,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            genesis_path: None,
            validator_key: None,
            profile: NetworkProfile::default(),
            network: NetworkConfig::default(),
            rpc: RpcConfig::default(),
            log: LogConfig::default(),
            ai: AiConfig::default(),
            metrics: MetricsConfig::default(),
            archive: false,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec![
                "/ip4/0.0.0.0/tcp/30333".into(),
                "/ip4/0.0.0.0/udp/30333/quic-v1".into(),
            ],
            boot_nodes: Vec::new(),
            idle_timeout_secs: 60,
            stun_servers: vec![
                "stun:stun.l.google.com:19302".into(),
                "stun:stun1.l.google.com:19302".into(),
            ],
            enable_webrtc: false,
            webrtc_listen_port: 9000,
        }
    }
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:9944".into(),
            enabled: true,
            rate_limit_per_ip: 100,
            max_body_bytes: 1_048_576,
            cors_allowed_origins: Vec::new(),
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: "info".into(),
        }
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            models: Vec::new(),
        }
    }
}

fn default_data_dir() -> PathBuf {
    directories::ProjectDirs::from("network", "aztibase", "aztibase-node")
        .map(|dirs| dirs.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from("./data"))
}

impl NodeConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let contents = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let config: NodeConfig =
            toml::from_str(&contents).context("Failed to parse config file")?;
        Ok(config)
    }

    pub fn load_or_default(path: Option<&Path>) -> Result<Self> {
        match path {
            Some(p) if p.exists() => Self::load(p),
            _ => Ok(Self::default()),
        }
    }

    pub fn storage_path(&self) -> PathBuf {
        self.data_dir.join("db")
    }

    pub fn execution_storage_path(&self) -> PathBuf {
        self.data_dir.join("execution_db")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profile_is_dev() {
        assert_eq!(NetworkProfile::default(), NetworkProfile::Dev);
    }

    #[test]
    fn mainnet_profile_gates_faucet() {
        assert!(NetworkProfile::Dev.faucet_enabled());
        assert!(NetworkProfile::Testnet.faucet_enabled());
        assert!(!NetworkProfile::Mainnet.faucet_enabled());
    }

    #[test]
    fn mainnet_profile_restricts_cors() {
        assert!(NetworkProfile::Dev.permissive_cors());
        assert!(NetworkProfile::Testnet.permissive_cors());
        assert!(!NetworkProfile::Mainnet.permissive_cors());
    }

    #[test]
    fn is_mainnet_only_for_mainnet() {
        assert!(!NetworkProfile::Dev.is_mainnet());
        assert!(!NetworkProfile::Testnet.is_mainnet());
        assert!(NetworkProfile::Mainnet.is_mainnet());
    }

    #[test]
    fn profile_display() {
        assert_eq!(NetworkProfile::Dev.to_string(), "dev");
        assert_eq!(NetworkProfile::Testnet.to_string(), "testnet");
        assert_eq!(NetworkProfile::Mainnet.to_string(), "mainnet");
    }

    #[test]
    fn profile_serde_roundtrip() {
        let json = serde_json::to_string(&NetworkProfile::Mainnet).unwrap();
        assert_eq!(json, r#""mainnet""#);
        let parsed: NetworkProfile = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, NetworkProfile::Mainnet);
    }

    #[test]
    fn rpc_config_defaults() {
        let rpc = RpcConfig::default();
        assert_eq!(rpc.rate_limit_per_ip, 100);
        assert_eq!(rpc.max_body_bytes, 1_048_576);
        assert!(rpc.cors_allowed_origins.is_empty());
    }
}
