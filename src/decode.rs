//! Decoding `CBOR` bytes into data items.
//!
//! The default [`Decoder`] accepts any well formed item without repeated map
//! keys, nested up to [`Decoder::DEFAULT_MAX_NESTING_DEPTH`]. Tighten it for
//! untrusted input.

use indexmap::IndexMap;
use indexmap::map::Entry;

use crate::content::{Array, ByteString, Map, Simple, Tag};
use crate::data_item::DataItem;
use crate::data_item::convert::{f16_to_f64, f32_to_f64};
use crate::deterministic::{self, DeterministicMode};
use crate::encode::shortest_float_additional;
use crate::error::Error;

/// Byte which closes an indefinite length item.
const BREAK: u8 = 0xFF;

/// What to do with a repeated map key (RFC 8949 section 5.6).
///
/// Keys compare as in section 5.6.1: `1 != 1.0`, `0.0 == -0.0`, and
/// containers compare by content.
///
/// # Example
/// ```rust
/// use cbor_next::{DataItem, Decoder, DuplicateKeyMode, Error};
///
/// // {1: 2, 1: 3}
/// let bytes = [0xa2, 0x01, 0x02, 0x01, 0x03];
/// assert!(matches!(
///     Decoder::new().decode(&bytes),
///     Err(Error::DuplicateMapKey(_))
/// ));
/// assert_eq!(
///     Decoder::new()
///         .duplicate_keys(DuplicateKeyMode::First)
///         .decode(&bytes)
///         .unwrap(),
///     DataItem::map([(1, 2)])
/// );
/// assert_eq!(
///     Decoder::new()
///         .duplicate_keys(DuplicateKeyMode::Last)
///         .decode(&bytes)
///         .unwrap(),
///     DataItem::map([(1, 3)])
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum DuplicateKeyMode {
    /// Fail with [`Error::DuplicateMapKey`].
    #[default]
    Reject,
    /// Keep the first value.
    First,
    /// Keep the last value.
    Last,
}

/// Iterator over a sequence, from [`Decoder::sequence`].
#[derive(Debug)]
pub struct Sequence<'a> {
    decoder: &'a Decoder,
    reader: Reader<'a>,
    failed: bool,
}

impl Iterator for Sequence<'_> {
    type Item = Result<DataItem, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.failed || self.reader.is_empty() {
            return None;
        }
        let item = self
            .decoder
            .read_item(&mut self.reader, self.decoder.max_nesting_depth)
            .and_then(|item| self.decoder.check_deterministic(&item).map(|()| item));
        self.failed = item.is_err();
        Some(item)
    }
}

/// A configurable `CBOR` decoder.
///
/// # Example
/// ```rust
/// use cbor_next::{DataItem, Decoder, DuplicateKeyMode, Error};
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
/// // a repeated map key is rejected unless a mode which keeps one is chosen
/// assert!(DataItem::decode(&[0xa2, 0x01, 0x02, 0x01, 0x03]).is_err());
/// let lenient = Decoder::new().duplicate_keys(DuplicateKeyMode::Last);
/// assert_eq!(
///     lenient.decode(&[0xa2, 0x01, 0x02, 0x01, 0x03]).unwrap(),
///     DataItem::map([(1, 3)])
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoder {
    max_nesting_depth: usize,
    max_items: usize,
    duplicate_keys: DuplicateKeyMode,
    allow_indefinite: bool,
    require_deterministic: Option<DeterministicMode>,
}

impl Default for Decoder {
    fn default() -> Self {
        Self {
            max_nesting_depth: Self::DEFAULT_MAX_NESTING_DEPTH,
            max_items: usize::MAX,
            duplicate_keys: DuplicateKeyMode::Reject,
            allow_indefinite: true,
            require_deterministic: None,
        }
    }
}

impl Decoder {
    /// Default nesting limit; it guards the stack against deep input.
    pub const DEFAULT_MAX_NESTING_DEPTH: usize = 128;

    /// Create a decoder with the default limits.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the nesting limit.
    #[must_use]
    pub fn max_nesting_depth(mut self, depth: usize) -> Self {
        self.max_nesting_depth = depth;
        self
    }

