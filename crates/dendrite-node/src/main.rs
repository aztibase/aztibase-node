use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "dendrite", about = "Dendrite Network Node")]
struct Cli {
    /// Path to the data directory
    #[arg(long, default_value = "./data")]
    data_dir: String,

    /// Listen address for P2P
    #[arg(long, default_value = "/ip4/0.0.0.0/tcp/30333")]
    listen_addr: String,

    /// RPC listen address
    #[arg(long, default_value = "127.0.0.1:9944")]
    rpc_addr: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("dendrite=info".parse()?))
        .init();

    let cli = Cli::parse();

    tracing::info!("Starting Dendrite node");
    tracing::info!("  Data dir: {}", cli.data_dir);
    tracing::info!("  P2P listen: {}", cli.listen_addr);
    tracing::info!("  RPC listen: {}", cli.rpc_addr);

    // TODO: Initialize subsystems:
    // 1. Storage (dendrite-storage)
    // 2. Network (dendrite-network)
    // 3. Consensus (dendrite-consensus)
    // 4. Execution (dendrite-execution)
    // 5. Runtime (dendrite-runtime)
    // 6. RPC server (dendrite-rpc)

    tracing::info!("Dendrite node initialized (skeleton)");

    // Keep running until Ctrl+C
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down...");

    Ok(())
}
