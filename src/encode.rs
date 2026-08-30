//! Turning data items into `CBOR` bytes.
//!
//! [`DataItem::encode`] uses the default settings of [`Encoder`], which keep
//! every item exactly as it is held. An [`Encoder`] built by hand can instead
//! normalize what it writes, for example to emit the deterministic form of a
//! value without first rewriting the value itself.

use crate::data_item::DataItem;
use crate::deterministic::DeterministicMode;

/// Length form used for byte strings, text strings, arrays and maps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum LengthMode {
    /// Write each item with the length form it carries.
    ///
    /// A decoded item therefore encodes back to the exact bytes it came from.
    #[default]
    Preserve,
    /// Write every item with a definite length.
    ///
    /// Chunks of an indefinite length string are joined into a single run of
    /// bytes.
    Definite,
}

/// Width used for floating point numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum FloatMode {
    /// Write each number in the shortest width which preserves its value, the
    /// preferred serialization of RFC 8949.
    #[default]
    Shortest,
    /// Write every number as a 64 bit double.
    Double,
}

/// A configurable `CBOR` encoder.
///
/// # Example
/// ```rust
/// use cbor_next::{DataItem, DeterministicMode, Encoder, FloatMode};
///
/// let item = DataItem::map([(2, 1.5), (1, 0.5)]);
///
/// // the default encoder writes the map in the order it is held
/// assert_eq!(Encoder::new().encode(&item), item.encode());
///
/// // a deterministic encoder sorts the keys and forces definite lengths
/// let deterministic = Encoder::new().deterministic(DeterministicMode::Core);
/// assert_eq!(
///     deterministic.encode(&item),
///     DataItem::map([(1, 0.5), (2, 1.5)]).encode()
/// );
///
/// // and every knob can be set on its own
/// let wide = Encoder::new().float(FloatMode::Double);
/// assert_eq!(wide.encode(&DataItem::from(1.5)).len(), 9);
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Encoder {
    deterministic: Option<DeterministicMode>,
    length: LengthMode,
    float: FloatMode,
}

impl Encoder {
    /// Create an encoder which writes every item exactly as it is held.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Write the deterministic form of every item in the provided mode.
    ///
    /// Map keys are sorted, definite lengths are used throughout and floating
    /// point numbers use their shortest width, whatever the other settings
    /// say. The value being encoded is left untouched, unlike
    /// [`DataItem::into_deterministic`].
    #[must_use]
    pub fn deterministic(mut self, mode: DeterministicMode) -> Self {
        self.deterministic = Some(mode);
        self
    }

    /// Select the length form to write items with.
    #[must_use]
    pub fn length(mut self, mode: LengthMode) -> Self {
        self.length = mode;
        self
    }

    /// Select the width to write floating point numbers with.
    #[must_use]
    pub fn float(mut self, mode: FloatMode) -> Self {
        self.float = mode;
        self
    }

    /// Encode one data item.
    #[must_use]
    pub fn encode(&self, item: &DataItem) -> Vec<u8> {
        let mut buffer = vec![];
        self.encode_into(item, &mut buffer);
        buffer
    }

    /// Append one encoded data item to an existing buffer.
    pub fn encode_into(&self, item: &DataItem, buffer: &mut Vec<u8>) {
        self.write_item(item, buffer);
    }

    /// Encode a sequence of data items into the concatenated form described by
    /// RFC 8742.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::{DataItem, Encoder};
    ///
    /// let items = [DataItem::from(1), DataItem::from(2)];
    /// assert_eq!(Encoder::new().encode_sequence(&items), vec![0x01, 0x02]);
    /// ```
    #[must_use]
    pub fn encode_sequence<'a, I>(&self, items: I) -> Vec<u8>
    where
        I: IntoIterator<Item = &'a DataItem>,
    {
        let mut buffer = vec![];
        self.encode_sequence_into(items, &mut buffer);
        buffer
    }

    /// Append a sequence of encoded data items to an existing buffer.
    pub fn encode_sequence_into<'a, I>(&self, items: I, buffer: &mut Vec<u8>)
    where
        I: IntoIterator<Item = &'a DataItem>,
    {
        for item in items {
            self.write_item(item, buffer);
        }
    }

