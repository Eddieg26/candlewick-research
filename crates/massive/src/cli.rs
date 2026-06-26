use clap::{Parser, Subcommand};
use crate::error::MassiveError;

#[derive(Parser)]
#[command(name = "massive", about = "Massive Forex API CLI client")]
pub struct Cli {
    /// API key (overrides MASSIVE_API_KEY env var)
    #[arg(long, global = true)]
    pub token: Option<String>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Search for forex pairs by keyword
    Search {
        /// Search query string (e.g., "EUR" or "USD")
        query: String,
        /// Maximum results to return (default 100, max 1000)
        #[arg(long, default_value = "100")]
        limit: u32,
    },
    /// Fetch historical OHLC bars
    Bars {
        /// Forex ticker (e.g., C:EURUSD)
        ticker: String,
        /// Size of timespan multiplier (e.g., 1)
        multiplier: u32,
        /// Timespan unit: minute, hour, day, week, month, quarter, year
        timespan: String,
        /// Start date (YYYY-MM-DD)
        from: String,
        /// End date (YYYY-MM-DD)
        to: String,
        /// Sort order: asc or desc
        #[arg(long, default_value = "asc")]
        sort: String,
        /// Max results (default 5000, max 50000)
        #[arg(long, default_value = "5000")]
        limit: u32,
    },
    /// Get last quote for a currency pair
    Quote {
        /// Source currency (e.g., EUR)
        from: String,
        /// Target currency (e.g., USD)
        to: String,
    },
    /// Convert between currencies
    Convert {
        /// Source currency (e.g., AUD)
        from: String,
        /// Target currency (e.g., USD)
        to: String,
        /// Amount to convert (default 1.0)
        #[arg(long, default_value = "1.0")]
        amount: f64,
        /// Decimal precision (default 2)
        #[arg(long, default_value = "2")]
        precision: u32,
    },
    /// Get ticker snapshot (current day/quote/prev day)
    Snapshot {
        /// Forex ticker (e.g., C:EURUSD)
        ticker: String,
    },
    /// Stream real-time forex minute aggregates via WebSocket
    Live {
        /// One or more currency pairs (e.g., C:EUR-USD C:GBP-USD)
        #[arg(required = true)]
        pairs: Vec<String>,
    },
}

