use std::fmt::Formatter;

use ::serde::de::{
    self, Deserialize, Deserializer, EnumAccess, MapAccess, SeqAccess, VariantAccess as _, Visitor,
};
use ::serde::ser::{Serialize, Serializer};
use indexmap::IndexMap;
use indexmap::map::Entry;

use super::de::Special;
use super::tags::{ByteBuf, Bytes};
use super::wrapper::{
    BYTE_CHUNKS_NAME, DATA_ITEM_NAME, INDEFINITE_NAME, TEXT_CHUNKS_NAME, Tagged, Undefined,
};
use crate::content::{Array, ByteString, Map, Simple, TextString};
use crate::data_item::DataItem;
use crate::data_item::convert::f32_to_f64;

/// Writes the value held; tags, `undefined` and simple values go as
/// [`Tagged`], [`Undefined`] and [`Simple`] do. This crate keeps the length
/// form and chunks; elsewhere chunks are a list of strings.
impl Serialize for DataItem {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Unsigned(number) => serializer.serialize_u64(*number),
            Self::Signed(number) => {
                let value = -(i128::from(*number) + 1);
                if let Ok(small) = i64::try_from(value) {
                    serializer.serialize_i64(small)
                } else {
                    serializer.serialize_i128(value)
                }
            }
            // the length form and chunks go under names this crate knows
            Self::Byte(bytes) if bytes.is_indefinite() => {
                serializer.serialize_newtype_struct(BYTE_CHUNKS_NAME, &ByteChunks(bytes.chunks()))
            }
            Self::Text(text) if text.is_indefinite() => {
                serializer.serialize_newtype_struct(TEXT_CHUNKS_NAME, text.chunks())
            }
            Self::Array(array) if array.is_indefinite() => {
                serializer.serialize_newtype_struct(INDEFINITE_NAME, array.items())
            }
            Self::Map(map) if map.is_indefinite() => {
                serializer.serialize_newtype_struct(INDEFINITE_NAME, &Entries(map))
            }
            Self::Byte(bytes) => serializer.serialize_bytes(&bytes.to_bytes()),
            Self::Text(text) => serializer.serialize_str(&text.to_text()),
            Self::Array(array) => array.items().serialize(serializer),
            Self::Map(map) => Entries(map).serialize(serializer),
            Self::Tag(tag) => tag.serialize(serializer),
            Self::Boolean(value) => serializer.serialize_bool(*value),
            Self::Null => serializer.serialize_unit(),
            Self::Undefined => Undefined.serialize(serializer),
            Self::Floating(number) => serializer.serialize_f64(*number),
            Self::GenericSimple(simple) => simple.serialize(serializer),
        }
    }
}

/// A map's entries, written as a map.
struct Entries<'a>(&'a Map);

impl Serialize for Entries<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_map(self.0.entries())
    }
}

/// Byte string chunks, written as a list of byte strings.
struct ByteChunks<'a>(&'a [Vec<u8>]);

impl Serialize for ByteChunks<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.0.iter().map(|chunk| Bytes(chunk)))
    }
}

/// A map's entries, read as pairs.
struct Pairs(Vec<(DataItem, DataItem)>);

impl<'de> Deserialize<'de> for Pairs {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct PairsVisitor;

        impl<'de> Visitor<'de> for PairsVisitor {
            type Value = Pairs;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a map")
            }

            fn visit_map<A>(self, mut access: A) -> Result<Pairs, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut pairs = Vec::with_capacity(access.size_hint().unwrap_or(0).min(1024));
                while let Some(pair) = access.next_entry()? {
                    pairs.push(pair);
                }
                Ok(Pairs(pairs))
            }
        }

        deserializer.deserialize_map(PairsVisitor)
    }
}

/// Reads from any value; from this crate it round trips exactly.
impl<'de> Deserialize<'de> for DataItem {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // our deserializer passes items serde lacks as a variant; others see
        // a plain newtype
        deserializer.deserialize_newtype_struct(DATA_ITEM_NAME, ItemVisitor)
    }
}

struct ItemVisitor;

