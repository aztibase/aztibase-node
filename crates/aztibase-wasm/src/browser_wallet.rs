use wasm_bindgen::prelude::*;

/// Maximum balance (in base units) that a browser wallet should hold.
/// Transactions exceeding this trigger a high-value warning.
/// Roughly equivalent to 10,000 AZTB at 18 decimals.
pub const BROWSER_SPENDING_LIMIT: u128 = 10_000_000_000_000_000_000_000;

/// Warning threshold: balances above this (in base units) trigger
/// a persistent warning that browser wallets are not suitable for
/// high-value storage. ~1,000 AZTB.
pub const HIGH_VALUE_WARNING_THRESHOLD: u128 = 1_000_000_000_000_000_000_000;

/// Per-transaction spending cap for browser wallets (100 AZTB).
pub const BROWSER_PER_TX_LIMIT: u128 = 100_000_000_000_000_000_000;

#[derive(Clone, Debug)]
pub enum BrowserWalletWarning {
    HighBalance { balance: u128, threshold: u128 },
    ExceedsPerTxLimit { amount: u128, limit: u128 },
    ExceedsSpendingLimit { balance: u128, limit: u128 },
}

pub fn check_browser_balance(balance: u128) -> Vec<BrowserWalletWarning> {
    let mut warnings = Vec::new();
    if balance > BROWSER_SPENDING_LIMIT {
        warnings.push(BrowserWalletWarning::ExceedsSpendingLimit {
            balance,
            limit: BROWSER_SPENDING_LIMIT,
        });
    }
    if balance > HIGH_VALUE_WARNING_THRESHOLD {
        warnings.push(BrowserWalletWarning::HighBalance {
            balance,
            threshold: HIGH_VALUE_WARNING_THRESHOLD,
        });
    }
    warnings
}

pub fn check_browser_tx(amount: u128) -> Option<BrowserWalletWarning> {
    if amount > BROWSER_PER_TX_LIMIT {
        Some(BrowserWalletWarning::ExceedsPerTxLimit {
            amount,
            limit: BROWSER_PER_TX_LIMIT,
        })
    } else {
        None
    }
}

#[wasm_bindgen(js_name = "checkBrowserBalance")]
pub fn js_check_browser_balance(balance_str: &str) -> JsValue {
    let balance: u128 = match balance_str.parse() {
        Ok(b) => b,
        Err(_) => return JsValue::from_str("error: invalid balance"),
    };
    let warnings = check_browser_balance(balance);
    if warnings.is_empty() {
        return JsValue::NULL;
    }
    let msgs: Vec<String> = warnings
        .iter()
        .map(|w| match w {
            BrowserWalletWarning::HighBalance { .. } => {
                "WARNING: Browser wallets are not suitable for high-value accounts. \
                 Use a hardware wallet for balances above 1,000 AZTB."
                    .to_string()
            }
            BrowserWalletWarning::ExceedsSpendingLimit { .. } => {
                "CRITICAL: Balance exceeds browser spending limit (10,000 AZTB). \
                 Transfer excess to a hardware-backed wallet immediately."
                    .to_string()
            }
            BrowserWalletWarning::ExceedsPerTxLimit { .. } => {
                "WARNING: Transaction exceeds per-tx browser limit (100 AZTB).".to_string()
            }
        })
        .collect();
    JsValue::from_str(&msgs.join("\n"))
}

#[wasm_bindgen(js_name = "checkBrowserTx")]
pub fn js_check_browser_tx(amount_str: &str) -> JsValue {
    let amount: u128 = match amount_str.parse() {
        Ok(a) => a,
        Err(_) => return JsValue::from_str("error: invalid amount"),
    };
    match check_browser_tx(amount) {
        Some(BrowserWalletWarning::ExceedsPerTxLimit { .. }) => JsValue::from_str(
            "WARNING: Transaction exceeds per-tx browser limit (100 AZTB). \
             Use a hardware wallet for large transfers.",
        ),
        _ => JsValue::NULL,
    }
}

#[wasm_bindgen(js_name = "browserSpendingLimit")]
pub fn js_browser_spending_limit() -> String {
    BROWSER_SPENDING_LIMIT.to_string()
}

#[wasm_bindgen(js_name = "browserPerTxLimit")]
pub fn js_browser_per_tx_limit() -> String {
    BROWSER_PER_TX_LIMIT.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_below_threshold_no_warnings() {
        let warnings = check_browser_balance(100);
        assert!(warnings.is_empty());
    }

    #[test]
    fn balance_above_warning_threshold() {
        let warnings = check_browser_balance(HIGH_VALUE_WARNING_THRESHOLD + 1);
        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            warnings[0],
            BrowserWalletWarning::HighBalance { .. }
        ));
    }

    #[test]
    fn balance_above_spending_limit() {
        let warnings = check_browser_balance(BROWSER_SPENDING_LIMIT + 1);
        assert_eq!(warnings.len(), 2);
        assert!(matches!(
            warnings[0],
            BrowserWalletWarning::ExceedsSpendingLimit { .. }
        ));
        assert!(matches!(
            warnings[1],
            BrowserWalletWarning::HighBalance { .. }
        ));
    }

    #[test]
    fn tx_within_limit_no_warning() {
        assert!(check_browser_tx(BROWSER_PER_TX_LIMIT).is_none());
    }

    #[test]
    fn tx_exceeds_limit_warning() {
        let warning = check_browser_tx(BROWSER_PER_TX_LIMIT + 1);
        assert!(warning.is_some());
        assert!(matches!(
            warning.unwrap(),
            BrowserWalletWarning::ExceedsPerTxLimit { .. }
        ));
    }
}
