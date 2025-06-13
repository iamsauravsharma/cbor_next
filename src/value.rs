use std::convert::Into;
use std::hash::Hash;
use std::num::TryFromIntError;
use std::slice::Iter;

use indexmap::IndexMap;

use crate::deterministic::DeterministicMode;
use crate::error::Error;
use crate::simple_number::SimpleNumber;

/// Enum representing different types of values that can be encoded or decoded
/// in `CBOR` (Concise Binary Object Representation).
///
/// `CBOR` is a data format designed for small code and message size, often used
/// in constrained environments. This `Value` enum covers all major types
/// defined in the `CBOR` specification (RFC 8949).
#[derive(PartialEq, Debug, Clone)]
#[non_exhaustive]
pub enum Value {
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
    Byte(Vec<u8>),
    /// UTF-8 string (text string) represented by `CBOR` major type 3.
    ///
    /// Contains a sequence of Unicode characters encoded as UTF-8.
    Text(String),
    /// Array of `CBOR` values represented by `CBOR` major type 4.
    ///
    /// An ordered sequence of zero or more `CBOR` data items.
    Array(Vec<Value>),
    /// Map of `CBOR` key-value pairs represented by `CBOR` major type 5.
    ///
    /// Keys within a map must be unique
    Map(IndexMap<Value, Value>),
    /// Tagged item (semantic tag) represented by `CBOR` major type 6.
    ///
    /// Consists of an unsigned integer (the tag) and a single `CBOR` data item
    /// (the tagged content). Tags provide semantic information about the
    /// enclosed data item, allowing for type extension
    /// or application-specific interpretations.
    Tag(u64, Box<Value>),
    /// Boolean value represented as a simple value within `CBOR` major type 7.
    ///
    /// Can be either `true` or `false`.
    Boolean(bool),
    /// Null value represented as a simple value within `CBOR` major type 7.
    ///
    /// Represents the absence of a value.
    Null,
    /// Undefined value represented as a simple value within `CBOR` major type
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
    /// An unknown simple value represented by `CBOR` major type 7.
    ///
    /// This variant handles simple values that are not explicitly covered by
    /// `Boolean`, `Null`, `Undefined`, or `Floating`. These unknown simple
    /// values have a numerical representation as defined in the `CBOR`
    /// specification.
    UnknownSimple(SimpleNumber),
}

impl Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Unsigned(val) | Value::Signed(val) => val.hash(state),
            Value::Byte(items) => items.hash(state),
            Value::Text(text) => text.hash(state),
            Value::Array(values) => values.hash(state),
            Value::Map(index_map) => {
                let mut vals = index_map.iter().collect::<Vec<(_, _)>>();
                vals.sort();
                vals.hash(state);
            }
            Value::Tag(num, value) => {
                num.hash(state);
                value.hash(state);
            }
            Value::Boolean(val) => val.hash(state),
            Value::Floating(val) => (val + 0.0).to_be_bytes().hash(state),
            Value::UnknownSimple(simple_number) => simple_number.hash(state),
            _ => {}
        }
    }
}

impl Eq for Value {}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.encode().cmp(&other.encode())
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Self::Unsigned(value)
    }
}

macro_rules! impl_from {
    ($i:ident, $($t:ty),+) => {
        $(
        impl From<$t> for Value {
            fn from(value: $t) -> Self {
                $i::from(value).into()
            }
        }
    )+
    };
}

impl_from!(u64, u32, u16, u8);

impl TryFrom<u128> for Value {
    type Error = TryFromIntError;

    fn try_from(value: u128) -> Result<Self, Self::Error> {
        Ok(u64::try_from(value)?.into())
    }
}

impl From<i64> for Value {
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

impl TryFrom<i128> for Value {
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

impl From<&[u8]> for Value {
    fn from(value: &[u8]) -> Self {
        Self::Byte(value.to_vec())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Floating(value)
    }
}

impl_from!(f64, f32, half::f16);

impl<T> From<Vec<T>> for Value
where
    T: Into<Value>,
{
    fn from(value: Vec<T>) -> Self {
        Self::Array(value.into_iter().map(Into::into).collect())
    }
}

impl<T, U> From<Vec<(T, U)>> for Value
where
    T: Into<Value>,
    U: Into<Value>,
{
    fn from(value: Vec<(T, U)>) -> Self {
        Self::Map(
            value
                .into_iter()
                .map(|(t, u)| (t.into(), u.into()))
                .collect(),
        )
    }
}

impl<T> From<&T> for Value
where
    T: Into<Value> + Clone,
{
    fn from(value: &T) -> Self {
        value.clone().into()
    }
}

impl Value {
    /// Is a unsigned integer value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::Value;
    ///
    /// assert!(Value::Unsigned(20).is_unsigned_integer());
    /// ```
    #[must_use]
    pub fn is_unsigned_integer(&self) -> bool {
        matches!(self, Self::Unsigned(_))
    }

