use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};

/// BLAKE3 hash output (32 bytes).
pub type Hash = [u8; 32];

/// Compute a BLAKE3 hash of the given data.
pub fn hash(data: &[u8]) -> Hash {
    *blake3::hash(data).as_bytes()
}

/// A keypair for Ed25519 signing.
#[derive(Clone)]
pub struct Keypair {
    signing_key: SigningKey,
}

impl Keypair {
    /// Generate a new random keypair.
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        Self { signing_key }
    }

    /// Sign a message.
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        let signature: Signature = self.signing_key.sign(message);
        signature.to_bytes().to_vec()
    }

    /// Get the public key.
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.signing_key.verifying_key())
    }
}

/// An Ed25519 public key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKey(#[serde(with = "pub_key_serde")] VerifyingKey);

impl PublicKey {
    /// Verify a signature against this public key.
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        if signature.len() != 64 {
            return false;
        }
        let sig_bytes: [u8; 64] = signature.try_into().unwrap();
        let sig = Signature::from_bytes(&sig_bytes);
        self.0.verify(message, &sig).is_ok()
    }

    /// Get the raw bytes of this public key.
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }
}

/// Abstraction over cryptographic operations.
///
/// Allows swapping the underlying algorithms (e.g., Ed25519 → Dilithium for PQC)
/// without changing consumer code. All consensus, networking, and storage code
/// should use this trait rather than calling hash/sign/verify directly.
pub trait CryptoProvider: Send + Sync {
    fn hash(&self, data: &[u8]) -> Hash;
    fn generate_keypair(&self) -> Keypair;
    fn sign(&self, keypair: &Keypair, message: &[u8]) -> Vec<u8>;
    fn verify(&self, public_key: &PublicKey, message: &[u8], signature: &[u8]) -> bool;
    fn algorithm_name(&self) -> &'static str;
}

/// Default provider using BLAKE3 + Ed25519.
pub struct DefaultCryptoProvider;

impl CryptoProvider for DefaultCryptoProvider {
    fn hash(&self, data: &[u8]) -> Hash {
        hash(data)
    }

    fn generate_keypair(&self) -> Keypair {
        Keypair::generate()
    }

    fn sign(&self, keypair: &Keypair, message: &[u8]) -> Vec<u8> {
        keypair.sign(message)
    }

    fn verify(&self, public_key: &PublicKey, message: &[u8], signature: &[u8]) -> bool {
        public_key.verify(message, signature)
    }

    fn algorithm_name(&self) -> &'static str {
        "Ed25519+BLAKE3"
    }
}

mod pub_key_serde {
    use super::*;
    use serde::{self, Deserializer, Serializer};

    pub fn serialize<S>(key: &VerifyingKey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(key.as_bytes())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<VerifyingKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = serde::Deserialize::deserialize(deserializer)?;
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("invalid public key length"))?;
        VerifyingKey::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}