    /// Whether an item has to be written with a definite length.
    fn is_definite(&self) -> bool {
        self.deterministic.is_some() || self.length == LengthMode::Definite
    }

    fn write_item(&self, item: &DataItem, buffer: &mut Vec<u8>) {
        let major_type = item.major_type();
        match item {
            DataItem::Unsigned(number) | DataItem::Signed(number) => {
                write_head(buffer, major_type, *number);
            }
            DataItem::Byte(byte_string) => {
                self.write_chunks(
                    buffer,
                    major_type,
                    byte_string.is_indefinite(),
                    byte_string.chunks(),
                );
            }
            DataItem::Text(text_string) => {
                self.write_chunks(
                    buffer,
                    major_type,
                    text_string.is_indefinite(),
                    text_string.chunks(),
                );
            }
            DataItem::Array(array) => {
                let indefinite = array.is_indefinite() && !self.is_definite();
                if indefinite {
                    buffer.push(major_type << 5 | 31);
                } else {
                    write_head(buffer, major_type, count_of(array.items().len()));
                }
                for value in array.items() {
                    self.write_item(value, buffer);
                }
                if indefinite {
                    buffer.push(BREAK);
                }
            }
            DataItem::Map(map) => {
                let indefinite = map.is_indefinite() && !self.is_definite();
                if indefinite {
                    buffer.push(major_type << 5 | 31);
                } else {
                    write_head(buffer, major_type, count_of(map.entries().len()));
                }
                if let Some(mode) = self.deterministic {
                    let mut entries = map
                        .entries()
                        .iter()
                        .map(|(key, value)| (self.encode(key), value))
                        .collect::<Vec<_>>();
                    entries.sort_by(|(first, _), (second, _)| mode.compare_keys(first, second));
                    for (key, value) in entries {
                        buffer.extend_from_slice(&key);
                        self.write_item(value, buffer);
                    }
                } else {
                    for (key, value) in map.entries() {
                        self.write_item(key, buffer);
                        self.write_item(value, buffer);
                    }
                }
                if indefinite {
                    buffer.push(BREAK);
                }
            }
            DataItem::Tag(tag) => {
                write_head(buffer, major_type, tag.number());
                self.write_item(tag.content(), buffer);
            }
            DataItem::Boolean(false) => buffer.push(major_type << 5 | 0x14), // 20
            DataItem::Boolean(true) => buffer.push(major_type << 5 | 0x15),  // 21
            DataItem::Null => buffer.push(major_type << 5 | 0x16),           // 22
            DataItem::Undefined => buffer.push(major_type << 5 | 0x17),      // 23
            DataItem::Floating(number) => {
                if self.float == FloatMode::Double && self.deterministic.is_none() {
                    buffer.push(major_type << 5 | 0x1B); // 27
                    buffer.extend_from_slice(&number.to_be_bytes());
                } else {
                    write_float(buffer, major_type, *number);
                }
            }
            DataItem::GenericSimple(simple) => {
                let value = simple.value();
                if value <= 23 {
                    buffer.push(major_type << 5 | value);
                } else {
                    buffer.push(major_type << 5 | 0x18); // 24
                    buffer.push(value);
                }
            }
        }
    }

    /// Write the chunks of a byte or text string without joining them into an
    /// intermediate allocation.
    fn write_chunks<T>(
        &self,
        buffer: &mut Vec<u8>,
        major_type: u8,
        is_indefinite: bool,
        chunks: &[T],
    ) where
        T: AsRef<[u8]>,
    {
        if is_indefinite && !self.is_definite() {
            buffer.push(major_type << 5 | 31);
            for chunk in chunks {
                let chunk = chunk.as_ref();
                write_head(buffer, major_type, count_of(chunk.len()));
                buffer.extend_from_slice(chunk);
            }
            buffer.push(BREAK);
        } else {
            let length = chunks
                .iter()
                .map(|chunk| count_of(chunk.as_ref().len()))
                .sum();
            write_head(buffer, major_type, length);
            for chunk in chunks {
                buffer.extend_from_slice(chunk.as_ref());
            }
        }
    }
}

