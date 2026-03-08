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

/// Validate a STUN/TURN server URI. Accepted formats:
/// - `stun:<host>:<port>`
/// - `stun:<host>` (port optional)
/// - `turn:<host>:<port>`
fn validate_stun_uri(uri: &str) -> Result<()> {
    let (scheme, rest) = uri
        .split_once(':')
        .ok_or_else(|| anyhow::anyhow!("invalid STUN URI: missing scheme in '{uri}'"))?;

    if scheme != "stun" && scheme != "turn" {
        anyhow::bail!("invalid STUN URI scheme '{scheme}': expected 'stun' or 'turn'");
    }

    if rest.is_empty() {
        anyhow::bail!("invalid STUN URI: empty host in '{uri}'");
    }

    let host_port = rest.trim_start_matches('/');
    if host_port.is_empty() {
        anyhow::bail!("invalid STUN URI: empty host in '{uri}'");
    }

    if let Some((host, port_str)) = host_port.rsplit_once(':') {
        if host.is_empty() {
            anyhow::bail!("invalid STUN URI: empty host in '{uri}'");
        }
        if port_str.parse::<u16>().is_err() {
            anyhow::bail!("invalid STUN URI: bad port '{port_str}' in '{uri}'");
        }
    }

    Ok(())
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
        for uri in &config.stun_servers {
            validate_stun_uri(uri).context("invalid STUN server configuration")?;
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

    #[test]
    fn webrtc_rejects_malformed_stun_uri() {
        let config = WebRtcConfig {
            stun_servers: vec!["not-a-valid-uri".into()],
            listen_port: 30334,
        };
        let err = WebRtcTransport::new(config).unwrap_err();
        assert!(err.to_string().contains("invalid STUN"));
    }

    #[test]
    fn webrtc_rejects_bad_stun_scheme() {
        let config = WebRtcConfig {
            stun_servers: vec!["http:example.com:3478".into()],
            listen_port: 30334,
        };
        assert!(WebRtcTransport::new(config).is_err());
    }

    #[test]
    fn webrtc_accepts_turn_uri() {
        let config = WebRtcConfig {
            stun_servers: vec!["turn:relay.example.com:3478".into()],
            listen_port: 30334,
        };
        assert!(WebRtcTransport::new(config).is_ok());
    }
}
