use std::cmp::Ordering;
use std::fmt::{Debug, Write as _};
use std::hash::{DefaultHasher, Hash, Hasher as _};
use std::num::TryFromIntError;
use std::slice::Iter;

use indexmap::IndexMap;

use crate::content::{ArrayContent, ByteContent, MapContent, SimpleValue, TagContent, TextContent};
use crate::deterministic::DeterministicMode;
use crate::error::Error;

/// Enum representing different types of data item that can be encoded or
/// decoded in `CBOR` (Concise Binary Object Representation).
///
/// `CBOR` is a data format designed for small code and message size, often used
/// in constrained environments. This `DataItem` enum covers all major types
/// defined in the `CBOR` specification (RFC 8949).
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
    /// as `-(1 + value)`.
    Signed(u64),
    /// Byte string represented by `CBOR` major type 2.
    ///
    /// Contains an arbitrary sequence of bytes.
    Byte(ByteContent),
    /// UTF-8 string (text string) represented by `CBOR` major type 3.
    ///
    /// Contains a sequence of Unicode characters encoded as UTF-8.
    Text(TextContent),
    /// Array of `CBOR` data items represented by `CBOR` major type 4.
    ///
    /// An ordered sequence of zero or more `CBOR` data items.
    Array(ArrayContent),
    /// Map of `CBOR` key-value pairs represented by `CBOR` major type 5.
    ///
    /// Keys within a map must be unique
    Map(MapContent),
    /// Tagged item (semantic tag) represented by `CBOR` major type 6.
    ///
    /// Consists of an unsigned integer (the tag) and a single `CBOR` data item
    /// (the tagged content). Tags provide semantic information about the
    /// enclosed data item, allowing for type extension
    /// or application-specific interpretations.
    Tag(TagContent),
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
    GenericSimple(SimpleValue),
}

impl Debug for DataItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsigned(number) => number.fmt(f),
            Self::Signed(number) => (-(i128::from(*number) + 1)).fmt(f),
            Self::Floating(number) => {
                if number.is_nan() {
                    return write!(f, "NaN");
                }
                match *number {
                    f64::INFINITY => write!(f, "Infinity"),
                    f64::NEG_INFINITY => write!(f, "-Infinity"),
                    _ => number.fmt(f),
                }
            }
            Self::Boolean(bool_val) => bool_val.fmt(f),
            Self::Null => write!(f, "null"),
            Self::Undefined => write!(f, "undefined"),
            Self::GenericSimple(simple_number) => simple_number.fmt(f),
            Self::Byte(bytes) => {
                if bytes.is_indefinite() {
                    write!(f, "(_ ")?;
                    let mut chunk_contents = vec![];
                    for chunk in bytes.chunk() {
                        let mut content = "h'".to_string();
                        for byte in chunk {
                            write!(content, "{byte:02x}")?;
                        }
                        content.push('\'');
                        chunk_contents.push(content);
                    }
                    let content = chunk_contents.join(", ");
                    write!(f, "{content}")?;
                    write!(f, ")")
                } else {
                    write!(f, "h'")?;
                    for byte in bytes.full() {
                        write!(f, "{byte:02x}")?;
                    }
                    write!(f, "'")
                }
            }
            Self::Text(text_content) => {
                if text_content.is_indefinite() {
                    write!(f, "(_ ")?;
                    let mut chunk_contents = vec![];
                    for chunk in text_content.chunk() {
                        chunk_contents.push(format!("{chunk:?}"));
                    }
                    let content = chunk_contents.join(", ");
                    write!(f, "{content}")?;
                    write!(f, ")")
                } else {
                    write!(f, "{:?}", text_content.full())
                }
            }
            Self::Array(array) => {
                let mut array_item_vec = vec![];
                for item in array.array() {
                    array_item_vec.push(format!("{item:?}"));
                }
                let array_item_str = array_item_vec.join(", ");
                if array.is_indefinite() {
                    write!(f, "[_ {array_item_str}]")
                } else {
                    write!(f, "[{array_item_str}]")
                }
            }
            Self::Map(map) => {
                let mut array_item_vec = vec![];
                for (key, value) in map.map() {
                    array_item_vec.push(format!("{key:?}: {value:?}"));
                }
                let array_item_str = array_item_vec.join(", ");
                if map.is_indefinite() {
                    write!(f, "{{_ {array_item_str}}}")
                } else {
                    write!(f, "{{{array_item_str}}}")
                }
            }
            Self::Tag(tag_content) => {
                write!(f, "{:?}({:?})", tag_content.number(), tag_content.content())
            }
        }
    }
}

