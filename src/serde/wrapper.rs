//! Wrappers for items serde lacks.
//!
//! Each is a struct with a reserved name: this crate keeps the item, other
//! formats see a plain structure.

use std::fmt::{Formatter, Result as FmtResult};
use std::marker::PhantomData;

use ::serde::de::{self, Deserialize, Deserializer, SeqAccess, Unexpected, Visitor};
use ::serde::ser::{Serialize, SerializeTupleStruct as _, Serializer};

use crate::content::{Simple, Tag};
use crate::data_item::DataItem;

/// Struct name for [`Tagged`]
pub(super) const TAG_NAME: &str = "$cbor_next::private::Tag";
/// Struct name for [`Undefined`]
pub(super) const UNDEFINED_NAME: &str = "$cbor_next::private::Undefined";
/// Struct name for [`Simple`]
pub(super) const SIMPLE_NAME: &str = "$cbor_next::private::Simple";
/// Struct name a [`DataItem`] is read as
pub(super) const DATA_ITEM_NAME: &str = "$cbor_next::private::DataItem";

/// A tag around any value; elsewhere a `(number, content)` pair.
///
/// # Example
/// ```rust
/// use cbor_next::DataItem;
/// use cbor_next::serde::{Tagged, from_slice, to_data_item, to_vec};
///
/// let tagged = Tagged {
///     number: 1,
///     content: 1_363_896_240_u64,
/// };
/// assert_eq!(
///     to_data_item(&tagged).unwrap(),
///     DataItem::tag(1, 1_363_896_240_u64)
/// );
/// let bytes = to_vec(&tagged).unwrap();
/// assert_eq!(bytes, [0xc1, 0x1a, 0x51, 0x4b, 0x67, 0xb0]);
/// assert_eq!(from_slice::<Tagged<u64>>(&bytes).unwrap(), tagged);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Tagged<T> {
    /// The tag number
    pub number: u64,
    /// The tagged value
    pub content: T,
}

impl<T> Serialize for Tagged<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut tuple = serializer.serialize_tuple_struct(TAG_NAME, 2)?;
        tuple.serialize_field(&self.number)?;
        tuple.serialize_field(&self.content)?;
        tuple.end()
    }
}

impl<'de, T> Deserialize<'de> for Tagged<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_tuple_struct(TAG_NAME, 2, TaggedVisitor(PhantomData))
    }
}

struct TaggedVisitor<T>(PhantomData<T>);

impl<'de, T> Visitor<'de> for TaggedVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = Tagged<T>;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.write_str("a tag number followed by its content")
    }

    fn visit_seq<A>(self, mut access: A) -> Result<Tagged<T>, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let number = access
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let content = access
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        Ok(Tagged { number, content })
    }
}

/// A tag of a fixed number; reading another number fails. Otherwise like
/// [`Tagged`].
///
/// # Example
/// ```rust
/// use cbor_next::serde::{TaggedWith, from_slice, to_vec};
///
/// let bytes = to_vec(&TaggedWith::<1, _>(10_u8)).unwrap();
/// assert_eq!(bytes, [0xc1, 0x0a]);
/// assert_eq!(from_slice::<TaggedWith<1, u8>>(&bytes).unwrap().0, 10);
/// assert!(from_slice::<TaggedWith<2, u8>>(&bytes).is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TaggedWith<const NUMBER: u64, T>(pub T);

impl<const NUMBER: u64, T> Serialize for TaggedWith<NUMBER, T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Tagged {
            number: NUMBER,
            content: &self.0,
        }
        .serialize(serializer)
    }
}

impl<'de, const NUMBER: u64, T> Deserialize<'de> for TaggedWith<NUMBER, T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let tagged = Tagged::<T>::deserialize(deserializer)?;
        if tagged.number == NUMBER {
            Ok(Self(tagged.content))
        } else {
            Err(de::Error::invalid_value(
                Unexpected::Unsigned(tagged.number),
                &format!("tag number {NUMBER}").as_str(),
            ))
        }
    }
}

/// `undefined`; elsewhere a unit (`null` in JSON).
///
/// # Example
/// ```rust
/// use cbor_next::serde::{Undefined, from_slice, to_vec};
///
/// assert_eq!(to_vec(&Undefined).unwrap(), [0xf7]);
/// assert_eq!(from_slice::<Undefined>(&[0xf7]).unwrap(), Undefined);
/// // null is a different value
/// assert!(from_slice::<Undefined>(&[0xf6]).is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Undefined;

impl Serialize for Undefined {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_unit_struct(UNDEFINED_NAME)
    }
}

impl<'de> Deserialize<'de> for Undefined {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_unit_struct(UNDEFINED_NAME, UndefinedVisitor)
    }
}

struct UndefinedVisitor;

impl Visitor<'_> for UndefinedVisitor {
    type Value = Undefined;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.write_str("undefined")
    }

    fn visit_unit<E>(self) -> Result<Undefined, E> {
        Ok(Undefined)
    }
}

/// A newtype around its number; elsewhere a plain number.
///
/// # Example
/// ```rust
/// use cbor_next::Simple;
/// use cbor_next::serde::{from_slice, to_vec};
///
/// let simple = Simple::try_from(100).unwrap();
/// assert_eq!(to_vec(&simple).unwrap(), [0xf8, 0x64]);
/// assert_eq!(from_slice::<Simple>(&[0xf8, 0x64]).unwrap(), simple);
/// ```
impl Serialize for Simple {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_newtype_struct(SIMPLE_NAME, &self.value())
    }
}

impl<'de> Deserialize<'de> for Simple {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_newtype_struct(SIMPLE_NAME, SimpleVisitor)
    }
}

struct SimpleVisitor;

impl<'de> Visitor<'de> for SimpleVisitor {
    type Value = Simple;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.write_str("a simple value from 0 to 19 or 32 to 255")
    }

    fn visit_u64<E>(self, value: u64) -> Result<Simple, E>
    where
        E: de::Error,
    {
        u8::try_from(value)
            .ok()
            .and_then(|number| Simple::try_from(number).ok())
            .ok_or_else(|| de::Error::invalid_value(Unexpected::Unsigned(value), &self))
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Simple, D::Error>
    where
        D: Deserializer<'de>,
    {
        self.visit_u64(u64::from(u8::deserialize(deserializer)?))
    }
}

/// Serialized as [`Tagged`].
impl Serialize for Tag {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Tagged {
            number: self.number(),
            content: self.content(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let tagged = Tagged::<DataItem>::deserialize(deserializer)?;
        Ok(Self::new(tagged.number, tagged.content))
    }
}
