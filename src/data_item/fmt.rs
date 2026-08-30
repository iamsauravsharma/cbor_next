use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use crate::data_item::DataItem;

/// Both [`Debug`] and [`Display`] write the diagnostic notation of RFC 8949
/// section 8, so a data item reads the way the specification writes it.
impl Debug for DataItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Unsigned(number) => Debug::fmt(number, f),
            Self::Signed(number) => Debug::fmt(&(-(i128::from(*number) + 1)), f),
            Self::Floating(number) => {
                if number.is_nan() {
                    return f.write_str("NaN");
                }
                match *number {
                    f64::INFINITY => f.write_str("Infinity"),
                    f64::NEG_INFINITY => f.write_str("-Infinity"),
                    _ => Debug::fmt(number, f),
                }
            }
            Self::Boolean(bool_val) => Debug::fmt(bool_val, f),
            Self::Null => f.write_str("null"),
            Self::Undefined => f.write_str("undefined"),
            Self::GenericSimple(simple) => simple.fmt(f),
            Self::Byte(byte_string) => byte_string.fmt(f),
            Self::Text(text_string) => text_string.fmt(f),
            Self::Array(array) => array.fmt(f),
            Self::Map(map) => map.fmt(f),
            Self::Tag(tag) => tag.fmt(f),
        }
    }
}

impl Display for DataItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        Debug::fmt(self, f)
    }
}
