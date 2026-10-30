use ::serde::Deserializer as _;
use ::serde::de::value::{U8Deserializer, U32Deserializer, U64Deserializer};
use ::serde::de::{
    self, Deserialize, DeserializeOwned, DeserializeSeed, EnumAccess, MapAccess, SeqAccess,
    VariantAccess, Visitor,
};
use indexmap::map::Iter as MapIter;

use super::error::{Error, unsupported};
use super::wrapper::{DATA_ITEM_NAME, SIMPLE_NAME, TAG_NAME, UNDEFINED_NAME};
use crate::data_item::{DataItem, convert};
use crate::tags::{NEGATIVE_BIGNUM, UNSIGNED_BIGNUM};

/// Turn a data item into a value.
///
/// # Errors
/// If the item does not fit the type, or holds data which serde cannot
/// represent: a tag which nothing asked for, `undefined` or a simple value in
/// place of another type
///
/// # Example
/// ```rust
/// use cbor_next::DataItem;
/// use cbor_next::serde::from_data_item;
///
/// let item = DataItem::array([DataItem::from(1), DataItem::from("a")]);
/// assert_eq!(
///     from_data_item::<(u8, String)>(&item).unwrap(),
///     (1, "a".into())
/// );
///
/// // a tag has no counterpart in serde and is not dropped
/// assert!(from_data_item::<u8>(&DataItem::tag(1, 1)).is_err());
/// ```
pub fn from_data_item<'de, T>(item: &'de DataItem) -> Result<T, Error>
where
    T: Deserialize<'de>,
{
    T::deserialize(item)
}

/// Decode `CBOR` bytes holding one data item into a value.
///
/// # Errors
/// If the bytes are not a well formed data item, if bytes follow it, or under
/// the conditions of [`from_data_item`]
///
/// # Example
/// ```rust
/// use cbor_next::serde::from_slice;
///
/// assert_eq!(from_slice::<Vec<u8>>(&[0x82, 0x01, 0x02]).unwrap(), [1, 2]);
/// ```
pub fn from_slice<T>(bytes: &[u8]) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    from_data_item(&DataItem::decode(bytes)?)
}

/// Variant index a tag takes when a [`DataItem`] is read from one
pub(super) const TAG_VARIANT: u32 = 0;
/// Variant index `undefined` takes when a [`DataItem`] is read from it
pub(super) const UNDEFINED_VARIANT: u32 = 1;
/// Variant index a simple value takes when a [`DataItem`] is read from one
pub(super) const SIMPLE_VARIANT: u32 = 2;

/// The magnitude a bignum holds, RFC 8949 section 3.4.3.
fn bignum_magnitude(number: u64, content: &DataItem) -> Result<u128, Error> {
    let DataItem::Byte(bytes) = content else {
        return Err(unsupported(format_args!(
            "tag {number} with content {content}, which is not a byte string"
        )));
    };
    convert::bignum_magnitude(&bytes.to_bytes()).ok_or_else(|| {
        unsupported(format_args!(
            "bignum {content}, which is wider than 128 bits"
        ))
    })
}

