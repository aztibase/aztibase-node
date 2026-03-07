mod config;
mod genesis;
#[cfg(test)]
mod integration;
mod mempool;
mod pipeline;
mod sync;
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
    Libp2pTransport, NetworkEvent, TOPIC_CONSENSUS, TOPIC_STATE_SYNC, TOPIC_TRANSACTIONS,
    TransportConfig,
};
use aztibase_rpc::RpcServer;
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

    /// Validator index for testing (1-255). Each node needs a unique index.
    #[arg(long, default_value = "1")]
    validator_index: u8,

    /// Total number of validators in the test network
    #[arg(long, default_value = "1")]
    validator_count: u8,

    /// Path to genesis.toml for initial state
    #[arg(long)]
    genesis: Option<PathBuf>,
}

#[derive(clap::Subcommand, Debug)]
enum Command {
    /// Generate a new genesis configuration
    Genesis {
        /// Number of validators
        #[arg(long, default_value = "4")]
        validators: usize,
        /// Number of pre-funded accounts
        #[arg(long, default_value = "2")]
        funded: usize,
        /// Output directory
        #[arg(long, default_value = "genesis")]
        output: PathBuf,
    },
    /// Wallet key management and transaction signing
    Wallet {
        #[command(subcommand)]
        action: WalletAction,
    },
}

