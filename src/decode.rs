//! Turning `CBOR` bytes into data items.
//!
//! [`DataItem::decode`] uses the default settings of [`Decoder`], which accept
//! any well formed data item that does not nest deeper than
//! [`Decoder::DEFAULT_MAX_NESTING_DEPTH`] and does not repeat a map key. A
//! [`Decoder`] built by hand can tighten or relax those limits, which is how
//! untrusted input is kept to the profile an application expects.

use indexmap::IndexMap;
use indexmap::map::Entry;

use crate::content::{Array, ByteString, Map, Simple, Tag};
use crate::data_item::DataItem;
use crate::deterministic::{self, DeterministicMode};
use crate::error::Error;

/// Byte which closes an indefinite length item.
const BREAK: u8 = 0xFF;

/// A configurable `CBOR` decoder.
///
/// # Example
/// ```rust
/// use cbor_next::{DataItem, Decoder, Error};
///
/// // a shallow limit rejects deeply nested input early
/// let nested = Decoder::new().max_nesting_depth(2);
/// assert_eq!(
///     nested.decode(&[0x81, 0x01]).unwrap(),
///     DataItem::from(vec![1])
/// );
/// assert_eq!(
///     nested.decode(&[0x81, 0x81, 0x01]),
///     Err(Error::RecursionLimit { limit: 2 })
/// );
///
/// // a repeated map key is rejected unless it is explicitly allowed
/// assert!(DataItem::decode(&[0xa2, 0x01, 0x02, 0x01, 0x03]).is_err());
/// let lenient = Decoder::new().allow_duplicate_keys(true);
/// assert_eq!(
///     lenient.decode(&[0xa2, 0x01, 0x02, 0x01, 0x03]).unwrap(),
///     DataItem::map([(1, 3)])
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoder {
    max_nesting_depth: usize,
    allow_duplicate_keys: bool,
    allow_indefinite: bool,
    require_deterministic: Option<DeterministicMode>,
}

impl Default for Decoder {
    fn default() -> Self {
        Self {
            max_nesting_depth: Self::DEFAULT_MAX_NESTING_DEPTH,
            allow_duplicate_keys: false,
            allow_indefinite: true,
            require_deterministic: None,
        }
    }
}

impl Decoder {
    /// Nesting depth a decoder accepts unless it is told otherwise.
    ///
    /// Bounding the depth keeps recursion on maliciously nested input from
    /// overflowing the stack.
    pub const DEFAULT_MAX_NESTING_DEPTH: usize = 128;

    /// Create a decoder with the default limits.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set how deeply data items may nest before decoding fails.
    #[must_use]
    pub fn max_nesting_depth(mut self, depth: usize) -> Self {
        self.max_nesting_depth = depth;
        self
    }

    /// Accept a map which uses the same key more than once, keeping the value
    /// of the last occurrence.
    ///
    /// RFC 8949 calls such a map invalid, so it is rejected by default.
    #[must_use]
    pub fn allow_duplicate_keys(mut self, allow: bool) -> Self {
        self.allow_duplicate_keys = allow;
        self
    }

    /// Accept indefinite length strings, arrays and maps.
    ///
    /// Turning this off restricts input to the length form used by every
    /// deterministic profile.
    #[must_use]
    pub fn allow_indefinite(mut self, allow: bool) -> Self {
        self.allow_indefinite = allow;
        self
    }

    /// Reject input which is not already in the deterministic form of the
    /// provided mode.
    ///
    /// This is the check an application needs when a value is going to be
    /// compared, hashed or signed in its encoded form.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::{Decoder, DeterministicMode, Error};
    ///
    /// let decoder = Decoder::new().require_deterministic(DeterministicMode::Core);
    /// assert!(decoder.decode(&[0xa2, 0x01, 0x02, 0x03, 0x04]).is_ok());
    /// assert_eq!(
    ///     decoder.decode(&[0xa2, 0x03, 0x04, 0x01, 0x02]),
    ///     Err(Error::NotDeterministic)
    /// );
    /// ```
    #[must_use]
    pub fn require_deterministic(mut self, mode: DeterministicMode) -> Self {
        self.require_deterministic = Some(mode);
        self
    }

