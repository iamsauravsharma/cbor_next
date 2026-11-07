//! Tag numbers of RFC 8949, RFC 8742, and the RFC 8943 dates and RFC 9164 IP
//! addresses.
//!
//! Names only; the encoder and decoder keep any tag as it is.
//!
//! # Example
//! ```rust
//! use cbor_next::{DataItem, tags};
//!
//! let time = DataItem::tag(tags::EPOCH_TIME, 1_363_896_240);
//! assert_eq!(time.encode(), [0xc1, 0x1a, 0x51, 0x4b, 0x67, 0xb0]);
//! assert_eq!(time.tag_numbers(), [tags::EPOCH_TIME]);
//! ```

/// Tag 0, an RFC 3339 date and time text.
pub const DATE_TIME_STRING: u64 = 0;

/// Tag 1, seconds since the epoch, integer or float.
pub const EPOCH_TIME: u64 = 1;

/// Tag 2, unsigned bignum as big endian bytes.
pub const UNSIGNED_BIGNUM: u64 = 2;

/// Tag 3, negative bignum: big endian bytes of `n` for `-1 - n`.
pub const NEGATIVE_BIGNUM: u64 = 3;

/// Tag 4, decimal fraction `[exponent, mantissa]`.
pub const DECIMAL_FRACTION: u64 = 4;

/// Tag 5, bigfloat `[exponent, mantissa]`.
pub const BIGFLOAT: u64 = 5;

/// Tag 21, write byte strings inside as base64url in JSON.
pub const EXPECTED_BASE64URL: u64 = 21;

/// Tag 22, write byte strings inside as base64 in JSON.
pub const EXPECTED_BASE64: u64 = 22;

/// Tag 23, write byte strings inside as base16 in JSON.
pub const EXPECTED_BASE16: u64 = 23;

/// Tag 24, bytes holding one encoded item.
pub const EMBEDDED_ITEM: u64 = 24;

/// Tag 32, URI text.
pub const URI: u64 = 32;

/// Tag 33, base64url text.
pub const BASE64URL: u64 = 33;

/// Tag 34, base64 text.
pub const BASE64: u64 = 34;

/// Tag 36, MIME message text.
pub const MIME_MESSAGE: u64 = 36;

/// Tag 52, an IPv4 address, prefix or interface (RFC 9164).
pub const IPV4: u64 = 52;

/// Tag 54, an IPv6 address, prefix or interface (RFC 9164).
pub const IPV6: u64 = 54;

/// Tag 63, bytes holding an encoded sequence (RFC 8742).
pub const EMBEDDED_SEQUENCE: u64 = 63;

/// Tag 100, days since 1970-01-01, an integer (RFC 8943).
pub const EPOCH_DAYS: u64 = 100;

/// Tag 1004, an RFC 3339 `full-date` text (RFC 8943).
pub const DATE_STRING: u64 = 1004;

/// Tag 55799, self-described `CBOR` (leading bytes `d9 d9 f7`).
pub const SELF_DESCRIBED: u64 = 55799;