impl Hash for DataItem {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Self::Unsigned(val) | Self::Signed(val) => val.hash(state),
            Self::Byte(items) => items.hash(state),
            Self::Text(text) => text.hash(state),
            Self::Array(values) => values.hash(state),
            Self::Map(index_map) => {
                index_map.is_indefinite().hash(state);
                index_map.map().len().hash(state);
                let mut combined: u64 = 0;
                for entry in index_map.map() {
                    let mut entry_hasher = DefaultHasher::new();
                    entry.hash(&mut entry_hasher);
                    combined = combined.wrapping_add(entry_hasher.finish());
                }
                state.write_u64(combined);
            }
            Self::Tag(tag_content) => {
                tag_content.number().hash(state);
                tag_content.content().hash(state);
            }
            Self::Boolean(val) => val.hash(state),
            Self::Floating(val) => {
                // normalize -0.0 and 0.0 to the same representation for hashing
                let normalized = if *val == 0.0 { 0.0 } else { *val };
                normalized.to_be_bytes().hash(state);
            }
            Self::GenericSimple(simple_number) => simple_number.hash(state),
            _ => {}
        }
    }
}

impl Eq for DataItem {}

impl From<u64> for DataItem {
    fn from(value: u64) -> Self {
        Self::Unsigned(value)
    }
}

macro_rules! impl_from {
    ($i:ident, $($t:ty),+) => {
        $(
        impl From<$t> for DataItem {
            fn from(value: $t) -> Self {
                $i::from(value).into()
            }
        }
    )+
    };
}

impl_from!(u64, u32, u16, u8);

impl TryFrom<u128> for DataItem {
    type Error = TryFromIntError;

    fn try_from(value: u128) -> Result<Self, Self::Error> {
        Ok(u64::try_from(value)?.into())
    }
}

impl From<i64> for DataItem {
    fn from(value: i64) -> Self {
        if value.is_negative() {
            let positive_val = -value - 1;
            let u64_val =
                u64::try_from(positive_val).expect("i64 positive can be converted to u64");
            Self::Signed(u64_val)
        } else {
            let u64_val = u64::try_from(value).expect("i64 positive can be converted to u64");
            Self::Unsigned(u64_val)
        }
    }
}

impl_from!(i64, i32, i16, i8);

impl TryFrom<i128> for DataItem {
    type Error = TryFromIntError;

    fn try_from(value: i128) -> Result<Self, Self::Error> {
        if value.is_negative() {
            let positive_val = -value - 1;
            Ok(Self::Signed(u64::try_from(positive_val)?))
        } else {
            Ok(Self::Unsigned(u64::try_from(value)?))
        }
    }
}

impl From<&[u8]> for DataItem {
    fn from(value: &[u8]) -> Self {
        Self::Byte(value.to_vec().into())
    }
}

impl From<String> for DataItem {
    fn from(value: String) -> Self {
        Self::Text(value.into())
    }
}

impl From<&str> for DataItem {
    fn from(value: &str) -> Self {
        Self::Text(value.into())
    }
}

impl From<bool> for DataItem {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<f64> for DataItem {
    fn from(value: f64) -> Self {
        Self::Floating(value)
    }
}

impl_from!(f64, f32, half::f16);

impl From<ArrayContent> for DataItem {
    fn from(value: ArrayContent) -> Self {
        Self::Array(value)
    }
}

impl<T> From<Vec<T>> for DataItem
where
    T: Into<DataItem>,
{
    fn from(value: Vec<T>) -> Self {
        ArrayContent::from(value.into_iter().map(Into::into).collect::<Vec<_>>()).into()
    }
}

impl From<MapContent> for DataItem {
    fn from(value: MapContent) -> Self {
        Self::Map(value)
    }
}

impl<T, U> From<Vec<(T, U)>> for DataItem
where
    T: Into<DataItem> + Hash + Eq,
    U: Into<DataItem>,
{
    fn from(value: Vec<(T, U)>) -> Self {
        IndexMap::from_iter(value).into()
    }
}

impl<T, U> From<IndexMap<T, U>> for DataItem
where
    T: Into<DataItem>,
    U: Into<DataItem>,
{
    fn from(value: IndexMap<T, U>) -> Self {
        MapContent::from(
            value
                .into_iter()
                .map(|(t, u)| (t.into(), u.into()))
                .collect::<IndexMap<_, _>>(),
        )
        .into()
    }
}

impl From<TagContent> for DataItem {
    fn from(value: TagContent) -> Self {
        Self::Tag(value)
    }
}

impl From<SimpleValue> for DataItem {
    fn from(value: SimpleValue) -> Self {
        Self::GenericSimple(value)
    }
}

impl<T> From<&T> for DataItem
where
    T: Into<DataItem> + Clone,
{
    fn from(value: &T) -> Self {
        value.clone().into()
    }
}

impl DataItem {
    /// Is a unsigned integer value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::from(20).is_unsigned_integer());
    /// ```
    #[must_use]
    pub fn is_unsigned_integer(&self) -> bool {
        matches!(self, Self::Unsigned(_))
    }

    /// Is a signed integer value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::from(-32).is_signed_integer());
    /// ```
    #[must_use]
    pub fn is_signed_integer(&self) -> bool {
        matches!(self, Self::Signed(_))
    }

