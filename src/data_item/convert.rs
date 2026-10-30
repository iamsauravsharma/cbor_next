use std::hash::Hash;
use std::num::TryFromIntError;

use indexmap::IndexMap;

use crate::content::{Array, ByteString, Map, Simple, Tag, TextString};
use crate::data_item::DataItem;

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
            // `!value` is `-value - 1` without overflowing on `i64::MIN`
            let u64_val = u64::try_from(!value).expect("i64 positive can be converted to u64");
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
            // `!value` is `-value - 1` without overflowing on `i128::MIN`
            Ok(Self::Signed(u64::try_from(!value)?))
        } else {
            Ok(Self::Unsigned(u64::try_from(value)?))
        }
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

impl From<f32> for DataItem {
    fn from(value: f32) -> Self {
        Self::Floating(f32_to_f64(value))
    }
}

impl From<half::f16> for DataItem {
    fn from(value: half::f16) -> Self {
        Self::Floating(f16_to_f64(value))
    }
}

/// Sign bit and exponent of an `f64` NaN
const F64_NAN_EXPONENT: u64 = 0x7FF0_0000_0000_0000;

/// Widen a half precision float, keeping the significand of a NaN.
///
/// A plain conversion sets the quiet bit of a signaling NaN.
pub(crate) fn f16_to_f64(value: half::f16) -> f64 {
    if value.is_nan() {
        let bits = value.to_bits();
        let sign = u64::from(bits & 0x8000) << 48;
        let significand = u64::from(bits & 0x03FF) << 42;
        f64::from_bits(sign | F64_NAN_EXPONENT | significand)
    } else {
        value.to_f64()
    }
}

/// Widen a single precision float, keeping the significand of a NaN.
pub(crate) fn f32_to_f64(value: f32) -> f64 {
    if value.is_nan() {
        let bits = value.to_bits();
        let sign = u64::from(bits & 0x8000_0000) << 32;
        let significand = u64::from(bits & 0x007F_FFFF) << 29;
        f64::from_bits(sign | F64_NAN_EXPONENT | significand)
    } else {
        f64::from(value)
    }
}

impl From<ByteString> for DataItem {
    fn from(value: ByteString) -> Self {
        Self::Byte(value)
    }
}

impl From<TextString> for DataItem {
    fn from(value: TextString) -> Self {
        Self::Text(value)
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

impl From<Array> for DataItem {
    fn from(value: Array) -> Self {
        Self::Array(value)
    }
}

/// A vector of any item which converts into a data item becomes an array. A
/// vector of bytes is therefore an array of unsigned integers, not a byte
/// string, which [`DataItem::bytes`] builds instead.
impl<T> From<Vec<T>> for DataItem
where
    T: Into<DataItem>,
{
    fn from(value: Vec<T>) -> Self {
        Self::Array(Array::from_items(value))
    }
}

impl<T, const N: usize> From<[T; N]> for DataItem
where
    T: Into<DataItem>,
{
    fn from(value: [T; N]) -> Self {
        Self::Array(Array::from_items(value))
    }
}

impl From<Map> for DataItem {
    fn from(value: Map) -> Self {
        Self::Map(value)
    }
}

impl<K, V> From<Vec<(K, V)>> for DataItem
where
    K: Into<DataItem> + Hash + Eq,
    V: Into<DataItem>,
{
    fn from(value: Vec<(K, V)>) -> Self {
        Self::Map(Map::from_entries(value))
    }
}

impl<K, V> From<IndexMap<K, V>> for DataItem
where
    K: Into<DataItem>,
    V: Into<DataItem>,
{
    fn from(value: IndexMap<K, V>) -> Self {
        Self::Map(Map::from_entries(value))
    }
}

impl From<Tag> for DataItem {
    fn from(value: Tag) -> Self {
        Self::Tag(value)
    }
}

impl From<Simple> for DataItem {
    fn from(value: Simple) -> Self {
        Self::GenericSimple(value)
    }
}

impl<T> From<Option<T>> for DataItem
where
    T: Into<DataItem>,
{
    fn from(value: Option<T>) -> Self {
        value.map_or(Self::Null, Into::into)
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