impl<'de> Visitor<'de> for ItemVisitor {
    type Value = DataItem;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a value which CBOR can represent")
    }

    fn visit_bool<E>(self, value: bool) -> Result<DataItem, E> {
        Ok(DataItem::from(value))
    }

    fn visit_i64<E>(self, value: i64) -> Result<DataItem, E> {
        Ok(DataItem::from(value))
    }

    fn visit_i128<E>(self, value: i128) -> Result<DataItem, E> {
        Ok(DataItem::from(value))
    }

    fn visit_u64<E>(self, value: u64) -> Result<DataItem, E> {
        Ok(DataItem::from(value))
    }

    fn visit_u128<E>(self, value: u128) -> Result<DataItem, E> {
        Ok(DataItem::from(value))
    }

    fn visit_f32<E>(self, value: f32) -> Result<DataItem, E> {
        // the default widening may quiet a signaling NaN
        Ok(DataItem::Floating(f32_to_f64(value)))
    }

    fn visit_f64<E>(self, value: f64) -> Result<DataItem, E> {
        Ok(DataItem::Floating(value))
    }

    fn visit_str<E>(self, value: &str) -> Result<DataItem, E> {
        Ok(DataItem::from(value))
    }

    fn visit_string<E>(self, value: String) -> Result<DataItem, E> {
        Ok(DataItem::from(value))
    }

    fn visit_bytes<E>(self, value: &[u8]) -> Result<DataItem, E> {
        Ok(DataItem::bytes(value))
    }

    fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<DataItem, E> {
        Ok(DataItem::bytes(value))
    }

    fn visit_none<E>(self) -> Result<DataItem, E> {
        Ok(DataItem::Null)
    }

    fn visit_unit<E>(self) -> Result<DataItem, E> {
        Ok(DataItem::Null)
    }

    fn visit_some<D>(self, deserializer: D) -> Result<DataItem, D::Error>
    where
        D: Deserializer<'de>,
    {
        DataItem::deserialize(deserializer)
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<DataItem, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ItemVisitor)
    }

    fn visit_seq<A>(self, mut access: A) -> Result<DataItem, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut items: Vec<DataItem> =
            Vec::with_capacity(access.size_hint().unwrap_or(0).min(1024));
        while let Some(item) = access.next_element()? {
            items.push(item);
        }
        Ok(DataItem::Array(Array::from_items(items)))
    }

    fn visit_map<A>(self, mut access: A) -> Result<DataItem, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut entries = IndexMap::new();
        while let Some((key, value)) = access.next_entry::<DataItem, DataItem>()? {
            match entries.entry(key) {
                Entry::Occupied(entry) => {
                    return Err(de::Error::custom(format_args!(
                        "map key {} used more than once",
                        entry.key()
                    )));
                }
                Entry::Vacant(entry) => {
                    entry.insert(value);
                }
            }
        }
        Ok(DataItem::Map(Map::from(entries)))
    }

    /// An item serde lacks, passed by our deserializer.
    fn visit_enum<A>(self, access: A) -> Result<DataItem, A::Error>
    where
        A: EnumAccess<'de>,
    {
        let (index, variant) = access.variant::<u32>()?;
        let special = Special::from_index(index)
            .ok_or_else(|| de::Error::custom("a data item is not an enum"))?;
        match special {
            Special::Tag => {
                let tagged = variant.newtype_variant::<Tagged<DataItem>>()?;
                Ok(DataItem::tag(tagged.number, tagged.content))
            }
            Special::Undefined => {
                variant.unit_variant()?;
                Ok(DataItem::Undefined)
            }
            Special::Simple => {
                let simple = variant.newtype_variant::<u8>()?;
                Simple::try_from(simple)
                    .map(DataItem::GenericSimple)
                    .map_err(de::Error::custom)
            }
            Special::IndefiniteArray => {
                let items = variant.newtype_variant::<Vec<DataItem>>()?;
                Ok(DataItem::Array(
                    Array::from_items(items).with_indefinite(true),
                ))
            }
            Special::IndefiniteMap => {
                let Pairs(pairs) = variant.newtype_variant()?;
                Ok(DataItem::Map(
                    Map::from_entries(pairs).with_indefinite(true),
                ))
            }
            Special::ByteChunks => {
                let chunks = variant.newtype_variant::<Vec<ByteBuf>>()?;
                Ok(DataItem::Byte(ByteString::from_chunks(
                    chunks.into_iter().map(|chunk| chunk.0),
                )))
            }
            Special::TextChunks => {
                let chunks = variant.newtype_variant::<Vec<String>>()?;
                Ok(DataItem::Text(TextString::from_chunks(chunks)))
            }
        }
    }
}
