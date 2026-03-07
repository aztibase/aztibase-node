use blst::BLST_ERROR;
use blst::min_pk::{AggregatePublicKey, AggregateSignature, PublicKey, SecretKey, Signature};
use serde::{Deserialize, Serialize};

const DST: &[u8] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";
const DST_POP: &[u8] = b"BLS_POP_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";

#[derive(Clone)]
pub struct BlsKeypair {
    secret: SecretKey,
    public: BlsPublicKey,
    proof_of_possession: BlsSignature,
}

impl BlsKeypair {
    pub fn generate() -> Self {
        let mut ikm = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut ikm);
        let secret = SecretKey::key_gen(&ikm, &[]).expect("valid IKM length");
        let pk = secret.sk_to_pk();
        let pk_bytes = pk.to_bytes();
        let pop = secret.sign(&pk_bytes, DST_POP, &[]);
        Self {
            secret,
            public: BlsPublicKey(pk_bytes),
            proof_of_possession: BlsSignature(pop.to_bytes()),
        }
    }

    pub fn sign(&self, message: &[u8]) -> BlsSignature {
        let sig = self.secret.sign(message, DST, &[]);
        BlsSignature(sig.to_bytes())
    }

    pub fn public_key(&self) -> &BlsPublicKey {
        &self.public
    }

    pub fn proof_of_possession(&self) -> &BlsSignature {
        &self.proof_of_possession
    }

    pub fn secret_bytes(&self) -> [u8; 32] {
        self.secret.to_bytes()
    }

    pub fn from_secret_bytes(bytes: &[u8; 32]) -> Option<Self> {
        let secret = SecretKey::from_bytes(bytes).ok()?;
        let pk = secret.sk_to_pk();
        let pk_bytes = pk.to_bytes();
        let pop = secret.sign(&pk_bytes, DST_POP, &[]);
        Some(Self {
            secret,
            public: BlsPublicKey(pk_bytes),
            proof_of_possession: BlsSignature(pop.to_bytes()),
        })
    }
}

/// Verify a proof-of-possession: the validator signed their own public key
/// using the dedicated PoP DST. This prevents rogue-key attacks on aggregate
/// signatures.
pub fn verify_proof_of_possession(public_key: &BlsPublicKey, pop: &BlsSignature) -> bool {
    let pk = match PublicKey::from_bytes(&public_key.0) {
        Ok(pk) => pk,
        Err(_) => return false,
    };
    let sig = match Signature::from_bytes(&pop.0) {
        Ok(sig) => sig,
        Err(_) => return false,
    };
    sig.verify(true, &public_key.0, DST_POP, &[], &pk, true) == BLST_ERROR::BLST_SUCCESS
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlsPublicKey([u8; 48]);

impl Serialize for BlsPublicKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for BlsPublicKey {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
        let arr: [u8; 48] = bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("expected 48 bytes for BLS public key"))?;
        Ok(Self(arr))
    }
}

