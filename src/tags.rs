//! Numbers of the tags registered by RFC 8949 and RFC 8742.
//!
//! A tag number carries no meaning to the encoder or the decoder, which keep
//! any tag as it is. These names only save looking the numbers up, and the
//! content each one requires is given with its name.
//!
//! # Example
//! ```rust
//! use cbor_next::{DataItem, tags};
//!
//! let time = DataItem::tag(tags::EPOCH_TIME, 1_363_896_240);
//! assert_eq!(time.encode(), [0xc1, 0x1a, 0x51, 0x4b, 0x67, 0xb0]);
//! assert_eq!(time.tag_numbers(), [tags::EPOCH_TIME]);
//! ```

/// Tag 0, a date and time as the text string of RFC 3339.
pub const DATE_TIME_STRING: u64 = 0;

/// Tag 1, a date and time as the seconds since the epoch, an integer or a
/// float.
pub const EPOCH_TIME: u64 = 1;

/// Tag 2, an unsigned integer of any size as a big endian byte string.
pub const UNSIGNED_BIGNUM: u64 = 2;

/// Tag 3, a negative integer of any size as the big endian byte string of
/// `-1 - value`.
pub const NEGATIVE_BIGNUM: u64 = 3;

/// Tag 4, a decimal fraction as an array of an exponent and a mantissa.
pub const DECIMAL_FRACTION: u64 = 4;

/// Tag 5, a bigfloat as an array of an exponent and a mantissa.
pub const BIGFLOAT: u64 = 5;

/// Tag 21, any item whose byte strings a converter to JSON should write in
/// base64url.
pub const EXPECTED_BASE64URL: u64 = 21;

/// Tag 22, any item whose byte strings a converter to JSON should write in
/// base64.
pub const EXPECTED_BASE64: u64 = 22;

/// Tag 23, any item whose byte strings a converter to JSON should write in
/// base16.
pub const EXPECTED_BASE16: u64 = 23;

/// Tag 24, a byte string holding one encoded data item.
pub const EMBEDDED_ITEM: u64 = 24;

/// Tag 32, a URI as a text string.
pub const URI: u64 = 32;

/// Tag 33, a byte string written in base64url as a text string.
pub const BASE64URL: u64 = 33;

/// Tag 34, a byte string written in base64 as a text string.
pub const BASE64: u64 = 34;

/// Tag 36, a MIME message as a text string.
pub const MIME_MESSAGE: u64 = 36;

/// Tag 63, a byte string holding an encoded sequence of data items, the form
/// of RFC 8742.
///
/// The number is registered with IANA against RFC 8742, which does not define
/// it in its text.
pub const EMBEDDED_SEQUENCE: u64 = 63;

/// Tag 55799, any item, which marks it as `CBOR` to anything which reads the
/// leading bytes `d9 d9 f7`.
pub const SELF_DESCRIBED: u64 = 55799;
