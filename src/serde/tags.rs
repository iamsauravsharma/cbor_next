//! Serde types for tags registered by RFC 8949 and RFC 8742.
//!
//! Each is a [`TaggedWith`] of its number, so reading checks the number.
//! [`Embedded`] and [`EmbeddedSequence`] also encode and decode their bytes.
//!
//! Bignums need no type (`i128`/`u128`). For tags 4 and 5 use
//! `TaggedWith<4, (i64, i128)>`; [`Tagged`] keeps any tag.
//!
//! The RFC 8943 and 9164 tags have types too: [`DateString`],
//! [`EpochDays`], [`IpAddress`], [`IpPrefix`] and [`IpInterface`].

use std::fmt::{Formatter, Result as FmtResult};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use ::serde::de::{self, Deserialize, DeserializeOwned, Deserializer, SeqAccess, Visitor};
use ::serde::ser::{Error as _, Serialize, Serializer};

use super::de::from_data_item;
use super::ser::to_data_item;
use super::wrapper::{Tagged, TaggedWith};
use crate::content::{Array, ByteString};
use crate::data_item::DataItem;
use crate::decode::Decoder;
use crate::encode::Encoder;
use crate::tags;
use crate::validate::has_clean_prefix;

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
pub(super) struct Bytes<'a>(pub(super) &'a [u8]);

impl Serialize for Bytes<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(self.0)
    }
}

/// Bytes read from a byte string or a sequence of numbers.
pub(super) struct ByteBuf(pub(super) Vec<u8>);

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

registered_tag! {
    /// Tag 1004, an RFC 3339 `full-date` text (RFC 8943).
    DateString(String) = tags::DATE_STRING
}

registered_tag! {
    /// Tag 100, days since 1970-01-01 (RFC 8943), any integer type.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::serde::tags::EpochDays;
    /// use cbor_next::serde::to_vec;
    ///
    /// // 1940-10-09
    /// assert_eq!(to_vec(&EpochDays(-10_676_i64)).unwrap(), [0xd8, 0x64, 0x39, 0x29, 0xb3]);
    /// ```
    EpochDays<T> = tags::EPOCH_DAYS
}

/// Tag 52 or 54, an IP address as its octets (RFC 9164).
///
/// # Example
/// ```rust
/// use std::net::{IpAddr, Ipv4Addr};
///
/// use cbor_next::serde::tags::IpAddress;
/// use cbor_next::serde::{from_slice, to_vec};
///
/// let address = IpAddress(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)));
/// let bytes = to_vec(&address).unwrap();
/// assert_eq!(bytes, [0xd8, 0x34, 0x44, 0xc0, 0x00, 0x02, 0x01]);
/// assert_eq!(from_slice::<IpAddress>(&bytes).unwrap(), address);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IpAddress(pub IpAddr);

impl Serialize for IpAddress {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Tagged {
            number: ip_tag(self.0),
            content: Bytes(&octets(self.0)),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for IpAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let tagged = Tagged::<ByteBuf>::deserialize(deserializer)?;
        address_from(tagged.number, &tagged.content.0)
            .map(Self)
            .map_err(de::Error::custom)
    }
}

/// Tag 52 or 54, a prefix: the leading `length` bits of an address (RFC
/// 9164).
///
/// Written with the bits past the length cleared and trailing zero bytes
/// dropped (section 4.2); read with the checks of section 4.3.
///
/// # Example
/// ```rust
/// use std::net::IpAddr;
///
/// use cbor_next::serde::tags::IpPrefix;
/// use cbor_next::serde::{from_slice, to_vec};
///
/// // 2001:db8::/64
/// let prefix = IpPrefix {
///     address: IpAddr::V6("2001:db8::".parse().unwrap()),
///     length: 64,
/// };
/// let bytes = to_vec(&prefix).unwrap();
/// assert_eq!(
///     bytes,
///     [0xd8, 0x36, 0x82, 0x18, 0x40, 0x44, 0x20, 0x01, 0x0d, 0xb8]
/// );
/// assert_eq!(from_slice::<IpPrefix>(&bytes).unwrap(), prefix);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IpPrefix {
    /// The address; bits past the length are ignored.
    pub address: IpAddr,
    /// Leading bits that matter, up to 32 or 128.
    pub length: u8,
}