#[derive(clap::Subcommand, Debug)]
enum WalletAction {
    /// Generate a new Ed25519 keypair
    Generate {
        /// Output key file path
        #[arg(long, default_value = "wallet.json")]
        output: PathBuf,
    },
    /// Show address and public key from a key file
    Show {
        /// Path to key file
        keyfile: PathBuf,
    },
    /// Sign and encode a transfer transaction
    Transfer {
        /// Path to sender key file
        #[arg(long)]
        from: PathBuf,
        /// Recipient address (hex)
        #[arg(long)]
        to: String,
        /// Transfer amount
        #[arg(long)]
        value: u64,
        /// Sender nonce
        #[arg(long)]
        nonce: u64,
        /// Gas price
        #[arg(long, default_value = "1")]
        gas_price: u64,
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
        cfg
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Genesis {
            validators,
            funded,
            output,
        }) => {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            let generated = genesis::generate_genesis(validators, funded, timestamp);
            genesis::write_genesis(&generated, &output)?;
            println!(
                "Genesis written to {} ({} validators, {} funded accounts)",
                output.display(),
                validators,
                funded
            );
            return Ok(());
        }
        Some(Command::Wallet { action }) => {
            match action {
                WalletAction::Generate { output } => {
                    wallet::generate_key(&output)?;
                }
                WalletAction::Show { keyfile } => {
                    wallet::show_key(&keyfile)?;
                }
                WalletAction::Transfer {
                    from,
                    to,
                    value,
                    nonce,
                    gas_price,
                } => {
                    let envelope = wallet::sign_transfer(&from, &to, value, nonce, gas_price)?;
                    let hex = genesis::hex_encode(&envelope);
                    println!("{hex}");
                }
            }
            return Ok(());
        }
        None => {}
    }

    let config = NodeConfig::load_or_default(cli.config.as_deref())?;
    let config = cli.apply_overrides(config);

    init_logging(&config.log.level)?;

    tracing::info!(validator = cli.validator_index, "Starting Aztibase node");
    tracing::info!(data_dir = %config.data_dir.display());

    // Storage
    std::fs::create_dir_all(&config.data_dir)
        .with_context(|| format!("Failed to create data dir: {}", config.data_dir.display()))?;
    let storage_path = config.storage_path();
    let storage_path_str = storage_path.to_str().context("Invalid storage path")?;
    let store = StateStore::open(storage_path_str).context("Failed to open storage")?;
    tracing::info!(path = %storage_path.display(), "Storage initialized");

    // Consensus
    let dag = DagStore::new(store).context("Failed to initialize DAG store")?;
    let identity = [cli.validator_index; 32];
    let mut validators = ValidatorSet::new();
    for i in 1..=cli.validator_count {
        validators.add([i; 32], 100);
    }

    let consensus_config = ConsensusConfig::default();
    let (consensus_tx, consensus_rx) = tokio::sync::mpsc::channel::<ConsensusInput>(256);
    let (output_tx, mut output_rx) = tokio::sync::mpsc::channel::<ConsensusOutput>(256);

    let mut engine = ConsensusEngine::new(
        consensus_config,
        identity,
        dag,
        validators,
        consensus_rx,
        output_tx,
    );
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

    let (pipeline_tx, pipeline_rx) = tokio::sync::mpsc::channel::<CommittedBatch>(256);
    let (result_tx, mut result_rx) = tokio::sync::mpsc::channel::<pipeline::PipelineResult>(256);
    let mut exec_pipeline =
        pipeline::ExecutionPipeline::with_storage(Arc::clone(&exec_store), pipeline_rx);
    exec_pipeline.set_result_sender(result_tx);
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

    if let Some(ref genesis_path) = cli.genesis {
        let gen_config = genesis::load_genesis(genesis_path)?;
        let shared = exec_pipeline.shared_state();
        let mut state_guard = shared.write().await;
        if state_guard.account_count() == 0 {
            genesis::apply_genesis(&gen_config, &mut state_guard);
            tracing::info!(
                accounts = state_guard.account_count(),
                "Genesis state applied"
            );
        } else {
            tracing::info!("State already populated, skipping genesis");
        }
        drop(state_guard);
    }

    // RPC server
    let (mempool_tx, mut mempool_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(4096);
    let rpc_server = RpcServer::new(
        exec_pipeline.shared_state(),
        mempool_tx,
        exec_pipeline.shared_batch_count(),
        Some(exec_store),
        exec_pipeline.shared_base_fee(),
    );

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
    let transport_config = TransportConfig {
        idle_timeout_secs: config.network.idle_timeout_secs,
    };
    let mut transport =
        Libp2pTransport::new(transport_config).context("Failed to create network transport")?;
    tracing::info!(peer_id = %transport.local_peer_id(), "Network identity");

    for addr_str in &config.network.listen_addresses {
        let addr: aztibase_network::Multiaddr = addr_str
            .parse()
            .with_context(|| format!("Invalid listen address: {addr_str}"))?;
        transport
            .listen_on(addr)
            .with_context(|| format!("Failed to listen on {addr_str}"))?;
    }

    for boot_str in &config.network.boot_nodes {
        let addr: aztibase_network::Multiaddr = boot_str
            .parse()
            .with_context(|| format!("Invalid boot node address: {boot_str}"))?;
        if let Err(e) = transport.dial(addr) {
            tracing::warn!(addr = %boot_str, error = %e, "Failed to dial boot node");
        }
    }

    tracing::info!("Node started — press Ctrl+C to shut down");

    let shutdown = Arc::new(Notify::new());
    let shutdown_signal = shutdown.clone();

    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("Shutdown signal received");
        shutdown_signal.notify_waiters();
    });

    // Spawn consensus engine
    let consensus_handle = tokio::spawn(async move {
        if let Err(e) = engine.run().await {
            tracing::error!(error = %e, "Consensus engine failed");
        }
    });

    // Shared state for sync protocol + mempool gas price validation
    let shared_state = exec_pipeline.shared_state();
    let shared_base_fee = exec_pipeline.shared_base_fee();

    // Spawn execution pipeline
    let pipeline_handle = tokio::spawn(async move {
        exec_pipeline.run().await;
    });

    let mut batch_index: u64 = 0;
    let mut sync_assembler: Option<SnapshotAssembler> = None;
    let mut sync_bootstrapped = false;

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
                        tracing::info!(peer = %peer, "Peer connected");
                    }
                    NetworkEvent::PeerDisconnected(peer) => {
                        tracing::debug!(peer = %peer, "Peer disconnected");
                    }
                    NetworkEvent::Message { source, topic, data } => {
                        tracing::debug!(
                            source = %source,
                            topic = %topic,
                            bytes = data.len(),
                            "Received message"
                        );
                        if topic == TOPIC_CONSENSUS {
                            let _ = consensus_tx.send(ConsensusInput::ReceivedVertex(data)).await;
                        } else if topic == TOPIC_STATE_SYNC {
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
                                                            let _ = transport.publish(TOPIC_STATE_SYNC, encoded);
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
                            } else if let Ok(announce) = bincode::deserialize::<StateRootAnnounce>(&data) {
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
                                            let _ = transport.publish(TOPIC_STATE_SYNC, encoded);
                                            tracing::info!("Sent snapshot request to peers");
                                        }
                                    }
                                }
                            }
                        } else if topic == TOPIC_TRANSACTIONS {
                            let state_guard = shared_state.read().await;
                            let min_gp = shared_base_fee.load(std::sync::atomic::Ordering::Relaxed);
                            if mempool.insert_checked(data.clone(), |addr| state_guard.nonce(addr), min_gp) {
                                drop(state_guard);
                                let _ = consensus_tx.send(ConsensusInput::Transaction(data)).await;
                            }
                        }
                    }
                }
            }
            output = output_rx.recv() => {
                match output {
                    Some(ConsensusOutput::BroadcastVertex(data)) => {
                        if let Err(e) = transport.publish(TOPIC_CONSENSUS, data) {
                            tracing::debug!(error = %e, "Failed to publish vertex");
                        }
                    }
                    Some(ConsensusOutput::BatchCommitted(batch)) => {
                        tracing::info!(
                            anchor = %format!("{:02x}{:02x}{:02x}{:02x}",
                                batch.anchor_hash[0], batch.anchor_hash[1],
                                batch.anchor_hash[2], batch.anchor_hash[3]),
                            txs = batch.transactions.len(),
                            "Batch committed — forwarding to execution"
                        );
                        if pipeline_tx.send(batch).await.is_err() {
                            tracing::error!(
                                "Execution pipeline channel closed — halting node"
                            );
                            break;
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
                    let _ = consensus_tx.send(ConsensusInput::Transaction(raw_tx)).await;
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
                if let Ok(data) = bincode::serialize(&announce)
                    && let Err(e) = transport.publish(TOPIC_STATE_SYNC, data)
                {
                    tracing::debug!(error = %e, "Failed to publish state root");
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

    tracing::info!("Aztibase node shut down");
    Ok(())
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
            validator_index: 1,
            validator_count: 1,
            genesis: None,
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
            validator_index: 1,
            validator_count: 1,
            genesis: None,
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
}