    /// Is a signed integer value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::Value;
    ///
    /// assert!(Value::from(-32).is_signed_integer());
    /// ```
    #[must_use]
    pub fn is_signed_integer(&self) -> bool {
        matches!(self, Self::Signed(_))
    }

    /// Is a byte value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::Value;
    ///
    /// assert!(Value::Byte(vec![65, 63, 62]).is_byte());
    /// ```
    #[must_use]
    pub fn is_byte(&self) -> bool {
        matches!(self, Self::Byte(_))
    }

    /// Is a text value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::Value;
    ///
    /// assert!(Value::Text("example".to_string()).is_text());
    /// ```
    #[must_use]
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Is a array value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::Value;
    ///
    /// assert!(Value::Array(vec![12.into()]).is_array());
    /// ```
    #[must_use]
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    /// Is a map value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::Value;
    /// use indexmap::IndexMap;
    ///
    /// assert!(Value::Map(IndexMap::new()).is_map());
    /// ```
    #[must_use]
    pub fn is_map(&self) -> bool {
        matches!(self, Self::Map(_))
    }

    /// Is a tag value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::Value;
    ///
    /// assert!(Value::Tag(12, Box::new(Value::Unsigned(20))).is_tag());
    /// ```
    #[must_use]
    pub fn is_tag(&self) -> bool {
        matches!(self, Self::Tag(_, _))
    }

    /// Is a boolean value?
    /// # Example
    /// ```
    /// use cbor_next::Value;
    ///
    /// assert!(Value::Boolean(false).is_boolean());
    /// ```
    #[must_use]
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Boolean(_))
    }

