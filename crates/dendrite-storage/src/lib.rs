pub mod store;
pub mod verkle;

pub use store::StateStore;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_store_open() {
        let tmp = std::env::temp_dir().join("dendrite_test_store");
        let store = StateStore::open(tmp.to_str().unwrap());
        assert!(store.is_ok());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