    /// Is a integer? Can be both signed as well as unsigned
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::from(-32).is_integer());
    /// assert!(DataItem::from(31).is_integer());
    /// ```
    #[must_use]
    pub fn is_integer(&self) -> bool {
        matches!(self, Self::Unsigned(_) | Self::Signed(_))
    }

    /// Is a byte value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::from(vec![65u8, 63, 62].as_slice()).is_byte());
    /// ```
    #[must_use]
    pub fn is_byte(&self) -> bool {
        matches!(self, Self::Byte(_))
    }

    /// Is a text value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::from("example").is_text());
    /// ```
    #[must_use]
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Is a array value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::from(vec![12]).is_array());
    /// ```
    #[must_use]
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    /// Is a map value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    /// use indexmap::IndexMap;
    ///
    /// assert!(DataItem::from(IndexMap::from_iter(vec![(12, "a")])).is_map());
    /// ```
    #[must_use]
    pub fn is_map(&self) -> bool {
        matches!(self, Self::Map(_))
    }

    /// Is a tag value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::{DataItem, TagContent};
    ///
    /// assert!(DataItem::from(TagContent::from((12, 20))).is_tag());
    /// ```
    #[must_use]
    pub fn is_tag(&self) -> bool {
        matches!(self, Self::Tag(_))
    }

    /// Is a boolean value?
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::from(false).is_boolean());
    /// ```
    #[must_use]
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Boolean(_))
    }

    /// Is a null value?
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::Null.is_null());
    /// ```
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Is a undefined value?
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::Undefined.is_undefined());
    /// ```
    #[must_use]
    pub fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }

    /// Is a floating value?
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::from(3.0).is_floating());
    /// ```
    #[must_use]
    pub fn is_floating(&self) -> bool {
        matches!(self, Self::Floating(_))
    }

    /// Is a simple value?
    /// # Example
    /// ```
    /// use cbor_next::{DataItem, SimpleValue};
    ///
    /// assert!(DataItem::from(SimpleValue::try_from(45).unwrap()).is_simple());
    /// ```
    #[must_use]
    pub fn is_simple(&self) -> bool {
        matches!(
            self,
            Self::GenericSimple(_) | Self::Boolean(_) | Self::Null | Self::Undefined
        )
    }

    /// Is a generic simple value?
    /// # Example
    /// ```
    /// use cbor_next::{DataItem, SimpleValue};
    ///
    /// assert!(DataItem::from(SimpleValue::try_from(45).unwrap()).is_generic_simple());
    /// ```
    #[must_use]
    pub fn is_generic_simple(&self) -> bool {
        matches!(self, Self::GenericSimple(_))
    }

    /// Recursively checks nested CBOR data items until a non-tag item is found,
    /// then applies the given checker function to that item.
    ///
    /// This is particularly useful for examining the underlying value of tagged
    /// data items without manually unwrapping each layer of tags. Also
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::{DataItem, TagContent};
    ///
    /// let tag = DataItem::from(TagContent::from((20, TagContent::from((30, "abc")))));
    /// assert!(tag.check_nested(DataItem::is_text));
    /// ```
    ///
    /// # Note
    /// The function will skip all outer tags before applying the checker.
    /// If you need to check the tags themselves, use [`DataItem::is_tag`]
    /// directly
    #[must_use]
    pub fn check_nested(&self, checker: impl Fn(&Self) -> bool) -> bool {
        match self {
            Self::Tag(tag_content) => tag_content.content().check_nested(checker),
            _ => checker(self),
        }
    }

    /// Get as unsigned number
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::from(20).as_unsigned(), Some(20));
    /// ```
    #[must_use]
    pub fn as_unsigned(&self) -> Option<u64> {
        match self {
            Self::Unsigned(num) => Some(*num),
            _ => None,
        }
    }

    /// Get as signed number. This will always return negative number
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::from(-21).as_signed(), Some(-21));
    /// ```
    #[must_use]
    pub fn as_signed(&self) -> Option<i128> {
        match self {
            Self::Signed(num) => Some(-(i128::from(*num) + 1)),
            _ => None,
        }
    }

    /// Get as number which can be both signed or unsigned
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::from(-21).as_number(), Some(-21));
    /// assert_eq!(DataItem::from(345).as_number(), Some(345));
    /// ```
    #[must_use]
    pub fn as_number(&self) -> Option<i128> {
        match self {
            Self::Unsigned(num) => Some(i128::from(*num)),
            Self::Signed(num) => Some(-(i128::from(*num) + 1)),
            _ => None,
        }
    }

    /// Get as byte
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(
    ///     DataItem::from(vec![0x6a].as_slice()).as_byte(),
    ///     Some(vec![0x6a])
    /// );
    /// ```
    #[must_use]
    pub fn as_byte(&self) -> Option<Vec<u8>> {
        match self {
            Self::Byte(byte) => Some(byte.full()),
            _ => None,
        }
    }

    /// Get as text
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::from("cbor").as_text(), Some("cbor".to_string()));
    /// ```
    #[must_use]
    pub fn as_text(&self) -> Option<String> {
        match self {
            Self::Text(text_content) => Some(text_content.full()),
            _ => None,
        }
    }

    /// Get as array
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::from(vec![12u64]).as_array().unwrap(), [12.into()]);
    /// ```
    #[must_use]
    pub fn as_array(&self) -> Option<&[DataItem]> {
        match self {
            Self::Array(arr) => Some(arr.array()),
            _ => None,
        }
    }

    /// Get as map
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    /// use indexmap::IndexMap;
    ///
    /// assert_eq!(
    ///     DataItem::from(IndexMap::<DataItem, DataItem>::new()).as_map(),
    ///     Some(&IndexMap::new())
    /// );
    /// ```
    #[must_use]
    pub fn as_map(&self) -> Option<&IndexMap<DataItem, DataItem>> {
        match self {
            Self::Map(map) => Some(map.map()),
            _ => None,
        }
    }

    /// Get as tag
    ///
    /// # Example
    /// ```
    /// use cbor_next::{DataItem, TagContent};
    ///
    /// assert_eq!(
    ///     DataItem::from(TagContent::from((20, -21))).as_tag(),
    ///     Some((20, &DataItem::Signed(20)))
    /// );
    /// ```
    #[must_use]
    pub fn as_tag(&self) -> Option<(u64, &DataItem)> {
        match self {
            Self::Tag(tag_content) => Some((tag_content.number(), tag_content.content())),
            _ => None,
        }
    }

    /// Get a list of nested list of tags and its internal data item
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::{DataItem, TagContent};
    ///
    /// let tag = DataItem::from(TagContent::from((20, TagContent::from((30, -21)))));
    /// let tag_unwrapped = tag.as_tag_nested();
    /// assert_eq!(tag_unwrapped, Some((vec![20, 30], DataItem::from(-21))));
    ///
    /// let untagged = DataItem::from(21);
    /// let untagged_unwrapped = untagged.as_tag_nested();
    /// assert_eq!(untagged_unwrapped, None);
    /// ```
    #[must_use]
    pub fn as_tag_nested(&self) -> Option<(Vec<u64>, DataItem)> {
        match self {
            Self::Tag(_) => {
                let mut tags = vec![];
                let data_item = as_tag_nested(self, &mut tags);
                Some((tags, data_item))
            }
            _ => None,
        }
    }

    /// Get as boolean number
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::from(true).as_boolean(), Some(true));
    /// ```
    #[must_use]
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(bool_val) => Some(*bool_val),
            _ => None,
        }
    }

    /// Get as floating number
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::from(-20.0).as_floating(), Some(-20.0));
    /// ```
    #[must_use]
    pub fn as_floating(&self) -> Option<f64> {
        match self {
            Self::Floating(num) => Some(*num),
            _ => None,
        }
    }

    /// Get as simple index value
    ///
    /// # Example
    /// ```
    /// use cbor_next::{DataItem, SimpleValue};
    ///
    /// assert_eq!(
    ///     DataItem::from(SimpleValue::try_from(10).unwrap()).as_simple(),
    ///     Some(10)
    /// );
    /// ```
    #[must_use]
    pub fn as_simple(&self) -> Option<u8> {
        match self {
            Self::GenericSimple(num) => Some(**num),
            Self::Boolean(false) => Some(20),
            Self::Boolean(true) => Some(21),
            Self::Null => Some(22),
            Self::Undefined => Some(23),
            _ => None,
        }
    }

    /// Get a major type of a value
    #[must_use]
    pub fn major_type(&self) -> u8 {
        match self {
            Self::Unsigned(_) => 0,
            Self::Signed(_) => 1,
            Self::Byte(_) => 2,
            Self::Text(_) => 3,
            Self::Array(_) => 4,
            Self::Map(_) => 5,
            Self::Tag(..) => 6,
            Self::Boolean(_)
            | Self::Null
            | Self::Undefined
            | Self::Floating(_)
            | Self::GenericSimple(_) => 7,
        }
    }

    /// Get a CBOR encoded representation of value
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::DataItem;
    ///
    /// let value = DataItem::Unsigned(10_000_000);
    /// let vector_data = vec![0x1a, 0x00, 0x98, 0x96, 0x80];
    /// assert_eq!(value.encode(), vector_data);
    /// ```
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut buffer = vec![];
        self.encode_into(&mut buffer);
        buffer
    }

    /// Encode a data item by appending its bytes to the provided buffer so
    /// nested items reuse one allocation
    fn encode_into(&self, buffer: &mut Vec<u8>) {
        match self {
            Self::Unsigned(number) | Self::Signed(number) => {
                encode_u64_number(buffer, self.major_type(), *number);
            }
            Self::Byte(byte) => {
                encode_chunks(
                    buffer,
                    self.major_type(),
                    byte.is_indefinite(),
                    byte.chunk(),
                );
            }
            Self::Text(text_content) => {
                encode_chunks(
                    buffer,
                    self.major_type(),
                    text_content.is_indefinite(),
                    text_content.chunk(),
                );
            }
            Self::Array(array) => {
                let is_finite = !array.is_indefinite();
                let definite_length = is_finite
                    .then(|| u64::try_from(array.array().len()).ok())
                    .flatten();
                if let Some(length) = definite_length {
                    encode_u64_number(buffer, self.major_type(), length);
                } else {
                    // Indefinite length array
                    buffer.push(self.major_type() << 5 | 31);
                }
                for val in array.array() {
                    val.encode_into(buffer);
                }
                // Append break code for indefinite length array
                if definite_length.is_none() {
                    buffer.push(255);
                }
            }
            Self::Map(map) => {
                let is_finite = !map.is_indefinite();
                let definite_length = is_finite
                    .then(|| u64::try_from(map.map().len()).ok())
                    .flatten();
                if let Some(length) = definite_length {
                    encode_u64_number(buffer, self.major_type(), length);
                } else {
                    // Indefinite length map
                    buffer.push(self.major_type() << 5 | 31);
                }
                for (key, value) in map.map() {
                    key.encode_into(buffer);
                    value.encode_into(buffer);
                }
                // Append break code for indefinite length map
                if definite_length.is_none() {
                    buffer.push(255);
                }
            }
            Self::Tag(tag_content) => {
                encode_u64_number(buffer, self.major_type(), tag_content.number());
                tag_content.content().encode_into(buffer);
            }
            Self::Boolean(false) => buffer.push(self.major_type() << 5 | 0x14), // 20
            Self::Boolean(true) => buffer.push(self.major_type() << 5 | 0x15),  // 21
            Self::Null => buffer.push(self.major_type() << 5 | 0x16),           // 22
            Self::Undefined => buffer.push(self.major_type() << 5 | 0x17),      // 23
            Self::Floating(number) => encode_f64_number(buffer, self.major_type(), *number),
            Self::GenericSimple(simple_number) => {
                if **simple_number <= 23 {
                    buffer.push(self.major_type() << 5 | **simple_number);
                } else {
                    buffer.push(self.major_type() << 5 | 0x18); // 24
                    buffer.push(**simple_number);
                }
            }
        }
    }

    /// Decode a CBOR representation to a value
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::DataItem;
    ///
    /// let vector_data = vec![0x1a, 0x00, 0x98, 0x96, 0x80];
    /// let value = DataItem::Unsigned(10_000_000);
    /// assert_eq!(DataItem::decode(&vector_data).unwrap(), value);
    /// ```
    ///
    /// # Errors
    /// If provided bytes cannot be converted to CBOR, if bytes remain after a
    /// complete data item, or if nesting exceeds the supported maximum depth
    pub fn decode(val: &[u8]) -> Result<Self, Error> {
        let mut iter = val.iter();
        let value = decode_value(&mut iter, MAX_NESTING_DEPTH)?;
        // if there are any remaining bytes after decoding, it indicates that the input
        // was not fully consumed, which is considered an error in this context.
        if iter.next().is_some() {
            return Err(Error::TrailingBytes);
        }
        Ok(value)
    }

    /// Check current data item is deterministic form
    #[must_use]
    pub fn is_deterministic(&self, mode: &DeterministicMode) -> bool {
        match self {
            Self::Map(index_map) => {
                if index_map.is_indefinite() {
                    return false;
                }
                let map = index_map.map();
                let encoded_keys = map.keys().map(Self::encode).collect::<Vec<_>>();
                let keys_sorted = encoded_keys.windows(2).all(|pair| {
                    match mode {
                        DeterministicMode::Core => pair[0] <= pair[1],
                        DeterministicMode::LengthFirst => {
                            match pair[0].len().cmp(&pair[1].len()) {
                                Ordering::Equal => pair[0] <= pair[1],
                                Ordering::Greater => false,
                                Ordering::Less => true,
                            }
                        }
                    }
                });
                keys_sorted
                    && map.iter().all(|(key, value)| {
                        key.is_deterministic(mode) && value.is_deterministic(mode)
                    })
            }
            Self::Array(val) => {
                if val.is_indefinite() {
                    return false;
                }
                val.array().iter().all(|v| v.is_deterministic(mode))
            }
            Self::Tag(tag_content) => tag_content.content().is_deterministic(mode),
            Self::Byte(byte_content) => !byte_content.is_indefinite(),
            Self::Text(text_content) => !text_content.is_indefinite(),
            _ => true,
        }
    }

    /// Get a deterministic ordering form in provided mode
    #[must_use]
    pub fn deterministic(self, mode: &DeterministicMode) -> Self {
        match self {
            Self::Map(map_content) => {
                let mut data = map_content
                    .map()
                    .iter()
                    .map(|(k, v)| (k.clone().deterministic(mode), v.clone().deterministic(mode)))
                    .collect::<Vec<(_, _)>>();
                data.sort_by(|(k1, _), (k2, _)| {
                    let key1_encode = k1.encode();
                    let key2_encode = k2.encode();
                    match mode {
                        DeterministicMode::Core => key1_encode.cmp(&key2_encode),
                        DeterministicMode::LengthFirst => {
                            match key1_encode.len().cmp(&key2_encode.len()) {
                                Ordering::Equal => key1_encode.cmp(&key2_encode),
                                order => order,
                            }
                        }
                    }
                });
                let mut index_map = IndexMap::new();
                index_map.extend(data);
                Self::Map(
                    MapContent::default()
                        .set_indefinite(false)
                        .set_content(&index_map)
                        .clone(),
                )
            }
            Self::Array(val) => {
                Self::Array(
                    ArrayContent::default()
                        .set_indefinite(false)
                        .set_content(
                            &val.array()
                                .iter()
                                .map(|v| v.clone().deterministic(mode))
                                .collect::<Vec<_>>(),
                        )
                        .clone(),
                )
            }
            Self::Tag(tag_content) => {
                Self::Tag(TagContent::from((
                    tag_content.number(),
                    tag_content.content().clone().deterministic(mode),
                )))
            }
            Self::Byte(byte_content) => {
                if byte_content.is_indefinite() {
                    Self::Byte(
                        ByteContent::default()
                            .set_indefinite(false)
                            .push_bytes(&byte_content.full())
                            .clone(),
                    )
                } else {
                    Self::Byte(byte_content)
                }
            }
            Self::Text(text_content) => {
                if text_content.is_indefinite() {
                    Self::Text(
                        TextContent::default()
                            .set_indefinite(false)
                            .push_string(&text_content.full())
                            .clone(),
                    )
                } else {
                    Self::Text(text_content)
                }
            }
            _ => self,
        }
    }
}

