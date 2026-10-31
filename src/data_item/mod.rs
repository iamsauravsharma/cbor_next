use std::hash::{DefaultHasher, Hash, Hasher as _};

use crate::content::{Array, ByteString, Map, Simple, Tag, TextString};

mod api;
pub(crate) mod convert;
mod fmt;

/// A `CBOR` data item (RFC 8949).
///
/// # Example
/// ```rust
/// use cbor_next::DataItem;
///
/// let item = DataItem::from(vec![("name", "cbor")]);
/// let encoded = item.encode();
/// assert_eq!(DataItem::decode(&encoded).unwrap(), item);
/// ```
#[derive(Clone)]
#[non_exhaustive]
pub enum DataItem {
    /// Unsigned integer, major type 0.
    Unsigned(u64),
    /// Negative integer, major type 1, stored as `n` for the value `-1 - n`.
    ///
    /// Build it with `DataItem::from(-10_i64)`; an `i128` past this range
    /// becomes a bignum.
    Signed(u64),
    /// Byte string, major type 2.
    Byte(ByteString),
    /// Text string, major type 3.
    Text(TextString),
    /// Array, major type 4.
    Array(Array),
    /// Map, major type 5.
    Map(Map),
    /// Tag, major type 6.
    Tag(Tag),
    /// `false` or `true`, simple values 20 and 21.
    Boolean(bool),
    /// `null`, simple value 22.
    Null,
    /// `undefined`, simple value 23.
    Undefined,
    /// Floating point number of any width, held as `f64`.
    Floating(f64),
    /// Any other simple value.
    GenericSimple(Simple),
}

/// Bits of an `f64` which hold its significand
const NAN_SIGNIFICAND: u64 = (1 << 52) - 1;

impl PartialEq for DataItem {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Unsigned(first), Self::Unsigned(second))
            | (Self::Signed(first), Self::Signed(second)) => first == second,
            (Self::Byte(first), Self::Byte(second)) => first == second,
            (Self::Text(first), Self::Text(second)) => first == second,
            (Self::Array(first), Self::Array(second)) => first == second,
            (Self::Map(first), Self::Map(second)) => first == second,
            (Self::Tag(first), Self::Tag(second)) => first == second,
            (Self::Boolean(first), Self::Boolean(second)) => first == second,
            (Self::Null, Self::Null) | (Self::Undefined, Self::Undefined) => true,
            (Self::Floating(first), Self::Floating(second)) => {
                // RFC 8949 section 5.6.1: `-0.0 == 0.0`, NaNs equal by
                // significand
                if first.is_nan() || second.is_nan() {
                    first.is_nan()
                        && second.is_nan()
                        && first.to_bits() & NAN_SIGNIFICAND == second.to_bits() & NAN_SIGNIFICAND
                } else {
                    first == second
                }
            }
            (Self::GenericSimple(first), Self::GenericSimple(second)) => first == second,
            _ => false,
        }
    }
}

impl Eq for DataItem {}

impl Hash for DataItem {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Unsigned(val) | Self::Signed(val) => val.hash(state),
            Self::Byte(items) => items.hash(state),
            Self::Text(text) => text.hash(state),
            Self::Array(values) => values.hash(state),
            Self::Map(map_content) => {
                map_content.entries().len().hash(state);
                // equality ignores entry order, so the hash must too
                let mut combined: u64 = 0;
                for entry in map_content.entries() {
                    let mut entry_hasher = DefaultHasher::new();
                    entry.hash(&mut entry_hasher);
                    combined = combined.wrapping_add(entry_hasher.finish());
                }
                state.write_u64(combined);
            }
            Self::Tag(tag_content) => tag_content.hash(state),
            Self::Boolean(val) => val.hash(state),
            Self::Floating(val) => {
                if val.is_nan() {
                    (val.to_bits() & NAN_SIGNIFICAND).hash(state);
                } else {
                    // `-0.0` and `0.0` hash alike
                    let normalized = if *val == 0.0 { 0.0 } else { *val };
                    normalized.to_bits().hash(state);
                }
            }
            Self::GenericSimple(simple_number) => simple_number.hash(state),
            Self::Null | Self::Undefined => {}
        }
    }
}
