use crate::error::Error;
use crate::{ArrayContent, DataItem, MapContent};

/// Serialize the given data structure as a bytes of CBOR
///
/// # Errors
/// When provided data structure cannot be serialized
pub fn to_bytes<T>(value: &T) -> Result<Vec<u8>, Error>
where
    T: serde::Serialize,
{
    let data_item = value.serialize(CborSerializer)?;
    Ok(data_item.encode())
}

/// Serializer for CBOR
pub struct CborSerializer;

impl serde::ser::Serializer for CborSerializer {
    type Error = Error;
    type Ok = DataItem;
    type SerializeMap = MapSerializer;
    type SerializeSeq = ArraySerializer;
    type SerializeStruct = MapSerializer;
    type SerializeStructVariant = StructVariantSerializer;
    type SerializeTuple = ArraySerializer;
    type SerializeTupleStruct = ArraySerializer;
    type SerializeTupleVariant = TupleVariantSerializer;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        Ok(Self::Ok::from(v))
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        self.serialize_i64(i64::from(v))
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        Ok(Self::Ok::from(v))
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        self.serialize_u64(u64::from(v))
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        Ok(Self::Ok::from(v))
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
        self.serialize_f64(f64::from(v))
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(Self::Ok::from(v))
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(&v.to_string())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        Ok(Self::Ok::from(v))
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        Ok(Self::Ok::from(v))
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(Self::Ok::Null)
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        self.serialize_unit()
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        let mut map_content = MapContent::default();
        let val = value.serialize(self)?;
        map_content.insert_content(variant, val);
        Ok(Self::Ok::Map(map_content))
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let mut array_content = ArrayContent::default();
        if len.is_none() {
            array_content.set_indefinite(true);
        }
        Ok(Self::SerializeSeq {
            content: array_content,
        })
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        self.serialize_seq(Some(len))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Ok(Self::SerializeTupleVariant {
            variant: variant.to_string(),
            items: vec![],
        })
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let mut map_content = MapContent::default();
        if len.is_none() {
            map_content.set_indefinite(true);
        }
        Ok(Self::SerializeMap {
            content: map_content,
            current_key: None,
        })
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.serialize_map(Some(len))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Ok(Self::SerializeStructVariant {
            variant: variant.to_string(),
            content: MapContent::default(),
        })
    }
}

/// Serialize array
pub struct ArraySerializer {
    content: ArrayContent,
}

impl ArraySerializer {
    /// push a content serialize to a serializer
    fn push_content<T>(&mut self, value: &T) -> Result<(), Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.content.push_content(value.serialize(CborSerializer)?);
        Ok(())
    }

    /// return as data item
    fn data_item(self) -> DataItem {
        DataItem::Array(self.content)
    }
}

impl serde::ser::SerializeSeq for ArraySerializer {
    type Error = Error;
    type Ok = DataItem;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.push_content(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.data_item())
    }
}

impl serde::ser::SerializeTuple for ArraySerializer {
    type Error = Error;
    type Ok = DataItem;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.push_content(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.data_item())
    }
}

impl serde::ser::SerializeTupleStruct for ArraySerializer {
    type Error = Error;
    type Ok = DataItem;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.push_content(value)
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(self.data_item())
    }
}

/// Serialize tuple variant
pub struct TupleVariantSerializer {
    variant: String,
    items: Vec<DataItem>,
}

impl serde::ser::SerializeTupleVariant for TupleVariantSerializer {
    type Error = Error;
    type Ok = DataItem;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.items.push(value.serialize(CborSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut map_content = MapContent::default();
        map_content.insert_content(self.variant, self.items);
        Ok(Self::Ok::Map(map_content))
    }
}

/// Serialize map
pub struct MapSerializer {
    content: MapContent,
    current_key: Option<DataItem>,
}

impl MapSerializer {
    /// return as data item
    fn data_item(self) -> Result<DataItem, Error> {
        if self.current_key.is_some() {
            return Err(Error::SerdeMessage(
                "incomplete map key not assigned to value".to_string(),
            ));
        }
        Ok(DataItem::Map(self.content))
    }
}

impl serde::ser::SerializeMap for MapSerializer {
    type Error = Error;
    type Ok = DataItem;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        if self.current_key.is_some() {
            return Err(Error::SerdeMessage(
                "serialize_key called multiple times before setting value".to_string(),
            ));
        }
        self.current_key = Some(key.serialize(CborSerializer)?);
        Ok(())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        let Some(key) = &self.current_key else {
            return Err(Error::SerdeMessage(
                "serialize_value called before setting key".to_string(),
            ));
        };
        self.content
            .insert_content(key, value.serialize(CborSerializer)?);
        self.current_key = None;
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.data_item()
    }
}

impl serde::ser::SerializeStruct for MapSerializer {
    type Error = Error;
    type Ok = DataItem;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.content
            .insert_content(key, value.serialize(CborSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.data_item()
    }
}

/// serialize struct variant
pub struct StructVariantSerializer {
    variant: String,
    content: MapContent,
}

impl serde::ser::SerializeStructVariant for StructVariantSerializer {
    type Error = Error;
    type Ok = DataItem;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + serde::Serialize,
    {
        self.content
            .insert_content(key, value.serialize(CborSerializer)?);
        Ok(())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        let mut map_content = MapContent::default();
        map_content.insert_content(self.variant, Self::Ok::Map(self.content));
        Ok(Self::Ok::Map(map_content))
    }
}