fn as_tag_nested(item: &DataItem, tags: &mut Vec<u64>) -> DataItem {
    match item {
        DataItem::Tag(tag_content) => {
            tags.push(tag_content.number());
            as_tag_nested(tag_content.content(), tags)
        }
        _ => item.clone(),
    }
}

fn encode_u64_number(buffer: &mut Vec<u8>, major_type: u8, number: u64) {
    let shifted_major_type = major_type << 5;
    if let Ok(u8_value) = u8::try_from(number) {
        if u8_value <= 23 {
            buffer.push(shifted_major_type | u8_value);
        } else {
            buffer.push(shifted_major_type | 0x18); // 24
            buffer.push(u8_value);
        }
    } else if let Ok(u16_value) = u16::try_from(number) {
        buffer.push(shifted_major_type | 0x19); // 25
        buffer.extend_from_slice(&u16_value.to_be_bytes());
    } else if let Ok(u32_value) = u32::try_from(number) {
        buffer.push(shifted_major_type | 0x1A); // 26
        buffer.extend_from_slice(&u32_value.to_be_bytes());
    } else {
        buffer.push(shifted_major_type | 0x1B); // 27
        buffer.extend_from_slice(&number.to_be_bytes());
    }
}

/// Encode a byte or text content from its chunks without concatenating them
/// into an intermediate allocation
fn encode_chunks<T>(buffer: &mut Vec<u8>, major_type: u8, is_indefinite: bool, chunks: &[T])
where
    T: AsRef<[u8]>,
{
    let definite_length = if is_indefinite {
        None
    } else {
        // Calculate the total length of all chunks, returning None if any chunk's
        // length cannot be represented as a u64 or if the total length would overflow
        // u64
        chunks.iter().try_fold(0u64, |length, chunk| {
            length.checked_add(u64::try_from(chunk.as_ref().len()).ok()?)
        })
    };
    if let Some(length) = definite_length {
        encode_u64_number(buffer, major_type, length);
        for chunk in chunks {
            buffer.extend_from_slice(chunk.as_ref());
        }
    } else {
        // Indefinite length encoding
        buffer.push(major_type << 5 | 31);
        for chunk in chunks {
            let chunk = chunk.as_ref();
            let chunk_length =
                u64::try_from(chunk.len()).expect("single in-memory chunk length fits in u64");
            encode_u64_number(buffer, major_type, chunk_length);
            buffer.extend_from_slice(chunk);
        }
        buffer.push(255);
    }
}

