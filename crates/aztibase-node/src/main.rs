mod config;
mod genesis;
#[cfg(test)]
mod integration;
mod mempool;
mod pipeline;
mod sync;
mod task_pool;
mod wallet;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use tokio::sync::Notify;
use tracing_subscriber::EnvFilter;

use aztibase_consensus::{
    CommittedBatch, ConsensusConfig, ConsensusEngine, ConsensusInput, ConsensusOutput, DagStore,
    StateRootAnnounce, ValidatorSet,
};
use aztibase_network::{
    BlockSyncProtocol, CommittedBatchAnnounce, Libp2pTransport, NetworkEvent, PeerId,
    PeerReputationStore, PeerStore, SyncBatch, TOPIC_COMMITTED_BATCHES, TOPIC_CONSENSUS,
    TOPIC_STATE_SYNC, TOPIC_TRANSACTIONS, TransportConfig, build_batch_response,
    build_header_response, chain_scoped_topics, decode_request, encode_response,
    genesis_hex_prefix,
};
use aztibase_rpc::{EventBus, NodeMetrics, RpcServer};
use aztibase_runtime::TractRuntime;
use aztibase_storage::StateStore;
use config::NodeConfig;
use sync::{
    SnapshotAssembler, SyncMessage, build_snapshot_request, build_snapshot_response,
    decode_sync_message, encode_sync_message,
};

#[derive(Parser, Debug)]
#[command(name = "aztibase", about = "Aztibase Network Node")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Path to TOML configuration file
    #[arg(long, short, global = true)]
    config: Option<PathBuf>,

    /// Override data directory
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,

    /// Override P2P listen address (may be repeated)
    #[arg(long)]
    listen: Vec<String>,

    /// Override RPC listen address
    #[arg(long)]
    rpc_addr: Option<String>,

    /// Override log level (trace, debug, info, warn, error)
    #[arg(long, global = true)]
    log_level: Option<String>,

    /// Path to validator key file (JSON). Determines this node's identity.
    #[arg(long)]
    validator_key: Option<PathBuf>,

    /// Validator index for testing (1-255). Fallback when no --validator-key.
    #[arg(long, default_value = "1")]
    validator_index: u8,

    /// Total number of validators in the test network (fallback without genesis)
    #[arg(long, default_value = "1")]
    validator_count: u8,

    /// Path to genesis.toml for initial state
    #[arg(long)]
    genesis: Option<PathBuf>,

    /// Enable metrics HTTP endpoint at GET /metrics on the RPC address
    #[arg(long)]
    metrics: bool,

    /// Run as a light node (header sync only, no execution or vertex proposal)
    #[arg(long)]
    light: bool,

    /// Enable WebRTC direct transport for browser-node connectivity (feature-gated)
    #[arg(long)]
    webrtc: bool,

    /// Run as an archive node (retain full history, disable eviction and DAG pruning)
    #[arg(long)]
    archive: bool,

    /// Trusted weak subjectivity checkpoint: batch_index:state_root_hex (e.g. 1000:abcdef01...)
    #[arg(long)]
    checkpoint: Option<String>,

    /// Override epoch length (rounds per epoch). Useful for testing epoch boundaries
    /// with short epochs (e.g. 100). Default: 10,000 rounds.
    #[arg(long)]
    epoch_length: Option<u64>,

    /// Use the built-in public testnet genesis config and boot nodes
    #[arg(long, conflicts_with = "mainnet")]
    testnet: bool,

    /// Run with mainnet profile (disables faucet, restricts CORS)
    #[arg(long, conflicts_with = "testnet")]
    mainnet: bool,

    /// Bootstrap peer address in multiaddr format (may be repeated)
    #[arg(long)]
    boot_node: Vec<String>,

    /// Bootstrap from a snapshot file instead of replaying from genesis
    #[arg(long)]
    snapshot: Option<PathBuf>,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Genesis configuration management
    Genesis {
        #[command(subcommand)]
        action: GenesisAction,
    },
    /// Export a full state snapshot from the node database
    Snapshot {
        #[command(subcommand)]
        action: SnapshotAction,
    },
    /// Wallet key management and transaction signing
    Wallet {
        #[command(subcommand)]
        action: WalletAction,
    },
}

#[derive(clap::Subcommand, Debug)]
enum GenesisAction {
    /// Generate a complete genesis with random keys (existing behavior)
    Generate {
        /// Number of validators
        #[arg(long, default_value = "4")]
        validators: usize,
        /// Number of pre-funded accounts
        #[arg(long, default_value = "2")]
        funded: usize,
        /// Output directory
        #[arg(long, default_value = "genesis")]
        output: PathBuf,
        /// Generate Docker-compatible layout for docker-compose
        #[arg(long)]
        docker: bool,
    },
    /// Initialize an empty genesis scaffold for a ceremony
    Init {
        /// Output directory (genesis.toml will be created here)
        #[arg(long, default_value = "genesis")]
        output: PathBuf,
    },
    /// Add a validator to an existing genesis config
    AddValidator {
        /// Path to genesis.toml
        #[arg(long)]
        genesis: PathBuf,
        /// Validator name
        #[arg(long)]
        name: String,
        /// Path to validator keyfile (extracts public info)
        #[arg(long, conflicts_with_all = ["address", "public_key", "bls_public_key"])]
        key: Option<PathBuf>,
        /// Validator address hex (when sharing public info only)
        #[arg(long, requires = "bls_public_key")]
        address: Option<String>,
        /// Ed25519 public key hex
        #[arg(long)]
        public_key: Option<String>,
        /// BLS12-381 public key hex
        #[arg(long)]
        bls_public_key: Option<String>,
        /// Validator stake amount
        #[arg(long, default_value = "1000000")]
        stake: u128,
    },
    /// Add a pre-funded account to an existing genesis config
    AddAccount {
        /// Path to genesis.toml
        #[arg(long)]
        genesis: PathBuf,
        /// Account address hex
        #[arg(long)]
        address: String,
        /// Account balance
        #[arg(long)]
        balance: u128,
    },
    /// Validate a genesis config
    Validate {
        /// Path to genesis.toml
        #[arg(long)]
        genesis: PathBuf,
    },
    /// Show genesis config summary and hash
    Show {
        /// Path to genesis.toml
        #[arg(long)]
        genesis: PathBuf,
    },
}

#[derive(clap::Subcommand, Debug)]
enum SnapshotAction {
    /// Export current state to a snapshot file
    Export {
        /// Path to the node database directory
        #[arg(long)]
        data_dir: PathBuf,
        /// Output snapshot file path
        #[arg(long, default_value = "snapshot.aztb")]
        output: PathBuf,
    },
    /// Inspect a snapshot file without importing
    Info {
        /// Path to snapshot file
        path: PathBuf,
    },
}

