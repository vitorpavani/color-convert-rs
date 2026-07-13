//! The crate's library error type.
//!
//! Every conversion route returns `Result<_, Error>` (AGENTS.md "Error
//! handling"). Two failure classes exist: input that cannot be interpreted
//! at all (`InvalidInput`) and a channel value outside its valid range
//! (`OutOfRange`). Display messages are human-readable and mention the
//! offending message/channel/value, as pinned by `tests/error_type.rs`.

/// Library error for all conversion routes.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum Error {
    /// Input that cannot be interpreted at all (wrong shape, unparseable).
    #[error("invalid input: {message}")]
    InvalidInput {
        /// Human-readable description of what was wrong with the input.
        message: String,
    },
    /// A channel value outside its valid range.
    #[error("channel '{channel}' value {value} out of range")]
    OutOfRange {
        /// The channel name, e.g. `"r"`.
        channel: &'static str,
        /// The offending value.
        value: f64,
    },
}