fn encode_f64_number(buffer: &mut Vec<u8>, major_type: u8, f64_number: f64) {
    let shifted_major_type = major_type << 5;
    if f64_number.is_nan() {
        encode_f64_nan(buffer, shifted_major_type, f64_number);
        return;
    }
    let f16_num = half::f16::from_f64(f64_number);
    #[expect(
        clippy::float_cmp,
        reason = "we want to compare without margin or error"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "we only want to check truncation data loss"
    )]
    if f16_num.to_f64() == f64_number {
        buffer.push(shifted_major_type | 0x19); // 25
        buffer.extend_from_slice(&f16_num.to_be_bytes());
    } else if f64::from(f64_number as f32) == f64_number {
        buffer.push(shifted_major_type | 0x1A); // 26
        buffer.extend_from_slice(&(f64_number as f32).to_be_bytes());
    } else {
        buffer.push(shifted_major_type | 0x1B); // 27
        buffer.extend_from_slice(&f64_number.to_be_bytes());
    }
}

/// Encode a NaN in the shortest floating-point form that preserves its sign
/// and payload, as required by RFC 8949 preferred serialization.
fn encode_f64_nan(buffer: &mut Vec<u8>, shifted_major_type: u8, f64_number: f64) {
    /// Low mantissa bits of a f64 which are discarded when narrowing to f16
    const F16_DISCARDED_MANTISSA: u64 = (1 << 42) - 1;
    /// Low mantissa bits of a f64 which are discarded when narrowing to f32
    const F32_DISCARDED_MANTISSA: u64 = (1 << 29) - 1;
    let bits = f64_number.to_bits();
    if bits & F16_DISCARDED_MANTISSA == 0 {
        let sign = (bits >> 48) & 0x8000;
        let exponent = 0x7C00;
        let mantissa = (bits >> 42) & 0x03FF;
        let f16_bits = sign | exponent | mantissa;
        buffer.push(shifted_major_type | 0x19); // 25
        buffer.extend_from_slice(
            &u16::try_from(f16_bits)
                .expect("value is masked to 16 bits")
                .to_be_bytes(),
        );
    } else if bits & F32_DISCARDED_MANTISSA == 0 {
        let sign = (bits >> 32) & 0x8000_0000;
        let exponent = 0x7F80_0000;
        let mantissa = (bits >> 29) & 0x007F_FFFF;
        let f32_bits = sign | exponent | mantissa;
        buffer.push(shifted_major_type | 0x1A); // 26
        buffer.extend_from_slice(
            &u32::try_from(f32_bits)
                .expect("value is masked to 32 bits")
                .to_be_bytes(),
        );
    } else {
        buffer.push(shifted_major_type | 0x1B); // 27
        buffer.extend_from_slice(&bits.to_be_bytes());
    }
}