impl Serialize for IpPrefix {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if u32::from(self.length) > address_bits(self.address) {
            return Err(S::Error::custom(format_args!(
                "prefix length {} is past the address",
                self.length
            )));
        }
        let mut octets = octets(self.address);
        let reached = usize::from(self.length.div_ceil(8));
        octets.truncate(reached);
        if let Some(last) = octets.last_mut() {
            *last &= u8::MAX
                .checked_shl(u32::from((8 - self.length % 8) % 8))
                .unwrap_or(0);
        }
        while octets.last() == Some(&0) {
            octets.pop();
        }
        Tagged {
            number: ip_tag(self.address),
            content: (self.length, Bytes(&octets)),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for IpPrefix {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let tagged = Tagged::<DataItem>::deserialize(deserializer)?;
        let refused = |what: &str| de::Error::custom(format!("an IP prefix {what}"));
        let (size, max_length) = match tagged.number {
            tags::IPV4 => (4, 32),
            tags::IPV6 => (16, 128),
            number => return Err(refused(&format!("is tag 52 or 54, not {number}"))),
        };
        let Some([length, prefix]) = tagged.content.as_array().map(Array::items) else {
            return Err(refused("is [length, bytes]"));
        };
        let (Some(length), Some(prefix)) = (length.as_unsigned(), prefix.as_byte()) else {
            return Err(refused("is [length, bytes]"));
        };
        if !has_clean_prefix(&tagged.content, max_length) {
            return Err(refused("has bytes or bits past its length"));
        }
        let mut octets = prefix.to_bytes();
        octets.resize(size, 0);
        Ok(Self {
            address: address_from(tagged.number, &octets).map_err(de::Error::custom)?,
            length: u8::try_from(length).map_err(|_| refused("length is too long"))?,
        })
    }
}

/// A zone of RFC 9164 section 3.1.3: an interface index or name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Zone {
    /// The interface index.
    Index(u64),
    /// The interface name.
    Name(String),
}

impl Serialize for Zone {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Index(index) => serializer.serialize_u64(*index),
            Self::Name(name) => serializer.serialize_str(name),
        }
    }
}

/// Tag 52 or 54, an interface: an address, its prefix length and maybe a
/// zone (RFC 9164).
///
/// # Example
/// ```rust
/// use std::net::{IpAddr, Ipv4Addr};
///
/// use cbor_next::serde::tags::{IpInterface, Zone};
/// use cbor_next::serde::{from_slice, to_vec};
///
/// // 192.0.2.1/24 on eth0
/// let interface = IpInterface {
///     address: IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)),
///     prefix_length: Some(24),
///     zone: Some(Zone::Name("eth0".into())),
/// };
/// let bytes = to_vec(&interface).unwrap();
/// assert_eq!(from_slice::<IpInterface>(&bytes).unwrap(), interface);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IpInterface {
    /// The full address.
    pub address: IpAddr,
    /// The prefix length, or `None` for `null`.
    pub prefix_length: Option<u8>,
    /// The zone.
    pub zone: Option<Zone>,
}

impl Serialize for IpInterface {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(length) = self.prefix_length
            && u32::from(length) > address_bits(self.address)
        {
            return Err(S::Error::custom(format_args!(
                "prefix length {length} is past the address"
            )));
        }
        let number = ip_tag(self.address);
        let address = octets(self.address);
        let address = Bytes(&address);
        // the zone is a third element only when there is one
        match &self.zone {
            Some(zone) => {
                Tagged {
                    number,
                    content: (address, self.prefix_length, zone),
                }
                .serialize(serializer)
            }
            None => {
                Tagged {
                    number,
                    content: (address, self.prefix_length),
                }
                .serialize(serializer)
            }
        }
    }
}

impl<'de> Deserialize<'de> for IpInterface {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let tagged = Tagged::<DataItem>::deserialize(deserializer)?;
        let refused = || de::Error::custom("an IP interface is [address, length or null, ?zone]");
        let Some([address, length, zone @ ..]) = tagged.content.as_array().map(Array::items) else {
            return Err(refused());
        };
        let octets = address
            .as_byte()
            .map(ByteString::to_bytes)
            .ok_or_else(refused)?;
        let address = address_from(tagged.number, &octets).map_err(de::Error::custom)?;
        let prefix_length = if length.is_null() {
            None
        } else {
            let length = length
                .as_unsigned()
                .and_then(|length| u8::try_from(length).ok())
                .filter(|length| u32::from(*length) <= address_bits(address))
                .ok_or_else(refused)?;
            Some(length)
        };
        let zone = match zone {
            [] => None,
            [zone] => {
                if let Some(index) = zone.as_unsigned() {
                    Some(Zone::Index(index))
                } else {
                    Some(Zone::Name(zone.as_text().ok_or_else(refused)?.to_text()))
                }
            }
            _ => return Err(refused()),
        };
        Ok(Self {
            address,
            prefix_length,
            zone,
        })
    }
}

/// Tag 52 or 54 for an address.
fn ip_tag(address: IpAddr) -> u64 {
    match address {
        IpAddr::V4(_) => tags::IPV4,
        IpAddr::V6(_) => tags::IPV6,
    }
}

/// Bits in an address.
fn address_bits(address: IpAddr) -> u32 {
    match address {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    }
}

/// An address's octets.
fn octets(address: IpAddr) -> Vec<u8> {
    match address {
        IpAddr::V4(address) => address.octets().to_vec(),
        IpAddr::V6(address) => address.octets().to_vec(),
    }
}

/// The address a tag number and octets stand for.
fn address_from(number: u64, octets: &[u8]) -> Result<IpAddr, String> {
    match number {
        tags::IPV4 => <[u8; 4]>::try_from(octets).map(|octets| IpAddr::V4(Ipv4Addr::from(octets))),
        tags::IPV6 => <[u8; 16]>::try_from(octets).map(|octets| IpAddr::V6(Ipv6Addr::from(octets))),
        number => return Err(format!("tag {number} is not an IP address tag")),
    }
    .map_err(|_| format!("{} bytes do not fit tag {number}", octets.len()))
}
