use std::collections::HashMap;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ParamValue {
    U64(u64),
    Bool(bool),
    Str(String),
}

impl fmt::Display for ParamValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParamValue::U64(v) => write!(f, "{v}"),
            ParamValue::Bool(v) => write!(f, "{v}"),
            ParamValue::Str(v) => write!(f, "{v}"),
        }
    }
}

impl ParamValue {
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            ParamValue::U64(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ParamValue::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn parse(s: &str, expected: &ParamType) -> Option<ParamValue> {
        match expected {
            ParamType::U64 => s.parse::<u64>().ok().map(ParamValue::U64),
            ParamType::Bool => s.parse::<bool>().ok().map(ParamValue::Bool),
            ParamType::Str => Some(ParamValue::Str(s.to_string())),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamType {
    U64,
    Bool,
    Str,
}

#[derive(Clone, Debug)]
pub struct ParamDef {
    pub key: &'static str,
    pub param_type: ParamType,
    pub default: ParamValue,
    pub min: Option<u64>,
    pub max: Option<u64>,
    pub description: &'static str,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ChainParamError {
    UnknownKey(String),
    TypeMismatch {
        key: String,
        expected: ParamType,
    },
    OutOfBounds {
        key: String,
        value: u64,
        min: u64,
        max: u64,
    },
}

impl fmt::Display for ChainParamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainParamError::UnknownKey(k) => write!(f, "unknown chain parameter: {k}"),
            ChainParamError::TypeMismatch { key, expected } => {
                write!(f, "type mismatch for {key}: expected {expected:?}")
            }
            ChainParamError::OutOfBounds {
                key,
                value,
                min,
                max,
            } => {
                write!(f, "{key}={value} out of bounds [{min}, {max}]")
            }
        }
    }
}

impl std::error::Error for ChainParamError {}

static PARAM_DEFS: &[ParamDef] = &[
    ParamDef {
        key: "base_fee_floor",
        param_type: ParamType::U64,
        default: ParamValue::U64(1),
        min: Some(1),
        max: Some(1_000_000),
        description: "Minimum base fee (EIP-1559 floor)",
    },
    ParamDef {
        key: "base_fee_ceiling",
        param_type: ParamType::U64,
        default: ParamValue::U64(1_000_000_000),
        min: Some(1_000),
        max: Some(10_000_000_000),
        description: "Maximum base fee (EIP-1559 ceiling)",
    },
    ParamDef {
        key: "target_gas_per_batch",
        param_type: ParamType::U64,
        default: ParamValue::U64(15_000_000),
        min: Some(1_000_000),
        max: Some(100_000_000),
        description: "Target gas utilization per batch",
    },
    ParamDef {
        key: "max_gas_per_batch",
        param_type: ParamType::U64,
        default: ParamValue::U64(30_000_000),
        min: Some(2_000_000),
        max: Some(200_000_000),
        description: "Hard cap on gas per batch",
    },
    ParamDef {
        key: "base_fee_change_denom",
        param_type: ParamType::U64,
        default: ParamValue::U64(8),
        min: Some(2),
        max: Some(128),
        description: "Base fee adjustment sensitivity denominator",
    },
    ParamDef {
        key: "max_block_range",
        param_type: ParamType::U64,
        default: ParamValue::U64(100),
        min: Some(10),
        max: Some(10_000),
        description: "Maximum blocks returned by getBlockRange RPC",
    },
    ParamDef {
        key: "max_stored_txs",
        param_type: ParamType::U64,
        default: ParamValue::U64(500_000),
        min: Some(10_000),
        max: Some(10_000_000),
        description: "Transaction store eviction cap",
    },
    ParamDef {
        key: "max_stored_batch_roots",
        param_type: ParamType::U64,
        default: ParamValue::U64(100_000),
        min: Some(1_000),
        max: Some(1_000_000),
        description: "Batch root store eviction cap",
    },
    ParamDef {
        key: "max_stored_receipts",
        param_type: ParamType::U64,
        default: ParamValue::U64(100_000),
        min: Some(1_000),
        max: Some(1_000_000),
        description: "Receipt store eviction cap",
    },
    ParamDef {
        key: "epoch_length",
        param_type: ParamType::U64,
        default: ParamValue::U64(1_000),
        min: Some(100),
        max: Some(100_000),
        description: "Rounds per epoch (controls reward distribution frequency)",
    },
    ParamDef {
        key: "min_validator_stake",
        param_type: ParamType::U64,
        default: ParamValue::U64(50_000_000_000_000),
        min: Some(10_000),
        max: Some(500_000),
        description: "Minimum stake to become an active validator (in base units)",
    },
    ParamDef {
        key: "max_stake_cap",
        param_type: ParamType::U64,
        default: ParamValue::U64(50_000_000_000_000_000),
        min: Some(1_000_000),
        max: Some(100_000_000),
        description: "Maximum effective stake per validator (in base units)",
    },
    ParamDef {
        key: "validator_commission_bps",
        param_type: ParamType::U64,
        default: ParamValue::U64(1000),
        min: Some(0),
        max: Some(3000),
        description: "Validator commission on delegation rewards (basis points, 100 = 1%)",
    },
];

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ChainParams {
    values: HashMap<String, ParamValue>,
}

impl ChainParams {
    pub fn defaults() -> Self {
        let mut values = HashMap::new();
        for def in PARAM_DEFS {
            values.insert(def.key.to_string(), def.default.clone());
        }
        Self { values }
    }

