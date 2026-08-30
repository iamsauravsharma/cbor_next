use std::hash::{DefaultHasher, Hash, Hasher as _};

use crate::content::{Array, ByteString, Map, Simple, Tag, TextString};

mod api;
mod convert;
mod fmt;

/// Enum representing different types of data item that can be encoded or
/// decoded in `CBOR` (Concise Binary Object Representation).
///
/// `CBOR` is a data format designed for small code and message size, often used
/// in constrained environments. This `DataItem` enum covers all major types
/// defined in the `CBOR` specification (RFC 8949).
///
/// # Example
/// ```rust
/// use cbor_next::DataItem;
///
/// let item = DataItem::from(vec![("name", "cbor")]);
/// let encoded = item.encode();
/// assert_eq!(DataItem::decode(&encoded).unwrap(), item);
/// ```
#[derive(PartialEq, Clone)]
#[non_exhaustive]
pub enum DataItem {
    /// Unsigned integer represented by `CBOR` major type 0.
    ///
    /// This variant can hold non-negative integer values up to `u64::MAX`.
    Unsigned(u64),
    /// Negative integer represented by `CBOR` major type 1.
    ///
    /// This variant stores the absolute value minus one of the negative
    /// integer. For example, a `CBOR` negative integer representing -1 would
    /// store `0`, -10 would store `9`. The actual negative value is derived
    /// as `-(1 + value)`. Building this variant from the value it stands for
    /// is done with `DataItem::from(-10_i64)` or, for the full `CBOR` range,
    /// `DataItem::try_from(-10_i128)`.
    Signed(u64),
    /// Byte string represented by `CBOR` major type 2.
    ///
    /// Contains an arbitrary sequence of bytes.
    Byte(ByteString),
    /// UTF-8 string (text string) represented by `CBOR` major type 3.
    ///
    /// Contains a sequence of Unicode characters encoded as UTF-8.
    Text(TextString),
    /// Array of `CBOR` data items represented by `CBOR` major type 4.
    ///
    /// An ordered sequence of zero or more `CBOR` data items.
    Array(Array),
    /// Map of `CBOR` key-value pairs represented by `CBOR` major type 5.
    ///
    /// Keys within a map must be unique
    Map(Map),
    /// Tagged item (semantic tag) represented by `CBOR` major type 6.
    ///
    /// Consists of an unsigned integer (the tag) and a single `CBOR` data item
    /// (the tagged content). Tags provide semantic information about the
    /// enclosed data item, allowing for type extension
    /// or application-specific interpretations.
    Tag(Tag),
    /// Boolean represented as a simple value within `CBOR` major type 7.
    ///
    /// Can be either `true` or `false`.
    Boolean(bool),
    /// Null represented as a simple value within `CBOR` major type 7.
    ///
    /// Represents the absence of a value.
    Null,
    /// Undefined represented as a simple value within `CBOR` major type
    /// 7.
    ///
    /// Distinct from `Null`, it represents an undefined state.
    Undefined,
    /// Floating-point number represented as a simple value within `CBOR` major
    /// type 7.
    ///
    /// Can represent half-precision (16-bit), single-precision (32-bit), or
    /// double-precision (64-bit) floating-point numbers. but locally saves
    /// data as f64
    Floating(f64),
    /// An generic simple value represented by `CBOR` major type 7.
    ///
    /// This variant handles simple values that are not explicitly covered by
    /// `Boolean`, `Null`, `Undefined`, or `Floating`. These generic simple
    /// values have a numerical representation as defined in the `CBOR`
    /// specification.
    GenericSimple(Simple),
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
                map_content.is_indefinite().hash(state);
                map_content.entries().len().hash(state);
                // entries are compared without regard to their order, so they
                // must be hashed with an order independent
                // combination as well
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
                // normalize -0.0 and 0.0 to the same representation for hashing
                let normalized = if *val == 0.0 { 0.0 } else { *val };
                normalized.to_be_bytes().hash(state);
            }
            Self::GenericSimple(simple_number) => simple_number.hash(state),
            Self::Null | Self::Undefined => {}
        }
    }
}
