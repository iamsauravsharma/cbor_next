use ::serde::ser::{
    Serialize, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant, SerializeTuple,
    SerializeTupleStruct, SerializeTupleVariant, Serializer,
};
use indexmap::IndexMap;
use indexmap::map::Entry;

use super::error::{Error, unsupported};
use super::wrapper::{SIMPLE_NAME, TAG_NAME, UNDEFINED_NAME};
use crate::content::{Array, Map, Simple};
use crate::data_item::DataItem;
use crate::data_item::convert::f32_to_f64;

/// Turn a value into a data item.
///
/// # Errors
/// If the value holds data which `CBOR` cannot represent
///
/// # Example
/// ```rust
/// use cbor_next::DataItem;
/// use cbor_next::serde::to_data_item;
///
/// assert_eq!(
///     to_data_item(&(1, "a", true)).unwrap(),
///     DataItem::array([DataItem::from(1), DataItem::from("a"), DataItem::from(true)])
/// );
/// ```
pub fn to_data_item<T>(value: &T) -> Result<DataItem, Error>
where
    T: Serialize + ?Sized,
{
    value.serialize(ItemSerializer)
}

/// Encode a value as `CBOR` bytes.
///
/// # Errors
/// Under the same conditions as [`to_data_item`]
///
/// # Example
/// ```rust
/// use cbor_next::serde::to_vec;
///
/// assert_eq!(to_vec(&[1, 2]).unwrap(), [0x82, 0x01, 0x02]);
/// ```
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, Error>
where
    T: Serialize + ?Sized,
{
    Ok(to_data_item(value)?.encode())
}

fn single_entry(name: &str, content: DataItem) -> DataItem {
    DataItem::map([(DataItem::from(name), content)])
}

/// Define the methods which turn a value into a data item with `From`.
macro_rules! serialize_with_from {
    ($($method:ident($type:ty)),* $(,)?) => {
        $(
            fn $method(self, value: $type) -> Result<DataItem, Error> {
                Ok(DataItem::from(value))
            }
        )*
    };
}

struct ItemSerializer;

impl Serializer for ItemSerializer {
    type Error = Error;
    type Ok = DataItem;
    type SerializeMap = MapSerializer;
    type SerializeSeq = SeqSerializer;
    type SerializeStruct = MapSerializer;
    type SerializeStructVariant = MapSerializer;
    type SerializeTuple = SeqSerializer;
    type SerializeTupleStruct = SeqSerializer;
    type SerializeTupleVariant = SeqSerializer;

    serialize_with_from! {
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_i128(i128),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_u128(u128),
        serialize_f64(f64),
        serialize_str(&str),
    }

    fn serialize_f32(self, value: f32) -> Result<DataItem, Error> {
        Ok(DataItem::Floating(f32_to_f64(value)))
    }