    /// Limit the total number of items, including map keys, tag contents and
    /// string chunks; a sequence shares one limit.
    ///
    /// Unset by default. Set it for untrusted input, since one byte can ask
    /// for a large allocation.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::{DataItem, Decoder, Error};
    ///
    /// let decoder = Decoder::new().max_items(3);
    /// // the array and its two elements are three items
    /// assert_eq!(
    ///     decoder.decode(&[0x82, 0x01, 0x02]).unwrap(),
    ///     DataItem::from(vec![1, 2])
    /// );
    /// assert_eq!(
    ///     decoder.decode(&[0x83, 0x01, 0x02, 0x03]),
    ///     Err(Error::ItemLimit { limit: 3 })
    /// );
    /// ```
    #[must_use]
    pub fn max_items(mut self, items: usize) -> Self {
        self.max_items = items;
        self
    }

    /// Set how repeated map keys are handled; rejected by default.
    #[must_use]
    pub fn duplicate_keys(mut self, mode: DuplicateKeyMode) -> Self {
        self.duplicate_keys = mode;
        self
    }

    /// Whether indefinite length strings, arrays and maps are accepted.
    #[must_use]
    pub fn allow_indefinite(mut self, allow: bool) -> Self {
        self.allow_indefinite = allow;
        self
    }

    /// Reject input not already deterministic in the mode, including heads
    /// and floats wider than needed (`18 05` for `5`).
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

    /// Decode exactly one item.
    ///
    /// # Errors
    /// If the item is malformed, breaks a limit, or bytes remain after it
    pub fn decode(&self, bytes: &[u8]) -> Result<DataItem, Error> {
        let (item, rest) = self.decode_partial(bytes)?;
        if !rest.is_empty() {
            return Err(Error::TrailingBytes { count: rest.len() });
        }
        Ok(item)
    }