/// Maximum nesting depth accepted while decoding. Bounding the depth keeps
/// recursion on maliciously nested input from overflowing the stack
const MAX_NESTING_DEPTH: usize = 128;

fn decode_value(iter: &mut Iter<'_, u8>, remaining_depth: usize) -> Result<DataItem, Error> {
    let Some(next_depth) = remaining_depth.checked_sub(1) else {
        return Err(Error::RecursionLimit);
    };
    let initial_info = iter.next().ok_or(Error::Incomplete)?;
    let major_type = initial_info >> 5;
    let additional = initial_info & 0b0001_1111;
    match major_type {
        0 => Ok(DataItem::Unsigned(extract_number(additional, iter)?)),
        1 => Ok(DataItem::Signed(extract_number(additional, iter)?)),
        2 => {
            Ok(DataItem::Byte(decode_byte_or_text(
                major_type, additional, iter,
            )?))
        }
        3 => {
            Ok(DataItem::Text(
                decode_byte_or_text(major_type, additional, iter)?.try_into()?,
            ))
        }
        4 => decode_array(additional, iter, next_depth),
        5 => decode_map(additional, iter, next_depth),
        6 => {
            let tag_number = extract_number(additional, iter)?;
            let tag_value = decode_value(iter, next_depth)?;
            Ok(DataItem::Tag(TagContent::from((tag_number, tag_value))))
        }
        7 => decode_simple_or_floating(additional, iter),
        _ => unreachable!("major type can only be between 0 to 7"),
    }
}

