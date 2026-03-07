use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NodeConfig {
    pub data_dir: PathBuf,
    pub genesis_path: Option<PathBuf>,
    pub validator_key: Option<PathBuf>,
    pub network: NetworkConfig,
    pub rpc: RpcConfig,
    pub log: LogConfig,
    pub ai: AiConfig,
    pub metrics: MetricsConfig,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RpcConfig {
    pub listen_addr: String,
    pub enabled: bool,
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
            network: NetworkConfig::default(),
            rpc: RpcConfig::default(),
            log: LogConfig::default(),
            ai: AiConfig::default(),
            metrics: MetricsConfig::default(),
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
        }
    }
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:9944".into(),
            enabled: true,
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
