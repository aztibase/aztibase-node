use anyhow::Result;

/// Dual storage architecture: state store (raw key-value) + state commitment.
/// Follows Sei's production-proven pattern.
pub struct StateStore;

impl StateStore {
    pub fn open(_path: &str) -> Result<Self> {
        Ok(Self)
    }
}
