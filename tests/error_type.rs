//! RED test for issue #4 — the crate's public library error type.
//!
//! This is an infra behavior (no JS reference vector applies): it pins the
//! PUBLIC contract of `color_convert_rs::Error` that all conversion routes
//! will return via `Result<_, Error>` (AGENTS.md "Error handling").
//!
//! CONTRACT for GREEN (implement exactly this, via `thiserror`):
//!
//! ```ignore
//! pub enum Error {
//!     /// Input that cannot be interpreted at all (wrong shape, unparseable).
//!     /// Display: non-empty, human-readable, includes `message`.
//!     InvalidInput { message: String },
//!     /// A channel value outside its valid range.
//!     /// Display: non-empty, mentions the channel name and offending value,
//!     /// e.g. `channel 'r' value 300 out of range`.
//!     OutOfRange { channel: &'static str, value: f64 },
//! }
//! ```
//!
//! Required derives/impls asserted below: `Debug`, `PartialEq`, `Display`
//! (non-empty), and `std::error::Error`.

use color_convert_rs::Error;

#[test]
fn error_enum_exposes_invalid_input_and_out_of_range_contract() {
    // 1. Both variants are constructible with the contracted field shapes.
    let out_of_range = Error::OutOfRange {
        channel: "r",
        value: 300.0,
    };
    let invalid_input = Error::InvalidInput {
        message: String::from("expected 3 channels, got 2"),
    };

    // 2. Implements `std::error::Error` (usable behind a trait object).
    let _: &dyn std::error::Error = &out_of_range;
    let _: &dyn std::error::Error = &invalid_input;

    // 3. Display is non-empty and human-readable, mentioning the offending
    //    channel/value (OutOfRange) and the message (InvalidInput).
    let oor_msg = out_of_range.to_string();
    assert!(!oor_msg.is_empty(), "OutOfRange Display must be non-empty");
    assert!(
        oor_msg.contains("r") && oor_msg.contains("300"),
        "OutOfRange Display must mention channel and value, got: {oor_msg:?}"
    );

    let inv_msg = invalid_input.to_string();
    assert!(
        !inv_msg.is_empty(),
        "InvalidInput Display must be non-empty"
    );
    assert!(
        inv_msg.contains("expected 3 channels, got 2"),
        "InvalidInput Display must include the message, got: {inv_msg:?}"
    );

    // 4. Derives Debug + PartialEq (equality of identically-built variants).
    assert_eq!(
        out_of_range,
        Error::OutOfRange {
            channel: "r",
            value: 300.0,
        },
        "identically constructed OutOfRange values must compare equal"
    );
    assert_ne!(
        out_of_range,
        Error::InvalidInput {
            message: String::from("expected 3 channels, got 2"),
        },
        "different variants must not compare equal"
    );
}
