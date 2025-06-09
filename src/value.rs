use std::slice::Iter;

use crate::error::Error;

/// Enum representing different type of value which can be represented in CBOR
#[derive(PartialEq, Debug)]
pub enum Value {
    /// Unsigned integer represented by major type 0
    Unsigned(u64),
    /// Negative integer represented by major type 1
    Signed(u64),
    /// Major type 2 byte string
    Byte(Vec<u8>),
    /// Major type 3 utf8 string
    Text(String),
    /// Major type 4 representing a array
    Array(Vec<Value>),
    /// Major type 5 representing a Map
    Map(Vec<(Value, Value)>),
    /// Major type 6 representing a tag object
    Tag(u64, Box<Value>),
    /// Boolean which is represented in major type 7 as simple value
    Boolean(bool),
    /// Null value
    Null,
    /// Undefined value
    Undefined,
    /// Floating value which is major byte 7
    Floating(f64),
    /// Unknown simple value
    UnknownSimple(u8),
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Self::Unsigned(value)
    }
}

impl From<Vec<u8>> for Value {
    fn from(value: Vec<u8>) -> Self {
        Self::Byte(value)
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

impl<T> From<Vec<T>> for Value
where
    T: Into<Value>,
{
    fn from(value: Vec<T>) -> Self {
        Self::Array(value.into_iter().map(|m| m.into()).collect())
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

impl Value {
    /// Get a major type of a value
    pub fn major_type(&self) -> u8 {
        match self {
            Value::Unsigned(_) => 0,
            Value::Signed(_) => 1,
            Value::Byte(_) => 2,
            Value::Text(_) => 3,
            Value::Array(_) => 4,
            Value::Map(_) => 5,
            Value::Tag(_, _) => 6,
            Value::Boolean(_)
            | Value::Null
            | Value::Undefined
            | Value::Floating(_)
            | Self::UnknownSimple(_) => 7,
        }
    }

    /// Get a CBOR encoded representation of value
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Value::Unsigned(number) | Value::Signed(number) => {
                u64_to_cbor_u8(self.major_type(), *number)
            }
            Value::Byte(byte) => vec_u8_to_cbor_u8(self.major_type(), byte),
            Value::Text(text_string) => {
                vec_u8_to_cbor_u8(self.major_type(), text_string.as_bytes())
            }
            Value::Array(array) => {
                let array_len = u64::try_from(array.len());
                let mut array_bytes = if let Ok(length) = array_len {
                    u64_to_cbor_u8(self.major_type(), length)
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
                    u64_to_cbor_u8(self.major_type(), length)
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
                let mut tag_bytes = u64_to_cbor_u8(self.major_type(), *number);
                tag_bytes.append(&mut value.encode());
                tag_bytes
            }
            Value::Boolean(bool_val) => match bool_val {
                false => vec![self.major_type() << 5 | 20],
                true => vec![self.major_type() << 5 | 21],
            },
            Value::Null => vec![self.major_type() << 5 | 22],
            Value::Undefined => vec![self.major_type() << 5 | 23],
            Value::Floating(number) => f64_to_cbor_u8(self.major_type(), *number),
            Value::UnknownSimple(simple_number) => {
                if *simple_number <= 23 {
                    vec![self.major_type() << 5 | simple_number]
                } else {
                    vec![self.major_type() << 5 | 24, *simple_number]
                }
            }
        }
    }

    /// Decode a CBOR representation to a value
    pub fn decode(val: &[u8]) -> Result<Self, Error> {
        let mut iter = val.iter();
        decode_value(&mut iter)
    }
}

fn decode_value(iter: &mut Iter<'_, u8>) -> Result<Value, Error> {
    let initial_info = iter.next().ok_or(Error::Empty)?;
    let major_type = initial_info >> 5;
    let additional = initial_info & 0b00011111;
    match major_type {
        0 => Ok(Value::Unsigned(extract_number(additional, iter)?)),
        1 => Ok(Value::Signed(extract_number(additional, iter)?)),
        2 => Ok(Value::Byte(decode_byte_or_text(
            major_type, additional, iter,
        )?)),
        3 => Ok(Value::Text(
            String::from_utf8(decode_byte_or_text(major_type, additional, iter)?)
                .map_err(|_| Error::Invalid("invalid UTF-8 string".to_string()))?,
        )),
        4 => {
            let length = extract_optional_number(additional, iter)?;
            let mut val_vec = vec![];
            if let Some(num) = length {
                for _ in 0..num {
                    val_vec.push(decode_value(iter)?);
                }
            } else {
                val_vec.append(&mut decode_array(iter)?);
                match iter.clone().next() {
                    Some(255) => {
                        iter.next();
                    }
                    None => {
                        return Err(Error::Invalid("incomplete indefinite array".to_string()));
                    }
                    _ => unreachable!("non 255 some value should be handled already"),
                }
            }
            Ok(Value::Array(val_vec))
        }
        5 => {
            let length = extract_optional_number(additional, iter)?;
            let mut val_vec = vec![];
            if let Some(num) = length {
                for _ in 0..num {
                    let key = decode_value(iter)?;
                    let val = decode_value(iter)?;
                    val_vec.push((key, val));
                }
            } else {
                val_vec.append(&mut decode_map(iter)?);
                match iter.clone().next() {
                    Some(255) => {
                        iter.next();
                    }
                    None => {
                        return Err(Error::Invalid("incomplete indefinite map".to_string()));
                    }
                    _ => unreachable!("non 255 some value should be handled already"),
                }
            }
            Ok(Value::Map(val_vec))
        }
        6 => {
            let tag_number = extract_number(additional, iter)?;
            let tag_value = decode_value(iter)?;
            Ok(Value::Tag(tag_number, Box::new(tag_value)))
        }
        7 => match additional {
            0..=19 => Ok(Value::UnknownSimple(additional)),
            20 => Ok(Value::Boolean(false)),
            21 => Ok(Value::Boolean(true)),
            22 => Ok(Value::Null),
            23 => Ok(Value::Undefined),
            24 => {
                if let Some(next_num) = iter.next() {
                    if *next_num < 32 {
                        Err(Error::Invalid("Simple value cannot have less than 32 value when using 24 additional info".to_string()))
                    } else {
                        Ok(Value::UnknownSimple(*next_num))
                    }
                } else {
                    Err(Error::Invalid("Missing number for simple".to_string()))
                }
            }
            25 => {
                let number_representation = u16::try_from(extract_number(additional, iter)?)
                    .map_err(|_| Error::Invalid("Invalid number for f16".to_string()))?;
                Ok(Value::Floating(f64::from(f16::from_bits(
                    number_representation,
                ))))
            }
            26 => {
                let number_representation = u32::try_from(extract_number(additional, iter)?)
                    .map_err(|_| Error::Invalid("Invalid number for f32".to_string()))?;
                Ok(Value::Floating(f64::from(f32::from_bits(
                    number_representation,
                ))))
            }
            27 => {
                let f64_number_representation = extract_number(additional, iter)?;
                Ok(Value::Floating(f64::from_bits(f64_number_representation)))
            }
            28..=30 => Err(Error::Invalid("not well formed currently".to_string())),
            31 => Err(Error::Invalid("break stop cannot be itself".to_string())),
            _ => unreachable!("Cannot have additional info value greater than 31"),
        },
        _ => unreachable!("major type can only be between 0 to 7"),
    }
}

fn u64_to_cbor_u8(major_type: u8, number: u64) -> Vec<u8> {
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

fn vec_u8_to_cbor_u8(major_type: u8, byte: &[u8]) -> Vec<u8> {
    let byte_length = u64::try_from(byte.len());
    let mut bytes = if let Ok(length) = byte_length {
        u64_to_cbor_u8(major_type, length)
    } else {
        vec![major_type << 5 | 31]
    };
    bytes.append(&mut byte.to_vec());
    if byte_length.is_err() {
        bytes.push(255);
    }
    bytes
}

fn f64_to_cbor_u8(major_type: u8, f64_number: f64) -> Vec<u8> {
    let shifted_major_type = major_type << 5;
    let mut cbor_representation = vec![];
    if f64::from(f64_number as f16) == f64_number {
        cbor_representation.push(shifted_major_type | 25);
        for byte in (f64_number as f16).to_be_bytes() {
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

fn decode_vec_u8(major_type: u8, iter: &mut Iter<'_, u8>) -> Result<Vec<u8>, Error> {
    let mut result = vec![];
    if let Some(peek_val) = iter.clone().next()
        && peek_val != &255
    {
        let val = decode_value(iter)?;
        if val.major_type() != major_type {
            return Err(Error::Invalid("invalid major in between".to_string()));
        }
        match val {
            Value::Byte(mut byte) => result.append(&mut byte),
            Value::Text(text) => result.append(&mut text.as_bytes().to_vec()),
            _ => unreachable!("only text and byte calls this function"),
        }
        result.append(&mut decode_vec_u8(major_type, iter)?);
    }
    Ok(result)
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
            None => return Err(Error::Invalid("incomplete indefinite map".to_string())),
            _ => unreachable!("non 255 some value should be handled already"),
        }
    }
    Ok(val_vec)
}

fn decode_array(iter: &mut Iter<'_, u8>) -> Result<Vec<Value>, Error> {
    let mut result = vec![];
    if let Some(peek_val) = iter.clone().next()
        && peek_val != &255
    {
        result.push(decode_value(iter)?);
        result.append(&mut decode_array(iter)?);
    }
    Ok(result)
}

fn decode_map(iter: &mut Iter<'_, u8>) -> Result<Vec<(Value, Value)>, Error> {
    let mut result = vec![];
    if let Some(peek_val) = iter.clone().next()
        && peek_val != &255
    {
        let key = decode_value(iter)?;
        let val = decode_value(iter)?;
        result.push((key, val));
        result.append(&mut decode_map(iter)?);
    }

    Ok(result)
}

fn collect_vec_u8(iter: &mut Iter<'_, u8>, number: u64) -> Result<Vec<u8>, Error> {
    let mut collected_val = Vec::new();
    for _ in 0..number {
        match iter.next() {
            Some(item) => collected_val.push(*item),
            None => return Err(Error::Invalid("incomplete value missing bytes".to_string())),
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
        28..=30 => Err(Error::Invalid(
            "invalid additional information number".to_string(),
        )),
        31 => Ok(None),
        _ => unreachable!("Cannot have additional info value greater than 31"),
    }
}

fn extract_number(additional: u8, iter: &mut Iter<'_, u8>) -> Result<u64, Error> {
    extract_optional_number(additional, iter)?.ok_or(Error::Invalid(
        "major type does not support indefinite value".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use core::f64;
    use std::vec;

    use crate::value::Value;

    fn encode_compare<I>(hex_cbor: &str, value_into: I)
    where
        I: Into<Value>,
    {
        let value = value_into.into();
        let vec_u8_cbor = hex::decode(hex_cbor)
            .unwrap_or_else(|err| panic!("{err} failed to decode hex {hex_cbor}"));
        let value_to_cbor = value.encode();
        assert_eq!(value_to_cbor, vec_u8_cbor, "{hex_cbor}");
    }

    fn decode_compare<I>(hex_cbor: &str, value_into: I)
    where
        I: Into<Value>,
    {
        let value = value_into.into();
        let vec_u8_cbor =
            hex::decode(hex_cbor).unwrap_or_else(|_| panic!(" failed to decode hex {hex_cbor}"));
        let cbor_to_value =
            Value::decode(&vec_u8_cbor).unwrap_or_else(|err: crate::error::Error| {
                panic!("{err} failed to decode value {hex_cbor}")
            });
        assert_eq!(&cbor_to_value, &value, "{hex_cbor}");
    }

    fn compare_cbor_value<I>(hex_cbor: &str, value_into: I)
    where
        I: Into<Value>,
    {
        let value = value_into.into();
        let vec_u8_cbor = hex::decode(hex_cbor)
            .unwrap_or_else(|err| panic!("{err} failed to decode hex {hex_cbor}"));
        let value_to_cbor = value.encode();
        assert_eq!(value_to_cbor, vec_u8_cbor, "{hex_cbor}");
        let cbor_to_value = Value::decode(&vec_u8_cbor)
            .unwrap_or_else(|err| panic!("{err} failed to decode value {hex_cbor}"));
        assert_eq!(&cbor_to_value, &value, "{hex_cbor}");
    }

    #[test]
    fn test_integer() {
        compare_cbor_value("00", 0);
        compare_cbor_value("01", 1);
        compare_cbor_value("0a", 10);
        compare_cbor_value("17", 23);
        compare_cbor_value("1818", 24);
        compare_cbor_value("1819", 25);
        compare_cbor_value("1864", 100);
        compare_cbor_value("1903e8", 1000);
        compare_cbor_value("1a000f4240", 1_000_000);
        compare_cbor_value("1b000000e8d4a51000", 1_000_000_000_000);
        compare_cbor_value("1bffffffffffffffff", 18_446_744_073_709_551_615);
        compare_cbor_value("3bffffffffffffffff", Value::Signed(18446744073709551615));
        compare_cbor_value("20", Value::Signed(0));
        compare_cbor_value("29", Value::Signed(9));
        compare_cbor_value("3863", Value::Signed(99));
        compare_cbor_value("3903e7", Value::Signed(999));
    }

    #[test]
    fn test_float() {
        compare_cbor_value("f90000", 0.0);
        compare_cbor_value("f98000", -0.0);
        compare_cbor_value("f93c00", 1.0);
        compare_cbor_value("fb3ff199999999999a", 1.1);
        compare_cbor_value("f93e00", 1.5);
        compare_cbor_value("f97bff", 65504.0);
        compare_cbor_value("fa47c35000", 100_000.0);
        compare_cbor_value("f90400", 6.103515625e-05);
        compare_cbor_value("f90001", 5.960464477539063e-08);
        compare_cbor_value("fa7f7fffff", 3.4028234663852886e+38);
        compare_cbor_value("fb7e37e43c8800759c", 1.0e+300);
        compare_cbor_value("f9c400", -4.0);
        compare_cbor_value("fbc010666666666666", -4.1);
        compare_cbor_value("f97c00", f64::INFINITY);
        compare_cbor_value("f9fc00", f64::NEG_INFINITY);
        decode_compare("fa7f800000", f64::INFINITY);
        decode_compare("faff800000", f64::NEG_INFINITY);
        decode_compare("fb7ff0000000000000", f64::INFINITY);
        decode_compare("fbfff0000000000000", f64::NEG_INFINITY);
        encode_compare("fb7ff8000000000000", f64::NAN);
    }

    #[test]
    fn test_simple() {
        compare_cbor_value("f4", false);
        compare_cbor_value("f5", true);
        compare_cbor_value("f6", Value::Null);
        compare_cbor_value("f7", Value::Undefined);
        compare_cbor_value("f0", Value::UnknownSimple(16));
        compare_cbor_value("f820", Value::UnknownSimple(32));
        compare_cbor_value("f8ff", Value::UnknownSimple(255));
    }

    #[test]
    fn test_tag() {
        compare_cbor_value(
            "c074323031332d30332d32315432303a30343a30305a",
            Value::Tag(0, Box::new("2013-03-21T20:04:00Z".into())),
        );
        compare_cbor_value(
            "c074323031332d30332d32315432303a30343a30305a",
            Value::Tag(0, Box::new("2013-03-21T20:04:00Z".into())),
        );
        compare_cbor_value("c11a514b67b0", Value::Tag(1, Box::new(1363896240.into())));
        compare_cbor_value(
            "c1fb41d452d9ec200000",
            Value::Tag(1, Box::new(1363896240.5.into())),
        );
        compare_cbor_value(
            "d74401020304",
            Value::Tag(23, Box::new(hex::decode("01020304").unwrap().into())),
        );
        compare_cbor_value(
            "d818456449455446",
            Value::Tag(24, Box::new(hex::decode("6449455446").unwrap().into())),
        );
        compare_cbor_value(
            "d82076687474703a2f2f7777772e6578616d706c652e636f6d",
            Value::Tag(32, Box::new("http://www.example.com".into())),
        );
    }

    #[test]
    fn test_byte() {
        compare_cbor_value("40", Vec::<u8>::new());
        compare_cbor_value("4401020304", hex::decode("01020304").unwrap());
        decode_compare("5f42010243030405ff", hex::decode("0102030405").unwrap());
    }

    #[test]
    fn test_text() {
        compare_cbor_value("60", "");
        compare_cbor_value("6161", "a");
        compare_cbor_value("6449455446", "IETF");
        compare_cbor_value("62225c", "\"\\");
        compare_cbor_value("62c3bc", "ü");
        compare_cbor_value("63e6b0b4", "水");
        compare_cbor_value("64f0908591", "𐅑");
        decode_compare("7f657374726561646d696e67ff", "streaming");
    }

    #[test]
    fn test_array() {
        compare_cbor_value("80", Vec::<u64>::new());
        compare_cbor_value("83010203", vec![1u64, 2, 3]);
        compare_cbor_value::<Vec<Value>>(
            "8301820203820405",
            vec![1u64.into(), vec![2u64, 3].into(), vec![4u64, 5u64].into()],
        );
        compare_cbor_value(
            "98190102030405060708090a0b0c0d0e0f101112131415161718181819",
            vec![
                1u64, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23, 24, 25,
            ],
        );
        compare_cbor_value::<Vec<Value>>(
            "826161a161626163",
            vec!["a".into(), vec![("b", "c")].into()],
        );
        decode_compare("9fff", Vec::<u64>::new());
        decode_compare::<Vec<Value>>(
            "9f018202039f0405ffff",
            vec![1u64.into(), vec![2u64, 3].into(), vec![4u64, 5u64].into()],
        );
        decode_compare::<Vec<Value>>(
            "9f01820203820405ff",
            vec![1u64.into(), vec![2u64, 3].into(), vec![4u64, 5u64].into()],
        );
        decode_compare::<Vec<Value>>(
            "83018202039f0405ff",
            vec![1u64.into(), vec![2u64, 3].into(), vec![4u64, 5u64].into()],
        );
        decode_compare::<Vec<Value>>(
            "83019f0203ff820405",
            vec![1u64.into(), vec![2u64, 3].into(), vec![4u64, 5u64].into()],
        );
        decode_compare(
            "9f0102030405060708090a0b0c0d0e0f101112131415161718181819ff",
            vec![
                1u64, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23, 24, 25,
            ],
        );
        decode_compare::<Vec<Value>>(
            "826161bf61626163ff",
            vec!["a".into(), vec![("b", "c")].into()],
        );
    }

    #[test]
    fn test_map() {
        compare_cbor_value("a0", Value::Map(vec![]));
        compare_cbor_value("a201020304", vec![(1, 2), (3, 4)]);
        compare_cbor_value(
            "a26161016162820203",
            vec![("a", Value::from(1)), ("b", vec![2u64, 3].into())],
        );
        compare_cbor_value(
            "a56161614161626142616361436164614461656145",
            vec![("a", "A"), ("b", "B"), ("c", "C"), ("d", "D"), ("e", "E")],
        );
        decode_compare(
            "bf61610161629f0203ffff",
            vec![("a", Value::from(1)), ("b", vec![2u64, 3].into())],
        );
        decode_compare(
            "bf6346756ef563416d7421ff",
            vec![("Fun", Value::from(true)), ("Amt", Value::Signed(1))],
        );
    }

    #[test]
    fn test_failure() {
        assert!(Value::decode(&hex::decode("1c").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("7f14").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("f801").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("9fde").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("bf3e").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("7fbb").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("dc").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("7f42").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("5f87").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("3f").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("5d").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("bc").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("5f4100").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("5fc000ff").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("9f819f819f9fffffff").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("9f829f819f9fffffffff").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("1a0102").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("5affffffff00").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("bf000000ff").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("a2000000").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("5fd9").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("bffc").unwrap()).is_err());
        assert!(Value::decode(&hex::decode("ff").unwrap()).is_err());
    }
}