#[derive(clap::Subcommand, Debug)]
enum WalletAction {
    /// Generate a new Ed25519 keypair
    Generate {
        /// Output key file path
        #[arg(long, default_value = "wallet.json")]
        output: PathBuf,
        /// Generate with BIP-39 mnemonic and Argon2id-encrypted keyfile
        #[arg(long)]
        mnemonic: bool,
        /// Passphrase for keyfile encryption (required with --mnemonic)
        #[arg(long)]
        passphrase: Option<String>,
        /// Generate a validator keypair (Ed25519 + BLS12-381)
        #[arg(long)]
        validator: bool,
    },
    /// Recover keypair from BIP-39 mnemonic phrase
    Recover {
        /// The BIP-39 mnemonic phrase (12 or 24 words)
        #[arg(long)]
        phrase: String,
        /// Output key file path
        #[arg(long, default_value = "recovered.json")]
        output: PathBuf,
    },
    /// Show address and public key from a key file
    Show {
        /// Path to key file
        keyfile: PathBuf,
        /// Passphrase to decrypt an encrypted keyfile
        #[arg(long)]
        passphrase: Option<String>,
    },
    /// Derive a child keypair from a mnemonic at a given account index
    Derive {
        /// The BIP-39 mnemonic phrase (12 or 24 words)
        #[arg(long)]
        phrase: String,
        /// Account index (m/44'/aztb'/INDEX'/0/0)
        #[arg(long)]
        index: u32,
        /// Output key file path (optional, saves encrypted keyfile)
        #[arg(long)]
        output: Option<PathBuf>,
        /// Passphrase for encrypting the derived keyfile
        #[arg(long)]
        passphrase: Option<String>,
    },
    /// List all keyfiles in the keys directory
    List {
        /// Directory containing keyfiles (default: {data_dir}/keys/)
        #[arg(long)]
        dir: Option<PathBuf>,
    },
    /// Query account balance and nonce from a full node
    Balance {
        /// Account address (hex, 32 bytes)
        #[arg(long)]
        address: String,
        /// RPC endpoint URL
        #[arg(long)]
        rpc: String,
    },
    /// Export a keyfile as portable JSON
    Export {
        /// Path to keyfile to export
        #[arg(long)]
        key: PathBuf,
    },
    /// Import a keyfile from JSON
    Import {
        /// Path to JSON file to import
        #[arg(long)]
        file: PathBuf,
        /// Output path (default: keys/{address}.json)
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Sign and broadcast a transfer transaction
    Transfer {
        /// Path to sender key file
        #[arg(long)]
        from: PathBuf,
        /// Recipient address (hex)
        #[arg(long)]
        to: String,
        /// Transfer amount
        #[arg(long)]
        value: u128,
        /// Sender nonce
        #[arg(long)]
        nonce: u64,
        /// Gas price
        #[arg(long, default_value = "1")]
        gas_price: u64,
        /// Passphrase for encrypted keyfile
        #[arg(long)]
        passphrase: Option<String>,
        /// RPC endpoint to broadcast to (e.g. http://127.0.0.1:9944)
        #[arg(long)]
        rpc: Option<String>,
    },
    /// Stake tokens as a validator
    Stake {
        /// Path to validator key file
        #[arg(long)]
        key: PathBuf,
        /// Amount to stake
        #[arg(long)]
        amount: u128,
        /// Account nonce
        #[arg(long)]
        nonce: u64,
        /// Gas price
        #[arg(long, default_value = "1")]
        gas_price: u64,
        /// Passphrase for encrypted keyfile
        #[arg(long)]
        passphrase: Option<String>,
        /// RPC endpoint to broadcast to
        #[arg(long)]
        rpc: Option<String>,
    },
    /// Unstake tokens (begins 21-day unbonding)
    Unstake {
        /// Path to validator key file
        #[arg(long)]
        key: PathBuf,
        /// Amount to unstake
        #[arg(long)]
        amount: u128,
        /// Account nonce
        #[arg(long)]
        nonce: u64,
        /// Gas price
        #[arg(long, default_value = "1")]
        gas_price: u64,
        /// Passphrase for encrypted keyfile
        #[arg(long)]
        passphrase: Option<String>,
        /// RPC endpoint to broadcast to
        #[arg(long)]
        rpc: Option<String>,
    },
    /// Delegate tokens to a validator
    Delegate {
        /// Path to delegator key file
        #[arg(long)]
        key: PathBuf,
        /// Validator address to delegate to (hex)
        #[arg(long)]
        validator: String,
        /// Amount to delegate
        #[arg(long)]
        amount: u128,
        /// Account nonce
        #[arg(long)]
        nonce: u64,
        /// Gas price
        #[arg(long, default_value = "1")]
        gas_price: u64,
        /// Passphrase for encrypted keyfile
        #[arg(long)]
        passphrase: Option<String>,
        /// RPC endpoint to broadcast to
        #[arg(long)]
        rpc: Option<String>,
    },
    /// Undelegate tokens from a validator (begins 21-day unbonding)
    Undelegate {
        /// Path to delegator key file
        #[arg(long)]
        key: PathBuf,
        /// Account nonce
        #[arg(long)]
        nonce: u64,
        /// Gas price
        #[arg(long, default_value = "1")]
        gas_price: u64,
        /// Passphrase for encrypted keyfile
        #[arg(long)]
        passphrase: Option<String>,
        /// RPC endpoint to broadcast to
        #[arg(long)]
        rpc: Option<String>,
    },
    /// Query staking info for an address (validator stake, delegation, unbonding)
    StakingInfo {
        /// Account address (hex, 32 bytes)
        #[arg(long)]
        address: String,
        /// RPC endpoint URL
        #[arg(long)]
        rpc: String,
    },
    /// List all active validators and their stakes
    Validators {
        /// RPC endpoint URL
        #[arg(long)]
        rpc: String,
    },
}

impl Cli {
    fn apply_overrides(&self, mut cfg: NodeConfig) -> NodeConfig {
        if let Some(ref dir) = self.data_dir {
            cfg.data_dir = dir.clone();
        }
        if !self.listen.is_empty() {
            cfg.network.listen_addresses = self.listen.clone();
        }
        if let Some(ref addr) = self.rpc_addr {
            cfg.rpc.listen_addr = addr.clone();
        }
        if let Some(ref level) = self.log_level {
            cfg.log.level = level.clone();
        }
        if let Some(ref p) = self.genesis {
            cfg.genesis_path = Some(p.clone());
        }
        if let Some(ref p) = self.validator_key {
            cfg.validator_key = Some(p.clone());
        }
        if self.metrics {
            cfg.metrics.enabled = true;
        }
        if self.archive {
            cfg.archive = true;
        }
        if !self.boot_node.is_empty() {
            cfg.network.boot_nodes = self.boot_node.clone();
        }
        cfg
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Genesis { action }) => {
            match action {
                GenesisAction::Generate {
                    validators,
                    funded,
                    output,
                    docker,
                } => {
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_millis() as u64;
                    let generated = genesis::generate_genesis(validators, funded, timestamp);
                    if docker {
                        genesis::write_docker_configs(&generated, &output)?;
                        let hash = genesis::genesis_hash(&generated.config)?;
                        println!(
                            "Docker testnet written to {} ({} validators, {} funded accounts)",
                            output.display(),
                            validators,
                            funded,
                        );
                        println!("Genesis hash: {}", genesis::hex_encode(&hash));
                        println!("\nLayout:");
                        println!("  {}/genesis/genesis.toml", output.display());
                        for i in 1..=validators {
                            println!("  {}/node{i}/node{i}.toml", output.display());
                            println!("  {}/node{i}/keys/validator{i}.json", output.display());
                        }
                    } else {
                        genesis::write_genesis(&generated, &output)?;
                        genesis::write_node_configs(&generated, &output)?;
                        println!(
                            "Genesis written to {} ({} validators, {} funded accounts, {} node configs)",
                            output.display(),
                            validators,
                            funded,
                            validators,
                        );
                    }
                }
                GenesisAction::Init { output } => {
                    genesis::init_genesis(&output)?;
                }
                GenesisAction::AddValidator {
                    genesis: genesis_path,
                    name,
                    key,
                    address,
                    public_key,
                    bls_public_key,
                    stake,
                } => {
                    genesis::add_validator_to_genesis(
                        &genesis_path,
                        &name,
                        key.as_deref(),
                        address.as_deref(),
                        public_key.as_deref(),
                        bls_public_key.as_deref(),
                        stake,
                    )?;
                }
                GenesisAction::AddAccount {
                    genesis: genesis_path,
                    address,
                    balance,
                } => {
                    genesis::add_account_to_genesis(&genesis_path, &address, balance)?;
                }
                GenesisAction::Validate {
                    genesis: genesis_path,
                } => {
                    let config = genesis::load_genesis(&genesis_path)?;
                    match genesis::validate_genesis(&config) {
                        Ok(()) => {
                            println!("Genesis config is valid.");
                            let hash = genesis::genesis_hash(&config)?;
                            println!("Genesis hash: {}", genesis::hex_encode(&hash));
                        }
                        Err(errors) => {
                            eprintln!("Genesis validation failed ({} errors):", errors.len());
                            for err in &errors {
                                eprintln!("  - {err}");
                            }
                            std::process::exit(1);
                        }
                    }
                }
                GenesisAction::Show {
                    genesis: genesis_path,
                } => {
                    genesis::show_genesis(&genesis_path)?;
                }
            }
            return Ok(());
        }
        Some(Command::Snapshot { action }) => {
            match action {
                SnapshotAction::Export { data_dir, output } => {
                    let storage_path = data_dir.join("execution_db");
                    let store = StateStore::open(storage_path.to_str().unwrap_or("execution_db"))
                        .context("Failed to open database")?;
                    let state = aztibase_execution::load_state(&store)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    let batch_index = aztibase_execution::latest_batch_index(&store)
                        .map_err(|e| anyhow::anyhow!("{e}"))?
                        .unwrap_or(0);
                    let base_fee = aztibase_execution::load_base_fee(&store)
                        .map_err(|e| anyhow::anyhow!("{e}"))?
                        .unwrap_or(1_000);
                    let staking = aztibase_execution::load_staking(&store)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    let governance = aztibase_execution::load_governance(&store)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    let emission = aztibase_execution::load_emission(&store)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    let chain_params = aztibase_execution::load_chain_params(&store)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    let agent_policies = aztibase_execution::load_agent_policies(&store)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    let (l2_registry, l2_anchors, bridge_escrow, bridge_proofs) =
                        aztibase_execution::load_bridge_stores(&store)
                            .map_err(|e| anyhow::anyhow!("{e}"))?;

                    let bundle = aztibase_execution::ProtocolStoreBundle {
                        staking,
                        governance,
                        emission,
                        chain_params,
                        agent_policies,
                        l2_registry,
                        l2_anchors,
                        bridge_escrow,
                        bridge_proofs,
                        base_fee,
                    };

                    let account_count = state.account_count();
                    let snapshot = aztibase_execution::create_full_snapshot(
                        &state,
                        batch_index,
                        batch_index,
                        vec![],
                        bundle,
                    );
                    aztibase_execution::write_snapshot_file(&output, &snapshot)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;

                    println!("Snapshot exported to {}", output.display());
                    println!("  Batch index: {batch_index}");
                    println!("  Accounts:    {account_count}");
                    println!(
                        "  State root:  {}",
                        genesis::hex_encode(&snapshot.state_root)
                    );
                    println!("  Base fee:    {base_fee}");
                }
                SnapshotAction::Info { path } => {
                    let snapshot = aztibase_execution::read_snapshot_file(&path)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    println!("Snapshot: {}", path.display());
                    println!("  Version:     {}", snapshot.version());
                    println!("  Batch index: {}", snapshot.batch_index);
                    println!("  Height:      {}", snapshot.height);
                    println!(
                        "  State root:  {}",
                        genesis::hex_encode(&snapshot.state_root)
                    );
                    println!(
                        "  Finality:    {} bytes",
                        snapshot.finality_certificate.len()
                    );
                    println!(
                        "  Protocol:    {}",
                        if snapshot.protocol_stores.is_some() {
                            "included"
                        } else {
                            "not included"
                        }
                    );
                    if let Some(ref bundle) = snapshot.protocol_stores {
                        println!("  Base fee:    {}", bundle.base_fee);
                    }
                }
            }
            return Ok(());
        }
        Some(Command::Wallet { action }) => {
            match action {
                WalletAction::Generate {
                    output,
                    mnemonic,
                    passphrase,
                    validator,
                } => {
                    if mnemonic {
                        let pass = passphrase.unwrap_or_else(|| {
                            eprintln!("Enter passphrase for keyfile encryption:");
                            let mut buf = String::new();
                            std::io::stdin().read_line(&mut buf).unwrap_or_else(|e| {
                                eprintln!("Failed to read from stdin: {e}");
                                std::process::exit(1);
                            });
                            buf.trim().to_string()
                        });
                        let phrase = wallet::generate_key_with_mnemonic(&output, &pass)?;
                        println!("\nBACKUP YOUR MNEMONIC (24 words):");
                        println!("{phrase}");
                        println!("\nStore this safely. It is the ONLY way to recover your key.");
                    } else if validator {
                        wallet::generate_validator_key(&output)?;
                    } else {
                        wallet::generate_key(&output)?;
                    }
                }
                WalletAction::Recover { phrase, output } => {
                    let kp = wallet::recover_from_mnemonic(&phrase)?;
                    let addr = aztibase_core::address_from_pubkey(kp.public_key().as_bytes());
                    wallet::generate_key(&output)?;
                    println!("Recovered address: {}", genesis::hex_encode(&addr));
                }
                WalletAction::Derive {
                    phrase,
                    index,
                    output,
                    passphrase,
                } => {
                    let kp = wallet::derive_account(&phrase, index)?;
                    let addr = aztibase_core::address_from_pubkey(kp.public_key().as_bytes());
                    println!("Account index: {index}");
                    println!("Address: {}", genesis::hex_encode(&addr));
                    println!(
                        "Public key: {}",
                        genesis::hex_encode(kp.public_key().as_bytes())
                    );
                    if let Some(out_path) = output {
                        let pass = passphrase.unwrap_or_else(|| {
                            eprintln!("Enter passphrase for keyfile encryption:");
                            let mut buf = String::new();
                            std::io::stdin().read_line(&mut buf).unwrap_or_else(|e| {
                                eprintln!("Failed to read from stdin: {e}");
                                std::process::exit(1);
                            });
                            buf.trim().to_string()
                        });
                        let encrypted = wallet::encrypt_keyfile_pub(&kp.secret_bytes(), &pass)?;
                        let enc_kf = wallet::EncryptedKeyFile {
                            public_key: genesis::hex_encode(kp.public_key().as_bytes()),
                            address: genesis::hex_encode(&addr),
                            encrypted,
                        };
                        if let Some(parent) = out_path.parent() {
                            std::fs::create_dir_all(parent)?;
                        }
                        let json = serde_json::to_string_pretty(&enc_kf)?;
                        std::fs::write(&out_path, json)?;
                        println!("Key file (encrypted): {}", out_path.display());
                    }
                }
                WalletAction::Show {
                    keyfile,
                    passphrase,
                } => {
                    if let Some(pass) = passphrase {
                        let kp = wallet::load_encrypted_keyfile(&keyfile, &pass)?;
                        let addr = aztibase_core::address_from_pubkey(kp.public_key().as_bytes());
                        println!("Address: {}", genesis::hex_encode(&addr));
                        println!(
                            "Public key: {}",
                            genesis::hex_encode(kp.public_key().as_bytes())
                        );
                    } else {
                        wallet::show_key(&keyfile)?;
                    }
                }
                WalletAction::List { dir } => {
                    let keys_dir = dir.unwrap_or_else(|| {
                        let cfg =
                            NodeConfig::load_or_default(cli.config.as_deref()).unwrap_or_default();
                        cfg.data_dir.join("keys")
                    });
                    let entries = wallet::list_keys(&keys_dir)?;
                    if entries.is_empty() {
                        println!("No keyfiles found in {}", keys_dir.display());
                    } else {
                        println!("{:<68}  ENCRYPTED", "ADDRESS");
                        for (addr, _pk, encrypted) in &entries {
                            let enc_str = if *encrypted { "yes" } else { "no" };
                            println!("0x{addr}  {enc_str}");
                        }
                        println!("\n{} keyfile(s) found", entries.len());
                    }
                }
                WalletAction::Balance { address, rpc } => {
                    let addr_hex = address.strip_prefix("0x").unwrap_or(&address);
                    wallet::query_balance(&rpc, addr_hex).await?;
                }
                WalletAction::Export { key } => {
                    let json = wallet::export_keyfile(&key)?;
                    println!("{json}");
                }
                WalletAction::Import { file, output } => {
                    let json = std::fs::read_to_string(&file)
                        .with_context(|| format!("Failed to read {}", file.display()))?;
                    let out = output.unwrap_or_else(|| PathBuf::from("imported.json"));
                    let address = wallet::import_keyfile(&json, &out)?;
                    println!("Imported address: 0x{address}");
                    println!("Saved to: {}", out.display());
                }
                WalletAction::Transfer {
                    from,
                    to,
                    value,
                    nonce,
                    gas_price,
                    passphrase,
                    rpc,
                } => {
                    let envelope = if let Some(pass) = passphrase {
                        wallet::sign_transfer_encrypted(&from, &pass, &to, value, nonce, gas_price)?
                    } else {
                        wallet::sign_transfer(&from, &to, value, nonce, gas_price)?
                    };
                    let hex = genesis::hex_encode(&envelope);
                    if let Some(rpc_url) = rpc {
                        let tx_hash = wallet::broadcast_transaction(&rpc_url, &hex).await?;
                        println!("Broadcast OK. TX hash: {tx_hash}");
                    } else {
                        println!("{hex}");
                    }
                }
                WalletAction::Stake {
                    key,
                    amount,
                    nonce,
                    gas_price,
                    passphrase,
                    rpc,
                } => {
                    let envelope = if let Some(pass) = passphrase {
                        wallet::sign_stake_encrypted(&key, &pass, amount, nonce, gas_price)?
                    } else {
                        wallet::sign_stake(&key, amount, nonce, gas_price)?
                    };
                    let hex = genesis::hex_encode(&envelope);
                    if let Some(rpc_url) = rpc {
                        let tx_hash = wallet::broadcast_transaction(&rpc_url, &hex).await?;
                        println!("Stake TX broadcast OK. TX hash: {tx_hash}");
                    } else {
                        println!("{hex}");
                    }
                }
                WalletAction::Unstake {
                    key,
                    amount,
                    nonce,
                    gas_price,
                    passphrase,
                    rpc,
                } => {
                    let envelope = if let Some(pass) = passphrase {
                        wallet::sign_unstake_encrypted(&key, &pass, amount, nonce, gas_price)?
                    } else {
                        wallet::sign_unstake(&key, amount, nonce, gas_price)?
                    };
                    let hex = genesis::hex_encode(&envelope);
                    if let Some(rpc_url) = rpc {
                        let tx_hash = wallet::broadcast_transaction(&rpc_url, &hex).await?;
                        println!("Unstake TX broadcast OK. TX hash: {tx_hash}");
                        println!("Unbonding period: ~21 days");
                    } else {
                        println!("{hex}");
                    }
                }
                WalletAction::Delegate {
                    key,
                    validator,
                    amount,
                    nonce,
                    gas_price,
                    passphrase,
                    rpc,
                } => {
                    let envelope = if let Some(pass) = passphrase {
                        wallet::sign_delegate_encrypted(
                            &key, &pass, &validator, amount, nonce, gas_price,
                        )?
                    } else {
                        wallet::sign_delegate(&key, &validator, amount, nonce, gas_price)?
                    };
                    let hex = genesis::hex_encode(&envelope);
                    if let Some(rpc_url) = rpc {
                        let tx_hash = wallet::broadcast_transaction(&rpc_url, &hex).await?;
                        println!("Delegate TX broadcast OK. TX hash: {tx_hash}");
                    } else {
                        println!("{hex}");
                    }
                }
                WalletAction::Undelegate {
                    key,
                    nonce,
                    gas_price,
                    passphrase,
                    rpc,
                } => {
                    let envelope = if let Some(pass) = passphrase {
                        wallet::sign_undelegate_encrypted(&key, &pass, nonce, gas_price)?
                    } else {
                        wallet::sign_undelegate(&key, nonce, gas_price)?
                    };
                    let hex = genesis::hex_encode(&envelope);
                    if let Some(rpc_url) = rpc {
                        let tx_hash = wallet::broadcast_transaction(&rpc_url, &hex).await?;
                        println!("Undelegate TX broadcast OK. TX hash: {tx_hash}");
                        println!("Unbonding period: ~21 days");
                    } else {
                        println!("{hex}");
                    }
                }
                WalletAction::StakingInfo { address, rpc } => {
                    let addr_hex = address.strip_prefix("0x").unwrap_or(&address);
                    wallet::query_staking_info(&rpc, addr_hex).await?;
                }
                WalletAction::Validators { rpc } => {
                    wallet::query_active_validators(&rpc).await?;
                }
            }
            return Ok(());
        }
        None => {}
    }

