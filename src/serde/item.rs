use std::fmt::Formatter;

use ::serde::de::{
    self, Deserialize, Deserializer, EnumAccess, MapAccess, SeqAccess, VariantAccess as _, Visitor,
};
use ::serde::ser::{Serialize, SerializeMap as _, SerializeSeq as _, Serializer};
use indexmap::IndexMap;
use indexmap::map::Entry;

use super::de::{SIMPLE_VARIANT, TAG_VARIANT, UNDEFINED_VARIANT};
use super::wrapper::{DATA_ITEM_NAME, Tagged, Undefined};
use crate::content::{Array, Map, Simple};
use crate::data_item::DataItem;
use crate::data_item::convert::f32_to_f64;

/// Writes the value held; tags, `undefined` and simple values go as
/// [`Tagged`], [`Undefined`] and [`Simple`] do.
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
            Self::Byte(bytes) => serializer.serialize_bytes(&bytes.to_bytes()),
            Self::Text(text) => serializer.serialize_str(&text.to_text()),
            Self::Array(array) => {
                let mut sequence = serializer.serialize_seq(Some(array.items().len()))?;
                for item in array.items() {
                    sequence.serialize_element(item)?;
                }
                sequence.end()
            }
            Self::Map(map) => {
                let mut entries = serializer.serialize_map(Some(map.entries().len()))?;
                for (key, value) in map.entries() {
                    entries.serialize_entry(key, value)?;
                }
                entries.end()
            }
            Self::Tag(tag) => tag.serialize(serializer),
            Self::Boolean(value) => serializer.serialize_bool(*value),
            Self::Null => serializer.serialize_unit(),
            Self::Undefined => Undefined.serialize(serializer),
            Self::Floating(number) => serializer.serialize_f64(*number),
            Self::GenericSimple(simple) => simple.serialize(serializer),
        }
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
        match index {
            TAG_VARIANT => {
                let tagged = variant.newtype_variant::<Tagged<DataItem>>()?;
                Ok(DataItem::tag(tagged.number, tagged.content))
            }
            UNDEFINED_VARIANT => {
                variant.unit_variant()?;
                Ok(DataItem::Undefined)
            }
            SIMPLE_VARIANT => {
                let simple = variant.newtype_variant::<u8>()?;
                Simple::try_from(simple)
                    .map(DataItem::GenericSimple)
                    .map_err(de::Error::custom)
            }
            _ => Err(de::Error::custom("a data item is not an enum")),
        }
    }
}