/// Resolve the API key from the --token flag or MASSIVE_API_KEY env var.
/// Trims leading/trailing whitespace before the emptiness check and in the returned value.
pub fn resolve_token(flag: Option<String>) -> Result<String, MassiveError> {
    let token = flag.unwrap_or_else(|| std::env::var("MASSIVE_API_KEY").unwrap_or_default());
    let trimmed = token.trim().to_string();
    if trimmed.is_empty() {
        return Err(MassiveError::NoApiKey);
    }
    Ok(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// **Validates: Requirements 1.1, 1.2, 1.3, 1.4**
    ///
    /// Property 1: Token resolution precedence
    /// - Flag with non-whitespace value always wins; absent flag falls back to env var;
    ///   both empty returns NoApiKey. Whitespace is always trimmed.
    mod token_resolution {
        use super::*;
        use std::sync::Mutex;

        /// Mutex to serialize env-var-dependent tests. Since env vars are process-global,
        /// tests that modify MASSIVE_API_KEY must not run concurrently.
        static ENV_MUTEX: Mutex<()> = Mutex::new(());

        // Helper functions to wrap unsafe env var operations.
        fn clear_env() {
            // SAFETY: caller holds ENV_MUTEX; no concurrent access.
            unsafe { std::env::remove_var("MASSIVE_API_KEY"); }
        }

        fn set_env(val: &str) {
            // SAFETY: caller holds ENV_MUTEX; no concurrent access.
            unsafe { std::env::set_var("MASSIVE_API_KEY", val); }
        }

        /// Strategy that generates non-empty strings (after trimming).
        /// Produces a core of at least one visible char, optionally wrapped in whitespace.
        fn non_empty_after_trim() -> impl Strategy<Value = String> {
            (
                "[^ \t\n\r\x0b\x0c]{1,64}",       // at least one non-whitespace char
                "[ \t]{0,4}",                       // optional leading whitespace
                "[ \t]{0,4}",                       // optional trailing whitespace
            )
                .prop_map(|(core, leading, trailing)| format!("{leading}{core}{trailing}"))
        }

        /// Strategy for env-var-safe non-empty strings (no NUL bytes).
        /// Windows SetEnvironmentVariable rejects strings containing NUL characters.
        fn env_safe_non_empty() -> impl Strategy<Value = String> {
            (
                "[a-zA-Z0-9_\\-\\.!@#$%^&*]{1,32}", // printable, NUL-free core
                "[ \t]{0,4}",                         // optional leading whitespace
                "[ \t]{0,4}",                         // optional trailing whitespace
            )
                .prop_map(|(core, leading, trailing)| format!("{leading}{core}{trailing}"))
        }

        /// Strategy that generates whitespace-only strings (empty after trim).
        fn whitespace_only() -> impl Strategy<Value = String> {
            prop::collection::vec(prop_oneof![Just(' '), Just('\t'), Just('\n'), Just('\r')], 0..8)
                .prop_map(|chars| chars.into_iter().collect::<String>())
        }

        proptest! {
            /// When flag is Some(s) and s.trim() is non-empty, resolve_token returns Ok(s.trim()).
            /// The flag value always takes precedence — env var state is irrelevant.
            #[test]
            fn flag_with_non_empty_value_always_wins(flag_val in non_empty_after_trim()) {
                let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
                clear_env();

                let result = resolve_token(Some(flag_val.clone()));
                prop_assert!(result.is_ok(), "Expected Ok, got {:?}", result);
                let resolved = result.unwrap();
                prop_assert_eq!(&resolved, flag_val.trim());
                // Verify no leading/trailing whitespace in result
                prop_assert_eq!(resolved.trim(), resolved.as_str());
            }

            /// When flag is Some(s) and s.trim() is non-empty, the result equals s.trim()
            /// even when MASSIVE_API_KEY env var is set to something else.
            #[test]
            fn flag_takes_precedence_over_env(
                flag_val in non_empty_after_trim(),
                env_val in env_safe_non_empty(),
            ) {
                let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
                set_env(&env_val);
                let result = resolve_token(Some(flag_val.clone()));
                clear_env();

                prop_assert!(result.is_ok());
                let resolved = result.unwrap();
                // Flag wins — result is the trimmed flag, not the env value
                prop_assert_eq!(&resolved, flag_val.trim());
            }

            /// When flag is Some but whitespace-only, resolve_token returns NoApiKey error.
            #[test]
            fn flag_whitespace_only_returns_no_api_key(flag_val in whitespace_only()) {
                let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
                clear_env();

                let result = resolve_token(Some(flag_val));
                prop_assert!(result.is_err(), "Expected Err(NoApiKey), got {:?}", result);
            }

            /// When flag is None and env var is set to non-empty, resolve_token returns
            /// the trimmed env var value.
            #[test]
            fn none_flag_falls_back_to_env(env_val in env_safe_non_empty()) {
                let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
                set_env(&env_val);
                let result = resolve_token(None);
                clear_env();

                prop_assert!(result.is_ok(), "Expected Ok, got {:?}", result);
                let resolved = result.unwrap();
                prop_assert_eq!(&resolved, env_val.trim());
                prop_assert_eq!(resolved.trim(), resolved.as_str());
            }

            /// When flag is None and env var is empty/whitespace-only, returns NoApiKey.
            #[test]
            fn none_flag_and_empty_env_returns_no_api_key(env_val in whitespace_only()) {
                let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
                set_env(&env_val);
                let result = resolve_token(None);
                clear_env();

                prop_assert!(result.is_err(), "Expected Err(NoApiKey), got {:?}", result);
            }
        }

        /// When flag is None and env var is unset entirely, returns NoApiKey.
        #[test]
        fn none_flag_and_unset_env_returns_no_api_key() {
            let _lock = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
            clear_env();
            let result = resolve_token(None);
            assert!(result.is_err());
        }
    }
}
