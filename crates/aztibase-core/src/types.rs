use crate::crypto::Hash;
use serde::{Deserialize, Serialize};

/// Unique identifier for a validator.
pub type ValidatorId = [u8; 32];

/// Hash of a block.
pub type BlockHash = Hash;

/// Hash of a transaction.
pub type TxHash = Hash;

/// A Verkle tree root commitment.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerkleRoot(pub [u8; 32]);

/// Block header for the Aztibase DAG-based chain.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockHeader {
    pub version: u16,
    pub slot: u64,
    pub round: u64,
    pub author: ValidatorId,
    pub parents: Vec<BlockHash>,
    pub state_root: VerkleRoot,
    pub transactions_root: Hash,
    pub receipts_root: Hash,
    pub ai_commitment_root: Hash,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

/// Block body.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BlockBody {
    pub transactions: Vec<Transaction>,
    pub ai_attestations: Vec<AIAttestation>,
}

/// A full block (header + body).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub body: BlockBody,
}

/// A transaction on the Aztibase network.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub hash: TxHash,
    pub from: [u8; 32],
    pub to: [u8; 32],
    pub value: u128,
    pub nonce: u64,
    pub data: Vec<u8>,
    pub gas_limit: u64,
    pub gas_price: u64,
    pub signature: Vec<u8>,
}

/// An AI work attestation (PoUW layer).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AIAttestation {
    pub model_hash: Hash,
    pub input_hash: Hash,
    pub output_hash: Hash,
    pub validator: ValidatorId,
    pub signature: Vec<u8>,
}