    /// Decode exactly one data item from the provided bytes.
    ///
    /// # Errors
    /// If the bytes are not a well formed data item, if they break one of the
    /// limits of the decoder, or if bytes remain once the item is complete
    pub fn decode(&self, bytes: &[u8]) -> Result<DataItem, Error> {
        let (item, rest) = self.decode_partial(bytes)?;
        if !rest.is_empty() {
            return Err(Error::TrailingBytes { count: rest.len() });
        }
        Ok(item)
    }

    /// Decode the first data item of the provided bytes and return it along
    /// with the bytes which follow it.
    ///
    /// # Errors
    /// If the leading bytes are not a well formed data item or if they break
    /// one of the limits of the decoder
    pub fn decode_partial<'a>(&self, bytes: &'a [u8]) -> Result<(DataItem, &'a [u8]), Error> {
        let mut reader = Reader::new(bytes);
        let item = self.read_item(&mut reader, self.max_nesting_depth)?;
        self.check_deterministic(&item)?;
        Ok((item, reader.rest()))
    }

    /// Decode every data item of a sequence of concatenated data items, the
    /// form described by RFC 8742.
    ///
    /// # Errors
    /// If any item of the sequence is not a well formed data item or if it
    /// breaks one of the limits of the decoder
    pub fn decode_sequence(&self, bytes: &[u8]) -> Result<Vec<DataItem>, Error> {
        let mut reader = Reader::new(bytes);
        let mut items = vec![];
        while !reader.is_empty() {
            let item = self.read_item(&mut reader, self.max_nesting_depth)?;
            self.check_deterministic(&item)?;
            items.push(item);
        }
        Ok(items)
    }

    fn check_deterministic(&self, item: &DataItem) -> Result<(), Error> {
        if let Some(mode) = self.require_deterministic
            && !deterministic::is_deterministic(item, mode)
        {
            return Err(Error::NotDeterministic);
        }
        Ok(())
    }

    fn read_item(
        &self,
        reader: &mut Reader<'_>,
        remaining_depth: usize,
    ) -> Result<DataItem, Error> {
        let Some(next_depth) = remaining_depth.checked_sub(1) else {
            return Err(Error::RecursionLimit {
                limit: self.max_nesting_depth,
            });
        };
        let head = reader.read_u8()?;
        let major_type = head >> 5;
        let additional = head & 0b0001_1111;
        match major_type {
            0 => {
                Ok(DataItem::Unsigned(read_argument(
                    major_type, additional, reader,
                )?))
            }
            1 => {
                Ok(DataItem::Signed(read_argument(
                    major_type, additional, reader,
                )?))
            }
            2 => {
                Ok(DataItem::Byte(
                    self.read_string(major_type, additional, reader)?,
                ))
            }
            3 => {
                let byte_string = self.read_string(major_type, additional, reader)?;
                Ok(DataItem::Text(byte_string.try_into()?))
            }
            4 => self.read_array(additional, reader, next_depth),
            5 => self.read_map(additional, reader, next_depth),
            6 => {
                let number = read_argument(major_type, additional, reader)?;
                let content = self.read_item(reader, next_depth)?;
                Ok(DataItem::Tag(Tag::new(number, content)))
            }
            7 => read_simple_or_float(additional, reader),
            _ => unreachable!("major type can only be between 0 to 7"),
        }
    }

    /// Read a byte string, which a text string is also read as before it is
    /// checked for being valid UTF-8.
    fn read_string(
        &self,
        major_type: u8,
        additional: u8,
        reader: &mut Reader<'_>,
    ) -> Result<ByteString, Error> {
        let Some(length) = self.read_length(major_type, additional, reader)? else {
            let mut chunks = vec![];
            loop {
                let head = reader.read_u8().map_err(|_| Error::IncompleteIndefinite)?;
                if head == BREAK {
                    break;
                }
                let chunk_major_type = head >> 5;
                if chunk_major_type != major_type {
                    return Err(Error::InvalidChunkMajorType {
                        expected: major_type,
                        found: chunk_major_type,
                    });
                }
                let chunk_length = read_argument(chunk_major_type, head & 0b0001_1111, reader)?;
                chunks.push(reader.read_slice(chunk_length)?.to_vec());
            }
            return Ok(ByteString::from_chunks(chunks));
        };
        Ok(ByteString::new(reader.read_slice(length)?))
    }

    fn read_array(
        &self,
        additional: u8,
        reader: &mut Reader<'_>,
        depth: usize,
    ) -> Result<DataItem, Error> {
        let length = self.read_length(4, additional, reader)?;
        let mut items = vec![];
        if let Some(count) = length {
            items.reserve(bounded_capacity(count, reader));
            for _ in 0..count {
                items.push(self.read_item(reader, depth)?);
            }
        } else {
            loop {
                match reader.peek() {
                    Some(BREAK) => {
                        reader.advance();
                        break;
                    }
                    Some(_) => items.push(self.read_item(reader, depth)?),
                    None => return Err(Error::IncompleteIndefinite),
                }
            }
        }
        Ok(DataItem::Array(
            Array::from_items(items).with_indefinite(length.is_none()),
        ))
    }

    fn read_map(
        &self,
        additional: u8,
        reader: &mut Reader<'_>,
        depth: usize,
    ) -> Result<DataItem, Error> {
        let length = self.read_length(5, additional, reader)?;
        let mut entries = IndexMap::new();
        if let Some(count) = length {
            entries.reserve(bounded_capacity(count, reader));
            for _ in 0..count {
                let key = self.read_item(reader, depth)?;
                let value = self.read_item(reader, depth)?;
                self.insert_entry(&mut entries, key, value)?;
            }
        } else {
            loop {
                match reader.peek() {
                    Some(BREAK) => {
                        reader.advance();
                        break;
                    }
                    Some(_) => {
                        let key = self.read_item(reader, depth)?;
                        let value = self.read_item(reader, depth)?;
                        self.insert_entry(&mut entries, key, value)?;
                    }
                    None => return Err(Error::IncompleteIndefinite),
                }
            }
        }
        Ok(DataItem::Map(
            Map::from(entries).with_indefinite(length.is_none()),
        ))
    }

    fn insert_entry(
        &self,
        entries: &mut IndexMap<DataItem, DataItem>,
        key: DataItem,
        value: DataItem,
    ) -> Result<(), Error> {
        match entries.entry(key) {
            Entry::Occupied(mut entry) => {
                if self.allow_duplicate_keys {
                    entry.insert(value);
                    Ok(())
                } else {
                    Err(Error::DuplicateMapKey(Box::new(entry.key().clone())))
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(value);
                Ok(())
            }
        }
    }

    /// Read the length of a string, an array or a map, where `None` stands for
    /// the indefinite length form.
    fn read_length(
        &self,
        major_type: u8,
        additional: u8,
        reader: &mut Reader<'_>,
    ) -> Result<Option<u64>, Error> {
        if additional == 31 {
            if !self.allow_indefinite {
                return Err(Error::IndefiniteNotAllowed);
            }
            return Ok(None);
        }
        read_argument(major_type, additional, reader).map(Some)
    }
}