impl<'de> de::Deserializer<'de> for &'de DataItem {
    type Error = Error;

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        unit seq tuple map struct identifier ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            DataItem::Unsigned(number) => visitor.visit_u64(*number),
            DataItem::Signed(number) => {
                let value = -(i128::from(*number) + 1);
                if let Ok(small) = i64::try_from(value) {
                    visitor.visit_i64(small)
                } else {
                    visitor.visit_i128(value)
                }
            }
            DataItem::Byte(bytes) => {
                match bytes.chunks() {
                    [chunk] => visitor.visit_borrowed_bytes(chunk),
                    _ => visitor.visit_byte_buf(bytes.to_bytes()),
                }
            }
            DataItem::Text(text) => {
                match text.chunks() {
                    [chunk] => visitor.visit_borrowed_str(chunk),
                    _ => visitor.visit_string(text.to_text()),
                }
            }
            DataItem::Array(array) => {
                let mut access = SeqDeserializer {
                    iter: array.items().iter(),
                };
                let value = visitor.visit_seq(&mut access)?;
                if access.iter.len() == 0 {
                    Ok(value)
                } else {
                    Err(de::Error::invalid_length(
                        array.items().len(),
                        &"fewer elements in array",
                    ))
                }
            }
            DataItem::Map(map) => {
                let mut access = MapDeserializer {
                    iter: map.entries().iter(),
                    value: None,
                };
                let value = visitor.visit_map(&mut access)?;
                if access.iter.len() == 0 {
                    Ok(value)
                } else {
                    Err(de::Error::invalid_length(
                        map.entries().len(),
                        &"fewer entries in map",
                    ))
                }
            }
            DataItem::Tag(tag) => {
                match tag.number() {
                    UNSIGNED_BIGNUM => {
                        let magnitude = bignum_magnitude(UNSIGNED_BIGNUM, tag.content())?;
                        match u64::try_from(magnitude) {
                            Ok(small) => visitor.visit_u64(small),
                            Err(_) => visitor.visit_u128(magnitude),
                        }
                    }
                    NEGATIVE_BIGNUM => {
                        let magnitude = bignum_magnitude(NEGATIVE_BIGNUM, tag.content())?;
                        let Ok(magnitude) = i128::try_from(magnitude) else {
                            return Err(unsupported(format_args!(
                                "bignum -1 - {magnitude}, which is below the range of i128"
                            )));
                        };
                        let value = -1 - magnitude;
                        match i64::try_from(value) {
                            Ok(small) => visitor.visit_i64(small),
                            Err(_) => visitor.visit_i128(value),
                        }
                    }
                    number => Err(unsupported(format_args!("tag {number}"))),
                }
            }
            DataItem::Boolean(value) => visitor.visit_bool(*value),
            DataItem::Null => visitor.visit_unit(),
            DataItem::Undefined => Err(unsupported("undefined")),
            DataItem::Floating(number) => visitor.visit_f64(*number),
            DataItem::GenericSimple(simple) => {
                Err(unsupported(format_args!("simple value {}", simple.value())))
            }
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            DataItem::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            DataItem::Undefined if name == UNDEFINED_NAME => visitor.visit_unit(),
            _ if name == UNDEFINED_NAME => {
                Err(de::Error::custom(format_args!(
                    "expected undefined, found {self}"
                )))
            }
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            DataItem::GenericSimple(simple) if name == SIMPLE_NAME => {
                visitor.visit_u8(simple.value())
            }
            _ if name == SIMPLE_NAME => {
                Err(de::Error::custom(format_args!(
                    "expected a simple value, found {self}"
                )))
            }
            // an item with no counterpart in serde is handed over as a
            // variant, which the visitor of a data item knows to unwrap
            DataItem::Tag(_) | DataItem::Undefined | DataItem::GenericSimple(_)
                if name == DATA_ITEM_NAME =>
            {
                visitor.visit_enum(SpecialAccess { item: self })
            }
            _ if name == DATA_ITEM_NAME => self.deserialize_any(visitor),
            _ => visitor.visit_newtype_struct(self),
        }
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            DataItem::Tag(tag) if name == TAG_NAME => {
                visitor.visit_seq(TagAccess {
                    number: Some(tag.number()),
                    content: Some(tag.content()),
                })
            }
            _ if name == TAG_NAME => {
                Err(de::Error::custom(format_args!(
                    "expected a tag, found {self}"
                )))
            }
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self {
            DataItem::Text(_) => {
                visitor.visit_enum(EnumDeserializer {
                    variant: self,
                    content: None,
                })
            }
            DataItem::Map(map) if map.entries().len() == 1 => {
                let (variant, content) = map
                    .entries()
                    .iter()
                    .next()
                    .expect("map holds exactly one entry");
                visitor.visit_enum(EnumDeserializer {
                    variant,
                    content: Some(content),
                })
            }
            _ => {
                Err(de::Error::custom(
                    "an enum is a text string or a map of one entry",
                ))
            }
        }
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

