use std::io;

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::StreamProtocol;
use libp2p::request_response;
use serde::{Deserialize, Serialize};

const LIGHT_SYNC_VERSION: u8 = 1;
pub const MAX_HEADERS_PER_REQUEST: u64 = 100;
const MAX_FRAME_SIZE: usize = 1_048_576; // 1 MB

pub const LIGHT_SYNC_PROTOCOL: StreamProtocol = StreamProtocol::new("/aztibase/light-sync/1");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightSyncRequest(pub Vec<u8>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightSyncResponse(pub Vec<u8>);

#[derive(Debug, Clone, Default)]
pub struct LightSyncCodec;

#[async_trait]
impl request_response::Codec for LightSyncCodec {
    type Protocol = StreamProtocol;
    type Request = LightSyncRequest;
    type Response = LightSyncResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_frame(io).await.map(LightSyncRequest)
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_frame(io).await.map(LightSyncResponse)
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_frame(io, &req.0).await
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_frame(io, &res.0).await
    }
}

async fn read_frame<T: AsyncRead + Unpin + Send>(io: &mut T) -> io::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    io.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > MAX_FRAME_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("frame too large: {len} > {MAX_FRAME_SIZE}"),
        ));
    }
    let mut buf = vec![0u8; len];
    io.read_exact(&mut buf).await?;
    Ok(buf)
}

async fn write_frame<T: AsyncWrite + Unpin + Send>(io: &mut T, data: &[u8]) -> io::Result<()> {
    if data.len() > MAX_FRAME_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "frame too large",
        ));
    }
    let len = (data.len() as u32).to_be_bytes();
    io.write_all(&len).await?;
    io.write_all(data).await?;
    io.flush().await?;
    Ok(())
}

pub fn encode_request(msg: &LightSyncMessage) -> Result<LightSyncRequest, String> {
    encode_light_sync(msg).map(LightSyncRequest)
}

pub fn decode_request(req: &LightSyncRequest) -> Result<LightSyncMessage, String> {
    decode_light_sync(&req.0)
}

pub fn encode_response(msg: &LightSyncMessage) -> Result<LightSyncResponse, String> {
    encode_light_sync(msg).map(LightSyncResponse)
}

