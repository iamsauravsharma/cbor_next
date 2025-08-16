use std::vec::IntoIter;

use serde::de::{DeserializeOwned, DeserializeSeed, Deserializer};

use crate::DataItem;
use crate::error::Error;

/// CBOR Deserializer
pub struct CborDeserializer {
    data_item: DataItem,
}

/// Deserialize a instance of T from bytes
///
/// # Errors
/// If bytes cannot be decoded as cbor content or instance be deserialized
pub fn from_bytes<T>(bytes: &[u8]) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    let data_item = DataItem::decode(bytes)?; // assuming you have a decode()
    let deserializer = CborDeserializer { data_item };
    T::deserialize(deserializer)
}

impl<'de> Deserializer<'de> for CborDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.data_item.clone() {
            DataItem::Unsigned(unsigned) => visitor.visit_u64(unsigned),
            DataItem::Signed(n) => visitor.visit_i128(-i128::from(n + 1)),
            DataItem::Byte(byte_content) => visitor.visit_byte_buf(byte_content.full()),
            DataItem::Text(text_content) => visitor.visit_string(text_content.full()),
            DataItem::Array(array_content) => {
                let array = ArrayDeserializer {
                    items: array_content.array().to_vec().into_iter(),
                };
                visitor.visit_seq(array)
            }
            DataItem::Map(map_content) => {
                let map = MapDeserializer {
                    items: map_content
                        .map()
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<Vec<_>>()
                        .into_iter(),
                    current_value: None,
                };
                visitor.visit_map(map)
            }
            DataItem::Tag(tag_content) => {
                let inner_content = CborDeserializer {
                    data_item: tag_content.content().clone(),
                };
                inner_content.deserialize_any(visitor)
            }
            DataItem::Boolean(b) => visitor.visit_bool(b),
            DataItem::Null | DataItem::Undefined => visitor.visit_none(),
            DataItem::Floating(f) => visitor.visit_f64(f),
            DataItem::GenericSimple(simple_value) => visitor.visit_u8(*simple_value),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let Some(val) = self.data_item.as_number() else {
            return Err(Error::SerdeMessage("invalid i8 value".to_string()));
        };
        let i8_val = i8::try_from(val)?;
        visitor.visit_i8(i8_val)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let Some(val) = self.data_item.as_number() else {
            return Err(Error::SerdeMessage("invalid i16 value".to_string()));
        };
        let i16_val = i16::try_from(val)?;
        visitor.visit_i16(i16_val)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let Some(val) = self.data_item.as_number() else {
            return Err(Error::SerdeMessage("invalid i32 value".to_string()));
        };
        let i32_val = i32::try_from(val)?;
        visitor.visit_i32(i32_val)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let Some(val) = self.data_item.as_number() else {
            return Err(Error::SerdeMessage("invalid i64 value".to_string()));
        };
        let i64_val = i64::try_from(val)?;
        visitor.visit_i64(i64_val)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let Some(val) = self.data_item.as_number() else {
            return Err(Error::SerdeMessage("invalid u8 value".to_string()));
        };
        let u8_val = u8::try_from(val)?;
        visitor.visit_u8(u8_val)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let Some(val) = self.data_item.as_number() else {
            return Err(Error::SerdeMessage("invalid u16 value".to_string()));
        };
        let u16_val = u16::try_from(val)?;
        visitor.visit_u16(u16_val)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let Some(val) = self.data_item.as_number() else {
            return Err(Error::SerdeMessage("invalid u32 value".to_string()));
        };
        let u32_val = u32::try_from(val)?;
        visitor.visit_u32(u32_val)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let Some(val) = self.data_item.as_number() else {
            return Err(Error::SerdeMessage("invalid u64 value".to_string()));
        };
        let u64_val = u64::try_from(val)?;
        visitor.visit_u64(u64_val)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let Some(val) = self.data_item.as_floating() else {
            return Err(Error::SerdeMessage("invalid f32 value".to_string()));
        };
        #[expect(clippy::cast_possible_truncation, reason = "we check for data loss")]
        let f32_val = val as f32;
        #[expect(
            clippy::float_cmp,
            reason = "we want to compare without margin or error"
        )]
        if f64::from(f32_val) != val {
            return Err(Error::SerdeMessage(
                "cannot represent f32 as f64".to_string(),
            ));
        }
        visitor.visit_f32(f32_val)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let Some(val) = self.data_item.as_floating() else {
            return Err(Error::SerdeMessage("invalid f64 value".to_string()));
        };
        visitor.visit_f64(val)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let Some(val) = self.data_item.as_text() else {
            return Err(Error::SerdeMessage("invalid char value".to_string()));
        };
        let mut chars = val.chars();
        let Some(char) = chars.next() else {
            return Err(Error::SerdeMessage("char cannot be empty".to_string()));
        };
        if chars.next().is_some() {
            return Err(Error::SerdeMessage(
                "char can only contain one character".to_string(),
            ));
        }
        visitor.visit_char(char)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let Some(val) = self.data_item.as_text() else {
            return Err(Error::SerdeMessage("invalid char value".to_string()));
        };
        visitor.visit_str(&val)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let Some(val) = self.data_item.as_text() else {
            return Err(Error::SerdeMessage("invalid char value".to_string()));
        };
        visitor.visit_string(val)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let Some(val) = self.data_item.as_byte() else {
            return Err(Error::SerdeMessage("invalid char value".to_string()));
        };
        visitor.visit_bytes(&val)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let Some(val) = self.data_item.as_byte() else {
            return Err(Error::SerdeMessage("invalid char value".to_string()));
        };
        visitor.visit_byte_buf(val)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.data_item.is_null() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.data_item.is_null() {
            visitor.visit_none()
        } else {
            Err(Error::SerdeMessage("invalid null value".to_string()))
        }
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let Some(val) = self.data_item.as_array() else {
            return Err(Error::SerdeMessage("invalid char value".to_string()));
        };
        let val_vec = val.to_vec();
        let array = ArrayDeserializer {
            items: val_vec.into_iter(),
        };
        visitor.visit_seq(array)
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let Some(val) = self.data_item.as_map() else {
            return Err(Error::SerdeMessage("invalid char value".to_string()));
        };
        let val_vec = val
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<_>>();
        let map = MapDeserializer {
            items: val_vec.into_iter(),
            current_value: None,
        };
        visitor.visit_map(map)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match &self.data_item {
            DataItem::Text(variant) => {
                visitor.visit_enum(EnumDeserializer {
                    variant: DataItem::from(variant.full()),
                    value: None,
                })
            }

            DataItem::Map(map_content) => {
                if map_content.map().len() == 1 {
                    let (key, value) = map_content.map().iter().next().unwrap();
                    let deserializer = EnumDeserializer {
                        variant: key.clone(),
                        value: Some(value.clone()),
                    };
                    visitor.visit_enum(deserializer)
                } else {
                    Err(Error::SerdeMessage(
                        "enum must be represented as a single-entry map or string".to_string(),
                    ))
                }
            }
            _ => {
                Err(Error::SerdeMessage(
                    "enum must be represented as a string or map".to_string(),
                ))
            }
        }
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