    let config = NodeConfig::load_or_default(cli.config.as_deref())?;
    let mut config = cli.apply_overrides(config);

    if cli.mainnet {
        config.profile = config::NetworkProfile::Mainnet;
    } else if cli.testnet {
        config.profile = config::NetworkProfile::Testnet;
    }

    if cli.testnet && config.network.boot_nodes.is_empty() {
        config.network.boot_nodes = genesis::testnet_boot_nodes();
    }

    init_logging(&config.log.level)?;

    tracing::info!(profile = %config.profile, "Network profile selected");

    if cli.light {
        return run_light_node(&config).await;
    }

    tracing::info!(validator = cli.validator_index, "Starting Aztibase node");
    tracing::info!(data_dir = %config.data_dir.display());

    // Storage
    std::fs::create_dir_all(&config.data_dir)
        .with_context(|| format!("Failed to create data dir: {}", config.data_dir.display()))?;
    let storage_path = config.storage_path();
    let storage_path_str = storage_path.to_str().context("Invalid storage path")?;
    let store = StateStore::open(storage_path_str).context("Failed to open storage")?;
    tracing::info!(path = %storage_path.display(), "Storage initialized");

    // Load genesis config (--genesis file > config file > --testnet bundled > none)
    let genesis_path = cli.genesis.as_ref().or(config.genesis_path.as_ref());
    let genesis_config = if let Some(path) = genesis_path {
        Some(genesis::load_genesis(path)?)
    } else if cli.testnet {
        tracing::info!("Using built-in testnet genesis config");
        Some(genesis::testnet_genesis())
    } else {
        None
    };

    if let Some(ref gen_cfg) = genesis_config {
        if let Err(errors) = genesis::validate_genesis(gen_cfg) {
            for err in &errors {
                tracing::error!(%err, "Genesis validation error");
            }
            anyhow::bail!(
                "Genesis config has {} validation error(s) — aborting",
                errors.len()
            );
        }
        let ghash = genesis::genesis_hash(gen_cfg)?;
        tracing::info!(
            genesis_hash = %genesis::hex_encode(&ghash),
            validators = gen_cfg.validators.len(),
            accounts = gen_cfg.accounts.len(),
            "Genesis config validated"
        );
    }

    // Load validator key file (CLI flag > config file > none)
    let key_path = cli.validator_key.as_ref().or(config.validator_key.as_ref());
    let validator_keypair = key_path.map(|p| genesis::load_keyfile(p)).transpose()?;

