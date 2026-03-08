use std::collections::BTreeMap;

use aztibase_core::{Hash, hash};
use serde::{Deserialize, Serialize};

type Address = [u8; 32];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelMetadata {
    pub model_id: String,
    pub owner: Address,
    pub fingerprint: Hash,
    pub compute_cost: u64,
    pub min_stake: u64,
    pub registered_round: u64,
    pub active: bool,
}

impl ModelMetadata {
    pub fn registry_key(model_id: &str) -> Vec<u8> {
        let mut key = b"model:".to_vec();
        key.extend_from_slice(model_id.as_bytes());
        key
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        postcard::to_allocvec(self).expect("ModelMetadata serialization cannot fail")
    }

    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        postcard::from_bytes(data).ok()
    }
}

/// On-chain model registry backed by account storage.
/// Models are stored as key-value entries in a dedicated registry address.
pub struct ModelRegistry;

pub const MODEL_REGISTRY_ADDRESS: Address = [0xFF; 32];

impl ModelRegistry {
    pub fn register(
        storage: &mut BTreeMap<Vec<u8>, Vec<u8>>,
        model_id: String,
        owner: Address,
        fingerprint: Hash,
        compute_cost: u64,
        min_stake: u64,
        current_round: u64,
    ) -> Result<ModelMetadata, RegistryError> {
        if model_id.is_empty() || model_id.len() > 128 {
            return Err(RegistryError::InvalidModelId);
        }
        let key = ModelMetadata::registry_key(&model_id);
        if storage.contains_key(&key) {
            return Err(RegistryError::AlreadyRegistered);
        }
        if compute_cost == 0 {
            return Err(RegistryError::InvalidComputeCost);
        }

        let meta = ModelMetadata {
            model_id,
            owner,
            fingerprint,
            compute_cost,
            min_stake,
            registered_round: current_round,
            active: true,
        };
        storage.insert(key, meta.to_bytes());
        Ok(meta)
    }

    pub fn get(storage: &BTreeMap<Vec<u8>, Vec<u8>>, model_id: &str) -> Option<ModelMetadata> {
        let key = ModelMetadata::registry_key(model_id);
        storage.get(&key).and_then(|v| ModelMetadata::from_bytes(v))
    }

    pub fn deregister(
        storage: &mut BTreeMap<Vec<u8>, Vec<u8>>,
        model_id: &str,
        caller: &Address,
    ) -> Result<(), RegistryError> {
        let key = ModelMetadata::registry_key(model_id);
        let data = storage.get(&key).ok_or(RegistryError::NotFound)?;
        let meta = ModelMetadata::from_bytes(data).ok_or(RegistryError::NotFound)?;
        if meta.owner != *caller {
            return Err(RegistryError::NotOwner);
        }
        let mut updated = meta;
        updated.active = false;
        storage.insert(key, updated.to_bytes());
        Ok(())
    }

    pub fn list_active(storage: &BTreeMap<Vec<u8>, Vec<u8>>) -> Vec<ModelMetadata> {
        let prefix = b"model:";
        storage
            .iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .filter_map(|(_, v)| ModelMetadata::from_bytes(v))
            .filter(|m| m.active)
            .collect()
    }

    pub fn model_fingerprint(model_id: &str, code_hash: &Hash) -> Hash {
        let mut buf = Vec::new();
        buf.extend_from_slice(model_id.as_bytes());
        buf.extend_from_slice(code_hash);
        hash(&buf)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RegistryError {
    InvalidModelId,
    AlreadyRegistered,
    InvalidComputeCost,
    NotFound,
    NotOwner,
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::InvalidModelId => write!(f, "invalid model ID"),
            RegistryError::AlreadyRegistered => write!(f, "model already registered"),
            RegistryError::InvalidComputeCost => write!(f, "compute cost must be > 0"),
            RegistryError::NotFound => write!(f, "model not found"),
            RegistryError::NotOwner => write!(f, "caller is not model owner"),
        }
    }
}

impl std::error::Error for RegistryError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_storage() -> BTreeMap<Vec<u8>, Vec<u8>> {
        BTreeMap::new()
    }

    #[test]
    fn register_and_query() {
        let mut storage = make_storage();
        let owner = [1u8; 32];
        let fingerprint = hash(b"model-weights");

        let meta = ModelRegistry::register(
            &mut storage,
            "sentiment_v1".into(),
            owner,
            fingerprint,
            1000,
            500,
            10,
        )
        .unwrap();

        assert_eq!(meta.model_id, "sentiment_v1");
        assert_eq!(meta.owner, owner);
        assert!(meta.active);

        let queried = ModelRegistry::get(&storage, "sentiment_v1").unwrap();
        assert_eq!(queried, meta);
    }

    #[test]
    fn duplicate_registration_rejected() {
        let mut storage = make_storage();
        let owner = [1u8; 32];
        let fp = hash(b"fp");

        ModelRegistry::register(&mut storage, "m1".into(), owner, fp, 100, 0, 0).unwrap();
        let err = ModelRegistry::register(&mut storage, "m1".into(), owner, fp, 100, 0, 1);
        assert_eq!(err.unwrap_err(), RegistryError::AlreadyRegistered);
    }

    #[test]
    fn deregister_by_owner() {
        let mut storage = make_storage();
        let owner = [1u8; 32];
        ModelRegistry::register(&mut storage, "m1".into(), owner, hash(b"f"), 100, 0, 0).unwrap();

        ModelRegistry::deregister(&mut storage, "m1", &owner).unwrap();
        let meta = ModelRegistry::get(&storage, "m1").unwrap();
        assert!(!meta.active);
    }

    #[test]
    fn deregister_by_non_owner_rejected() {
        let mut storage = make_storage();
        let owner = [1u8; 32];
        let other = [2u8; 32];
        ModelRegistry::register(&mut storage, "m1".into(), owner, hash(b"f"), 100, 0, 0).unwrap();

        let err = ModelRegistry::deregister(&mut storage, "m1", &other);
        assert_eq!(err.unwrap_err(), RegistryError::NotOwner);
    }

    #[test]
    fn list_active_filters_deregistered() {
        let mut storage = make_storage();
        let owner = [1u8; 32];
        ModelRegistry::register(&mut storage, "m1".into(), owner, hash(b"a"), 100, 0, 0).unwrap();
        ModelRegistry::register(&mut storage, "m2".into(), owner, hash(b"b"), 200, 0, 0).unwrap();
        ModelRegistry::deregister(&mut storage, "m1", &owner).unwrap();

        let active = ModelRegistry::list_active(&storage);
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].model_id, "m2");
    }

    #[test]
    fn empty_model_id_rejected() {
        let mut storage = make_storage();
        let err =
            ModelRegistry::register(&mut storage, "".into(), [1u8; 32], hash(b"f"), 100, 0, 0);
        assert_eq!(err.unwrap_err(), RegistryError::InvalidModelId);
    }

    #[test]
    fn zero_compute_cost_rejected() {
        let mut storage = make_storage();
        let err =
            ModelRegistry::register(&mut storage, "m1".into(), [1u8; 32], hash(b"f"), 0, 0, 0);
        assert_eq!(err.unwrap_err(), RegistryError::InvalidComputeCost);
    }

    #[test]
    fn model_fingerprint_deterministic() {
        let f1 = ModelRegistry::model_fingerprint("m1", &hash(b"code"));
        let f2 = ModelRegistry::model_fingerprint("m1", &hash(b"code"));
        assert_eq!(f1, f2);

        let f3 = ModelRegistry::model_fingerprint("m2", &hash(b"code"));
        assert_ne!(f1, f3);
    }
}