    /// Decode the first item and return it with the remaining bytes.
    ///
    /// # Errors
    /// If the item is malformed or breaks a limit
    pub fn decode_partial<'a>(&self, bytes: &'a [u8]) -> Result<(DataItem, &'a [u8]), Error> {
        let mut reader = Reader::new(bytes, self.max_items);
        let item = self.read_item(&mut reader, self.max_nesting_depth)?;
        self.check_deterministic(&item)?;
        Ok((item, reader.rest()))
    }

    /// Decode a whole sequence (RFC 8742).
    ///
    /// # Errors
    /// If any item is malformed or breaks a limit
    pub fn decode_sequence(&self, bytes: &[u8]) -> Result<Vec<DataItem>, Error> {
        self.sequence(bytes).collect()
    }

    /// Decode a sequence one data item at a time.
    ///
    /// Items before a bad one are still yielded; iteration stops at the first
    /// error.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::{DataItem, Decoder};
    ///
    /// // the third item is cut short
    /// let decoder = Decoder::new();
    /// let mut items = decoder.sequence(&[0x01, 0x02, 0x18]);
    /// assert_eq!(items.next(), Some(Ok(DataItem::from(1))));
    /// assert_eq!(items.next(), Some(Ok(DataItem::from(2))));
    /// assert!(matches!(items.next(), Some(Err(_))));
    /// assert_eq!(items.next(), None);
    /// ```
    #[must_use]
    pub fn sequence<'a>(&'a self, bytes: &'a [u8]) -> Sequence<'a> {
        Sequence {
            decoder: self,
            reader: Reader::new(bytes, self.max_items),
            failed: false,
        }
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
        reader.count_item()?;
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
                Ok(DataItem::Unsigned(
                    self.read_argument(major_type, additional, reader)?,
                ))
            }
            1 => {
                Ok(DataItem::Signed(
                    self.read_argument(major_type, additional, reader)?,
                ))
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
                let number = self.read_argument(major_type, additional, reader)?;
                let content = self.read_item(reader, next_depth)?;
                Ok(DataItem::Tag(Tag::new(number, content)))
            }
            7 => self.read_simple_or_float(additional, reader),
            _ => unreachable!("major type can only be between 0 to 7"),
        }
    }

    /// Read a byte string; text is read this way, then checked as UTF-8.
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
                // each chunk is kept in memory, so it counts as an item
                reader.count_item()?;
                let chunk_length =
                    self.read_argument(chunk_major_type, head & 0b0001_1111, reader)?;
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
                match self.duplicate_keys {
                    DuplicateKeyMode::Reject => {
                        Err(Error::DuplicateMapKey(Box::new(entry.key().clone())))
                    }
                    DuplicateKeyMode::First => Ok(()),
                    DuplicateKeyMode::Last => {
                        entry.insert(value);
                        Ok(())
                    }
                }
            }
            Entry::Vacant(entry) => {
                entry.insert(value);
                Ok(())
            }
        }
    }

    /// Read a length; `None` means indefinite.
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
        self.read_argument(major_type, additional, reader).map(Some)
    }

    /// Read a head argument, rejecting a wide form when deterministic.
    fn read_argument(
        &self,
        major_type: u8,
        additional: u8,
        reader: &mut Reader<'_>,
    ) -> Result<u64, Error> {
        let argument = match additional {
            0..=23 => u64::from(additional),
            24 => u64::from(reader.read_u8()?),
            25 => u64::from(u16::from_be_bytes(reader.read_array()?)),
            26 => u64::from(u32::from_be_bytes(reader.read_array()?)),
            27 => u64::from_be_bytes(reader.read_array()?),
            _ => {
                return Err(Error::InvalidAdditionalInfo {
                    major_type,
                    additional,
                });
            }
        };
        if self.require_deterministic.is_some() {
            let shortest = match additional {
                24 => argument > 23,
                25 => argument > u64::from(u8::MAX),
                26 => argument > u64::from(u16::MAX),
                27 => argument > u64::from(u32::MAX),
                _ => true,
            };
            if !shortest {
                return Err(Error::NotDeterministic);
            }
        }
        Ok(argument)
    }

    fn read_simple_or_float(
        &self,
        additional: u8,
        reader: &mut Reader<'_>,
    ) -> Result<DataItem, Error> {
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
                self.float_item(f16_to_f64(half::f16::from_be_bytes(bits)), additional)
            }
            26 => {
                let bits = reader.read_array::<4>()?;
                self.float_item(f32_to_f64(f32::from_be_bytes(bits)), additional)
            }
            27 => {
                let bits = reader.read_array::<8>()?;
                self.float_item(f64::from_be_bytes(bits), additional)
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

    /// Build a float, rejecting a wide form when deterministic.
    fn float_item(&self, number: f64, additional: u8) -> Result<DataItem, Error> {
        if self.require_deterministic.is_some() && shortest_float_additional(number) != additional {
            return Err(Error::NotDeterministic);
        }
        Ok(DataItem::Floating(number))
    }
}

/// Capacity to reserve for `count` announced entries, capped by the remaining
/// bytes and a fixed ceiling, since the input may lie.
fn bounded_capacity(count: u64, reader: &Reader<'_>) -> usize {
    /// Most entries reserved up front
    const MAX_PREALLOCATED: usize = 1024;
    let possible = reader.remaining();
    usize::try_from(count)
        .unwrap_or(usize::MAX)
        .min(possible)
        .min(MAX_PREALLOCATED)
}

/// Cursor over the bytes being decoded.
#[derive(Debug)]
struct Reader<'a> {
    bytes: &'a [u8],
    position: usize,
    items: usize,
    max_items: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], max_items: usize) -> Self {
        Self {
            bytes,
            position: 0,
            items: 0,
            max_items,
        }
    }

    /// Count one more item, failing past the limit.
    fn count_item(&mut self) -> Result<(), Error> {
        if self.items >= self.max_items {
            return Err(Error::ItemLimit {
                limit: self.max_items,
            });
        }
        self.items += 1;
        Ok(())
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

    /// Read bytes of a length the input announced (it may lie).
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

    /// Read bytes of a length the decoder knows.
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