    // Consensus: build validator set from genesis or fallback to hardcoded
    let dag = DagStore::new(store).context("Failed to initialize DAG store")?;
    let (identity, validators, node_is_validator) = if let Some(ref gen_cfg) = genesis_config {
        let mut vs = ValidatorSet::new();
        for entry in &gen_cfg.validators {
            if let Some(addr) = genesis::hex_decode(&entry.address)
                .and_then(|b| <[u8; 32]>::try_from(b.as_slice()).ok())
            {
                let bls_pk = entry
                    .bls_public_key
                    .as_ref()
                    .and_then(|hex| genesis::hex_decode(hex))
                    .and_then(|bytes| <[u8; 48]>::try_from(bytes.as_slice()).ok())
                    .and_then(aztibase_core::BlsPublicKey::from_bytes);
                vs.add_with_bls(addr, entry.stake, bls_pk);
                if let Some(ed_pk) = entry
                    .public_key
                    .as_ref()
                    .and_then(|hex| genesis::hex_decode(hex))
                    .and_then(|bytes| <[u8; 32]>::try_from(bytes.as_slice()).ok())
                {
                    vs.set_ed25519_key(&addr, ed_pk);
                }
            }
        }
        let (id, node_is_validator) = if let Some((_, key_addr)) = &validator_keypair {
            if !vs.contains(key_addr) {
                tracing::info!(
                    address = %genesis::hex_encode(key_addr),
                    "Key not in genesis validators — running as full node (can stake to become validator)"
                );
                (*key_addr, false)
            } else {
                tracing::info!(
                    address = %genesis::hex_encode(key_addr),
                    "Validator identity from key file"
                );
                (*key_addr, true)
            }
        } else {
            let mut rng_bytes = [0u8; 32];
            rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut rng_bytes);
            tracing::info!(
                address = %genesis::hex_encode(&rng_bytes),
                "No validator key — running as full node"
            );
            (rng_bytes, false)
        };
        tracing::info!(validators = vs.len(), "Validator set loaded from genesis");
        (id, vs, node_is_validator)
    } else {
        let mut vs = ValidatorSet::new();
        for i in 1..=cli.validator_count {
            vs.add([i; 32], 100);
        }
        ([cli.validator_index; 32], vs, true)
    };

    let consensus_config = ConsensusConfig {
        archive: config.archive,
        ..ConsensusConfig::default()
    };
    let (consensus_tx, consensus_rx) = tokio::sync::mpsc::channel::<ConsensusInput>(256);
    let (output_tx, mut output_rx) = tokio::sync::mpsc::channel::<ConsensusOutput>(256);

    let signing_key = if let Some((ref kp, _)) = validator_keypair {
        kp.clone()
    } else {
        aztibase_core::Keypair::from_secret_bytes(&identity)
    };
    let mut engine = ConsensusEngine::new(
        consensus_config,
        identity,
        signing_key,
        dag,
        validators,
        consensus_rx,
        output_tx,
    );
    let consensus_metrics = engine.metrics();
    tracing::info!("Consensus engine initialized");

    // Mempool
    let mut mempool = mempool::Mempool::new(10_000);
    tracing::info!("Mempool initialized (capacity: 10000)");

    // Execution pipeline (separate redb for account state)
    let exec_storage_path = config.execution_storage_path();
    let exec_storage_str = exec_storage_path
        .to_str()
        .context("Invalid execution storage path")?;
    let exec_store =
        StateStore::open(exec_storage_str).context("Failed to open execution storage")?;
    let exec_store = Arc::new(exec_store);
    tracing::info!(path = %exec_storage_path.display(), "Execution storage initialized");

    if aztibase_execution::check_sentinel(&exec_store).unwrap_or(false) {
        tracing::warn!(
            "Previous shutdown was unclean — sentinel still present. State may be stale."
        );
    }
    aztibase_execution::write_sentinel(&exec_store).context("Failed to write startup sentinel")?;
    let shutdown_store = Arc::clone(&exec_store);

    let (pipeline_tx, pipeline_rx) = tokio::sync::mpsc::channel::<CommittedBatch>(256);
    let (result_tx, mut result_rx) = tokio::sync::mpsc::channel::<pipeline::PipelineResult>(256);
    let mut exec_pipeline =
        pipeline::ExecutionPipeline::with_storage(Arc::clone(&exec_store), pipeline_rx);
    exec_pipeline.set_result_sender(result_tx);
    if config.archive {
        exec_pipeline.set_archive(true);
        tracing::info!("Archive mode enabled — eviction disabled, full history retained");
    }

    if let Some(epoch_len) = cli.epoch_length {
        let cp = exec_pipeline.shared_chain_params();
        let mut cp_guard = cp.write().await;
        if let Err(e) = cp_guard.set(
            "epoch_length",
            aztibase_execution::ParamValue::U64(epoch_len),
        ) {
            tracing::warn!(epoch_length = epoch_len, error = %e, "Failed to set epoch_length override");
        } else {
            tracing::info!(epoch_length = epoch_len, "Epoch length overridden via CLI");
        }
    }

    if let Some(ref cp_str) = cli.checkpoint {
        let parts: Vec<&str> = cp_str.splitn(2, ':').collect();
        if parts.len() != 2 {
            anyhow::bail!(
                "Invalid --checkpoint format. Expected batch_index:state_root_hex (e.g. 1000:abcdef01...)"
            );
        }
        let cp_batch: u64 = parts[0]
            .parse()
            .context("--checkpoint batch_index must be a u64")?;
        let cp_root_hex = parts[1];
        if cp_root_hex.len() != 64 {
            anyhow::bail!("--checkpoint state_root must be exactly 64 hex characters (32 bytes)");
        }
        let mut cp_root = [0u8; 32];
        for (i, chunk) in cp_root_hex.as_bytes().chunks(2).enumerate() {
            let hex_str = std::str::from_utf8(chunk).context("Invalid hex in --checkpoint")?;
            cp_root[i] = u8::from_str_radix(hex_str, 16).context("Invalid hex in --checkpoint")?;
        }

        match aztibase_execution::get_checkpoint_raw(&exec_store, cp_batch) {
            Ok(Some(bytes)) => {
                match postcard::from_bytes::<aztibase_consensus::Checkpoint>(&bytes) {
                    Ok(stored) => {
                        if stored.state_root != cp_root {
                            let stored_hex: String = stored
                                .state_root
                                .iter()
                                .map(|b| format!("{b:02x}"))
                                .collect();
                            anyhow::bail!(
                                "Checkpoint mismatch at batch {cp_batch}: stored state_root={stored_hex}, trusted={cp_root_hex}. Node may be on wrong chain."
                            );
                        }
                        tracing::info!(
                            batch = cp_batch,
                            "Trusted checkpoint verified — state root matches"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "Failed to deserialize stored checkpoint");
                    }
                }
            }
            Ok(None) => {
                tracing::warn!(
                    batch = cp_batch,
                    "No stored checkpoint at batch {cp_batch} — will be verified when reached"
                );
            }
            Err(e) => {
                tracing::warn!(error = %e, "Failed to read checkpoint from storage");
            }
        }
    }

    let ai_runtime = Arc::new(TractRuntime::new());
    let models_dir = config.data_dir.join("models");
    if config.ai.enabled {
        for entry in &config.ai.models {
            if let Ok(canonical) = entry.path.canonicalize() {
                if !canonical.starts_with(&config.data_dir) {
                    tracing::warn!(
                        model_id = %entry.model_id,
                        path = %entry.path.display(),
                        "Model path outside data_dir, skipping (path traversal blocked)"
                    );
                    continue;
                }
            } else if !entry.path.starts_with(&config.data_dir)
                && !entry.path.starts_with(&models_dir)
            {
                tracing::warn!(
                    model_id = %entry.model_id,
                    path = %entry.path.display(),
                    "Model path outside data_dir, skipping"
                );
                continue;
            }
            match std::fs::read(&entry.path) {
                Ok(bytes) => match ai_runtime.register_model(&entry.model_id, &bytes) {
                    Ok(()) => {
                        tracing::info!(
                            model_id = %entry.model_id,
                            path = %entry.path.display(),
                            "AI model loaded"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(
                            model_id = %entry.model_id,
                            error = %e,
                            "Failed to register AI model"
                        );
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        model_id = %entry.model_id,
                        path = %entry.path.display(),
                        error = %e,
                        "Failed to read AI model file"
                    );
                }
            }
        }
    }
    exec_pipeline.set_ai_runtime(ai_runtime);
    tracing::info!("Execution pipeline initialized (AI runtime: tract)");

    // Bootstrap from snapshot file if --snapshot is provided
    if let Some(ref snapshot_path) = cli.snapshot {
        let shared = exec_pipeline.shared_state();
        let state_guard = shared.read().await;
        if state_guard.account_count() == 0 {
            drop(state_guard);
            tracing::info!(path = %snapshot_path.display(), "Loading snapshot file");
            let snap = aztibase_execution::read_snapshot_file(snapshot_path)
                .map_err(|e| anyhow::anyhow!("Failed to load snapshot: {e}"))?;
            let (restored_state, bundle) = aztibase_execution::apply_full_snapshot(&snap)
                .map_err(|e| anyhow::anyhow!("Failed to apply snapshot: {e}"))?;
            let mut state_guard = shared.write().await;
            *state_guard = restored_state;
            tracing::info!(
                batch_index = snap.batch_index,
                accounts = state_guard.account_count(),
                state_root = %genesis::hex_encode(&snap.state_root),
                "State bootstrapped from snapshot"
            );
            drop(state_guard);
            exec_pipeline.apply_protocol_bundle(bundle).await;
            tracing::info!("Protocol stores restored from snapshot");
        } else {
            tracing::info!("Database already has state — ignoring --snapshot flag");
        }
    }

    if let Some(ref gen_cfg) = genesis_config {
        let shared = exec_pipeline.shared_state();
        let mut state_guard = shared.write().await;
        if state_guard.account_count() == 0 {
            genesis::apply_genesis(gen_cfg, &mut state_guard);
            tracing::info!(
                accounts = state_guard.account_count(),
                "Genesis state applied"
            );

            // Bootstrap staking store from genesis validators.
            let genesis_validators: Vec<([u8; 32], u128, Option<[u8; 32]>)> = gen_cfg
                .validators
                .iter()
                .filter_map(|v| {
                    genesis::hex_decode(&v.address)
                        .and_then(|b| <[u8; 32]>::try_from(b.as_slice()).ok())
                        .map(|addr| {
                            let ed25519_pk = v.public_key.as_ref().and_then(|pk| {
                                genesis::hex_decode(pk)
                                    .and_then(|b| <[u8; 32]>::try_from(b.as_slice()).ok())
                            });
                            (addr, v.stake, ed25519_pk)
                        })
                })
                .collect();
            drop(state_guard);
            exec_pipeline
                .bootstrap_genesis_validators(&genesis_validators)
                .await;
        } else {
            tracing::info!("State already populated, skipping genesis");
            drop(state_guard);
        }
    }

    // Prometheus metrics registry
    let node_metrics = NodeMetrics::new();

    // Event bus for WebSocket subscriptions
    let event_bus = Arc::new(EventBus::new());

    // RPC server
    let (mempool_tx, mut mempool_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(4096);
    let mut rpc_server = RpcServer::new(
        exec_pipeline.shared_state(),
        mempool_tx,
        exec_pipeline.shared_batch_count(),
        Some(exec_store),
        exec_pipeline.shared_base_fee(),
    )
    .with_faucet_enabled(config.profile.faucet_enabled())
    .with_rate_limit(config.rpc.rate_limit_per_ip)
    .with_cors_config(
        config.profile.permissive_cors(),
        config.rpc.cors_allowed_origins.clone(),
    )
    .with_max_body_bytes(config.rpc.max_body_bytes)
    .with_event_bus(Arc::clone(&event_bus))
    .with_pending_task_count(exec_pipeline.shared_pending_task_count())
    .with_compute_commitments(exec_pipeline.shared_compute_commitments())
    .with_governance(exec_pipeline.shared_governance())
    .with_chain_params(exec_pipeline.shared_chain_params())
    .with_emission_tracker(exec_pipeline.shared_emission_tracker())
    .with_staking_store(exec_pipeline.shared_staking_store())
    .with_agent_policy_store(exec_pipeline.shared_agent_policy_store())
    .with_l2_registry(exec_pipeline.shared_l2_registry())
    .with_l2_anchor_store(exec_pipeline.shared_l2_anchor_store())
    .with_bridge_escrow(exec_pipeline.shared_bridge_escrow())
    .with_bridge_withdraw_proofs(exec_pipeline.shared_bridge_withdraw_proofs())
    .with_validator(node_is_validator);

    if let Some(ref gen_cfg) = genesis_config {
        rpc_server = rpc_server.with_genesis_hash(genesis::genesis_hash(gen_cfg)?);
    }

    if config.metrics.enabled || cli.metrics {
        rpc_server = rpc_server.with_metrics(node_metrics.clone());
        tracing::info!("Metrics enabled at GET /metrics (Prometheus) and GET /metrics/json");
    }

    if config.rpc.enabled {
        let rpc_addr: std::net::SocketAddr = config
            .rpc
            .listen_addr
            .parse()
            .with_context(|| format!("Invalid RPC address: {}", config.rpc.listen_addr))?;
        tokio::spawn(async move {
            if let Err(e) = rpc_server.serve(rpc_addr).await {
                tracing::error!(error = %e, "RPC server failed");
            }
        });
        tracing::info!(addr = %config.rpc.listen_addr, "RPC server started");
    }

    // Network
    let rep_db_path = config.data_dir.join("peer_reputation.redb");
    let rep_store = Arc::new(
        PeerReputationStore::open(&rep_db_path).context("Failed to open peer reputation store")?,
    );
    let peer_db_path = config.data_dir.join("peer_store.redb");
    let peer_store = Arc::new(PeerStore::open(&peer_db_path).context("Failed to open peer store")?);
    let batch_archive_path = config.data_dir.join("batch_archive.redb");
    let batch_archive = aztibase_network::BatchArchive::open(&batch_archive_path)
        .context("Failed to open batch archive")?;
    let boot_addrs: Vec<aztibase_network::Multiaddr> = config
        .network
        .boot_nodes
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();
    let transport_genesis_hash = match genesis_config.as_ref() {
        Some(cfg) => Some(genesis::genesis_hash(cfg)?),
        None => None,
    };
    let transport_config = TransportConfig {
        idle_timeout_secs: config.network.idle_timeout_secs,
        reputation_store: Some(rep_store),
        peer_store: Some(Arc::clone(&peer_store)),
        relay_servers: boot_addrs,
        enable_webrtc: cli.webrtc || config.network.enable_webrtc,
        webrtc_listen_port: config.network.webrtc_listen_port,
        genesis_hash: transport_genesis_hash,
        ..TransportConfig::default()
    };
    let mut transport =
        Libp2pTransport::new(transport_config).context("Failed to create network transport")?;
    let cached = transport.load_cached_peers(50);
    tracing::info!(peer_id = %transport.local_peer_id(), nat = %transport.nat_status(), cached_peers = cached, "Network identity");

    let mut listen_count = 0usize;
    for addr_str in &config.network.listen_addresses {
        let addr: aztibase_network::Multiaddr = addr_str
            .parse()
            .with_context(|| format!("Invalid listen address: {addr_str}"))?;
        match transport.listen_on(addr) {
            Ok(_) => {
                listen_count += 1;
            }
            Err(e) => {
                tracing::warn!(addr = %addr_str, error = %e, "Failed to listen (non-fatal)");
            }
        }
    }
    if listen_count == 0 {
        anyhow::bail!("Failed to listen on any address");
    }

    for boot_str in &config.network.boot_nodes {
        let addr: aztibase_network::Multiaddr = boot_str
            .parse()
            .with_context(|| format!("Invalid boot node address: {boot_str}"))?;
        if let Err(e) = transport.dial(addr) {
            tracing::warn!(addr = %boot_str, error = %e, "Failed to dial boot node");
        }
    }

    let scoped_genesis_hex = transport_genesis_hash.as_ref().map(genesis_hex_prefix);
    let scoped_topics = chain_scoped_topics(scoped_genesis_hex.as_deref());
    let topic_consensus = scoped_topics
        .get(2)
        .cloned()
        .unwrap_or(TOPIC_CONSENSUS.to_string());
    let topic_state_sync = scoped_topics
        .get(3)
        .cloned()
        .unwrap_or(TOPIC_STATE_SYNC.to_string());
    let topic_transactions = scoped_topics
        .get(1)
        .cloned()
        .unwrap_or(TOPIC_TRANSACTIONS.to_string());
    let topic_committed_batches = scoped_topics
        .get(7)
        .cloned()
        .unwrap_or(TOPIC_COMMITTED_BATCHES.to_string());

    tracing::info!("Node started — press Ctrl+C to shut down");

    let shutdown = Arc::new(Notify::new());
    let shutdown_signal = shutdown.clone();

    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("Shutdown signal received");
        shutdown_signal.notify_waiters();
    });

    // Spawn consensus engine
    tracing::info!(
        identity = %genesis::hex_encode(&identity),
        "Spawning consensus engine"
    );
    let consensus_handle = tokio::spawn(async move {
        if let Err(e) = engine.run().await {
            tracing::error!(error = %e, "Consensus engine failed");
        }
        tracing::warn!("Consensus engine task exited");
    });

    // Wire consensus ↔ pipeline bridges
    let slash_tx = exec_pipeline.slash_sender();
    exec_pipeline.set_consensus_tx(consensus_tx.clone());

    // Shared state for sync protocol + mempool gas price validation
    let shared_state = exec_pipeline.shared_state();
    let shared_base_fee = exec_pipeline.shared_base_fee();
    let shared_staking = exec_pipeline.shared_staking_store();

    // Spawn execution pipeline
    let pipeline_handle = tokio::spawn(async move {
        exec_pipeline.run().await;
    });

    let mut sync_assembler: Option<SnapshotAssembler> = None;
    let mut sync_bootstrapped = false;
    let mut peer_count: u64 = 0;

    // Block sync: recent batch buffer for serving catch-up requests
    const MAX_BATCH_HISTORY: usize = 1000;
    let archived_batches = batch_archive
        .load_recent(MAX_BATCH_HISTORY)
        .unwrap_or_default();
    let mut batch_history: std::collections::VecDeque<SyncBatch> =
        std::collections::VecDeque::from(archived_batches);
    let mut batch_index: u64 = batch_history.back().map(|b| b.index).unwrap_or(0);
    if !batch_history.is_empty() {
        tracing::info!(
            count = batch_history.len(),
            from = batch_history.front().map(|b| b.index).unwrap_or(0),
            to = batch_index,
            "Loaded batch history from archive"
        );
    }

    // Buffer raw transactions between BatchCommitted and PipelineResult
    let mut pending_batch_txs: std::collections::HashMap<[u8; 32], Vec<Vec<u8>>> =
        std::collections::HashMap::new();

    // Block sync: catch-up protocol for full nodes
    let mut sync_proto = BlockSyncProtocol::new(batch_index);
    let mut connected_peers: Vec<PeerId> = Vec::new();
    let mut catchup_pending = false;
    let mut catchup_ticker = tokio::time::interval(std::time::Duration::from_secs(5));
    catchup_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    // Request snapshot if state is empty (new node joining the network)
    {
        let state_guard = shared_state.read().await;
        if state_guard.account_count() == 0 {
            tracing::info!("State is empty — will request snapshot from peers after connecting");
        }
    }

    // Main event loop
    loop {
        tokio::select! {
            event = transport.next_event() => {
                match event {
                    NetworkEvent::Listening(addr) => {
                        tracing::info!(addr = %addr, "Listening");
                    }
                    NetworkEvent::PeerConnected(peer) => {
                        peer_count += 1;
                        node_metrics.set_peer_count(peer_count);
                        if !connected_peers.contains(&peer) {
                            connected_peers.push(peer);
                        }
                        tracing::info!(peer = %peer, peers = peer_count, "Peer connected");
                        let _ = consensus_tx.send(ConsensusInput::PeerCountChanged(peer_count)).await;
                    }
                    NetworkEvent::PeerDisconnected(peer) => {
                        peer_count = peer_count.saturating_sub(1);
                        node_metrics.set_peer_count(peer_count);
                        connected_peers.retain(|p| *p != peer);
                        tracing::debug!(peer = %peer, peers = peer_count, "Peer disconnected");
                        let _ = consensus_tx.send(ConsensusInput::PeerCountChanged(peer_count)).await;
                    }
                    NetworkEvent::Message { source, topic, data } => {
                        tracing::debug!(
                            source = %source,
                            topic = %topic,
                            bytes = data.len(),
                            "Received message"
                        );
                        if topic == topic_consensus {
                            tracing::debug!(
                                source = %source,
                                bytes = data.len(),
                                "Forwarding consensus vertex from gossip to engine"
                            );
                            let _ = consensus_tx.send(ConsensusInput::ReceivedVertex(data)).await;
                        } else if topic == topic_state_sync {
                            if let Ok(msg) = decode_sync_message(&data) {
                                match msg {
                                    SyncMessage::SnapshotRequest { requester, .. } => {
                                        tracing::info!(
                                            requester = requester[0],
                                            "Peer requested state snapshot"
                                        );
                                        let state_guard = shared_state.read().await;
                                        if state_guard.account_count() > 0 {
                                            match build_snapshot_response(&state_guard, batch_index) {
                                                Ok(msgs) => {
                                                    for m in &msgs {
                                                        if let Ok(encoded) = encode_sync_message(m) {
                                                            let _ = transport.publish(&topic_state_sync, encoded);
                                                        }
                                                    }
                                                    tracing::info!(
                                                        chunks = msgs.len(),
                                                        batch = batch_index,
                                                        "Sent state snapshot"
                                                    );
                                                }
                                                Err(e) => {
                                                    tracing::warn!(error = %e, "Failed to build snapshot response");
                                                }
                                            }
                                        }
                                    }
                                    SyncMessage::SnapshotResponse {
                                        batch_index: bi,
                                        state_root: sr,
                                        total_chunks,
                                        chunk_index,
                                        chunk_hash,
                                        data: chunk_data,
                                        ..
                                    } => {
                                        if sync_bootstrapped {
                                            continue;
                                        }
                                        if sync_assembler.is_none() {
                                            match SnapshotAssembler::new(total_chunks, bi, sr) {
                                                Ok(a) => {
                                                    tracing::info!(
                                                        total_chunks,
                                                        batch = bi,
                                                        "Receiving state snapshot"
                                                    );
                                                    sync_assembler = Some(a);
                                                }
                                                Err(e) => {
                                                    tracing::warn!(error = %e, "Invalid snapshot metadata");
                                                    continue;
                                                }
                                            }
                                        }
                                        let assembler = sync_assembler.as_mut().unwrap();
                                        if let Err(e) = assembler.add_chunk(chunk_index, chunk_hash, chunk_data) {
                                            tracing::warn!(error = %e, "Bad snapshot chunk");
                                            sync_assembler = None;
                                        } else if assembler.is_complete() {
                                            let asm = sync_assembler.take().unwrap();
                                            match asm.assemble() {
                                                Ok(snapshot) => {
                                                    match sync::bootstrap_from_snapshot(
                                                        &snapshot,
                                                        &shared_state,
                                                        None,
                                                    ).await {
                                                        Ok(()) => {
                                                            tracing::info!(
                                                                batch = snapshot.batch_index,
                                                                "State bootstrapped from snapshot"
                                                            );
                                                            batch_index = snapshot.batch_index;
                                                            sync_bootstrapped = true;
                                                        }
                                                        Err(e) => {
                                                            tracing::warn!(error = %e, "Snapshot bootstrap failed");
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::warn!(error = %e, "Snapshot assembly failed");
                                                }
                                            }
                                        }
                                    }
                                }
                            } else if let Ok(announce) = postcard::from_bytes::<StateRootAnnounce>(&data) {
                                tracing::debug!(
                                    peer_validator = announce.validator[0],
                                    batch = announce.batch_index,
                                    "Received state root announcement"
                                );
                                // If state is empty and we haven't bootstrapped, request a snapshot
                                if !sync_bootstrapped && sync_assembler.is_none() {
                                    let state_guard = shared_state.read().await;
                                    if state_guard.account_count() == 0 {
                                        let req = build_snapshot_request(identity);
                                        if let Ok(encoded) = encode_sync_message(&req) {
                                            let _ = transport.publish(&topic_state_sync, encoded);
                                            tracing::info!("Sent snapshot request to peers");
                                        }
                                    }
                                }
                            }
                        } else if topic == topic_transactions {
                            let state_guard = shared_state.read().await;
                            let min_gp = shared_base_fee.load(std::sync::atomic::Ordering::Relaxed);
                            if mempool.insert_checked(data.clone(), |addr| state_guard.nonce(addr), min_gp) {
                                drop(state_guard);
                                tracing::info!(
                                    tx_len = data.len(),
                                    mempool_size = mempool.len(),
                                    "Gossip tx accepted into mempool"
                                );
                                let _ = consensus_tx.send(ConsensusInput::Transaction(data)).await;
                                node_metrics.set_mempool_size(mempool.len() as u64);
                            } else {
                                drop(state_guard);
                                tracing::debug!(
                                    tx_len = data.len(),
                                    "Gossip tx rejected by mempool (dup/nonce/gas)"
                                );
                            }
                        } else if topic == topic_committed_batches
                            && let Ok(announce) = aztibase_network::decode_batch_announce(&data)
                        {
                            tracing::info!(
                                batch = announce.index,
                                txs = announce.transactions.len(),
                                "Received committed batch via gossip"
                            );
                            sync_proto.set_tip_index(announce.index);
                            if announce.index > batch_index + 1 && !catchup_pending {
                                tracing::info!(
                                    local = batch_index,
                                    remote = announce.index,
                                    gap = announce.index - batch_index - 1,
                                    "Gap detected, triggering catch-up"
                                );
                            }
                            let batch = CommittedBatch {
                                anchor_hash: announce.anchor_hash,
                                vertex_order: vec![],
                                transactions: announce.transactions,
                            };
                            if pipeline_tx.send(batch).await.is_err() {
                                tracing::error!("Pipeline channel closed during block sync");
                                break;
                            }
                        }
                    }
                    NetworkEvent::LightSyncRequest { peer, request, channel } => {
                        match decode_request(&request) {
                            Ok(aztibase_network::LightSyncMessage::RequestHeaders { from_round, count, .. }) => {
                                tracing::debug!(peer = %peer, from = from_round, count, "Light sync: headers requested");
                                let resp = build_header_response(vec![], None);
                                if let Ok(encoded) = encode_response(&resp) {
                                    let _ = transport.send_light_sync_response(channel, encoded);
                                }
                            }
                            Ok(aztibase_network::LightSyncMessage::RequestProof { state_key, at_round, .. }) => {
                                tracing::debug!(peer = %peer, round = at_round, "Light sync: proof requested");
                                let resp = aztibase_network::build_proof_response(state_key, vec![], at_round);
                                if let Ok(encoded) = encode_response(&resp) {
                                    let _ = transport.send_light_sync_response(channel, encoded);
                                }
                            }
                            Ok(_) => {
                                tracing::warn!(peer = %peer, "Light sync: unexpected request type");
                            }
                            Err(e) => {
                                tracing::warn!(peer = %peer, error = %e, "Light sync: failed to decode request");
                            }
                        }
                    }
                    NetworkEvent::LightSyncResponse { peer, response, .. } => {
                        tracing::debug!(peer = %peer, bytes = response.0.len(), "Light sync: response received");
                    }
                    NetworkEvent::LightSyncOutboundFailure { peer, error, .. } => {
                        tracing::warn!(peer = %peer, error = ?error, "Light sync: outbound failure");
                    }
                    NetworkEvent::BlockSyncRequest { peer, request, channel } => {
                        match aztibase_network::block_sync::decode_request(&request) {
                            Ok(aztibase_network::BlockSyncMessage::RequestBatches { from_index, count, .. }) => {
                                tracing::info!(peer = %peer, from = from_index, count, "Block sync: batches requested");
                                let batches: Vec<SyncBatch> = batch_history
                                    .iter()
                                    .filter(|b| b.index >= from_index && b.index < from_index + count)
                                    .cloned()
                                    .collect();
                                let batches = if batches.len() < count as usize {
                                    batch_archive.get_range(from_index, count).unwrap_or(batches)
                                } else {
                                    batches
                                };
                                let resp = build_batch_response(batches, batch_index);
                                if let Ok(encoded) = aztibase_network::block_sync::encode_response(&resp) {
                                    let _ = transport.send_block_sync_response(channel, encoded);
                                }
                            }
                            Ok(_) => {
                                tracing::warn!(peer = %peer, "Block sync: unexpected request type");
                            }
                            Err(e) => {
                                tracing::warn!(peer = %peer, error = %e, "Block sync: failed to decode request");
                            }
                        }
                    }
                    NetworkEvent::BlockSyncResponse { peer, response, .. } => {
                        catchup_pending = false;
                        match aztibase_network::block_sync::decode_response(&response) {
                            Ok(aztibase_network::BlockSyncMessage::ResponseBatches { batches, tip_index, .. }) => {
                                tracing::info!(
                                    peer = %peer,
                                    received = batches.len(),
                                    tip = tip_index,
                                    "Block sync: catch-up response"
                                );
                                let applied = sync_proto.apply_response(&batches, tip_index);
                                for b in &batches {
                                    let committed = CommittedBatch {
                                        anchor_hash: b.anchor_hash,
                                        vertex_order: vec![],
                                        transactions: b.transactions.clone(),
                                    };
                                    if pipeline_tx.send(committed).await.is_err() {
                                        tracing::error!("Pipeline channel closed during catch-up");
                                        break;
                                    }
                                }
                                if applied > 0 {
                                    let behind = sync_proto.batches_behind();
                                    let synced = sync_proto.last_synced_index();
                                    let tip = sync_proto.tip_index();
                                    let pct = if tip > 0 { (synced as f64 / tip as f64 * 100.0) as u64 } else { 100 };
                                    tracing::info!(
                                        applied,
                                        synced,
                                        tip,
                                        behind,
                                        progress = %format!("{pct}%"),
                                        "Catch-up progress"
                                    );
                                }
                            }
                            Ok(_) => {}
                            Err(e) => {
                                tracing::warn!(peer = %peer, error = %e, "Block sync: bad response");
                            }
                        }
                    }
                    NetworkEvent::BlockSyncOutboundFailure { peer, error, .. } => {
                        catchup_pending = false;
                        tracing::warn!(peer = %peer, error = ?error, "Block sync: outbound failure");
                    }
                }
            }
            output = output_rx.recv() => {
                match output {
                    Some(ConsensusOutput::BroadcastVertex(data)) => {
                        if let Err(e) = transport.publish(&topic_consensus, data) {
                            tracing::debug!(error = %e, "Vertex publish failed (no subscribers yet)");
                        }
                        let snap = consensus_metrics.snapshot();
                        node_metrics.update_consensus(
                            snap.vertices_proposed,
                            snap.vertices_received,
                            snap.commits,
                            snap.rounds_advanced,
                            snap.equivocations,
                            snap.last_commit_latency_us,
                        );
                    }
                    Some(ConsensusOutput::EquivocationDetected {
                        author,
                        round,
                        existing_hash,
                        duplicate_hash,
                    }) => {
                        tracing::warn!(
                            author = %format!("{:02x}{:02x}{:02x}{:02x}",
                                author[0], author[1], author[2], author[3]),
                            round,
                            "Equivocation detected — sending slash event to pipeline"
                        );
                        let event = pipeline::SlashEvent {
                            validator_id: author,
                            offense: aztibase_execution::OffenseType::Equivocation,
                            round,
                            existing_hash: Some(existing_hash),
                            duplicate_hash: Some(duplicate_hash),
                        };
                        let _ = slash_tx.send(event).await;
                    }
                    Some(ConsensusOutput::BatchCommitted(batch)) => {
                        let snap = consensus_metrics.snapshot();
                        node_metrics.update_consensus(
                            snap.vertices_proposed,
                            snap.vertices_received,
                            snap.commits,
                            snap.rounds_advanced,
                            snap.equivocations,
                            snap.last_commit_latency_us,
                        );
                        if node_is_validator {
                            tracing::info!(
                                anchor = %format!("{:02x}{:02x}{:02x}{:02x}",
                                    batch.anchor_hash[0], batch.anchor_hash[1],
                                    batch.anchor_hash[2], batch.anchor_hash[3]),
                                txs = batch.transactions.len(),
                                "Batch committed — forwarding to execution"
                            );
                            let announce = CommittedBatchAnnounce {
                                index: batch_index + 1,
                                anchor_hash: batch.anchor_hash,
                                state_root: [0u8; 32],
                                transactions: batch.transactions.clone(),
                            };
                            if let Ok(encoded) = aztibase_network::encode_batch_announce(&announce) {
                                let _ = transport.publish(&topic_committed_batches, encoded);
                            }
                            pending_batch_txs.insert(batch.anchor_hash, batch.transactions.clone());
                            if pipeline_tx.send(batch).await.is_err() {
                                tracing::error!(
                                    "Execution pipeline channel closed — halting node"
                                );
                                break;
                            }
                        }
                    }
                    None => {
                        tracing::info!("Consensus output channel closed");
                        break;
                    }
                }
            }
            Some(raw_tx) = mempool_rx.recv() => {
                let state_guard = shared_state.read().await;
                let min_gp = shared_base_fee.load(std::sync::atomic::Ordering::Relaxed);
                if mempool.insert_checked(raw_tx.clone(), |addr| state_guard.nonce(addr), min_gp) {
                    drop(state_guard);
                    tracing::info!(
                        tx_len = raw_tx.len(),
                        mempool_size = mempool.len(),
                        "Transaction accepted into mempool — forwarding to consensus"
                    );
                    if let Err(e) = transport.publish(&topic_transactions, raw_tx.clone()) {
                        tracing::debug!(error = %e, "Failed to gossip transaction (no subscribers yet)");
                    }
                    let _ = consensus_tx.send(ConsensusInput::Transaction(raw_tx)).await;
                    node_metrics.set_mempool_size(mempool.len() as u64);
                } else {
                    drop(state_guard);
                    tracing::warn!(
                        tx_len = raw_tx.len(),
                        "Transaction rejected by mempool (nonce/gas/decode failure)"
                    );
                }
            }
            Some(result) = result_rx.recv() => {
                batch_index += 1;
                let announce = StateRootAnnounce {
                    anchor_hash: result.batch_anchor,
                    state_root: result.state_root,
                    batch_index,
                    validator: identity,
                };
                if let Ok(data) = postcard::to_allocvec(&announce)
                    && let Err(e) = transport.publish(&topic_state_sync, data)
                {
                    tracing::debug!(error = %e, "Failed to publish state root");
                }

                // Store in batch history for block sync requests
                let raw_txs = pending_batch_txs
                    .remove(&result.batch_anchor)
                    .unwrap_or_default();
                let sync_batch = SyncBatch {
                    index: batch_index,
                    anchor_hash: result.batch_anchor,
                    state_root: result.state_root,
                    transactions: raw_txs,
                };
                if let Err(e) = batch_archive.store_batch(&sync_batch) {
                    tracing::warn!(error = %e, "Failed to persist batch to archive");
                }
                batch_history.push_back(sync_batch);
                if batch_history.len() > MAX_BATCH_HISTORY {
                    batch_history.pop_front();
                }

                // Publish events for WebSocket subscribers
                event_bus.publish_new_head(serde_json::json!({
                    "round": batch_index,
                    "anchor_hash": format!("0x{}", hex::encode(result.batch_anchor)),
                    "state_root": format!("0x{}", hex::encode(result.state_root)),
                }));

                // Update Prometheus metrics
                let snap = consensus_metrics.snapshot();
                let base_fee_val = shared_base_fee.load(std::sync::atomic::Ordering::Relaxed);
                node_metrics.update_consensus(
                    snap.vertices_proposed,
                    snap.vertices_received,
                    snap.commits,
                    snap.rounds_advanced,
                    snap.equivocations,
                    snap.last_commit_latency_us,
                );
                node_metrics.update_execution(batch_index, base_fee_val);
                node_metrics.inc_txs_processed(result.receipts.len() as u64);

                {
                    let staking = shared_staking.read().await;
                    let validators = staking.active_validators();
                    let total: u128 = validators.iter().map(|v| v.effective_stake()).sum();
                    node_metrics.update_staking(validators.len() as u64, total);
                }
            }
            _ = catchup_ticker.tick() => {
                if !node_is_validator {
                    let needs = sync_proto.needs_sync();
                    let has_peers = !connected_peers.is_empty();
                    tracing::debug!(
                        needs_sync = needs,
                        catchup_pending,
                        peers = connected_peers.len(),
                        tip = sync_proto.tip_index(),
                        synced = sync_proto.last_synced_index(),
                        batch_index,
                        "Catch-up ticker"
                    );
                    if needs
                        && !catchup_pending
                        && has_peers
                        && let Some(req_msg) = sync_proto.next_request()
                    {
                        let peer_idx = (batch_index as usize) % connected_peers.len();
                        let peer = connected_peers[peer_idx];
                        if let Ok(encoded) = aztibase_network::block_sync::encode_request(&req_msg) {
                            tracing::info!(
                                peer = %peer,
                                from = sync_proto.last_synced_index() + 1,
                                behind = sync_proto.batches_behind(),
                                "Sending catch-up request"
                            );
                            transport.send_block_sync_request(&peer, encoded);
                            catchup_pending = true;
                        }
                    }
                }
            }
            _ = shutdown.notified() => {
                break;
            }
        }
    }

    drop(consensus_tx);
    drop(pipeline_tx);
    let _ = consensus_handle.await;
    let _ = pipeline_handle.await;

    if let Err(e) = aztibase_execution::clear_sentinel(&shutdown_store) {
        tracing::warn!(error = %e, "Failed to clear shutdown sentinel");
    } else {
        tracing::info!("Clean shutdown — sentinel cleared");
    }

    tracing::info!("Aztibase node shut down");
    Ok(())
}

async fn run_light_node(config: &NodeConfig) -> Result<()> {
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    use aztibase_network::{
        LightSyncProtocol, PeerId, SyncHeader, decode_response, encode_request, encode_response,
        verify_header_chain,
    };
    use aztibase_storage::{LightFinalityCert, LightHeader, LightStore};

    const SYNC_INTERVAL: Duration = Duration::from_secs(2);
    const MAX_PEER_FAILURES: u32 = 3;

    tracing::info!("Starting Aztibase LIGHT node");
    tracing::info!(data_dir = %config.data_dir.display());

    std::fs::create_dir_all(&config.data_dir)
        .with_context(|| format!("Failed to create data dir: {}", config.data_dir.display()))?;

    let light_db_path = config.data_dir.join("light.redb");
    let light_store =
        LightStore::open(&light_db_path).context("Failed to open light client database")?;

    let last_round = light_store.latest_header_round()?.unwrap_or(0);
    let mut sync_proto = LightSyncProtocol::new(last_round);

    tracing::info!(
        last_synced_round = last_round,
        db_path = %light_db_path.display(),
        "Light node initialized (header sync only, no execution)"
    );

    let light_rep_path = config.data_dir.join("peer_reputation.redb");
    let light_rep_store = Arc::new(
        PeerReputationStore::open(&light_rep_path)
            .context("Failed to open peer reputation store")?,
    );
    let light_peer_path = config.data_dir.join("peer_store.redb");
    let light_peer_store =
        Arc::new(PeerStore::open(&light_peer_path).context("Failed to open peer store")?);
    let light_boot_addrs: Vec<aztibase_network::Multiaddr> = config
        .network
        .boot_nodes
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();
    let light_genesis_hash = config
        .genesis_path
        .as_ref()
        .and_then(|p| genesis::load_genesis(p).ok())
        .and_then(|g| genesis::genesis_hash(&g).ok());
    let transport_config = TransportConfig {
        idle_timeout_secs: config.network.idle_timeout_secs,
        reputation_store: Some(light_rep_store),
        peer_store: Some(light_peer_store),
        relay_servers: light_boot_addrs,
        enable_webrtc: config.network.enable_webrtc,
        webrtc_listen_port: config.network.webrtc_listen_port,
        genesis_hash: light_genesis_hash,
        ..TransportConfig::default()
    };
    let mut transport =
        Libp2pTransport::new(transport_config).context("Failed to create P2P transport")?;
    transport.load_cached_peers(50);

    for addr_str in &config.network.listen_addresses {
        if let Ok(addr) = addr_str.parse() {
            let _ = transport.listen_on(addr);
        }
    }
    for boot in &config.network.boot_nodes {
        if let Ok(addr) = boot.parse() {
            let _ = transport.dial(addr);
        }
    }

    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = shutdown.clone();
    tokio::spawn(async move {
        if let Ok(()) = tokio::signal::ctrl_c().await {
            tracing::info!("Shutdown signal received");
            shutdown_clone.notify_one();
        }
    });

    struct PeerScore {
        failures: u32,
        last_response: Instant,
    }

    let mut peer_scores: HashMap<PeerId, PeerScore> = HashMap::new();
    let mut connected_peers: Vec<PeerId> = Vec::new();
    let mut sync_timer = tokio::time::interval(SYNC_INTERVAL);
    let mut pending_request_peer: Option<PeerId> = None;

    fn best_peer(peers: &[PeerId], scores: &HashMap<PeerId, PeerScore>) -> Option<PeerId> {
        peers
            .iter()
            .filter(|p| scores.get(p).is_none_or(|s| s.failures < MAX_PEER_FAILURES))
            .min_by_key(|p| {
                scores
                    .get(p)
                    .map_or(Duration::MAX, |s| s.last_response.elapsed())
            })
            .copied()
    }

    fn to_light_header(h: &SyncHeader) -> LightHeader {
        LightHeader {
            round: h.round,
            author: h.author,
            parents: h.parents.clone(),
            state_root: h.state_root,
            timestamp: h.timestamp,
        }
    }

    loop {
        tokio::select! {
            _ = shutdown.notified() => {
                tracing::info!(
                    last_round = sync_proto.last_synced_round(),
                    "Light node shut down"
                );
                return Ok(());
            }
            _ = sync_timer.tick() => {
                if !sync_proto.needs_sync() || connected_peers.is_empty() || pending_request_peer.is_some() {
                    continue;
                }
                if let Some(peer) = best_peer(&connected_peers, &peer_scores)
                    && let Some(req_msg) = sync_proto.next_request()
                    && let Ok(req) = encode_request(&req_msg)
                {
                    transport.send_light_sync_request(&peer, req);
                    pending_request_peer = Some(peer);
                    tracing::debug!(
                        peer = %peer,
                        from = sync_proto.last_synced_round() + 1,
                        "Sent header sync request"
                    );
                }
            }
            event = transport.next_event() => {
                match event {
                    NetworkEvent::Listening(addr) => {
                        tracing::info!(addr = %addr, "Listening");
                    }
                    NetworkEvent::PeerConnected(peer) => {
                        tracing::info!(peer = %peer, "Peer connected");
                        if !connected_peers.contains(&peer) {
                            connected_peers.push(peer);
                        }
                    }
                    NetworkEvent::PeerDisconnected(peer) => {
                        tracing::debug!(peer = %peer, "Peer disconnected");
                        connected_peers.retain(|p| *p != peer);
                        if pending_request_peer == Some(peer) {
                            pending_request_peer = None;
                        }
                    }
                    NetworkEvent::LightSyncResponse { peer, response, .. } => {
                        pending_request_peer = None;
                        match decode_response(&response) {
                            Ok(aztibase_network::LightSyncMessage::ResponseHeaders {
                                headers,
                                finality_cert,
                                ..
                            }) => {
                                if headers.is_empty() {
                                    tracing::debug!(peer = %peer, "Empty response (no new headers)");
                                    peer_scores.entry(peer).or_insert(PeerScore {
                                        failures: 0,
                                        last_response: Instant::now(),
                                    }).last_response = Instant::now();
                                    continue;
                                }
                                let expected_start = sync_proto.last_synced_round() + 1;
                                if let Some(ref cert) = finality_cert {
                                    let sync_cert = aztibase_network::SyncFinalityCert {
                                        anchor_round: cert.anchor_round,
                                        batch_hash: cert.batch_hash,
                                        state_root: cert.state_root,
                                        aggregate_signature: cert.aggregate_signature.clone(),
                                        signer_bitmap: cert.signer_bitmap.clone(),
                                    };
                                    if let Err(e) = verify_header_chain(&headers, &sync_cert, expected_start) {
                                        tracing::warn!(peer = %peer, error = %e, "Invalid header chain");
                                        peer_scores.entry(peer)
                                            .and_modify(|s| s.failures += 1)
                                            .or_insert(PeerScore { failures: 1, last_response: Instant::now() });
                                        continue;
                                    }
                                }
                                let applied = sync_proto.apply_response(&headers);
                                if applied > 0 {
                                    let light_headers: Vec<LightHeader> = headers[..applied as usize]
                                        .iter()
                                        .map(to_light_header)
                                        .collect();
                                    if let Err(e) = light_store.store_headers_batch(&light_headers) {
                                        tracing::warn!(error = %e, "Failed to persist headers");
                                    }
                                    if let Some(ref cert) = finality_cert {
                                        let lc = LightFinalityCert {
                                            anchor_round: cert.anchor_round,
                                            batch_hash: cert.batch_hash,
                                            state_root: cert.state_root,
                                            aggregate_signature: cert.aggregate_signature.clone(),
                                            signer_bitmap: cert.signer_bitmap.clone(),
                                        };
                                        let _ = light_store.store_finality_cert(&lc);
                                    }
                                    tracing::info!(
                                        applied,
                                        synced_round = sync_proto.last_synced_round(),
                                        "Headers synced"
                                    );
                                }
                                peer_scores.entry(peer).or_insert(PeerScore {
                                    failures: 0,
                                    last_response: Instant::now(),
                                }).last_response = Instant::now();
                            }
                            Ok(_) => {
                                tracing::warn!(peer = %peer, "Unexpected response type");
                            }
                            Err(e) => {
                                tracing::warn!(peer = %peer, error = %e, "Failed to decode response");
                                peer_scores.entry(peer)
                                    .and_modify(|s| s.failures += 1)
                                    .or_insert(PeerScore { failures: 1, last_response: Instant::now() });
                            }
                        }
                    }
                    NetworkEvent::LightSyncOutboundFailure { peer, error, .. } => {
                        tracing::warn!(peer = %peer, error = ?error, "Light sync outbound failure");
                        pending_request_peer = None;
                        peer_scores.entry(peer)
                            .and_modify(|s| s.failures += 1)
                            .or_insert(PeerScore { failures: 1, last_response: Instant::now() });
                    }
                    NetworkEvent::LightSyncRequest { channel, .. } => {
                        let empty = build_header_response(vec![], None);
                        if let Ok(encoded) = encode_response(&empty) {
                            let _ = transport.send_light_sync_response(channel, encoded);
                        }
                    }
                    NetworkEvent::BlockSyncRequest { channel, .. } => {
                        let resp = build_batch_response(vec![], 0);
                        if let Ok(encoded) = aztibase_network::block_sync::encode_response(&resp) {
                            let _ = transport.send_block_sync_response(channel, encoded);
                        }
                    }
                    NetworkEvent::BlockSyncResponse { .. }
                    | NetworkEvent::BlockSyncOutboundFailure { .. } => {}
                    NetworkEvent::Message { .. } => {}
                }
            }
        }
    }
}

fn init_logging(level: &str) -> Result<()> {
    let directive = format!("aztibase={level}");
    let filter = EnvFilter::from_default_env()
        .add_directive(directive.parse().context("Invalid log level")?);
    tracing_subscriber::fmt().with_env_filter(filter).init();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_valid() {
        let config = NodeConfig::default();
        assert!(!config.network.listen_addresses.is_empty());
        assert!(config.rpc.enabled);
        assert_eq!(config.log.level, "info");
    }

    #[test]
    fn cli_overrides_data_dir() {
        let cli = Cli {
            command: None,
            config: None,
            data_dir: Some(PathBuf::from("/tmp/test")),
            listen: vec![],
            rpc_addr: None,
            log_level: None,
            validator_key: None,
            validator_index: 1,
            validator_count: 1,
            genesis: None,
            metrics: false,
            light: false,
            webrtc: false,
            archive: false,
            checkpoint: None,
            epoch_length: None,
            testnet: false,
            mainnet: false,
            boot_node: vec![],
            snapshot: None,
        };
        let config = cli.apply_overrides(NodeConfig::default());
        assert_eq!(config.data_dir, PathBuf::from("/tmp/test"));
    }

    #[test]
    fn cli_overrides_listen_addresses() {
        let cli = Cli {
            command: None,
            config: None,
            data_dir: None,
            listen: vec!["/ip4/127.0.0.1/tcp/9999".into()],
            rpc_addr: None,
            log_level: None,
            validator_key: None,
            validator_index: 1,
            validator_count: 1,
            genesis: None,
            metrics: false,
            light: false,
            webrtc: false,
            archive: false,
            checkpoint: None,
            epoch_length: None,
            testnet: false,
            mainnet: false,
            boot_node: vec![],
            snapshot: None,
        };
        let config = cli.apply_overrides(NodeConfig::default());
        assert_eq!(config.network.listen_addresses.len(), 1);
        assert_eq!(
            config.network.listen_addresses[0],
            "/ip4/127.0.0.1/tcp/9999"
        );
    }

    #[test]
    fn config_roundtrip_toml() {
        let config = NodeConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: NodeConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.rpc.listen_addr, config.rpc.listen_addr);
        assert_eq!(
            parsed.network.idle_timeout_secs,
            config.network.idle_timeout_secs
        );
    }

    #[test]
    fn storage_path_is_under_data_dir() {
        let mut config = NodeConfig::default();
        config.data_dir = PathBuf::from("/var/aztibase");
        assert_eq!(config.storage_path(), PathBuf::from("/var/aztibase/db"));
    }

    #[tokio::test]
    async fn node_opens_storage() {
        let dir = std::env::temp_dir().join(format!("aztibase_node_test_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("db");
        let store = StateStore::open(db_path.to_str().unwrap());
        assert!(store.is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn validator_key_matches_genesis() {
        let dir = std::env::temp_dir().join(format!("aztibase_vkey_test_{}", std::process::id()));
        let generated = genesis::generate_genesis(3, 1, 1000);
        genesis::write_genesis(&generated, &dir).unwrap();

        // Load the first validator's key file
        let (hex_addr, _, _) = &generated.validator_keys[0];
        let key_path = dir.join("keys").join(format!("{hex_addr}.json"));
        let (_, loaded_addr) = genesis::load_keyfile(&key_path).unwrap();

        // Build validator set from genesis
        let mut vs = ValidatorSet::new();
        for entry in &generated.config.validators {
            if let Some(addr) = genesis::hex_decode(&entry.address)
                .and_then(|b| <[u8; 32]>::try_from(b.as_slice()).ok())
            {
                vs.add(addr, entry.stake);
            }
        }

        // Loaded key address must be in the validator set
        assert!(vs.contains(&loaded_addr));
        // 20M validator pool / 3 validators = 6,666,666 each
        assert_eq!(vs.get(&loaded_addr), Some(6_666_666));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn config_override_precedence() {
        let mut config = NodeConfig::default();
        config.genesis_path = Some(PathBuf::from("/config/genesis.toml"));
        config.validator_key = Some(PathBuf::from("/config/key.json"));

        let cli = Cli {
            command: None,
            config: None,
            data_dir: None,
            listen: vec![],
            rpc_addr: None,
            log_level: None,
            validator_key: Some(PathBuf::from("/cli/key.json")),
            validator_index: 1,
            validator_count: 1,
            genesis: Some(PathBuf::from("/cli/genesis.toml")),
            metrics: false,
            light: false,
            webrtc: false,
            archive: false,
            checkpoint: None,
            epoch_length: None,
            testnet: false,
            mainnet: false,
            boot_node: vec![],
            snapshot: None,
        };
        let result = cli.apply_overrides(config);

        // CLI flags override config file values
        assert_eq!(
            result.genesis_path,
            Some(PathBuf::from("/cli/genesis.toml"))
        );
        assert_eq!(result.validator_key, Some(PathBuf::from("/cli/key.json")));
    }

    #[test]
    fn validators_loaded_from_genesis() {
        let generated = genesis::generate_genesis(3, 1, 1000);
        let cfg = &generated.config;
        let mut vs = ValidatorSet::new();
        for entry in &cfg.validators {
            if let Some(addr) = genesis::hex_decode(&entry.address)
                .and_then(|b| <[u8; 32]>::try_from(b.as_slice()).ok())
            {
                vs.add(addr, entry.stake);
            }
        }
        assert_eq!(vs.len(), 3);
        // 20M validator pool / 3 validators = 6,666,666 each × 3 = 19,999,998
        assert_eq!(vs.total_stake(), 19_999_998);

        // Verify each genesis validator is in the set with correct stake
        for entry in &cfg.validators {
            let addr_bytes = genesis::hex_decode(&entry.address).unwrap();
            let addr: [u8; 32] = addr_bytes.as_slice().try_into().unwrap();
            assert_eq!(vs.get(&addr), Some(entry.stake));
        }
    }

    #[test]
    fn light_sync_protocol_advances_on_valid_response() {
        use aztibase_network::{LightSyncProtocol, SyncHeader};

        let mut proto = LightSyncProtocol::new(10);
        proto.set_target_round(20);
        assert!(proto.needs_sync());

        let headers: Vec<SyncHeader> = (11..=20)
            .map(|r| SyncHeader {
                round: r,
                author: [r as u8; 32],
                parents: vec![[0u8; 32]],
                state_root: [r as u8; 32],
                timestamp: 1000 + r,
            })
            .collect();
        let applied = proto.apply_response(&headers);
        assert_eq!(applied, 10);
        assert_eq!(proto.last_synced_round(), 20);
        assert!(!proto.needs_sync());
    }

    #[test]
    fn light_sync_rejects_gap_in_headers() {
        use aztibase_network::{LightSyncProtocol, SyncHeader};

        let mut proto = LightSyncProtocol::new(5);
        proto.set_target_round(10);

        let mut headers: Vec<SyncHeader> = (6..=10)
            .map(|r| SyncHeader {
                round: r,
                author: [r as u8; 32],
                parents: vec![[0u8; 32]],
                state_root: [r as u8; 32],
                timestamp: 1000 + r,
            })
            .collect();
        headers[2].round = 99; // gap

        let applied = proto.apply_response(&headers);
        assert_eq!(applied, 2); // only rounds 6, 7 accepted before gap
        assert_eq!(proto.last_synced_round(), 7);
    }

    #[test]
    fn light_store_persists_synced_headers() {
        use aztibase_storage::{LightHeader, LightStore};

        let dir = std::env::temp_dir().join(format!("aztibase_light_sync_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let store = LightStore::open(&dir.join("light.redb")).unwrap();

        let headers: Vec<LightHeader> = (1..=5)
            .map(|r| LightHeader {
                round: r,
                author: [r as u8; 32],
                parents: vec![[0u8; 32]],
                state_root: [r as u8; 32],
                timestamp: 1000 + r,
            })
            .collect();
        store.store_headers_batch(&headers).unwrap();

        assert_eq!(store.latest_header_round().unwrap(), Some(5));
        assert_eq!(store.header_count().unwrap(), 5);

        let h3 = store.get_header(3).unwrap().unwrap();
        assert_eq!(h3.round, 3);
        assert_eq!(h3.author, [3u8; 32]);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