fn decode_byte_or_text(
    major_type: u8,
    additional: u8,
    iter: &mut Iter<'_, u8>,
) -> Result<ByteContent, Error> {
    let length = extract_optional_number(additional, iter)?;
    let mut byte_content = ByteContent::default();
    if let Some(num) = length {
        byte_content.set_indefinite(false);
        byte_content.set_bytes(&collect_vec_u8(iter, num)?);
    } else {
        byte_content.set_indefinite(true);
        loop {
            let initial_info = iter.next().ok_or(Error::IncompleteIndefinite)?;
            if *initial_info == 255 {
                break;
            }
            let chunk_major_type = initial_info >> 5;
            if chunk_major_type != major_type {
                return Err(Error::NotWellFormed(format!(
                    "contains invalid major type {chunk_major_type} for indefinite major type \
                     {major_type}"
                )));
            }
            let chunk_additional = initial_info & 0b0001_1111;
            let chunk_length = extract_number(chunk_additional, iter)?;
            byte_content.push_bytes(&collect_vec_u8(iter, chunk_length)?);
        }
    }
    Ok(byte_content)
}

fn decode_array(additional: u8, iter: &mut Iter<'_, u8>, depth: usize) -> Result<DataItem, Error> {
    let length = extract_optional_number(additional, iter)?;
    let mut val_vec = vec![];
    let mut array_content = ArrayContent::default();
    array_content.set_indefinite(length.is_none());
    if let Some(num) = length {
        for _ in 0..num {
            val_vec.push(decode_value(iter, depth)?);
        }
    } else {
        loop {
            match iter.clone().next() {
                Some(255) => {
                    iter.next();
                    break;
                }
                Some(_) => val_vec.push(decode_value(iter, depth)?),
                None => return Err(Error::IncompleteIndefinite),
            }
        }
    }
    Ok(DataItem::Array(array_content.set_content(&val_vec).clone()))
}

