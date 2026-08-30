//! Deterministic, also called canonical, encoding of data items.
//!
//! Two encoders which follow the same deterministic mode turn equal values
//! into equal bytes, which is what comparing, hashing or signing an encoded
//! value relies on.

use std::cmp::Ordering;

use crate::content::{Array, ByteString, Map, TextString};
use crate::data_item::DataItem;
use crate::encode::Encoder;

/// Ordering a deterministic encoding applies to the keys of a map.
///
/// # Example
/// ```rust
/// use cbor_next::{DataItem, DeterministicMode, Encoder};
///
/// // the key `100` encodes to `1864` and the key `false` to `f4`
/// let item = DataItem::map([(DataItem::from(100), "a"), (DataItem::from(false), "b")]);
/// // core ordering compares the encoded keys byte by byte
/// assert_eq!(
///     Encoder::new()
///         .deterministic(DeterministicMode::Core)
///         .encode(&item),
///     DataItem::map([(DataItem::from(100), "a"), (DataItem::from(false), "b")]).encode()
/// );
/// // length first ordering puts the shorter key first
/// assert_eq!(
///     Encoder::new()
///         .deterministic(DeterministicMode::LengthFirst)
///         .encode(&item),
///     DataItem::map([(DataItem::from(false), "b"), (DataItem::from(100), "a")]).encode()
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum DeterministicMode {
    /// Sort keys by their encoded bytes, the core deterministic encoding of
    /// RFC 8949 section 4.2.1.
    #[default]
    Core,
    /// Sort shorter encoded keys first and compare keys of the same length by
    /// their bytes, the length first ordering of RFC 8949 section 4.2.3.
    LengthFirst,
}

impl DeterministicMode {
    /// Compare two encoded map keys the way this mode orders them.
    ///
    /// # Example
    /// ```rust
    /// use std::cmp::Ordering;
    ///
    /// use cbor_next::DeterministicMode;
    ///
    /// let short = [0x01];
    /// let long = [0x00, 0x00];
    /// assert_eq!(
    ///     DeterministicMode::Core.compare_keys(&short, &long),
    ///     Ordering::Greater
    /// );
    /// assert_eq!(
    ///     DeterministicMode::LengthFirst.compare_keys(&short, &long),
    ///     Ordering::Less
    /// );
    /// ```
    #[must_use]
    pub fn compare_keys(self, first: &[u8], second: &[u8]) -> Ordering {
        match self {
            Self::Core => first.cmp(second),
            Self::LengthFirst => {
                first
                    .len()
                    .cmp(&second.len())
                    .then_with(|| first.cmp(second))
            }
        }
    }
}

/// Whether an item is already in the deterministic form of the provided mode.
pub(crate) fn is_deterministic(item: &DataItem, mode: DeterministicMode) -> bool {
    let encoder = Encoder::new().deterministic(mode);
    match item {
        DataItem::Map(map) => {
            if map.is_indefinite() {
                return false;
            }
            let keys = map
                .entries()
                .keys()
                .map(|key| encoder.encode(key))
                .collect::<Vec<_>>();
            let sorted = keys
                .windows(2)
                .all(|pair| mode.compare_keys(&pair[0], &pair[1]).is_lt());
            sorted
                && map.entries().iter().all(|(key, value)| {
                    is_deterministic(key, mode) && is_deterministic(value, mode)
                })
        }
        DataItem::Array(array) => {
            !array.is_indefinite()
                && array
                    .items()
                    .iter()
                    .all(|value| is_deterministic(value, mode))
        }
        DataItem::Tag(tag) => is_deterministic(tag.content(), mode),
        DataItem::Byte(byte_string) => !byte_string.is_indefinite(),
        DataItem::Text(text_string) => !text_string.is_indefinite(),
        _ => true,
    }
}

/// Rewrite an item into the deterministic form of the provided mode.
pub(crate) fn into_deterministic(item: DataItem, mode: DeterministicMode) -> DataItem {
    let encoder = Encoder::new().deterministic(mode);
    match item {
        DataItem::Map(map) => {
            let mut entries = map
                .into_entries()
                .into_iter()
                .map(|(key, value)| {
                    (
                        into_deterministic(key, mode),
                        into_deterministic(value, mode),
                    )
                })
                .map(|(key, value)| (encoder.encode(&key), key, value))
                .collect::<Vec<_>>();
            entries.sort_by(|(first, ..), (second, ..)| mode.compare_keys(first, second));
            DataItem::Map(Map::from_entries(
                entries.into_iter().map(|(_, key, value)| (key, value)),
            ))
        }
        DataItem::Array(array) => {
            DataItem::Array(Array::from_items(
                array
                    .into_items()
                    .into_iter()
                    .map(|value| into_deterministic(value, mode)),
            ))
        }
        DataItem::Tag(tag) => {
            let number = tag.number();
            DataItem::tag(number, into_deterministic(tag.into_content(), mode))
        }
        DataItem::Byte(byte_string) => DataItem::Byte(ByteString::new(byte_string.to_bytes())),
        DataItem::Text(text_string) => DataItem::Text(TextString::new(text_string.to_text())),
        _ => item,
    }
}
