use anyhow::Result;

/// Abstraction over the P2P transport layer.
/// Per the architect's stack ruling, this trait allows swapping libp2p
/// for an alternative transport if needed.
#[allow(async_fn_in_trait)]
pub trait NetworkTransport: Send + Sync + 'static {
    /// Start listening for incoming connections.
    async fn start(&mut self) -> Result<()>;

    /// Stop the transport.
    async fn stop(&mut self) -> Result<()>;

    /// Broadcast a message to all connected peers.
    async fn broadcast(&self, topic: &str, data: Vec<u8>) -> Result<()>;
}
