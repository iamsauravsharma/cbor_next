//! Serde types for tags registered by RFC 8949 and RFC 8742.
//!
//! Each is a [`TaggedWith`] of its number, so reading checks the number.
//! [`Embedded`] and [`EmbeddedSequence`] also encode and decode their bytes.
//!
//! Bignums need no type (`i128`/`u128`). For tags 4 and 5 use
//! `TaggedWith<4, (i64, i128)>`; [`Tagged`](super::Tagged) keeps any tag.

use std::fmt::{Formatter, Result as FmtResult};

use ::serde::de::{self, Deserialize, DeserializeOwned, Deserializer, SeqAccess, Visitor};
use ::serde::ser::{Error as _, Serialize, Serializer};

use super::de::from_data_item;
use super::ser::to_data_item;
use super::wrapper::TaggedWith;
use crate::data_item::DataItem;
use crate::decode::Decoder;
use crate::encode::Encoder;
use crate::tags;

/// Define a newtype over a [`TaggedWith`] of a registered number.
macro_rules! registered_tag {
    ($(#[$meta:meta])* $name:ident<$content:ident> = $number:expr) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
        pub struct $name<$content>(pub $content);

        impl<$content> Serialize for $name<$content>
        where
            $content: Serialize,
        {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                TaggedWith::<{ $number }, &$content>(&self.0).serialize(serializer)
            }
        }

        impl<'de, $content> Deserialize<'de> for $name<$content>
        where
            $content: Deserialize<'de>,
        {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                TaggedWith::<{ $number }, $content>::deserialize(deserializer)
                    .map(|tagged| Self(tagged.0))
            }
        }
    };
    ($(#[$meta:meta])* $name:ident($content:ty) = $number:expr) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
        pub struct $name(pub $content);

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                TaggedWith::<{ $number }, &$content>(&self.0).serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                TaggedWith::<{ $number }, $content>::deserialize(deserializer)
                    .map(|tagged| Self(tagged.0))
            }
        }
    };
}

registered_tag! {
    /// Tag 0, an RFC 3339 date and time text.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::serde::tags::DateTimeString;
    /// use cbor_next::serde::to_vec;
    ///
    /// let time = DateTimeString("2013-03-21T20:04:00Z".into());
    /// assert_eq!(to_vec(&time).unwrap()[..2], [0xc0, 0x74]);
    /// ```
    DateTimeString(String) = tags::DATE_TIME_STRING
}

registered_tag! {
    /// Tag 1, seconds since the epoch: `EpochTime<i64>` or `EpochTime<f64>`.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::serde::tags::EpochTime;
    /// use cbor_next::serde::{from_slice, to_vec};
    ///
    /// let bytes = to_vec(&EpochTime(1_363_896_240_i64)).unwrap();
    /// assert_eq!(bytes, [0xc1, 0x1a, 0x51, 0x4b, 0x67, 0xb0]);
    /// // an integer is read as a float as well
    /// assert_eq!(from_slice::<EpochTime<f64>>(&bytes).unwrap().0, 1_363_896_240.0);
    /// ```
    EpochTime<T> = tags::EPOCH_TIME
}

registered_tag! {
    /// Tag 32, URI text.
    Uri(String) = tags::URI
}

registered_tag! {
    /// Tag 33, base64url text.
    Base64Url(String) = tags::BASE64URL
}

registered_tag! {
    /// Tag 34, base64 text.
    Base64(String) = tags::BASE64
}

registered_tag! {
    /// Tag 36, MIME message text.
    MimeMessage(String) = tags::MIME_MESSAGE
}

registered_tag! {
    /// Tag 55799, self-described `CBOR` (leading bytes `d9 d9 f7`).
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::serde::tags::SelfDescribed;
    /// use cbor_next::serde::{from_slice, to_vec};
    ///
    /// let bytes = to_vec(&SelfDescribed(1_u8)).unwrap();
    /// assert_eq!(bytes, [0xd9, 0xd9, 0xf7, 0x01]);
    /// assert_eq!(from_slice::<SelfDescribed<u8>>(&bytes).unwrap().0, 1);
    /// ```
    SelfDescribed<T> = tags::SELF_DESCRIBED
}