fn decode_map(additional: u8, iter: &mut Iter<'_, u8>, depth: usize) -> Result<DataItem, Error> {
    let length: Option<u64> = extract_optional_number(additional, iter)?;
    let mut map_index_map = IndexMap::new();
    let mut map_content = MapContent::default();
    map_content.set_indefinite(length.is_none());
    if let Some(num) = length {
        for _ in 0..num {
            let key = decode_value(iter, depth)?;
            let val = decode_value(iter, depth)?;
            insert_unique_key(&mut map_index_map, key, val)?;
        }
    } else {
        loop {
            match iter.clone().next() {
                Some(255) => {
                    iter.next();
                    break;
                }
                Some(_) => {
                    let key = decode_value(iter, depth)?;
                    let val = decode_value(iter, depth)?;
                    insert_unique_key(&mut map_index_map, key, val)?;
                }
                None => return Err(Error::IncompleteIndefinite),
            }
        }
    }
    Ok(DataItem::Map(
        map_content.set_content(&map_index_map).clone(),
    ))
}

fn insert_unique_key(
    map: &mut IndexMap<DataItem, DataItem>,
    key: DataItem,
    value: DataItem,
) -> Result<(), Error> {
    match map.entry(key) {
        indexmap::map::Entry::Occupied(entry) => {
            Err(Error::NotWellFormed(format!(
                "same map key {:#?} is repeated multiple times",
                entry.key()
            )))
        }
        indexmap::map::Entry::Vacant(entry) => {
            entry.insert(value);
            Ok(())
        }
    }
}

fn decode_simple_or_floating(additional: u8, iter: &mut Iter<'_, u8>) -> Result<DataItem, Error> {
    match additional {
        0..=19 => Ok(DataItem::GenericSimple(additional.try_into()?)),
        20 => Ok(DataItem::Boolean(false)),
        21 => Ok(DataItem::Boolean(true)),
        22 => Ok(DataItem::Null),
        23 => Ok(DataItem::Undefined),
        24 => {
            let next_num = iter.next().ok_or(Error::Incomplete)?;
            if *next_num < 32 {
                Err(Error::InvalidSimple)
            } else {
                Ok(DataItem::GenericSimple((*next_num).try_into()?))
            }
        }
        25 => {
            let number_representation = u16::try_from(extract_number(additional, iter)?)?;
            Ok(DataItem::Floating(f64::from(half::f16::from_bits(
                number_representation,
            ))))
        }
        26 => {
            let number_representation = u32::try_from(extract_number(additional, iter)?)?;
            Ok(DataItem::Floating(f64::from(f32::from_bits(
                number_representation,
            ))))
        }
        27 => {
            let f64_number_representation = extract_number(additional, iter)?;
            Ok(DataItem::Floating(f64::from_bits(
                f64_number_representation,
            )))
        }
        28..=30 => {
            Err(Error::NotWellFormed(format!(
                "invalid value {additional} for major type 7"
            )))
        }
        31 => Err(Error::InvalidBreakStop),
        _ => unreachable!("Cannot have additional info value greater than 31"),
    }
}

fn collect_vec_u8(iter: &mut Iter<'_, u8>, number: u64) -> Result<Vec<u8>, Error> {
    let mut collected_val = Vec::new();
    for i in 0..number {
        match iter.next() {
            Some(item) => collected_val.push(*item),
            None => {
                return Err(Error::NotWellFormed(format!(
                    "incomplete array of byte missing {} byte",
                    number - i
                )));
            }
        }
    }
    Ok(collected_val)
}

fn extract_optional_number(additional: u8, iter: &mut Iter<'_, u8>) -> Result<Option<u64>, Error> {
    match additional {
        0..=23 => Ok(Some(u64::from(additional))),
        24..=27 => {
            let number_bytes = collect_vec_u8(iter, 2u64.pow(u32::from(additional - 24)))?;
            let mut array = [0u8; 8];
            let len = number_bytes.len();
            array[8 - len..].copy_from_slice(&number_bytes[..len]);
            Ok(Some(u64::from_be_bytes(array)))
        }
        28..=30 => {
            Err(Error::NotWellFormed(format!(
                "invalid additional number {additional}"
            )))
        }
        31 => Ok(None),
        _ => unreachable!("Cannot have additional info value greater than 31"),
    }
}

fn extract_number(additional: u8, iter: &mut Iter<'_, u8>) -> Result<u64, Error> {
    extract_optional_number(additional, iter)?
        .ok_or(Error::NotWellFormed("failed to extract number".to_string()))
}