/// Byte which closes an indefinite length item.
const BREAK: u8 = 0xFF;

/// Length or count of an in memory collection as the `u64` a `CBOR` head needs.
fn count_of(length: usize) -> u64 {
    u64::try_from(length).expect("in memory length fits in u64")
}

/// Write the head of a data item, using the shortest form which fits the
/// argument as RFC 8949 prefers.
fn write_head(buffer: &mut Vec<u8>, major_type: u8, argument: u64) {
    let shifted_major_type = major_type << 5;
    if let Ok(u8_value) = u8::try_from(argument) {
        if u8_value <= 23 {
            buffer.push(shifted_major_type | u8_value);
        } else {
            buffer.push(shifted_major_type | 0x18); // 24
            buffer.push(u8_value);
        }
    } else if let Ok(u16_value) = u16::try_from(argument) {
        buffer.push(shifted_major_type | 0x19); // 25
        buffer.extend_from_slice(&u16_value.to_be_bytes());
    } else if let Ok(u32_value) = u32::try_from(argument) {
        buffer.push(shifted_major_type | 0x1A); // 26
        buffer.extend_from_slice(&u32_value.to_be_bytes());
    } else {
        buffer.push(shifted_major_type | 0x1B); // 27
        buffer.extend_from_slice(&argument.to_be_bytes());
    }
}

/// Write a floating point number in the shortest width which preserves it.
fn write_float(buffer: &mut Vec<u8>, major_type: u8, number: f64) {
    let shifted_major_type = major_type << 5;
    if number.is_nan() {
        write_nan(buffer, shifted_major_type, number);
        return;
    }
    let f16_number = half::f16::from_f64(number);
    #[expect(
        clippy::float_cmp,
        reason = "we want to compare without margin or error"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        reason = "we only want to check truncation data loss"
    )]
    if f16_number.to_f64() == number {
        buffer.push(shifted_major_type | 0x19); // 25
        buffer.extend_from_slice(&f16_number.to_be_bytes());
    } else if f64::from(number as f32) == number {
        buffer.push(shifted_major_type | 0x1A); // 26
        buffer.extend_from_slice(&(number as f32).to_be_bytes());
    } else {
        buffer.push(shifted_major_type | 0x1B); // 27
        buffer.extend_from_slice(&number.to_be_bytes());
    }
}

/// Write a NaN in the shortest floating point width which preserves its sign
/// and payload, as required by RFC 8949 preferred serialization.
fn write_nan(buffer: &mut Vec<u8>, shifted_major_type: u8, number: f64) {
    /// Low mantissa bits of a f64 which are discarded when narrowing to f16
    const F16_DISCARDED_MANTISSA: u64 = (1 << 42) - 1;
    /// Low mantissa bits of a f64 which are discarded when narrowing to f32
    const F32_DISCARDED_MANTISSA: u64 = (1 << 29) - 1;
    let bits = number.to_bits();
    if bits & F16_DISCARDED_MANTISSA == 0 {
        let sign = (bits >> 48) & 0x8000;
        let exponent = 0x7C00;
        let mantissa = (bits >> 42) & 0x03FF;
        let f16_bits = sign | exponent | mantissa;
        buffer.push(shifted_major_type | 0x19); // 25
        buffer.extend_from_slice(
            &u16::try_from(f16_bits)
                .expect("value is masked to 16 bits")
                .to_be_bytes(),
        );
    } else if bits & F32_DISCARDED_MANTISSA == 0 {
        let sign = (bits >> 32) & 0x8000_0000;
        let exponent = 0x7F80_0000;
        let mantissa = (bits >> 29) & 0x007F_FFFF;
        let f32_bits = sign | exponent | mantissa;
        buffer.push(shifted_major_type | 0x1A); // 26
        buffer.extend_from_slice(
            &u32::try_from(f32_bits)
                .expect("value is masked to 32 bits")
                .to_be_bytes(),
        );
    } else {
        buffer.push(shifted_major_type | 0x1B); // 27
        buffer.extend_from_slice(&bits.to_be_bytes());
    }
}