struct SeqDeserializer<'de> {
    iter: std::slice::Iter<'de, DataItem>,
}

impl<'de> SeqAccess<'de> for SeqDeserializer<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        self.iter
            .next()
            .map(|item| seed.deserialize(item))
            .transpose()
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

struct MapDeserializer<'de> {
    iter: MapIter<'de, DataItem, DataItem>,
    value: Option<&'de DataItem>,
}

impl<'de> MapAccess<'de> for MapDeserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        let Some((key, value)) = self.iter.next() else {
            return Ok(None);
        };
        self.value = Some(value);
        seed.deserialize(key).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        let value = self
            .value
            .take()
            .ok_or_else(|| Error::Message("map value read before its key".into()))?;
        seed.deserialize(value)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.iter.len())
    }
}

/// The number and the content of a tag, as the two elements a [`Tagged`]
/// value reads.
///
/// [`Tagged`]: super::Tagged
struct TagAccess<'de> {
    number: Option<u64>,
    content: Option<&'de DataItem>,
}

impl<'de> SeqAccess<'de> for TagAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Error>
    where
        T: DeserializeSeed<'de>,
    {
        if let Some(number) = self.number.take() {
            seed.deserialize(U64Deserializer::<Error>::new(number))
                .map(Some)
        } else if let Some(content) = self.content.take() {
            seed.deserialize(content).map(Some)
        } else {
            Ok(None)
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(usize::from(self.number.is_some()) + usize::from(self.content.is_some()))
    }
}

/// A tag, `undefined` or a simple value handed to the visitor of a
/// [`DataItem`] as a variant.
struct SpecialAccess<'de> {
    item: &'de DataItem,
}

impl<'de> EnumAccess<'de> for SpecialAccess<'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self), Error>
    where
        V: DeserializeSeed<'de>,
    {
        let index = match self.item {
            DataItem::Tag(_) => TAG_VARIANT,
            DataItem::Undefined => UNDEFINED_VARIANT,
            _ => SIMPLE_VARIANT,
        };
        let variant = seed.deserialize(U32Deserializer::<Error>::new(index))?;
        Ok((variant, self))
    }
}

impl<'de> VariantAccess<'de> for SpecialAccess<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        match self.item {
            DataItem::Undefined => Ok(()),
            _ => Err(de::Error::custom("only undefined is a unit")),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        match self.item {
            DataItem::Tag(_) => seed.deserialize(self.item),
            DataItem::GenericSimple(simple) => {
                seed.deserialize(U8Deserializer::<Error>::new(simple.value()))
            }
            _ => Err(de::Error::custom("undefined has no content")),
        }
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(de::Error::custom("a data item is not a tuple variant"))
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        Err(de::Error::custom("a data item is not a struct variant"))
    }
}

struct EnumDeserializer<'de> {
    variant: &'de DataItem,
    content: Option<&'de DataItem>,
}

impl<'de> EnumAccess<'de> for EnumDeserializer<'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self), Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant = seed.deserialize(self.variant)?;
        Ok((variant, self))
    }
}

impl<'de> VariantAccess<'de> for EnumDeserializer<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        match self.content {
            None => Ok(()),
            Some(_) => Err(de::Error::custom("a unit variant has no content")),
        }
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Error>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(self.content()?)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.content()?.deserialize_seq(visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.content()?.deserialize_map(visitor)
    }
}

impl<'de> EnumDeserializer<'de> {
    fn content(&self) -> Result<&'de DataItem, Error> {
        self.content
            .ok_or_else(|| de::Error::custom("a variant with content is a map of one entry"))
    }
}