impl BlsPublicKey {
    pub fn from_bytes(bytes: [u8; 48]) -> Option<Self> {
        // Validate by attempting to deserialize
        PublicKey::from_bytes(&bytes).ok()?;
        Some(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 48] {
        &self.0
    }

    pub fn verify(&self, message: &[u8], signature: &BlsSignature) -> bool {
        let pk = match PublicKey::from_bytes(&self.0) {
            Ok(pk) => pk,
            Err(_) => return false,
        };
        let sig = match Signature::from_bytes(&signature.0) {
            Ok(sig) => sig,
            Err(_) => return false,
        };
        sig.verify(true, message, DST, &[], &pk, true) == BLST_ERROR::BLST_SUCCESS
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlsSignature([u8; 96]);

impl Serialize for BlsSignature {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for BlsSignature {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes: Vec<u8> = Deserialize::deserialize(deserializer)?;
        let arr: [u8; 96] = bytes
            .try_into()
            .map_err(|_| serde::de::Error::custom("expected 96 bytes for BLS signature"))?;
        Ok(Self(arr))
    }
}

impl BlsSignature {
    pub fn from_bytes(bytes: [u8; 96]) -> Option<Self> {
        Signature::from_bytes(&bytes).ok()?;
        Some(Self(bytes))
    }

    pub fn as_bytes(&self) -> &[u8; 96] {
        &self.0
    }
}

/// Aggregate multiple BLS signatures into one.
/// All signatures must be over the same message.
pub fn aggregate_signatures(signatures: &[BlsSignature]) -> Option<BlsSignature> {
    if signatures.is_empty() {
        return None;
    }

    let sigs: Vec<Signature> = signatures
        .iter()
        .map(|s| Signature::from_bytes(&s.0))
        .collect::<Result<Vec<_>, _>>()
        .ok()?;

    let sig_refs: Vec<&Signature> = sigs.iter().collect();
    let agg = AggregateSignature::aggregate(&sig_refs, true).ok()?;
    Some(BlsSignature(agg.to_signature().to_bytes()))
}

/// Verify an aggregated signature against multiple public keys.
/// All signers must have signed the same message.
pub fn verify_aggregate(
    public_keys: &[BlsPublicKey],
    message: &[u8],
    aggregate_sig: &BlsSignature,
) -> bool {
    if public_keys.is_empty() {
        return false;
    }

    let pks: Vec<PublicKey> = match public_keys
        .iter()
        .map(|pk| PublicKey::from_bytes(&pk.0))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(pks) => pks,
        Err(_) => return false,
    };

    let pk_refs: Vec<&PublicKey> = pks.iter().collect();
    let agg_pk = match AggregatePublicKey::aggregate(&pk_refs, true) {
        Ok(apk) => apk,
        Err(_) => return false,
    };

    let sig = match Signature::from_bytes(&aggregate_sig.0) {
        Ok(sig) => sig,
        Err(_) => return false,
    };

    sig.verify(true, message, DST, &[], &agg_pk.to_public_key(), true) == BLST_ERROR::BLST_SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keypair_sign_verify() {
        let kp = BlsKeypair::generate();
        let msg = b"test message";
        let sig = kp.sign(msg);
        assert!(kp.public_key().verify(msg, &sig));
    }

    #[test]
    fn proof_of_possession_valid() {
        let kp = BlsKeypair::generate();
        assert!(verify_proof_of_possession(
            kp.public_key(),
            kp.proof_of_possession()
        ));
    }

    #[test]
    fn proof_of_possession_rejects_wrong_key() {
        let kp1 = BlsKeypair::generate();
        let kp2 = BlsKeypair::generate();
        assert!(!verify_proof_of_possession(
            kp2.public_key(),
            kp1.proof_of_possession()
        ));
    }

    #[test]
    fn rejects_wrong_message() {
        let kp = BlsKeypair::generate();
        let sig = kp.sign(b"correct");
        assert!(!kp.public_key().verify(b"wrong", &sig));
    }

    #[test]
    fn rejects_wrong_key() {
        let kp1 = BlsKeypair::generate();
        let kp2 = BlsKeypair::generate();
        let sig = kp1.sign(b"message");
        assert!(!kp2.public_key().verify(b"message", &sig));
    }

    #[test]
    fn public_key_roundtrip() {
        let kp = BlsKeypair::generate();
        let bytes = *kp.public_key().as_bytes();
        let restored = BlsPublicKey::from_bytes(bytes).unwrap();
        assert_eq!(restored, *kp.public_key());
    }

    #[test]
    fn signature_roundtrip() {
        let kp = BlsKeypair::generate();
        let sig = kp.sign(b"data");
        let bytes = *sig.as_bytes();
        let restored = BlsSignature::from_bytes(bytes).unwrap();
        assert_eq!(restored, sig);
    }

    #[test]
    fn aggregate_two_signatures() {
        let kp1 = BlsKeypair::generate();
        let kp2 = BlsKeypair::generate();
        let msg = b"consensus commit";

        let sig1 = kp1.sign(msg);
        let sig2 = kp2.sign(msg);

        let agg = aggregate_signatures(&[sig1, sig2]).unwrap();
        let pks = vec![kp1.public_key().clone(), kp2.public_key().clone()];
        assert!(verify_aggregate(&pks, msg, &agg));
    }

    #[test]
    fn aggregate_rejects_missing_signer() {
        let kp1 = BlsKeypair::generate();
        let kp2 = BlsKeypair::generate();
        let kp3 = BlsKeypair::generate();
        let msg = b"consensus commit";

        let sig1 = kp1.sign(msg);
        let sig2 = kp2.sign(msg);
        let agg = aggregate_signatures(&[sig1, sig2]).unwrap();

        // Verify against wrong set (includes kp3 instead of kp2)
        let wrong_pks = vec![kp1.public_key().clone(), kp3.public_key().clone()];
        assert!(!verify_aggregate(&wrong_pks, msg, &agg));
    }

    #[test]
    fn aggregate_rejects_wrong_message() {
        let kp1 = BlsKeypair::generate();
        let kp2 = BlsKeypair::generate();
        let msg = b"correct message";

        let sig1 = kp1.sign(msg);
        let sig2 = kp2.sign(msg);
        let agg = aggregate_signatures(&[sig1, sig2]).unwrap();

        let pks = vec![kp1.public_key().clone(), kp2.public_key().clone()];
        assert!(!verify_aggregate(&pks, b"wrong message", &agg));
    }

    #[test]
    fn aggregate_empty_returns_none() {
        assert!(aggregate_signatures(&[]).is_none());
    }

    #[test]
    fn verify_aggregate_empty_keys_returns_false() {
        let kp = BlsKeypair::generate();
        let sig = kp.sign(b"msg");
        assert!(!verify_aggregate(&[], b"msg", &sig));
    }

    #[test]
    fn aggregate_supermajority() {
        let keypairs: Vec<BlsKeypair> = (0..4).map(|_| BlsKeypair::generate()).collect();
        let msg = b"batch commit";

        // 3 out of 4 sign (supermajority for n=4, f=1, need n-f=3)
        let sigs: Vec<BlsSignature> = keypairs[..3].iter().map(|kp| kp.sign(msg)).collect();
        let agg = aggregate_signatures(&sigs).unwrap();

        let signer_pks: Vec<BlsPublicKey> = keypairs[..3]
            .iter()
            .map(|kp| kp.public_key().clone())
            .collect();
        assert!(verify_aggregate(&signer_pks, msg, &agg));
    }
}
