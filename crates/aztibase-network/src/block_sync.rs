use std::io;

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::StreamProtocol;
use libp2p::request_response;
use serde::{Deserialize, Serialize};

const BLOCK_SYNC_VERSION: u8 = 1;
pub const MAX_BATCHES_PER_REQUEST: u64 = 50;
const MAX_FRAME_SIZE: usize = 4_194_304; // 4 MiB (batches can be large)

pub const BLOCK_SYNC_PROTOCOL: StreamProtocol = StreamProtocol::new("/aztibase/block-sync/1");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSyncRequest(pub Vec<u8>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockSyncResponse(pub Vec<u8>);

#[derive(Debug, Clone, Default)]
pub struct BlockSyncCodec;

#[async_trait]
impl request_response::Codec for BlockSyncCodec {
    type Protocol = StreamProtocol;
    type Request = BlockSyncRequest;
    type Response = BlockSyncResponse;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_frame(io).await.map(BlockSyncRequest)
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        read_frame(io).await.map(BlockSyncResponse)
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BlockSyncMessage {
    RequestBatches {
        version: u8,
        from_index: u64,
        count: u64,
    },
    ResponseBatches {
        version: u8,
        batches: Vec<SyncBatch>,
        tip_index: u64,
    },
}

/// A committed batch in wire format for sync.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SyncBatch {
    pub index: u64,
    pub anchor_hash: [u8; 32],
    pub state_root: [u8; 32],
    pub transactions: Vec<Vec<u8>>,
}

pub fn encode_block_sync(msg: &BlockSyncMessage) -> Result<Vec<u8>, String> {
    postcard::to_allocvec(msg).map_err(|e| e.to_string())
}

pub fn decode_block_sync(data: &[u8]) -> Result<BlockSyncMessage, String> {
    let msg: BlockSyncMessage = postcard::from_bytes(data).map_err(|e| e.to_string())?;
    let version = match &msg {
        BlockSyncMessage::RequestBatches { version, .. }
        | BlockSyncMessage::ResponseBatches { version, .. } => *version,
    };
    if version != BLOCK_SYNC_VERSION {
        return Err(format!("unsupported block sync version: {version}"));
    }
    Ok(msg)
}

pub fn encode_request(msg: &BlockSyncMessage) -> Result<BlockSyncRequest, String> {
    encode_block_sync(msg).map(BlockSyncRequest)
}

pub fn decode_request(req: &BlockSyncRequest) -> Result<BlockSyncMessage, String> {
    decode_block_sync(&req.0)
}

pub fn encode_response(msg: &BlockSyncMessage) -> Result<BlockSyncResponse, String> {
    encode_block_sync(msg).map(BlockSyncResponse)
}

pub fn decode_response(resp: &BlockSyncResponse) -> Result<BlockSyncMessage, String> {
    decode_block_sync(&resp.0)
}

pub fn build_batch_request(from_index: u64, count: u64) -> BlockSyncMessage {
    BlockSyncMessage::RequestBatches {
        version: BLOCK_SYNC_VERSION,
        from_index,
        count: count.min(MAX_BATCHES_PER_REQUEST),
    }
}

pub fn build_batch_response(batches: Vec<SyncBatch>, tip_index: u64) -> BlockSyncMessage {
    BlockSyncMessage::ResponseBatches {
        version: BLOCK_SYNC_VERSION,
        batches,
        tip_index,
    }
}

/// Tracks block sync state for a full node catching up to the network tip.
pub struct BlockSyncProtocol {
    last_synced_index: u64,
    tip_index: u64,
}

impl BlockSyncProtocol {
    pub fn new(last_synced_index: u64) -> Self {
        Self {
            last_synced_index,
            tip_index: last_synced_index,
        }
    }

    pub fn last_synced_index(&self) -> u64 {
        self.last_synced_index
    }

    pub fn tip_index(&self) -> u64 {
        self.tip_index
    }

    pub fn set_tip_index(&mut self, tip: u64) {
        if tip > self.tip_index {
            self.tip_index = tip;
        }
    }

    pub fn needs_sync(&self) -> bool {
        self.last_synced_index < self.tip_index
    }

    pub fn batches_behind(&self) -> u64 {
        self.tip_index.saturating_sub(self.last_synced_index)
    }

    pub fn next_request(&self) -> Option<BlockSyncMessage> {
        if !self.needs_sync() {
            return None;
        }
        let remaining = self.tip_index - self.last_synced_index;
        let count = remaining.min(MAX_BATCHES_PER_REQUEST);
        Some(build_batch_request(self.last_synced_index + 1, count))
    }

    pub fn apply_response(&mut self, batches: &[SyncBatch], peer_tip: u64) -> u64 {
        let mut applied = 0u64;
        for b in batches {
            if b.index == self.last_synced_index + 1 {
                self.last_synced_index = b.index;
                applied += 1;
            }
        }
        self.set_tip_index(peer_tip);
        applied
    }
}

/// Gossip-published committed batch for live sync.
/// Validators publish this after each commit so full nodes can follow in real time.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommittedBatchAnnounce {
    pub index: u64,
    pub anchor_hash: [u8; 32],
    pub state_root: [u8; 32],
    pub transactions: Vec<Vec<u8>>,
}

pub fn encode_batch_announce(ann: &CommittedBatchAnnounce) -> Result<Vec<u8>, String> {
    postcard::to_allocvec(ann).map_err(|e| e.to_string())
}

