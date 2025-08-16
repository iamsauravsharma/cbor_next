use core::f64;
use std::cmp::Ordering;
use std::fmt::{Debug, Write as _};
use std::hash::Hash;
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
            Self::Signed(number) => (-i128::from(number + 1)).fmt(f),
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
                let vals = index_map.map().iter().collect::<Vec<(_, _)>>();
                vals.hash(state);
            }
            Self::Tag(tag_content) => {
                tag_content.number().hash(state);
                tag_content.content().hash(state);
            }
            Self::Boolean(val) => val.hash(state),
            Self::Floating(val) => val.to_be_bytes().hash(state),
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
    /// assert!(tag.check_inner(DataItem::is_text));
    /// ```
    ///
    /// # Note
    /// The function will skip all outer tags before applying the checker.
    /// If you need to check the tags themselves, use [`DataItem::is_tag`]
    /// directly
    #[must_use]
    pub fn check_inner(&self, checker: impl Fn(&Self) -> bool) -> bool {
        match self {
            Self::Tag(tag_content) => tag_content.content().check_inner(checker),
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
            Self::Signed(num) => Some(-i128::from(num + 1)),
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
            Self::Signed(num) => Some(-i128::from(num + 1)),
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

    /// Recursively extract tagged values, collecting all tag numbers and
    /// returning them with the extracted value. Tags are vector of tag numbers
    /// in outer-to-inner order
    ///
    ///  When extractor is tag extractor i.e [`DataItem::as_tag`] than this
    /// would always return `None` since it only supports non tag extract but
    /// would successfully returns tag list
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::{DataItem, TagContent};
    ///
    /// let tag = DataItem::from(TagContent::from((20, TagContent::from((30, -21)))));
    /// let tag_unwrapped = tag.as_inner(DataItem::as_signed);
    /// assert_eq!(tag_unwrapped, Some((vec![20, 30], -21)));
    /// ```
    #[must_use]
    pub fn as_inner<T>(&self, extractor: impl Fn(&Self) -> Option<T>) -> Option<(Vec<u64>, T)> {
        let mut tags = vec![];
        extract_and_extend_tags(self, extractor, &mut tags).map(|val| (tags, val))
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
        match self {
            Self::Unsigned(number) | Self::Signed(number) => {
                encode_u64_number(self.major_type(), *number)
            }
            Self::Byte(byte) => encode_vec_u8(self.major_type(), byte),
            Self::Text(text_content) => {
                encode_vec_u8(self.major_type(), &text_content.clone().into())
            }
            Self::Array(array) => {
                let mut array_bytes = vec![];
                if array.is_indefinite() {
                    array_bytes.push(self.major_type() << 5 | 31);
                    for val in array.array() {
                        array_bytes.append(&mut val.encode());
                    }
                    array_bytes.push(255);
                } else {
                    let array_len = u64::try_from(array.array().len());
                    if let Ok(length) = array_len {
                        array_bytes.extend(encode_u64_number(self.major_type(), length));
                        for val in array.array() {
                            array_bytes.append(&mut val.encode());
                        }
                    } else {
                        array_bytes.extend(
                            Self::Array(
                                ArrayContent::default()
                                    .set_indefinite(true)
                                    .set_content(array.array())
                                    .clone(),
                            )
                            .encode(),
                        );
                    }
                }
                array_bytes
            }
            Self::Map(map) => {
                let mut map_bytes = vec![];
                if map.is_indefinite() {
                    map_bytes.push(self.major_type() << 5 | 31);
                    for (key, value) in map.map() {
                        map_bytes.append(&mut key.encode());
                        map_bytes.append(&mut value.encode());
                    }
                    map_bytes.push(255);
                } else {
                    let map_len = u64::try_from(map.map().len());
                    if let Ok(length) = map_len {
                        map_bytes.extend(encode_u64_number(self.major_type(), length));
                        for (key, value) in map.map() {
                            map_bytes.append(&mut key.encode());
                            map_bytes.append(&mut value.encode());
                        }
                    } else {
                        map_bytes.extend(
                            Self::Map(
                                MapContent::default()
                                    .set_indefinite(true)
                                    .set_content(map.map())
                                    .clone(),
                            )
                            .encode(),
                        );
                    }
                }
                map_bytes
            }
            Self::Tag(tag_content) => {
                let mut tag_bytes = encode_u64_number(self.major_type(), tag_content.number());
                tag_bytes.append(&mut tag_content.content().encode());
                tag_bytes
            }
            Self::Boolean(bool_val) => {
                match bool_val {
                    false => vec![self.major_type() << 5 | 20],
                    true => vec![self.major_type() << 5 | 21],
                }
            }
            Self::Null => vec![self.major_type() << 5 | 22],
            Self::Undefined => vec![self.major_type() << 5 | 23],
            Self::Floating(number) => encode_f64_number(self.major_type(), *number),
            Self::GenericSimple(simple_number) => {
                if **simple_number <= 23 {
                    vec![self.major_type() << 5 | **simple_number]
                } else {
                    vec![self.major_type() << 5 | 24, **simple_number]
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
    /// If provided bytes cannot be converted to CBOR
    pub fn decode(val: &[u8]) -> Result<Self, Error> {
        let mut iter = val.iter();
        decode_value(&mut iter)
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
                map.iter()
                    .zip(map.iter().skip(1))
                    .all(|((k1, _), (k2, _))| {
                        let key1_encode = k1.encode();
                        let key2_encode = k2.encode();
                        match mode {
                            DeterministicMode::Core => key1_encode <= key2_encode,
                            DeterministicMode::LengthFirst => {
                                match key1_encode.len().cmp(&key2_encode.len()) {
                                    Ordering::Equal => key1_encode <= key2_encode,
                                    Ordering::Greater => false,
                                    Ordering::Less => true,
                                }
                            }
                        }
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

fn extract_and_extend_tags<T>(
    item: &DataItem,
    extractor: impl Fn(&DataItem) -> Option<T>,
    tags: &mut Vec<u64>,
) -> Option<T> {
    match item {
        DataItem::Tag(tag_content) => {
            tags.push(tag_content.number());
            extract_and_extend_tags(tag_content.content(), extractor, tags)
        }
        _ => extractor(item),
    }
}

fn encode_u64_number(major_type: u8, number: u64) -> Vec<u8> {
    let shifted_major_type = major_type << 5;
    let mut cbor_representation = vec![];
    if let Ok(u8_value) = u8::try_from(number) {
        if u8_value <= 23 {
            cbor_representation.push(shifted_major_type | u8_value);
        } else {
            cbor_representation.push(shifted_major_type | 24);
            cbor_representation.push(u8_value);
        }
    } else if let Ok(u16_value) = u16::try_from(number) {
        cbor_representation.push(shifted_major_type | 25);
        for byte in u16_value.to_be_bytes() {
            cbor_representation.push(byte);
        }
    } else if let Ok(u32_value) = u32::try_from(number) {
        cbor_representation.push(shifted_major_type | 26);
        for byte in u32_value.to_be_bytes() {
            cbor_representation.push(byte);
        }
    } else {
        cbor_representation.push(shifted_major_type | 27);
        for byte in number.to_be_bytes() {
            cbor_representation.push(byte);
        }
    }
    cbor_representation
}

fn encode_vec_u8(major_type: u8, byte: &ByteContent) -> Vec<u8> {
    let mut bytes = vec![];
    if byte.is_indefinite() {
        bytes.push(major_type << 5 | 31);
        for chunk in byte.chunk() {
            let mut encoded_fixed_length = encode_vec_u8(
                major_type,
                ByteContent::default()
                    .set_indefinite(false)
                    .set_bytes(chunk),
            );
            bytes.append(&mut encoded_fixed_length);
        }
        bytes.push(255);
    } else {
        let byte_length = u64::try_from(byte.full().len());
        if let Ok(length) = byte_length {
            bytes.append(&mut encode_u64_number(major_type, length));
            bytes.append(&mut byte.full().clone());
        } else {
            bytes.append(&mut encode_vec_u8(
                major_type,
                ByteContent::default()
                    .set_indefinite(true)
                    .set_bytes(&byte.full()),
            ));
        }
    }
    bytes
}

fn encode_f64_number(major_type: u8, f64_number: f64) -> Vec<u8> {
    let shifted_major_type = major_type << 5;
    let mut cbor_representation = vec![];
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
        cbor_representation.push(shifted_major_type | 25);
        for byte in (f16_num).to_be_bytes() {
            cbor_representation.push(byte);
        }
    } else if f64::from(f64_number as f32) == f64_number {
        cbor_representation.push(shifted_major_type | 26);
        for byte in (f64_number as f32).to_be_bytes() {
            cbor_representation.push(byte);
        }
    } else {
        cbor_representation.push(shifted_major_type | 27);
        for byte in f64_number.to_be_bytes() {
            cbor_representation.push(byte);
        }
    }
    cbor_representation
}

fn decode_value(iter: &mut Iter<'_, u8>) -> Result<DataItem, Error> {
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
        4 => decode_array(additional, iter),
        5 => decode_map(additional, iter),
        6 => {
            let tag_number = extract_number(additional, iter)?;
            let tag_value = decode_value(iter)?;
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
        byte_content.extend_bytes(&decode_indefinite_byte_or_text(major_type, iter)?);
        iter.next();
    }
    Ok(byte_content)
}

fn decode_array(additional: u8, iter: &mut Iter<'_, u8>) -> Result<DataItem, Error> {
    let length = extract_optional_number(additional, iter)?;
    let mut val_vec = vec![];
    let mut array_content = ArrayContent::default();
    array_content.set_indefinite(length.is_none());
    if let Some(num) = length {
        for _ in 0..num {
            val_vec.push(decode_value(iter)?);
        }
    } else {
        val_vec.append(&mut extract_array_item(iter)?);
        match iter.clone().next() {
            Some(255) => {
                iter.next();
            }
            None => {
                return Err(Error::IncompleteIndefinite);
            }
            _ => unreachable!("non 255 some value should be handled already"),
        }
    }
    Ok(DataItem::Array(array_content.set_content(&val_vec).clone()))
}

fn decode_map(additional: u8, iter: &mut Iter<'_, u8>) -> Result<DataItem, Error> {
    let length: Option<u64> = extract_optional_number(additional, iter)?;
    let mut map_index_map = IndexMap::new();
    let mut map_content = MapContent::default();
    map_content.set_indefinite(length.is_none());
    if let Some(num) = length {
        for _ in 0..num {
            let key = decode_value(iter)?;
            let val = decode_value(iter)?;
            if map_index_map.insert(key.clone(), val).is_some() {
                return Err(Error::NotWellFormed(format!(
                    "same map key {key:#?} is repeated multiple times"
                )));
            }
        }
    } else {
        map_index_map.extend(extract_map_item(iter)?);
        match iter.clone().next() {
            Some(255) => {
                iter.next();
            }
            None => {
                return Err(Error::IncompleteIndefinite);
            }
            _ => unreachable!("non 255 some value should be handled already"),
        }
    }
    Ok(DataItem::Map(
        map_content.set_content(&map_index_map).clone(),
    ))
}

fn decode_simple_or_floating(additional: u8, iter: &mut Iter<'_, u8>) -> Result<DataItem, Error> {
    match additional {
        0..=19 => Ok(DataItem::GenericSimple(additional.try_into()?)),
        20 => Ok(DataItem::Boolean(false)),
        21 => Ok(DataItem::Boolean(true)),
        22 => Ok(DataItem::Null),
        23 => Ok(DataItem::Undefined),
        24 => {
            if let Some(next_num) = iter.next() {
                if *next_num < 32 {
                    Err(Error::InvalidSimple)
                } else {
                    Ok(DataItem::GenericSimple((*next_num).try_into()?))
                }
            } else {
                Err(Error::InvalidSimple)
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

fn decode_indefinite_byte_or_text(
    expected_major_type: u8,
    iter: &mut Iter<'_, u8>,
) -> Result<Vec<Vec<u8>>, Error> {
    let mut result = vec![];
    if let Some(peek_val) = iter.clone().next() {
        if *peek_val == 255 {
            return Ok(result);
        }
        let initial_info = iter.next().ok_or(Error::Incomplete)?;
        let major_type = initial_info >> 5;
        if expected_major_type != major_type {
            return Err(Error::NotWellFormed(format!(
                "contains invalid major type {major_type} for indefinite major type \
                 {expected_major_type}"
            )));
        }
        let additional = initial_info & 0b0001_1111;
        let length = extract_number(additional, iter)?;
        result.push(collect_vec_u8(iter, length)?);
        result.extend(decode_indefinite_byte_or_text(expected_major_type, iter)?);
        return Ok(result);
    }
    Err(Error::IncompleteIndefinite)
}

fn extract_array_item(iter: &mut Iter<'_, u8>) -> Result<Vec<DataItem>, Error> {
    let mut result = vec![];
    if let Some(peek_val) = iter.clone().next()
        && *peek_val != 255
    {
        result.push(decode_value(iter)?);
        result.append(&mut extract_array_item(iter)?);
    }
    Ok(result)
}

fn extract_map_item(iter: &mut Iter<'_, u8>) -> Result<IndexMap<DataItem, DataItem>, Error> {
    let mut result = IndexMap::new();
    if let Some(peek_val) = iter.clone().next()
        && *peek_val != 255
    {
        let key = decode_value(iter)?;
        let val = decode_value(iter)?;
        if result.insert(key.clone(), val).is_some() {
            return Err(Error::NotWellFormed(format!(
                "same map key {key:#?} is repeated multiple times"
            )));
        }
        result.extend(extract_map_item(iter)?);
    }
    Ok(result)
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