/// struct for array deserializer
struct ArrayDeserializer {
    items: IntoIter<DataItem>,
}

impl<'de> serde::de::SeqAccess<'de> for ArrayDeserializer {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.items.next() {
            Some(data_item) => {
                let de = CborDeserializer { data_item };
                seed.deserialize(de).map(Some)
            }
            None => Ok(None),
        }
    }
}

/// struct for map deserializer
struct MapDeserializer {
    items: IntoIter<(DataItem, DataItem)>,
    current_value: Option<DataItem>,
}

impl<'de> serde::de::MapAccess<'de> for MapDeserializer {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        if self.current_value.is_some() {
            return Err(Error::SerdeMessage(
                "key requested multiple time without value".to_string(),
            ));
        }
        match self.items.next() {
            Some((k, v)) => {
                self.current_value = Some(v);
                let de = CborDeserializer { data_item: k };
                seed.deserialize(de).map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let Some(data_item) = &self.current_value else {
            return Err(Error::SerdeMessage(
                "value requested without matching key".to_string(),
            ));
        };
        let de = CborDeserializer {
            data_item: data_item.clone(),
        };
        self.current_value = None;
        seed.deserialize(de)
    }
}

struct EnumDeserializer {
    variant: DataItem,
    value: Option<DataItem>,
}

impl<'de> serde::de::EnumAccess<'de> for EnumDeserializer {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<S>(self, seed: S) -> Result<(S::Value, Self::Variant), Self::Error>
    where
        S: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(CborDeserializer {
            data_item: self.variant.clone(),
        })?;
        Ok((variant, self))
    }
}

impl<'de> serde::de::VariantAccess<'de> for EnumDeserializer {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        match self.value {
            None => Ok(()),
            Some(_) => {
                Err(Error::SerdeMessage(
                    "expected unit variant but got data".to_string(),
                ))
            }
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.value {
            Some(value) => seed.deserialize(CborDeserializer { data_item: value }),
            None => {
                Err(Error::SerdeMessage(
                    "expected newtype variant but got nothing".to_string(),
                ))
            }
        }
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            Some(DataItem::Array(items)) => {
                let array = ArrayDeserializer {
                    items: items.array().to_vec().into_iter(),
                };
                visitor.visit_seq(array)
            }
            Some(_) => {
                Err(Error::SerdeMessage(
                    "expected tuple variant but got non-array".to_string(),
                ))
            }
            None => {
                Err(Error::SerdeMessage(
                    "expected tuple variant but got nothing".to_string(),
                ))
            }
        }
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            Some(DataItem::Map(map)) => {
                let map = MapDeserializer {
                    items: map
                        .map()
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<Vec<_>>()
                        .into_iter(),
                    current_value: None,
                };
                visitor.visit_map(map)
            }
            Some(_) => {
                Err(Error::SerdeMessage(
                    "expected struct variant but got non-map".to_string(),
                ))
            }
            None => {
                Err(Error::SerdeMessage(
                    "expected struct variant but got nothing".to_string(),
                ))
            }
        }
    }
}