    fn serialize_char(self, value: char) -> Result<DataItem, Error> {
        Ok(DataItem::from(value.to_string()))
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<DataItem, Error> {
        Ok(DataItem::bytes(value))
    }

    fn serialize_none(self) -> Result<DataItem, Error> {
        Ok(DataItem::Null)
    }

    fn serialize_some<T>(self, value: &T) -> Result<DataItem, Error>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<DataItem, Error> {
        Ok(DataItem::Null)
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<DataItem, Error> {
        if name == UNDEFINED_NAME {
            Ok(DataItem::Undefined)
        } else {
            Ok(DataItem::Null)
        }
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<DataItem, Error> {
        Ok(DataItem::from(variant))
    }

    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<DataItem, Error>
    where
        T: Serialize + ?Sized,
    {
        let item = value.serialize(self)?;
        if name != SIMPLE_NAME {
            return Ok(item);
        }
        match item {
            DataItem::Unsigned(number) => {
                u8::try_from(number)
                    .ok()
                    .and_then(|number| Simple::try_from(number).ok())
                    .map(DataItem::GenericSimple)
                    .ok_or_else(|| {
                        Error::Message(format!(
                            "simple value {number} is not in the range 0 to 19 or 32 to 255"
                        ))
                    })
            }
            other => {
                Err(Error::Message(format!(
                    "a simple value is a number, not {other}"
                )))
            }
        }
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<DataItem, Error>
    where
        T: Serialize + ?Sized,
    {
        Ok(single_entry(variant, value.serialize(self)?))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<SeqSerializer, Error> {
        Ok(SeqSerializer::new(SeqKind::Array, len))
    }

    fn serialize_tuple(self, len: usize) -> Result<SeqSerializer, Error> {
        Ok(SeqSerializer::new(SeqKind::Array, Some(len)))
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<SeqSerializer, Error> {
        let kind = if name == TAG_NAME {
            SeqKind::Tag
        } else {
            SeqKind::Array
        };
        Ok(SeqSerializer::new(kind, Some(len)))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<SeqSerializer, Error> {
        Ok(SeqSerializer::new(SeqKind::Variant(variant), Some(len)))
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<MapSerializer, Error> {
        Ok(MapSerializer::new(None))
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<MapSerializer, Error> {
        Ok(MapSerializer::new(None))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<MapSerializer, Error> {
        Ok(MapSerializer::new(Some(variant)))
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

/// What the elements of a sequence are turned into once it ends.
enum SeqKind {
    /// An array
    Array,
    /// An array under the name of an enum variant
    Variant(&'static str),
    /// A tag, from the number and the content a [`Tagged`] value writes
    ///
    /// [`Tagged`]: super::Tagged
    Tag,
}

struct SeqSerializer {
    kind: SeqKind,
    items: Vec<DataItem>,
}

impl SeqSerializer {
    fn new(kind: SeqKind, len: Option<usize>) -> Self {
        Self {
            kind,
            items: Vec::with_capacity(len.unwrap_or(0).min(1024)),
        }
    }

    fn push<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        self.items.push(value.serialize(ItemSerializer)?);
        Ok(())
    }

    fn finish(self) -> Result<DataItem, Error> {
        match self.kind {
            SeqKind::Array => Ok(DataItem::Array(Array::from_items(self.items))),
            SeqKind::Variant(variant) => {
                Ok(single_entry(
                    variant,
                    DataItem::Array(Array::from_items(self.items)),
                ))
            }
            SeqKind::Tag => {
                let mut items = self.items.into_iter();
                match (items.next(), items.next(), items.next()) {
                    (Some(DataItem::Unsigned(number)), Some(content), None) => {
                        Ok(DataItem::tag(number, content))
                    }
                    _ => {
                        Err(Error::Message(
                            "a tag is its number followed by its content".into(),
                        ))
                    }
                }
            }
        }
    }
}

/// Implement the traits whose elements all land in a [`SeqSerializer`].
macro_rules! impl_serialize_seq {
    ($($trait:ident::$method:ident),* $(,)?) => {
        $(
            impl $trait for SeqSerializer {
                type Error = Error;
                type Ok = DataItem;

                fn $method<T>(&mut self, value: &T) -> Result<(), Error>
                where
                    T: Serialize + ?Sized,
                {
                    self.push(value)
                }

                fn end(self) -> Result<DataItem, Error> {
                    self.finish()
                }
            }
        )*
    };
}

impl_serialize_seq! {
    SerializeSeq::serialize_element,
    SerializeTuple::serialize_element,
    SerializeTupleStruct::serialize_field,
    SerializeTupleVariant::serialize_field,
}

struct MapSerializer {
    variant: Option<&'static str>,
    entries: IndexMap<DataItem, DataItem>,
    key: Option<DataItem>,
}

impl MapSerializer {
    fn new(variant: Option<&'static str>) -> Self {
        Self {
            variant,
            entries: IndexMap::new(),
            key: None,
        }
    }

    fn insert(&mut self, key: DataItem, value: DataItem) -> Result<(), Error> {
        match self.entries.entry(key) {
            Entry::Occupied(entry) => {
                Err(unsupported(format_args!(
                    "map key {} used more than once",
                    entry.key()
                )))
            }
            Entry::Vacant(entry) => {
                entry.insert(value);
                Ok(())
            }
        }
    }

    fn finish(self) -> DataItem {
        let map = DataItem::Map(Map::from(self.entries));
        match self.variant {
            Some(variant) => single_entry(variant, map),
            None => map,
        }
    }
}

impl SerializeMap for MapSerializer {
    type Error = Error;
    type Ok = DataItem;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        self.key = Some(key.serialize(ItemSerializer)?);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        let key = self
            .key
            .take()
            .ok_or_else(|| Error::Message("map value written before its key".into()))?;
        self.insert(key, value.serialize(ItemSerializer)?)
    }

    fn end(self) -> Result<DataItem, Error> {
        Ok(self.finish())
    }
}

/// Implement the traits whose fields all land in a [`MapSerializer`] under
/// their name.
macro_rules! impl_serialize_struct {
    ($($trait:ident),* $(,)?) => {
        $(
            impl $trait for MapSerializer {
                type Error = Error;
                type Ok = DataItem;

                fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
                where
                    T: Serialize + ?Sized,
                {
                    self.insert(DataItem::from(key), value.serialize(ItemSerializer)?)
                }

                fn end(self) -> Result<DataItem, Error> {
                    Ok(self.finish())
                }
            }
        )*
    };
}

impl_serialize_struct!(SerializeStruct, SerializeStructVariant);