    pub fn get(&self, key: &str) -> Option<&ParamValue> {
        self.values.get(key)
    }

    pub fn get_u64(&self, key: &str) -> Option<u64> {
        self.values.get(key).and_then(|v| v.as_u64())
    }

    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.values.get(key).and_then(|v| v.as_bool())
    }

    pub fn set(&mut self, key: &str, value: ParamValue) -> Result<ParamValue, ChainParamError> {
        let def = param_def(key).ok_or_else(|| ChainParamError::UnknownKey(key.to_string()))?;

        if std::mem::discriminant(&value) != std::mem::discriminant(&def.default) {
            return Err(ChainParamError::TypeMismatch {
                key: key.to_string(),
                expected: def.param_type,
            });
        }

        if let ParamValue::U64(v) = &value {
            let min = def.min.unwrap_or(0);
            let max = def.max.unwrap_or(u64::MAX);
            if *v < min || *v > max {
                return Err(ChainParamError::OutOfBounds {
                    key: key.to_string(),
                    value: *v,
                    min,
                    max,
                });
            }
        }

        let old = self
            .values
            .insert(key.to_string(), value)
            .unwrap_or_else(|| def.default.clone());
        Ok(old)
    }

    pub fn set_from_str(
        &mut self,
        key: &str,
        value_str: &str,
    ) -> Result<ParamValue, ChainParamError> {
        let def = param_def(key).ok_or_else(|| ChainParamError::UnknownKey(key.to_string()))?;
        let value = ParamValue::parse(value_str, &def.param_type).ok_or_else(|| {
            ChainParamError::TypeMismatch {
                key: key.to_string(),
                expected: def.param_type,
            }
        })?;
        self.set(key, value)
    }

    pub fn list(&self) -> Vec<(&str, &ParamValue)> {
        let mut entries: Vec<(&str, &ParamValue)> =
            self.values.iter().map(|(k, v)| (k.as_str(), v)).collect();
        entries.sort_by_key(|(k, _)| *k);
        entries
    }
}

impl Default for ChainParams {
    fn default() -> Self {
        Self::defaults()
    }
}

pub fn param_def(key: &str) -> Option<&'static ParamDef> {
    PARAM_DEFS.iter().find(|d| d.key == key)
}