/// Read the argument of a head, which is either encoded in the additional
/// information itself or in the bytes which follow it.
fn read_argument(major_type: u8, additional: u8, reader: &mut Reader<'_>) -> Result<u64, Error> {
    match additional {
        0..=23 => Ok(u64::from(additional)),
        24 => Ok(u64::from(reader.read_u8()?)),
        25 => Ok(u64::from(u16::from_be_bytes(reader.read_array()?))),
        26 => Ok(u64::from(u32::from_be_bytes(reader.read_array()?))),
        27 => Ok(u64::from_be_bytes(reader.read_array()?)),
        _ => {
            Err(Error::InvalidAdditionalInfo {
                major_type,
                additional,
            })
        }
    }
}

fn read_simple_or_float(additional: u8, reader: &mut Reader<'_>) -> Result<DataItem, Error> {
    match additional {
        0..=19 => Ok(DataItem::GenericSimple(Simple::try_from(additional)?)),
        20 => Ok(DataItem::Boolean(false)),
        21 => Ok(DataItem::Boolean(true)),
        22 => Ok(DataItem::Null),
        23 => Ok(DataItem::Undefined),
        24 => {
            let value = reader.read_u8()?;
            if value < 32 {
                Err(Error::InvalidSimpleValue(value))
            } else {
                Ok(DataItem::GenericSimple(Simple::try_from(value)?))
            }
        }
        25 => {
            let bits = reader.read_array::<2>()?;
            Ok(DataItem::Floating(f64::from(half::f16::from_be_bytes(
                bits,
            ))))
        }
        26 => {
            let bits = reader.read_array::<4>()?;
            Ok(DataItem::Floating(f64::from(f32::from_be_bytes(bits))))
        }
        27 => {
            let bits = reader.read_array::<8>()?;
            Ok(DataItem::Floating(f64::from_be_bytes(bits)))
        }
        28..=30 => {
            Err(Error::InvalidAdditionalInfo {
                major_type: 7,
                additional,
            })
        }
        _ => Err(Error::UnexpectedBreak),
    }
}