pub fn decode_batch_announce(data: &[u8]) -> Result<CommittedBatchAnnounce, String> {
    postcard::from_bytes(data).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_batch(index: u64) -> SyncBatch {
        SyncBatch {
            index,
            anchor_hash: [index as u8; 32],
            state_root: [index as u8; 32],
            transactions: vec![vec![1, 2, 3], vec![4, 5, 6]],
        }
    }

    #[test]
    fn message_serde_roundtrip() {
        let req = build_batch_request(10, 50);
        let encoded = encode_block_sync(&req).unwrap();
        let decoded = decode_block_sync(&encoded).unwrap();
        match decoded {
            BlockSyncMessage::RequestBatches {
                from_index, count, ..
            } => {
                assert_eq!(from_index, 10);
                assert_eq!(count, 50);
            }
            _ => panic!("expected RequestBatches"),
        }

        let batches = vec![make_batch(1), make_batch(2), make_batch(3)];
        let resp = build_batch_response(batches, 100);
        let encoded = encode_block_sync(&resp).unwrap();
        let decoded = decode_block_sync(&encoded).unwrap();
        match decoded {
            BlockSyncMessage::ResponseBatches {
                batches, tip_index, ..
            } => {
                assert_eq!(batches.len(), 3);
                assert_eq!(batches[0].index, 1);
                assert_eq!(tip_index, 100);
            }
            _ => panic!("expected ResponseBatches"),
        }
    }

    #[test]
    fn version_mismatch_rejected() {
        let bad = BlockSyncMessage::RequestBatches {
            version: 99,
            from_index: 1,
            count: 10,
        };
        let encoded = postcard::to_allocvec(&bad).unwrap();
        assert!(decode_block_sync(&encoded).is_err());
    }

    #[test]
    fn request_caps_at_max() {
        let req = build_batch_request(0, 500);
        match req {
            BlockSyncMessage::RequestBatches { count, .. } => {
                assert_eq!(count, MAX_BATCHES_PER_REQUEST);
            }
            _ => panic!("expected RequestBatches"),
        }
    }

    #[test]
    fn sync_protocol_state_machine() {
        let mut proto = BlockSyncProtocol::new(0);
        assert!(!proto.needs_sync());
        assert_eq!(proto.batches_behind(), 0);

        proto.set_tip_index(100);
        assert!(proto.needs_sync());
        assert_eq!(proto.batches_behind(), 100);

        let req = proto.next_request().unwrap();
        match req {
            BlockSyncMessage::RequestBatches {
                from_index, count, ..
            } => {
                assert_eq!(from_index, 1);
                assert_eq!(count, 50);
            }
            _ => panic!("expected RequestBatches"),
        }

        let batches: Vec<SyncBatch> = (1..=50).map(make_batch).collect();
        let applied = proto.apply_response(&batches, 100);
        assert_eq!(applied, 50);
        assert_eq!(proto.last_synced_index(), 50);
        assert!(proto.needs_sync());

        let batches2: Vec<SyncBatch> = (51..=100).map(make_batch).collect();
        let applied2 = proto.apply_response(&batches2, 100);
        assert_eq!(applied2, 50);
        assert_eq!(proto.last_synced_index(), 100);
        assert!(!proto.needs_sync());
        assert!(proto.next_request().is_none());
    }

    #[test]
    fn sync_protocol_tip_only_increases() {
        let mut proto = BlockSyncProtocol::new(5);
        proto.set_tip_index(100);
        assert_eq!(proto.tip_index(), 100);
        proto.set_tip_index(50);
        assert_eq!(proto.tip_index(), 100);
    }

    #[test]
    fn sync_protocol_skips_out_of_order() {
        let mut proto = BlockSyncProtocol::new(0);
        proto.set_tip_index(10);
        let batch5 = make_batch(5);
        let applied = proto.apply_response(&[batch5], 10);
        assert_eq!(applied, 0);
        assert_eq!(proto.last_synced_index(), 0);
    }

    #[test]
    fn codec_encode_decode_request() {
        let msg = build_batch_request(5, 20);
        let req = encode_request(&msg).unwrap();
        let decoded = decode_request(&req).unwrap();
        match decoded {
            BlockSyncMessage::RequestBatches {
                from_index, count, ..
            } => {
                assert_eq!(from_index, 5);
                assert_eq!(count, 20);
            }
            _ => panic!("expected RequestBatches"),
        }
    }

    #[test]
    fn codec_encode_decode_response() {
        let batches = vec![make_batch(1), make_batch(2)];
        let msg = build_batch_response(batches, 50);
        let resp = encode_response(&msg).unwrap();
        let decoded = decode_response(&resp).unwrap();
        match decoded {
            BlockSyncMessage::ResponseBatches {
                batches, tip_index, ..
            } => {
                assert_eq!(batches.len(), 2);
                assert_eq!(tip_index, 50);
            }
            _ => panic!("expected ResponseBatches"),
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
    fn batch_announce_roundtrip() {
        let ann = CommittedBatchAnnounce {
            index: 42,
            anchor_hash: [0xAA; 32],
            state_root: [0xBB; 32],
            transactions: vec![vec![1, 2], vec![3, 4, 5]],
        };
        let encoded = encode_batch_announce(&ann).unwrap();
        let decoded = decode_batch_announce(&encoded).unwrap();
        assert_eq!(decoded.index, 42);
        assert_eq!(decoded.anchor_hash, [0xAA; 32]);
        assert_eq!(decoded.state_root, [0xBB; 32]);
        assert_eq!(decoded.transactions.len(), 2);
    }

    #[test]
    fn empty_batch_response() {
        let resp = build_batch_response(vec![], 0);
        let encoded = encode_block_sync(&resp).unwrap();
        let decoded = decode_block_sync(&encoded).unwrap();
        match decoded {
            BlockSyncMessage::ResponseBatches {
                batches, tip_index, ..
            } => {
                assert!(batches.is_empty());
                assert_eq!(tip_index, 0);
            }
            _ => panic!("expected ResponseBatches"),
        }
    }
}