/// Tag 24, a value encoded as `CBOR` inside a byte string.
///
/// Read with the default [`Decoder`]; malformed bytes are rejected.
///
/// # Example
/// ```rust
/// use cbor_next::serde::tags::Embedded;
/// use cbor_next::serde::{from_slice, to_vec};
///
/// let bytes = to_vec(&Embedded((1_u8, "a"))).unwrap();
/// // tag 24 around the three bytes of [1, "a"]
/// assert_eq!(bytes, [0xd8, 0x18, 0x44, 0x82, 0x01, 0x61, 0x61]);
/// assert_eq!(
///     from_slice::<Embedded<(u8, String)>>(&bytes).unwrap().0,
///     (1, "a".into())
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Embedded<T>(pub T);

impl<T> Serialize for Embedded<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let bytes = to_data_item(&self.0).map_err(S::Error::custom)?.encode();
        TaggedWith::<{ tags::EMBEDDED_ITEM }, _>(Bytes(&bytes)).serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for Embedded<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = TaggedWith::<{ tags::EMBEDDED_ITEM }, ByteBuf>::deserialize(deserializer)?;
        let item = DataItem::decode(&bytes.0.0).map_err(de::Error::custom)?;
        from_data_item(&item).map(Self).map_err(de::Error::custom)
    }
}

/// Tag 63, a sequence (RFC 8742) encoded inside a byte string, like
/// [`Embedded`].
///
/// # Example
/// ```rust
/// use cbor_next::serde::tags::EmbeddedSequence;
/// use cbor_next::serde::{from_slice, to_vec};
///
/// let bytes = to_vec(&EmbeddedSequence(vec![1_u8, 2])).unwrap();
/// assert_eq!(bytes, [0xd8, 0x3f, 0x42, 0x01, 0x02]);
/// assert_eq!(
///     from_slice::<EmbeddedSequence<u8>>(&bytes).unwrap().0,
///     [1, 2]
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct EmbeddedSequence<T>(pub Vec<T>);

impl<T> Serialize for EmbeddedSequence<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let encoder = Encoder::new();
        let mut bytes = vec![];
        for value in &self.0 {
            let item = to_data_item(value).map_err(S::Error::custom)?;
            encoder.encode_into(&item, &mut bytes);
        }
        TaggedWith::<{ tags::EMBEDDED_SEQUENCE }, _>(Bytes(&bytes)).serialize(serializer)
    }
}

impl<'de, T> Deserialize<'de> for EmbeddedSequence<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes = TaggedWith::<{ tags::EMBEDDED_SEQUENCE }, ByteBuf>::deserialize(deserializer)?;
        let mut values = vec![];
        for item in Decoder::new().sequence(&bytes.0.0) {
            let item = item.map_err(de::Error::custom)?;
            values.push(from_data_item(&item).map_err(de::Error::custom)?);
        }
        Ok(Self(values))
    }
}

/// Bytes serialized as a byte string, not a sequence of numbers.
struct Bytes<'a>(&'a [u8]);

impl Serialize for Bytes<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(self.0)
    }
}

/// Bytes read from a byte string or a sequence of numbers.
struct ByteBuf(Vec<u8>);

impl<'de> Deserialize<'de> for ByteBuf {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_byte_buf(ByteBufVisitor)
    }
}

struct ByteBufVisitor;

impl<'de> Visitor<'de> for ByteBufVisitor {
    type Value = ByteBuf;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.write_str("a byte string")
    }

    fn visit_bytes<E>(self, value: &[u8]) -> Result<ByteBuf, E> {
        Ok(ByteBuf(value.to_vec()))
    }

    fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<ByteBuf, E> {
        Ok(ByteBuf(value))
    }

    fn visit_seq<A>(self, mut access: A) -> Result<ByteBuf, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut bytes = Vec::with_capacity(access.size_hint().unwrap_or(0).min(1024));
        while let Some(byte) = access.next_element()? {
            bytes.push(byte);
        }
        Ok(ByteBuf(bytes))
    }
}