pub fn decode_response(resp: &LightSyncResponse) -> Result<LightSyncMessage, String> {
    decode_light_sync(&resp.0)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum LightSyncMessage {
    RequestHeaders {
        version: u8,
        from_round: u64,
        count: u64,
    },
    ResponseHeaders {
        version: u8,
        headers: Vec<SyncHeader>,
        finality_cert: Option<SyncFinalityCert>,
    },
    RequestProof {
        version: u8,
        state_key: Vec<u8>,
        at_round: u64,
    },
    ResponseProof {
        version: u8,
        state_key: Vec<u8>,
        proof_data: Vec<u8>,
        at_round: u64,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncHeader {
    pub round: u64,
    pub author: [u8; 32],
    pub parents: Vec<[u8; 32]>,
    pub state_root: [u8; 32],
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncFinalityCert {
    pub anchor_round: u64,
    pub batch_hash: [u8; 32],
    pub state_root: [u8; 32],
    pub aggregate_signature: Vec<u8>,
    pub signer_bitmap: Vec<bool>,
}

pub fn encode_light_sync(msg: &LightSyncMessage) -> Result<Vec<u8>, String> {
    bincode::serialize(msg).map_err(|e| e.to_string())
}

pub fn decode_light_sync(data: &[u8]) -> Result<LightSyncMessage, String> {
    let msg: LightSyncMessage = bincode::deserialize(data).map_err(|e| e.to_string())?;
    let version = match &msg {
        LightSyncMessage::RequestHeaders { version, .. }
        | LightSyncMessage::ResponseHeaders { version, .. }
        | LightSyncMessage::RequestProof { version, .. }
        | LightSyncMessage::ResponseProof { version, .. } => *version,
    };
    if version != LIGHT_SYNC_VERSION {
        return Err(format!("unsupported light sync version: {version}"));
    }
    Ok(msg)
}

pub fn build_header_request(from_round: u64, count: u64) -> LightSyncMessage {
    LightSyncMessage::RequestHeaders {
        version: LIGHT_SYNC_VERSION,
        from_round,
        count: count.min(MAX_HEADERS_PER_REQUEST),
    }
}

pub fn build_header_response(
    headers: Vec<SyncHeader>,
    finality_cert: Option<SyncFinalityCert>,
) -> LightSyncMessage {
    LightSyncMessage::ResponseHeaders {
        version: LIGHT_SYNC_VERSION,
        headers,
        finality_cert,
    }
}

pub fn build_proof_request(state_key: Vec<u8>, at_round: u64) -> LightSyncMessage {
    LightSyncMessage::RequestProof {
        version: LIGHT_SYNC_VERSION,
        state_key,
        at_round,
    }
}

pub fn build_proof_response(
    state_key: Vec<u8>,
    proof_data: Vec<u8>,
    at_round: u64,
) -> LightSyncMessage {
    LightSyncMessage::ResponseProof {
        version: LIGHT_SYNC_VERSION,
        state_key,
        proof_data,
        at_round,
    }
}

/// Validates header responses from multiple peers. Accepts only if a
/// majority of responding peers return matching state roots for the
/// same round, defending against eclipse attacks.
pub struct MultiPeerValidator {
    min_peers: usize,
}

impl MultiPeerValidator {
    pub fn new(min_peers: usize) -> Self {
        Self {
            min_peers: min_peers.max(1),
        }
    }

    /// Compare header responses from multiple peers. Returns `Ok(())` if
    /// a strict majority agree on the state root for the last header,
    /// or `Err` if no consensus is reached.
    pub fn validate_responses(
        &self,
        responses: &[(libp2p::PeerId, Vec<SyncHeader>)],
    ) -> Result<(), String> {
        let valid: Vec<_> = responses.iter().filter(|(_, h)| !h.is_empty()).collect();

        if valid.len() < self.min_peers {
            return Err(format!(
                "too few peer responses: {} < {}",
                valid.len(),
                self.min_peers
            ));
        }

        let mut root_counts: std::collections::HashMap<[u8; 32], usize> =
            std::collections::HashMap::new();
        for (_, headers) in &valid {
            if let Some(last) = headers.last() {
                *root_counts.entry(last.state_root).or_insert(0) += 1;
            }
        }

        let majority = valid.len() / 2 + 1;
        let best = root_counts.values().max().copied().unwrap_or(0);
        if best >= majority {
            Ok(())
        } else {
            Err(format!(
                "no majority consensus: best agreement {best}/{}, need {majority}",
                valid.len()
            ))
        }
    }
}

pub struct LightSyncProtocol {
    last_synced_round: u64,
    target_round: u64,
}

impl LightSyncProtocol {
    pub fn new(last_synced_round: u64) -> Self {
        Self {
            last_synced_round,
            target_round: last_synced_round,
        }
    }

    pub fn last_synced_round(&self) -> u64 {
        self.last_synced_round
    }

    pub fn target_round(&self) -> u64 {
        self.target_round
    }

    pub fn set_target_round(&mut self, target: u64) {
        self.target_round = target;
    }

    pub fn needs_sync(&self) -> bool {
        self.last_synced_round < self.target_round
    }

    pub fn next_request(&self) -> Option<LightSyncMessage> {
        if !self.needs_sync() {
            return None;
        }
        let remaining = self.target_round - self.last_synced_round;
        let count = remaining.min(MAX_HEADERS_PER_REQUEST);
        Some(build_header_request(self.last_synced_round + 1, count))
    }

    pub fn apply_response(&mut self, headers: &[SyncHeader]) -> u64 {
        let mut applied = 0u64;
        for h in headers {
            if h.round == self.last_synced_round + 1 {
                self.last_synced_round = h.round;
                applied += 1;
            }
        }
        applied
    }
}

pub fn verify_header_chain(
    headers: &[SyncHeader],
    cert: &SyncFinalityCert,
    expected_start: u64,
) -> Result<(), String> {
    if headers.is_empty() {
        return Err("empty header batch".into());
    }

    // Check sequential round numbers starting from expected_start
    for (i, h) in headers.iter().enumerate() {
        let expected_round = expected_start + i as u64;
        if h.round != expected_round {
            return Err(format!(
                "gap at index {i}: expected round {expected_round}, got {}",
                h.round
            ));
        }
    }

    // The cert must cover some round in the batch range
    let first_round = headers[0].round;
    let last_round = headers[headers.len() - 1].round;
    if cert.anchor_round < first_round || cert.anchor_round > last_round {
        return Err(format!(
            "finality cert anchor round {} not in batch range [{first_round}, {last_round}]",
            cert.anchor_round
        ));
    }

    // Verify quorum: at least 2/3 of signers must have signed
    let total_signers = cert.signer_bitmap.len();
    if total_signers == 0 {
        return Err("empty signer bitmap".into());
    }
    let signed = cert.signer_bitmap.iter().filter(|&&b| b).count();
    let required = (total_signers * 2).div_ceil(3);
    if signed < required {
        return Err(format!(
            "insufficient quorum: {signed}/{total_signers} signed, need {required}"
        ));
    }

    // Signature must not be empty
    if cert.aggregate_signature.is_empty() {
        return Err("empty aggregate signature".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_headers(start: u64, count: u64) -> Vec<SyncHeader> {
        (start..start + count)
            .map(|r| SyncHeader {
                round: r,
                author: [r as u8; 32],
                parents: vec![[0u8; 32]],
                state_root: [r as u8; 32],
                timestamp: 1000 + r,
            })
            .collect()
    }

    fn make_cert(anchor_round: u64, n_validators: usize, n_signed: usize) -> SyncFinalityCert {
        let mut bitmap = vec![false; n_validators];
        for i in 0..n_signed {
            bitmap[i] = true;
        }
        SyncFinalityCert {
            anchor_round,
            batch_hash: [anchor_round as u8; 32],
            state_root: [anchor_round as u8; 32],
            aggregate_signature: vec![0u8; 96],
            signer_bitmap: bitmap,
        }
    }

    #[test]
    fn message_serde_roundtrip() {
        let req = build_header_request(10, 50);
        let encoded = encode_light_sync(&req).unwrap();
        let decoded = decode_light_sync(&encoded).unwrap();
        match decoded {
            LightSyncMessage::RequestHeaders {
                from_round, count, ..
            } => {
                assert_eq!(from_round, 10);
                assert_eq!(count, 50);
            }
            _ => panic!("expected RequestHeaders"),
        }

        let resp = build_header_response(make_headers(10, 3), Some(make_cert(12, 4, 3)));
        let encoded = encode_light_sync(&resp).unwrap();
        let decoded = decode_light_sync(&encoded).unwrap();
        match decoded {
            LightSyncMessage::ResponseHeaders { headers, .. } => {
                assert_eq!(headers.len(), 3);
                assert_eq!(headers[0].round, 10);
            }
            _ => panic!("expected ResponseHeaders"),
        }
    }

    #[test]
    fn version_mismatch_rejected() {
        let bad = LightSyncMessage::RequestHeaders {
            version: 99,
            from_round: 1,
            count: 10,
        };
        let encoded = bincode::serialize(&bad).unwrap();
        assert!(decode_light_sync(&encoded).is_err());
    }

    #[test]
    fn request_caps_at_max() {
        let req = build_header_request(0, 500);
        match req {
            LightSyncMessage::RequestHeaders { count, .. } => {
                assert_eq!(count, MAX_HEADERS_PER_REQUEST);
            }
            _ => panic!("expected RequestHeaders"),
        }
    }

    #[test]
    fn valid_header_chain_passes() {
        let headers = make_headers(5, 10);
        let cert = make_cert(10, 4, 3);
        assert!(verify_header_chain(&headers, &cert, 5).is_ok());
    }

    #[test]
    fn gap_in_headers_rejected() {
        let mut headers = make_headers(5, 5);
        headers[2].round = 99; // introduce gap
        let cert = make_cert(7, 4, 3);
        let err = verify_header_chain(&headers, &cert, 5).unwrap_err();
        assert!(err.contains("gap"));
    }

    #[test]
    fn cert_outside_batch_rejected() {
        let headers = make_headers(5, 5);
        let cert = make_cert(100, 4, 3); // anchor way outside batch
        let err = verify_header_chain(&headers, &cert, 5).unwrap_err();
        assert!(err.contains("anchor round"));
    }

    #[test]
    fn insufficient_quorum_rejected() {
        let headers = make_headers(5, 5);
        let cert = make_cert(7, 4, 1); // 1/4 signed, need 3
        let err = verify_header_chain(&headers, &cert, 5).unwrap_err();
        assert!(err.contains("quorum"));
    }

    #[test]
    fn empty_signature_rejected() {
        let headers = make_headers(5, 3);
        let mut cert = make_cert(6, 4, 3);
        cert.aggregate_signature = vec![];
        let err = verify_header_chain(&headers, &cert, 5).unwrap_err();
        assert!(err.contains("signature"));
    }

    #[test]
    fn sync_protocol_state_machine() {
        let mut proto = LightSyncProtocol::new(0);
        assert!(!proto.needs_sync());

        proto.set_target_round(50);
        assert!(proto.needs_sync());
        assert_eq!(proto.last_synced_round(), 0);

        let req = proto.next_request().unwrap();
        match req {
            LightSyncMessage::RequestHeaders {
                from_round, count, ..
            } => {
                assert_eq!(from_round, 1);
                assert_eq!(count, 50);
            }
            _ => panic!("expected RequestHeaders"),
        }

        let headers = make_headers(1, 50);
        let applied = proto.apply_response(&headers);
        assert_eq!(applied, 50);
        assert_eq!(proto.last_synced_round(), 50);
        assert!(!proto.needs_sync());
        assert!(proto.next_request().is_none());
    }

    #[test]
    fn codec_encode_decode_request() {
        let msg = build_header_request(5, 20);
        let req = encode_request(&msg).unwrap();
        let decoded = decode_request(&req).unwrap();
        match decoded {
            LightSyncMessage::RequestHeaders {
                from_round, count, ..
            } => {
                assert_eq!(from_round, 5);
                assert_eq!(count, 20);
            }
            _ => panic!("expected RequestHeaders"),
        }
    }

    #[test]
    fn codec_encode_decode_response() {
        let headers = make_headers(1, 3);
        let cert = make_cert(2, 4, 3);
        let msg = build_header_response(headers, Some(cert));
        let resp = encode_response(&msg).unwrap();
        let decoded = decode_response(&resp).unwrap();
        match decoded {
            LightSyncMessage::ResponseHeaders {
                headers,
                finality_cert,
                ..
            } => {
                assert_eq!(headers.len(), 3);
                assert!(finality_cert.is_some());
            }
            _ => panic!("expected ResponseHeaders"),
        }
    }

    #[tokio::test]
    async fn frame_roundtrip() {
        let data = vec![1u8, 2, 3, 4, 5];
        let mut buf = Vec::new();
        write_frame(&mut buf, &data).await.unwrap();
        let mut cursor = futures::io::Cursor::new(buf);
        let result = read_frame(&mut cursor).await.unwrap();
        assert_eq!(result, data);
    }

    #[tokio::test]
    async fn oversized_frame_rejected() {
        let len = (MAX_FRAME_SIZE as u32 + 1).to_be_bytes();
        let mut buf = Vec::new();
        buf.extend_from_slice(&len);
        buf.extend(vec![0u8; 16]);
        let mut cursor = futures::io::Cursor::new(buf);
        let err = read_frame(&mut cursor).await.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn multi_peer_validator_accepts_majority() {
        let peer_a = libp2p::PeerId::random();
        let peer_b = libp2p::PeerId::random();
        let peer_c = libp2p::PeerId::random();

        let headers = make_headers(1, 3);
        let responses = vec![
            (peer_a, headers.clone()),
            (peer_b, headers.clone()),
            (peer_c, headers.clone()),
        ];

        let validator = MultiPeerValidator::new(3);
        assert!(validator.validate_responses(&responses).is_ok());
    }

    #[test]
    fn multi_peer_validator_rejects_too_few_peers() {
        let peer_a = libp2p::PeerId::random();
        let responses = vec![(peer_a, make_headers(1, 3))];

        let validator = MultiPeerValidator::new(3);
        let err = validator.validate_responses(&responses).unwrap_err();
        assert!(err.contains("too few"));
    }

    #[test]
    fn multi_peer_validator_rejects_no_majority() {
        let peer_a = libp2p::PeerId::random();
        let peer_b = libp2p::PeerId::random();
        let peer_c = libp2p::PeerId::random();

        let mut headers_b = make_headers(1, 3);
        headers_b.last_mut().unwrap().state_root = [0xBB; 32];

        let mut headers_c = make_headers(1, 3);
        headers_c.last_mut().unwrap().state_root = [0xCC; 32];

        let responses = vec![
            (peer_a, make_headers(1, 3)),
            (peer_b, headers_b),
            (peer_c, headers_c),
        ];

        let validator = MultiPeerValidator::new(2);
        let err = validator.validate_responses(&responses).unwrap_err();
        assert!(err.contains("no majority"));
    }

    #[test]
    fn multi_peer_validator_ignores_empty_responses() {
        let peer_a = libp2p::PeerId::random();
        let peer_b = libp2p::PeerId::random();
        let peer_c = libp2p::PeerId::random();

        let headers = make_headers(1, 3);
        let responses = vec![
            (peer_a, headers.clone()),
            (peer_b, headers.clone()),
            (peer_c, vec![]),
        ];

        let validator = MultiPeerValidator::new(2);
        assert!(validator.validate_responses(&responses).is_ok());
    }

    #[test]
    fn proof_request_response_roundtrip() {
        let req = build_proof_request(b"account_key".to_vec(), 42);
        let encoded = encode_light_sync(&req).unwrap();
        let decoded = decode_light_sync(&encoded).unwrap();
        match decoded {
            LightSyncMessage::RequestProof {
                state_key,
                at_round,
                ..
            } => {
                assert_eq!(state_key, b"account_key");
                assert_eq!(at_round, 42);
            }
            _ => panic!("expected RequestProof"),
        }

        let resp = build_proof_response(b"account_key".to_vec(), vec![1, 2, 3], 42);
        let encoded = encode_light_sync(&resp).unwrap();
        let decoded = decode_light_sync(&encoded).unwrap();
        match decoded {
            LightSyncMessage::ResponseProof {
                proof_data,
                at_round,
                ..
            } => {
                assert_eq!(proof_data, vec![1, 2, 3]);
                assert_eq!(at_round, 42);
            }
            _ => panic!("expected ResponseProof"),
        }
    }
}
