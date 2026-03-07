use anyhow::{Context, Result};
use libp2p::Multiaddr;

/// Configuration for the WebRTC transport layer.
#[derive(Clone, Debug)]
pub struct WebRtcConfig {
    pub stun_servers: Vec<String>,
    pub listen_port: u16,
}

impl Default for WebRtcConfig {
    fn default() -> Self {
        Self {
            stun_servers: vec![
                "stun:stun.l.google.com:19302".into(),
                "stun:stun1.l.google.com:19302".into(),
            ],
            listen_port: 30334,
        }
    }
}

/// WebRTC transport wrapper for browser-node connectivity.
/// Uses libp2p-webrtc under the hood with DTLS for encryption.
pub struct WebRtcTransport {
    config: WebRtcConfig,
    listen_addr: Option<Multiaddr>,
}

impl WebRtcTransport {
    pub fn new(config: WebRtcConfig) -> Result<Self> {
        if config.stun_servers.is_empty() {
            anyhow::bail!("at least one STUN server is required for WebRTC");
        }
        Ok(Self {
            config,
            listen_addr: None,
        })
    }

    pub fn stun_servers(&self) -> &[String] {
        &self.config.stun_servers
    }

    pub fn listen_port(&self) -> u16 {
        self.config.listen_port
    }

    /// Build the multiaddr for the WebRTC UDP listener.
    pub fn listen_multiaddr(&self) -> Result<Multiaddr> {
        let addr_str = format!("/ip4/0.0.0.0/udp/{}/webrtc-direct", self.config.listen_port);
        addr_str.parse().context("invalid WebRTC listen address")
    }

    /// Initialize the transport. In a full implementation this would
    /// configure the libp2p swarm with the WebRTC transport alongside QUIC.
    /// For now it validates the config and builds the listen address.
    pub fn initialize(&mut self) -> Result<Multiaddr> {
        let addr = self.listen_multiaddr()?;
        self.listen_addr = Some(addr.clone());
        Ok(addr)
    }

    pub fn is_initialized(&self) -> bool {
        self.listen_addr.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webrtc_transport_initializes() {
        let config = WebRtcConfig::default();
        let mut transport = WebRtcTransport::new(config).unwrap();
        assert!(!transport.is_initialized());

        let addr = transport.initialize().unwrap();
        assert!(transport.is_initialized());
        assert!(addr.to_string().contains("webrtc-direct"));
    }

    #[test]
    fn webrtc_rejects_empty_stun() {
        let config = WebRtcConfig {
            stun_servers: vec![],
            listen_port: 30334,
        };
        assert!(WebRtcTransport::new(config).is_err());
    }

    #[test]
    fn webrtc_custom_port() {
        let config = WebRtcConfig {
            listen_port: 40000,
            ..Default::default()
        };
        let transport = WebRtcTransport::new(config).unwrap();
        let addr = transport.listen_multiaddr().unwrap();
        assert!(addr.to_string().contains("40000"));
    }

    #[test]
    fn webrtc_stun_defaults() {
        let config = WebRtcConfig::default();
        let transport = WebRtcTransport::new(config).unwrap();
        assert_eq!(transport.stun_servers().len(), 2);
        assert!(transport.stun_servers()[0].contains("google.com"));
    }
}
