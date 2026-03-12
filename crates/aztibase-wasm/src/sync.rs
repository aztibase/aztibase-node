use serde::{Deserialize, Serialize};

pub const MAX_HEADERS_PER_REQUEST: u64 = 100;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncHeader {
    pub round: u64,
    pub author: Vec<u8>,
    pub parents: Vec<Vec<u8>>,
    pub state_root: Vec<u8>,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncFinalityCert {
    pub anchor_round: u64,
    pub batch_hash: Vec<u8>,
    pub state_root: Vec<u8>,
    pub aggregate_signature: Vec<u8>,
    pub signer_bitmap: Vec<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsSyncState {
    pub last_synced_round: u64,
    pub target_round: u64,
}

pub fn verify_header_chain(
    headers: &[SyncHeader],
    cert: &SyncFinalityCert,
    expected_start: u64,
) -> Result<(), String> {
    if headers.is_empty() {
        return Err("empty header batch".into());
    }

    for (i, h) in headers.iter().enumerate() {
        let expected_round = expected_start + i as u64;
        if h.round != expected_round {
            return Err(format!(
                "gap at index {i}: expected round {expected_round}, got {}",
                h.round
            ));
        }
    }

    let first_round = headers[0].round;
    let last_round = headers[headers.len() - 1].round;
    if cert.anchor_round < first_round || cert.anchor_round > last_round {
        return Err(format!(
            "finality cert anchor round {} not in batch range [{first_round}, {last_round}]",
            cert.anchor_round
        ));
    }

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

    if cert.aggregate_signature.is_empty() {
        return Err("empty aggregate signature".into());
    }

    // SECURITY: BLS verification deferred — WASM cannot link blst (C dependency). Verify via full node.
    // The checks above (quorum threshold, non-empty signature) provide structural validation,
    // but cryptographic BLS aggregate verification must happen on a full node.

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_headers(start: u64, count: u64) -> Vec<SyncHeader> {
        (start..start + count)
            .map(|r| SyncHeader {
                round: r,
                author: vec![r as u8; 32],
                parents: vec![vec![0u8; 32]],
                state_root: vec![r as u8; 32],
                timestamp: 1000 + r,
            })
            .collect()
    }

    fn make_cert(anchor_round: u64, n_validators: usize, n_signed: usize) -> SyncFinalityCert {
        let mut bitmap = vec![false; n_validators];
        for b in bitmap.iter_mut().take(n_signed) {
            *b = true;
        }
        SyncFinalityCert {
            anchor_round,
            batch_hash: vec![anchor_round as u8; 32],
            state_root: vec![anchor_round as u8; 32],
            aggregate_signature: vec![0u8; 96],
            signer_bitmap: bitmap,
        }
    }

    #[test]
    fn valid_chain_passes() {
        let headers = make_headers(5, 10);
        let cert = make_cert(10, 4, 3);
        assert!(verify_header_chain(&headers, &cert, 5).is_ok());
    }

    #[test]
    fn gap_rejected() {
        let mut headers = make_headers(5, 5);
        headers[2].round = 99;
        let cert = make_cert(7, 4, 3);
        let err = verify_header_chain(&headers, &cert, 5).unwrap_err();
        assert!(err.contains("gap"));
    }

    #[test]
    fn cert_outside_batch_rejected() {
        let headers = make_headers(5, 5);
        let cert = make_cert(100, 4, 3);
        let err = verify_header_chain(&headers, &cert, 5).unwrap_err();
        assert!(err.contains("anchor round"));
    }

    #[test]
    fn insufficient_quorum_rejected() {
        let headers = make_headers(5, 5);
        let cert = make_cert(7, 4, 1);
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
    fn empty_batch_rejected() {
        let cert = make_cert(5, 4, 3);
        let err = verify_header_chain(&[], &cert, 5).unwrap_err();
        assert!(err.contains("empty"));
    }

    #[test]
    fn json_roundtrip() {
        let headers = make_headers(1, 3);
        let json = serde_json::to_string(&headers).unwrap();
        let parsed: Vec<SyncHeader> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].round, 1);
    }
}