pub fn all_param_defs() -> &'static [ParamDef] {
    PARAM_DEFS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_set_roundtrip() {
        let mut params = ChainParams::defaults();
        assert_eq!(params.get_u64("base_fee_floor"), Some(1));

        let old = params.set("base_fee_floor", ParamValue::U64(100)).unwrap();
        assert_eq!(old, ParamValue::U64(1));
        assert_eq!(params.get_u64("base_fee_floor"), Some(100));
    }

    #[test]
    fn type_mismatch_rejected() {
        let mut params = ChainParams::defaults();
        let err = params
            .set("base_fee_floor", ParamValue::Bool(true))
            .unwrap_err();
        assert!(matches!(err, ChainParamError::TypeMismatch { .. }));
    }

    #[test]
    fn bounds_validation() {
        let mut params = ChainParams::defaults();

        let err = params
            .set("base_fee_floor", ParamValue::U64(0))
            .unwrap_err();
        assert!(matches!(err, ChainParamError::OutOfBounds { .. }));

        let err = params
            .set("base_fee_floor", ParamValue::U64(2_000_000))
            .unwrap_err();
        assert!(matches!(err, ChainParamError::OutOfBounds { .. }));

        params
            .set("base_fee_floor", ParamValue::U64(1_000_000))
            .unwrap();
        assert_eq!(params.get_u64("base_fee_floor"), Some(1_000_000));
    }

    #[test]
    fn defaults_populated() {
        let params = ChainParams::defaults();
        for def in PARAM_DEFS {
            assert!(
                params.get(def.key).is_some(),
                "missing default for {}",
                def.key
            );
        }
        assert_eq!(params.get_u64("target_gas_per_batch"), Some(15_000_000));
        assert_eq!(params.get_u64("max_stored_txs"), Some(500_000));
    }

    #[test]
    fn list_returns_all_sorted() {
        let params = ChainParams::defaults();
        let list = params.list();
        assert_eq!(list.len(), PARAM_DEFS.len());

        for i in 1..list.len() {
            assert!(list[i - 1].0 <= list[i].0);
        }
    }

    #[test]
    fn unknown_key_rejected() {
        let mut params = ChainParams::defaults();
        let err = params
            .set("nonexistent_param", ParamValue::U64(42))
            .unwrap_err();
        assert!(matches!(err, ChainParamError::UnknownKey(_)));
    }

    #[test]
    fn staking_params_defaults_and_bounds() {
        let mut params = ChainParams::defaults();

        assert_eq!(
            params.get_u64("min_validator_stake"),
            Some(50_000_000_000_000)
        );
        assert_eq!(
            params.get_u64("max_stake_cap"),
            Some(50_000_000_000_000_000)
        );
        assert_eq!(params.get_u64("validator_commission_bps"), Some(1000));

        params
            .set("validator_commission_bps", ParamValue::U64(0))
            .unwrap();
        assert_eq!(params.get_u64("validator_commission_bps"), Some(0));

        let err = params
            .set("validator_commission_bps", ParamValue::U64(5000))
            .unwrap_err();
        assert!(matches!(err, ChainParamError::OutOfBounds { .. }));
    }

    #[test]
    fn epoch_length_param_defaults_and_bounds() {
        let mut params = ChainParams::defaults();
        assert_eq!(params.get_u64("epoch_length"), Some(1_000));

        params.set("epoch_length", ParamValue::U64(5_000)).unwrap();
        assert_eq!(params.get_u64("epoch_length"), Some(5_000));

        let err = params.set("epoch_length", ParamValue::U64(50)).unwrap_err();
        assert!(matches!(err, ChainParamError::OutOfBounds { .. }));

        let err = params
            .set("epoch_length", ParamValue::U64(200_000))
            .unwrap_err();
        assert!(matches!(err, ChainParamError::OutOfBounds { .. }));
    }

    #[test]
    fn set_from_str_parses_correctly() {
        let mut params = ChainParams::defaults();

        params.set_from_str("max_block_range", "500").unwrap();
        assert_eq!(params.get_u64("max_block_range"), Some(500));

        let err = params
            .set_from_str("base_fee_floor", "not_a_number")
            .unwrap_err();
        assert!(matches!(err, ChainParamError::TypeMismatch { .. }));

        let err = params.set_from_str("unknown_key", "42").unwrap_err();
        assert!(matches!(err, ChainParamError::UnknownKey(_)));
    }
}