    /// Is a null value?
    /// # Example
    /// ```
    /// use cbor_next::Value;
    ///
    /// assert!(Value::Null.is_null());
    /// ```
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Is a undefined value?
    /// # Example
    /// ```
    /// use cbor_next::Value;
    ///
    /// assert!(Value::Undefined.is_undefined());
    /// ```
    #[must_use]
    pub fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }

    /// Is a floating value?
    /// # Example
    /// ```
    /// use cbor_next::Value;
    ///
    /// assert!(Value::Floating(3.0).is_floating());
    /// ```
    #[must_use]
    pub fn is_floating(&self) -> bool {
        matches!(self, Self::Floating(_))
    }

    /// Is a simple value?
    /// # Example
    /// ```
    /// use cbor_next::{SimpleNumber, Value};
    ///
    /// assert!(Value::UnknownSimple(SimpleNumber::try_from(45).unwrap()).is_simple());
    /// ```
    #[must_use]
    pub fn is_simple(&self) -> bool {
        matches!(
            self,
            Self::UnknownSimple(_) | Self::Boolean(_) | Self::Null | Self::Undefined
        )
    }

    /// Is a unknown simple value?
    /// # Example
    /// ```
    /// use cbor_next::{SimpleNumber, Value};
    ///
    /// assert!(Value::UnknownSimple(SimpleNumber::try_from(45).unwrap()).is_simple());
    /// ```
    #[must_use]
    pub fn is_unknown_simple(&self) -> bool {
        matches!(self, Self::UnknownSimple(_))
    }

    /// Get as unsigned number
    ///
    /// # Example
    /// ```
    /// use cbor_next::Value;
    ///
    /// assert!(Value::Unsigned(20).as_unsigned().is_some());
    /// ```
    #[must_use]
    pub fn as_unsigned(&self) -> Option<u64> {
        match self {
            Self::Unsigned(num) => Some(*num),
            _ => None,
        }
    }

    /// Get as signed number
    ///
    /// # Example
    /// ```
    /// use cbor_next::Value;
    ///
    /// assert!(Value::Signed(20).as_signed().is_some());
    /// ```
    #[must_use]
    pub fn as_signed(&self) -> Option<i128> {
        match self {
            Self::Signed(num) => Some(-i128::from(num + 1)),
            _ => None,
        }
    }

    /// Get as byte number
    ///
    /// # Example
    /// ```
    /// use cbor_next::Value;
    ///
    /// assert!(Value::Byte(vec![]).as_byte().is_some());
    /// ```
    #[must_use]
    pub fn as_byte(&self) -> Option<&Vec<u8>> {
        match self {
            Self::Byte(byte) => Some(byte),
            _ => None,
        }
    }

    /// Get as text number
    ///
    /// # Example
    /// ```
    /// use cbor_next::Value;
    ///
    /// assert!(Value::Text(String::new()).as_text().is_some());
    /// ```
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(string) => Some(string),
            _ => None,
        }
    }

    /// Get as array number
    ///
    /// # Example
    /// ```
    /// use cbor_next::Value;
    ///
    /// assert!(Value::Array(vec![]).as_array().is_some());
    /// ```
    #[must_use]
    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Self::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Get as map number
    ///
    /// # Example
    /// ```
    /// use cbor_next::Value;
    /// use indexmap::IndexMap;
    ///
    /// assert!(Value::Map(IndexMap::new()).as_map().is_some());
    /// ```
    #[must_use]
    pub fn as_map(&self) -> Option<&IndexMap<Value, Value>> {
        match self {
            Self::Map(map) => Some(map),
            _ => None,
        }
    }

    /// Get as tag number
    ///
    /// # Example
    /// ```
    /// use cbor_next::Value;
    ///
    /// assert!(
    ///     Value::Tag(20, Box::new(Value::Signed(20)))
    ///         .as_tag()
    ///         .is_some()
    /// );
    /// ```
    #[must_use]
    pub fn as_tag(&self) -> Option<(&u64, &Value)> {
        match self {
            Self::Tag(tag_num, value) => Some((tag_num, value)),
            _ => None,
        }
    }

    /// Get as boolean number
    ///
    /// # Example
    /// ```
    /// use cbor_next::Value;
    ///
    /// assert!(Value::Boolean(true).as_boolean().is_some());
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
    /// use cbor_next::Value;
    ///
    /// assert!(Value::Floating(-20.0).as_floating().is_some());
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
    /// use cbor_next::{SimpleNumber, Value};
    ///
    /// assert!(
    ///     Value::UnknownSimple(SimpleNumber::try_from(10).unwrap())
    ///         .as_simple()
    ///         .is_some()
    /// );
    /// ```
    #[must_use]
    pub fn as_simple(&self) -> Option<u8> {
        match self {
            Self::UnknownSimple(num) => Some(num.val()),
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
            Value::Unsigned(_) => 0,
            Value::Signed(_) => 1,
            Value::Byte(_) => 2,
            Value::Text(_) => 3,
            Value::Array(_) => 4,
            Value::Map(_) => 5,
            Value::Tag(..) => 6,
            Value::Boolean(_)
            | Value::Null
            | Value::Undefined
            | Value::Floating(_)
            | Self::UnknownSimple(_) => 7,
        }
    }

    /// Get a CBOR encoded representation of value
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::Value;
    ///
    /// let value = Value::Unsigned(10_000_000);
    /// let vector_data = vec![0x1a, 0x00, 0x98, 0x96, 0x80];
    /// assert_eq!(value.encode(), vector_data);
    /// ```
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Value::Unsigned(number) | Value::Signed(number) => {
                encode_u64_number(self.major_type(), *number)
            }
            Value::Byte(byte) => encode_vec_u8(self.major_type(), byte),
            Value::Text(text_string) => encode_vec_u8(self.major_type(), text_string.as_bytes()),
            Value::Array(array) => {
                let array_len = u64::try_from(array.len());
                let mut array_bytes = if let Ok(length) = array_len {
                    encode_u64_number(self.major_type(), length)
                } else {
                    vec![self.major_type() << 5 | 31]
                };
                for value in array {
                    array_bytes.append(&mut value.encode());
                }
                if array_len.is_err() {
                    array_bytes.push(255);
                }
                array_bytes
            }
            Value::Map(map) => {
                let map_len = u64::try_from(map.len());
                let mut map_bytes = if let Ok(length) = map_len {
                    encode_u64_number(self.major_type(), length)
                } else {
                    vec![self.major_type() << 5 | 31]
                };
                for (key, value) in map {
                    map_bytes.append(&mut key.encode());
                    map_bytes.append(&mut value.encode());
                }
                if map_len.is_err() {
                    map_bytes.push(255);
                }
                map_bytes
            }
            Value::Tag(number, value) => {
                let mut tag_bytes = encode_u64_number(self.major_type(), *number);
                tag_bytes.append(&mut value.encode());
                tag_bytes
            }
            Value::Boolean(bool_val) => {
                match bool_val {
                    false => vec![self.major_type() << 5 | 20],
                    true => vec![self.major_type() << 5 | 21],
                }
            }
            Value::Null => vec![self.major_type() << 5 | 22],
            Value::Undefined => vec![self.major_type() << 5 | 23],
            Value::Floating(number) => encode_f64_number(self.major_type(), *number),
            Value::UnknownSimple(simple_number) => {
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
    /// use cbor_next::Value;
    ///
    /// let vector_data = vec![0x1a, 0x00, 0x98, 0x96, 0x80];
    /// let value = Value::Unsigned(10_000_000);
    /// assert_eq!(Value::decode(&vector_data).unwrap(), value);
    /// ```
    ///
    /// # Errors
    /// If provided bytes cannot be converted to CBOR
    pub fn decode(val: &[u8]) -> Result<Self, Error> {
        let mut iter = val.iter();
        decode_value(&mut iter)
    }

    /// Get a deterministic form in provided mode
    #[must_use]
    pub fn deterministic(self, mode: &DeterministicMode) -> Self {
        match self {
            Self::Map(vector) => {
                let mut data = vector
                    .into_iter()
                    .map(|(k, v)| (k.deterministic(mode), v.deterministic(mode)))
                    .collect::<Vec<(_, _)>>();
                match mode {
                    DeterministicMode::Core => data.sort(),
                    DeterministicMode::LengthFirst => {
                        data.sort_by(|(k1, _), (k2, _)| {
                            let key1_encode = k1.encode();
                            let key2_encode = k2.encode();
                            match key1_encode.len().cmp(&key2_encode.len()) {
                                std::cmp::Ordering::Equal => key1_encode.cmp(&key2_encode),
                                order => order,
                            }
                        });
                    }
                }
                let mut index_map = IndexMap::new();
                index_map.extend(data);
                Self::Map(index_map)
            }
            Self::Array(val) => {
                Self::Array(val.into_iter().map(|v| v.deterministic(mode)).collect())
            }
            Self::Tag(tag_num, val) => Self::Tag(tag_num, Box::new(val.deterministic(mode))),
            _ => self,
        }
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

fn encode_vec_u8(major_type: u8, byte: &[u8]) -> Vec<u8> {
    let byte_length = u64::try_from(byte.len());
    let mut bytes = if let Ok(length) = byte_length {
        encode_u64_number(major_type, length)
    } else {
        vec![major_type << 5 | 31]
    };
    bytes.append(&mut byte.to_vec());
    if byte_length.is_err() {
        bytes.push(255);
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

fn decode_value(iter: &mut Iter<'_, u8>) -> Result<Value, Error> {
    let initial_info = iter.next().ok_or(Error::Empty)?;
    let major_type = initial_info >> 5;
    let additional = initial_info & 0b0001_1111;
    match major_type {
        0 => Ok(Value::Unsigned(extract_number(additional, iter)?)),
        1 => Ok(Value::Signed(extract_number(additional, iter)?)),
        2 => {
            Ok(Value::Byte(decode_byte_or_text(
                major_type, additional, iter,
            )?))
        }
        3 => {
            Ok(Value::Text(String::from_utf8(decode_byte_or_text(
                major_type, additional, iter,
            )?)?))
        }
        4 => decode_array(additional, iter),
        5 => decode_map(additional, iter),
        6 => {
            let tag_number = extract_number(additional, iter)?;
            let tag_value = decode_value(iter)?;
            Ok(Value::Tag(tag_number, Box::new(tag_value)))
        }
        7 => decode_simple_or_floating(additional, iter),
        _ => unreachable!("major type can only be between 0 to 7"),
    }
}

fn decode_byte_or_text(
    major_type: u8,
    additional: u8,
    iter: &mut Iter<'_, u8>,
) -> Result<Vec<u8>, Error> {
    let length = extract_optional_number(additional, iter)?;
    let mut val_vec = vec![];
    if let Some(num) = length {
        val_vec.append(&mut collect_vec_u8(iter, num)?);
    } else {
        val_vec.append(&mut decode_vec_u8(major_type, iter)?);
        match iter.clone().next() {
            Some(255) => {
                iter.next();
            }
            None => return Err(Error::IncompleteIndefinite),
            _ => unreachable!("non 255 some value should be handled already"),
        }
    }
    Ok(val_vec)
}

fn decode_array(additional: u8, iter: &mut Iter<'_, u8>) -> Result<Value, Error> {
    let length = extract_optional_number(additional, iter)?;
    let mut val_vec = vec![];
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
    Ok(Value::Array(val_vec))
}

fn decode_map(additional: u8, iter: &mut Iter<'_, u8>) -> Result<Value, Error> {
    let length: Option<u64> = extract_optional_number(additional, iter)?;
    let mut map_index_map = IndexMap::new();
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
    Ok(Value::Map(map_index_map))
}

fn decode_simple_or_floating(additional: u8, iter: &mut Iter<'_, u8>) -> Result<Value, Error> {
    match additional {
        0..=19 => Ok(Value::UnknownSimple(additional.try_into()?)),
        20 => Ok(Value::Boolean(false)),
        21 => Ok(Value::Boolean(true)),
        22 => Ok(Value::Null),
        23 => Ok(Value::Undefined),
        24 => {
            if let Some(next_num) = iter.next() {
                if *next_num < 32 {
                    Err(Error::InvalidSimple)
                } else {
                    Ok(Value::UnknownSimple((*next_num).try_into()?))
                }
            } else {
                Err(Error::InvalidSimple)
            }
        }
        25 => {
            let number_representation = u16::try_from(extract_number(additional, iter)?)?;
            Ok(Value::Floating(f64::from(half::f16::from_bits(
                number_representation,
            ))))
        }
        26 => {
            let number_representation = u32::try_from(extract_number(additional, iter)?)?;
            Ok(Value::Floating(f64::from(f32::from_bits(
                number_representation,
            ))))
        }
        27 => {
            let f64_number_representation = extract_number(additional, iter)?;
            Ok(Value::Floating(f64::from_bits(f64_number_representation)))
        }
        28..=30 => {
            Err(Error::NotWellFormed(format!(
                "invalid value {additional} for major type 7"
            )))
        }
        31 => Err(Error::LonelyBreakStop),
        _ => unreachable!("Cannot have additional info value greater than 31"),
    }
}

fn decode_vec_u8(major_type: u8, iter: &mut Iter<'_, u8>) -> Result<Vec<u8>, Error> {
    let mut result = vec![];
    #[expect(clippy::collapsible_if, reason = "not supported in stable version")]
    if let Some(peek_val) = iter.clone().next() {
        if *peek_val != 255 {
            let val = decode_value(iter)?;
            if val.major_type() != major_type {
                return Err(Error::NotWellFormed(format!(
                    "contains invalid major type {} for indefinite major type {}",
                    val.major_type(),
                    major_type
                )));
            }
            match val {
                Value::Byte(mut byte) => result.append(&mut byte),
                Value::Text(text) => result.append(&mut text.as_bytes().to_vec()),
                _ => unreachable!("only text and byte calls this function"),
            }
            result.append(&mut decode_vec_u8(major_type, iter)?);
        }
    }
    Ok(result)
}

fn extract_array_item(iter: &mut Iter<'_, u8>) -> Result<Vec<Value>, Error> {
    let mut result = vec![];
    #[expect(clippy::collapsible_if, reason = "not supported in stable version")]
    if let Some(peek_val) = iter.clone().next() {
        if *peek_val != 255 {
            result.push(decode_value(iter)?);
            result.append(&mut extract_array_item(iter)?);
        }
    }
    Ok(result)
}

fn extract_map_item(iter: &mut Iter<'_, u8>) -> Result<IndexMap<Value, Value>, Error> {
    let mut result = IndexMap::new();
    #[expect(clippy::collapsible_if, reason = "not supported in stable version")]
    if let Some(peek_val) = iter.clone().next() {
        if *peek_val != 255 {
            let key = decode_value(iter)?;
            let val = decode_value(iter)?;
            if result.insert(key.clone(), val).is_some() {
                return Err(Error::NotWellFormed(format!(
                    "same map key {key:#?} is repeated multiple times"
                )));
            }
            result.extend(extract_map_item(iter)?);
        }
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