/// Capacity to reserve for a collection whose head announces `count` entries.
///
/// The announced count is not trusted on its own, since a short input may claim
/// a huge collection, so it is capped both by the number of entries the
/// remaining bytes could possibly hold and by a fixed ceiling. Growing past the
/// ceiling is left to the reallocation the collection does on its own.
fn bounded_capacity(count: u64, reader: &Reader<'_>) -> usize {
    /// Highest number of entries which is reserved up front
    const MAX_PREALLOCATED: usize = 1024;
    let possible = reader.remaining();
    usize::try_from(count)
        .unwrap_or(usize::MAX)
        .min(possible)
        .min(MAX_PREALLOCATED)
}

/// Cursor over the bytes being decoded.
struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.position
    }

    fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    fn rest(&self) -> &'a [u8] {
        &self.bytes[self.position..]
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.position).copied()
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn read_u8(&mut self) -> Result<u8, Error> {
        let byte = self.peek().ok_or(Error::UnexpectedEnd { missing: 1 })?;
        self.advance();
        Ok(byte)
    }

    /// Read a run of bytes whose length was announced by the input, which may
    /// be far longer than the input itself.
    fn read_slice(&mut self, length: u64) -> Result<&'a [u8], Error> {
        let remaining = u64::try_from(self.remaining()).unwrap_or(u64::MAX);
        if length > remaining {
            return Err(Error::UnexpectedEnd {
                missing: length - remaining,
            });
        }
        let length = usize::try_from(length).expect("length is not greater than remaining bytes");
        self.take(length)
    }

    /// Read a run of bytes of a length which is known to the decoder itself.
    fn take(&mut self, length: usize) -> Result<&'a [u8], Error> {
        let remaining = self.remaining();
        if length > remaining {
            return Err(Error::UnexpectedEnd {
                missing: u64::try_from(length - remaining).unwrap_or(u64::MAX),
            });
        }
        let start = self.position;
        self.position += length;
        Ok(&self.bytes[start..self.position])
    }

    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let slice = self.take(N)?;
        Ok(slice.try_into().expect("slice is exactly N bytes long"))
    }
}
