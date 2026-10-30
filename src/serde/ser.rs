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

/// Number of the tag which holds an unsigned bignum
const UNSIGNED_BIGNUM: u64 = 2;

/// Number of the tag which holds a negative bignum
const NEGATIVE_BIGNUM: u64 = 3;

/// An integer which does not fit a major type 0 or 1 item, as a bignum.
///
/// The magnitude of a negative number is `-1 - value`, as RFC 8949 section
/// 3.4.3 defines tag 3, and leading zero bytes are left out as its preferred
/// serialization asks.
pub(super) fn bignum(negative: bool, magnitude: u128) -> DataItem {
    let bytes = magnitude.to_be_bytes();
    let start = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len());
    let number = if negative {
        NEGATIVE_BIGNUM
    } else {
        UNSIGNED_BIGNUM
    };
    DataItem::tag(number, DataItem::bytes(&bytes[start..]))
}

/// The data item of a 128 bit integer: a major type 0 or 1 item when it fits
/// one, a bignum otherwise.
pub(super) fn integer_item(value: i128) -> DataItem {
    if let Ok(unsigned) = u64::try_from(value) {
        DataItem::Unsigned(unsigned)
    } else if let Ok(negative) = u64::try_from(!value) {
        DataItem::Signed(negative)
    } else if value < 0 {
        // `!value` is `-1 - value`, which is not negative
        bignum(
            true,
            u128::try_from(!value).expect("`!value` is not negative"),
        )
    } else {
        bignum(false, u128::try_from(value).expect("value is not negative"))
    }
}

fn single_entry(name: &str, content: DataItem) -> DataItem {
    DataItem::map([(DataItem::from(name), content)])
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

    fn serialize_bool(self, value: bool) -> Result<DataItem, Error> {
        Ok(DataItem::from(value))
    }

    fn serialize_i8(self, value: i8) -> Result<DataItem, Error> {
        Ok(DataItem::from(value))
    }

    fn serialize_i16(self, value: i16) -> Result<DataItem, Error> {
        Ok(DataItem::from(value))
    }

    fn serialize_i32(self, value: i32) -> Result<DataItem, Error> {
        Ok(DataItem::from(value))
    }

    fn serialize_i64(self, value: i64) -> Result<DataItem, Error> {
        Ok(DataItem::from(value))
    }

    fn serialize_i128(self, value: i128) -> Result<DataItem, Error> {
        Ok(integer_item(value))
    }

    fn serialize_u8(self, value: u8) -> Result<DataItem, Error> {
        Ok(DataItem::from(value))
    }

    fn serialize_u16(self, value: u16) -> Result<DataItem, Error> {
        Ok(DataItem::from(value))
    }

    fn serialize_u32(self, value: u32) -> Result<DataItem, Error> {
        Ok(DataItem::from(value))
    }

    fn serialize_u64(self, value: u64) -> Result<DataItem, Error> {
        Ok(DataItem::from(value))
    }

    fn serialize_u128(self, value: u128) -> Result<DataItem, Error> {
        Ok(u64::try_from(value).map_or_else(|_| bignum(false, value), DataItem::Unsigned))
    }

    fn serialize_f32(self, value: f32) -> Result<DataItem, Error> {
        Ok(DataItem::Floating(f32_to_f64(value)))
    }

    fn serialize_f64(self, value: f64) -> Result<DataItem, Error> {
        Ok(DataItem::Floating(value))
    }

    fn serialize_char(self, value: char) -> Result<DataItem, Error> {
        Ok(DataItem::from(value.to_string()))
    }

    fn serialize_str(self, value: &str) -> Result<DataItem, Error> {
        Ok(DataItem::from(value))
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

impl SerializeSeq for SeqSerializer {
    type Error = Error;
    type Ok = DataItem;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        self.push(value)
    }

    fn end(self) -> Result<DataItem, Error> {
        self.finish()
    }
}

impl SerializeTuple for SeqSerializer {
    type Error = Error;
    type Ok = DataItem;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        self.push(value)
    }

    fn end(self) -> Result<DataItem, Error> {
        self.finish()
    }
}

impl SerializeTupleStruct for SeqSerializer {
    type Error = Error;
    type Ok = DataItem;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        self.push(value)
    }

    fn end(self) -> Result<DataItem, Error> {
        self.finish()
    }
}

impl SerializeTupleVariant for SeqSerializer {
    type Error = Error;
    type Ok = DataItem;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        self.push(value)
    }

    fn end(self) -> Result<DataItem, Error> {
        self.finish()
    }
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

    fn insert_field<T>(&mut self, name: &'static str, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        self.insert(DataItem::from(name), value.serialize(ItemSerializer)?)
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

impl SerializeStruct for MapSerializer {
    type Error = Error;
    type Ok = DataItem;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        self.insert_field(key, value)
    }

    fn end(self) -> Result<DataItem, Error> {
        Ok(self.finish())
    }
}

impl SerializeStructVariant for MapSerializer {
    type Error = Error;
    type Ok = DataItem;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Error>
    where
        T: Serialize + ?Sized,
    {
        self.insert_field(key, value)
    }

    fn end(self) -> Result<DataItem, Error> {
        Ok(self.finish())
    }
}
