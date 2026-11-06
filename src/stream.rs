//! Streaming: `CBOR` as tokens, one per head (RFC 8949 section 5.1).
//!
//! A [`StreamDecoder`] is fed bytes in any pieces and yields a [`Token`] for
//! each head it can complete, so nested items and RFC 8742 sequences are seen
//! as they arrive. A [`StreamEncoder`] writes tokens back to bytes.
//!
//! Both check well-formedness, including where each break goes, and the
//! decoder checks UTF-8. Of the [`Decoder`] settings, the stream applies the
//! nesting depth, item count and indefinite lengths. It cannot see repeated
//! map keys, tag content or the deterministic form (section 5.6), so those
//! are left to the application.
//!
//! # Example
//! ```rust
//! use cbor_next::{Decoder, Token};
//!
//! // [1, "a"] arriving in two pieces
//! let mut stream = Decoder::new().stream();
//! stream.feed(&[0x82, 0x01, 0x61]);
//! assert_eq!(
//!     stream.next_token().unwrap(),
//!     Some(Token::ArrayStart(Some(2)))
//! );
//! assert_eq!(stream.next_token().unwrap(), Some(Token::Unsigned(1)));
//! // the text is not complete yet
//! assert_eq!(stream.next_token().unwrap(), None);
//! stream.feed(&[0x61]);
//! assert_eq!(stream.next_token().unwrap(), Some(Token::Text("a".into())));
//! assert!(stream.is_at_boundary());
//! ```

use crate::content::Simple;
use crate::data_item::convert::{f16_to_f64, f32_to_f64};
use crate::decode::Decoder;
use crate::encode::{count_of, write_float, write_head};
use crate::error::Error;

/// Byte which closes an indefinite length item.
const BREAK: u8 = 0xFF;

/// One head of `CBOR`, with the payload of a definite length string.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Token {
    /// Unsigned integer, major type 0.
    Unsigned(u64),
    /// Negative integer `-1 - n`, major type 1.
    Signed(u64),
    /// A definite length byte string, or a chunk of an indefinite one.
    Bytes(Vec<u8>),
    /// A definite length text string, or a chunk of an indefinite one.
    Text(String),
    /// Start of an indefinite length byte string; chunks follow, then a break.
    BytesStart,
    /// Start of an indefinite length text string; chunks follow, then a break.
    TextStart,
    /// Start of an array of `n` items, or of indefinite length.
    ArrayStart(Option<u64>),
    /// Start of a map of `n` pairs, or of indefinite length.
    MapStart(Option<u64>),
    /// A tag number; the tagged item follows.
    Tag(u64),
    /// `false` or `true`.
    Boolean(bool),
    /// `null`.
    Null,
    /// `undefined`.
    Undefined,
    /// A float of any width.
    Floating(f64),
    /// Any other simple value.
    Simple(Simple),
    /// End of an indefinite length item.
    Break,
}

/// An open item, waiting for what completes it.
#[derive(Debug, Clone, Copy)]
enum Frame {
    /// Items left, or `None` until a break
    Array(Option<u64>),
    /// Pairs left, or `None` until a break, and whether a value is due
    Map {
        remaining: Option<u64>,
        value_next: bool,
    },
    /// The tagged item is due
    Tag,
    /// Chunks of this major type until a break
    Chunks(u8),
}

/// The open items, checked token by token.
#[derive(Debug, Clone)]
struct Nesting {
    frames: Vec<Frame>,
    limit: usize,
}

impl Nesting {
    fn new(limit: usize) -> Self {
        Self {
            frames: vec![],
            limit,
        }
    }

    /// The major type of the indefinite string open innermost.
    fn chunk_type(&self) -> Option<u8> {
        match self.frames.last() {
            Some(Frame::Chunks(major_type)) => Some(*major_type),
            _ => None,
        }
    }

    /// Account for a token, or fail without changing anything.
    fn settle(&mut self, token: &Token) -> Result<(), Error> {
        if let Some(expected) = self.chunk_type() {
            let found = token_major_type(token);
            let is_chunk = matches!(token, Token::Bytes(_) | Token::Text(_)) && found == expected;
            if !is_chunk && !matches!(token, Token::Break) {
                return Err(Error::InvalidChunkMajorType { expected, found });
            }
        }
        match token {
            Token::Break => {
                let closes = matches!(
                    self.frames.last(),
                    Some(
                        Frame::Array(None)
                            | Frame::Chunks(_)
                            | Frame::Map {
                                remaining: None,
                                value_next: false,
                            }
                    )
                );
                if !closes {
                    return Err(Error::UnexpectedBreak);
                }
                self.frames.pop();
                self.finish_item();
            }
            Token::ArrayStart(Some(0)) | Token::MapStart(Some(0)) => self.finish_item(),
            Token::ArrayStart(count) => self.open(Frame::Array(*count))?,
            Token::MapStart(count) => {
                self.open(Frame::Map {
                    remaining: *count,
                    value_next: false,
                })?;
            }
            Token::Tag(_) => self.open(Frame::Tag)?,
            Token::BytesStart => self.open(Frame::Chunks(2))?,
            Token::TextStart => self.open(Frame::Chunks(3))?,
            // a chunk completes nothing on its own
            Token::Bytes(_) | Token::Text(_) if self.chunk_type().is_some() => {}
            _ => self.finish_item(),
        }
        Ok(())
    }

    fn open(&mut self, frame: Frame) -> Result<(), Error> {
        // the top-level item takes one level, as in the decoder
        if self.frames.len() + 1 >= self.limit {
            return Err(Error::RecursionLimit { limit: self.limit });
        }
        self.frames.push(frame);
        Ok(())
    }

    /// One item completed: close every frame it completes.
    fn finish_item(&mut self) {
        loop {
            match self.frames.last_mut() {
                Some(Frame::Tag) => {
                    self.frames.pop();
                }
                Some(Frame::Array(Some(remaining))) => {
                    *remaining -= 1;
                    if *remaining > 0 {
                        return;
                    }
                    self.frames.pop();
                }
                Some(Frame::Map {
                    remaining,
                    value_next,
                }) => {
                    *value_next = !*value_next;
                    if *value_next {
                        return;
                    }
                    match remaining {
                        Some(0) => unreachable!("a map with no pair left is closed"),
                        Some(count) => {
                            *count -= 1;
                            if *count > 0 {
                                return;
                            }
                            self.frames.pop();
                        }
                        None => return,
                    }
                }
                Some(Frame::Array(None) | Frame::Chunks(_)) | None => return,
            }
        }
    }
}

/// The major type a token is written with.
fn token_major_type(token: &Token) -> u8 {
    match token {
        Token::Unsigned(_) => 0,
        Token::Signed(_) => 1,
        Token::Bytes(_) | Token::BytesStart => 2,
        Token::Text(_) | Token::TextStart => 3,
        Token::ArrayStart(_) => 4,
        Token::MapStart(_) => 5,
        Token::Tag(_) => 6,
        _ => 7,
    }
}

/// Decodes bytes into [`Token`]s as they arrive, from [`Decoder::stream`].
///
/// After an error it yields nothing more.
#[derive(Debug, Clone)]
pub struct StreamDecoder {
    buffer: Vec<u8>,
    consumed: usize,
    nesting: Nesting,
    items: usize,
    item_limit: usize,
    allow_indefinite: bool,
    failed: bool,
}

impl StreamDecoder {
    pub(crate) fn new(nesting_limit: usize, item_limit: usize, allow_indefinite: bool) -> Self {
        Self {
            buffer: vec![],
            consumed: 0,
            nesting: Nesting::new(nesting_limit),
            items: 0,
            item_limit,
            allow_indefinite,
            failed: false,
        }
    }

    /// Add bytes to decode.
    pub fn feed(&mut self, bytes: &[u8]) {
        // drop handed out bytes once they outweigh what is waiting
        if self.consumed > 0 && self.consumed >= self.buffer.len() / 2 {
            self.buffer.drain(..self.consumed);
            self.consumed = 0;
        }
        self.buffer.extend_from_slice(bytes);
    }

    /// Bytes fed but not yet handed out as tokens.
    #[must_use]
    pub fn pending(&self) -> usize {
        self.buffer.len() - self.consumed
    }

    /// Items open now.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.nesting.frames.len()
    }

    /// Whether every item started has ended and no byte is waiting, so a
    /// sequence may end here.
    #[must_use]
    pub fn is_at_boundary(&self) -> bool {
        !self.failed && self.nesting.frames.is_empty() && self.pending() == 0
    }

    /// The next token, or `None` until more bytes are fed.
    ///
    /// # Errors
    /// If the input is not well formed or breaks a limit of the decoder
    pub fn next_token(&mut self) -> Result<Option<Token>, Error> {
        if self.failed {
            return Ok(None);
        }
        let result = self.read_token().and_then(|read| {
            let Some((token, used)) = read else {
                return Ok(None);
            };
            self.nesting.settle(&token)?;
            self.consumed += used;
            // a break is not an item, as in the decoder
            if token != Token::Break {
                self.items += 1;
            }
            Ok(Some(token))
        });
        if result.is_err() {
            self.failed = true;
        }
        result
    }

    /// The token at the front and its length, or `None` if incomplete.
    fn read_token(&self) -> Result<Option<(Token, usize)>, Error> {
        let bytes = &self.buffer[self.consumed..];
        let Some(&head) = bytes.first() else {
            return Ok(None);
        };
        if head == BREAK {
            return Ok(Some((Token::Break, 1)));
        }
        if self.items >= self.item_limit {
            return Err(Error::ItemLimit {
                limit: self.item_limit,
            });
        }
        let major_type = head >> 5;
        let additional = head & 0b0001_1111;
        // chunks are definite strings of the string's major type
        if let Some(expected) = self.nesting.chunk_type() {
            if major_type != expected {
                return Err(Error::InvalidChunkMajorType {
                    expected,
                    found: major_type,
                });
            }
            if additional == 31 {
                return Err(Error::InvalidAdditionalInfo {
                    major_type,
                    additional,
                });
            }
        }
        if additional == 31 {
            return self
                .start_indefinite(major_type)
                .map(|token| Some((token, 1)));
        }
        if major_type == 7 {
            let read = read_simple_or_float(additional, &bytes[1..])?;
            return Ok(read.map(|(token, used)| (token, used + 1)));
        }
        let Some((argument, used)) = read_argument(major_type, additional, &bytes[1..])? else {
            return Ok(None);
        };
        let used = used + 1;
        let token = match major_type {
            0 => Token::Unsigned(argument),
            1 => Token::Signed(argument),
            2 | 3 => {
                let Some(payload) = take(&bytes[used..], argument) else {
                    return Ok(None);
                };
                let used = used + payload.len();
                if major_type == 2 {
                    return Ok(Some((Token::Bytes(payload.to_vec()), used)));
                }
                let text = String::from_utf8(payload.to_vec())?;
                return Ok(Some((Token::Text(text), used)));
            }
            4 => Token::ArrayStart(Some(argument)),
            5 => Token::MapStart(Some(argument)),
            _ => Token::Tag(argument),
        };
        Ok(Some((token, used)))
    }

    /// The token for an indefinite length head.
    fn start_indefinite(&self, major_type: u8) -> Result<Token, Error> {
        if !matches!(major_type, 2..=5) {
            return Err(Error::InvalidAdditionalInfo {
                major_type,
                additional: 31,
            });
        }
        if !self.allow_indefinite {
            return Err(Error::IndefiniteNotAllowed);
        }
        Ok(match major_type {
            2 => Token::BytesStart,
            3 => Token::TextStart,
            4 => Token::ArrayStart(None),
            _ => Token::MapStart(None),
        })
    }
}

/// A head's argument and its length, or `None` if incomplete.
fn read_argument(
    major_type: u8,
    additional: u8,
    bytes: &[u8],
) -> Result<Option<(u64, usize)>, Error> {
    let width = match additional {
        0..=23 => return Ok(Some((u64::from(additional), 0))),
        24 => 1,
        25 => 2,
        26 => 4,
        27 => 8,
        _ => {
            return Err(Error::InvalidAdditionalInfo {
                major_type,
                additional,
            });
        }
    };
    let Some(bytes) = bytes.get(..width) else {
        return Ok(None);
    };
    let argument = bytes
        .iter()
        .fold(0_u64, |argument, byte| argument << 8 | u64::from(*byte));
    Ok(Some((argument, width)))
}

/// The first `length` bytes, if there are that many.
fn take(bytes: &[u8], length: u64) -> Option<&[u8]> {
    bytes.get(..usize::try_from(length).ok()?)
}

/// A major type 7 token and its length after the head, or `None` if
/// incomplete.
fn read_simple_or_float(additional: u8, bytes: &[u8]) -> Result<Option<(Token, usize)>, Error> {
    let token = match additional {
        0..=19 => Token::Simple(Simple::try_from(additional)?),
        20 => Token::Boolean(false),
        21 => Token::Boolean(true),
        22 => Token::Null,
        23 => Token::Undefined,
        24 => {
            let Some(&value) = bytes.first() else {
                return Ok(None);
            };
            if value < 32 {
                return Err(Error::InvalidSimpleValue(value));
            }
            return Ok(Some((Token::Simple(Simple::try_from(value)?), 1)));
        }
        25 => {
            let Some(&[first, second]) = bytes.first_chunk::<2>() else {
                return Ok(None);
            };
            let number = f16_to_f64(half::f16::from_be_bytes([first, second]));
            return Ok(Some((Token::Floating(number), 2)));
        }
        26 => {
            let Some(bits) = bytes.first_chunk::<4>() else {
                return Ok(None);
            };
            let number = f32_to_f64(f32::from_be_bytes(*bits));
            return Ok(Some((Token::Floating(number), 4)));
        }
        27 => {
            let Some(bits) = bytes.first_chunk::<8>() else {
                return Ok(None);
            };
            return Ok(Some((Token::Floating(f64::from_be_bytes(*bits)), 8)));
        }
        _ => {
            return Err(Error::InvalidAdditionalInfo {
                major_type: 7,
                additional,
            });
        }
    };
    Ok(Some((token, 0)))
}

/// Writes [`Token`]s as bytes, refusing any that would not be well formed.
///
/// Numbers are written in their shortest form. Repeated map keys and tag
/// content are not checked.
///
/// # Example
/// ```rust
/// use cbor_next::{StreamEncoder, Token};
///
/// let mut encoder = StreamEncoder::new();
/// encoder.push(&Token::ArrayStart(None)).unwrap();
/// encoder.push(&Token::Unsigned(1)).unwrap();
/// // a break cannot stand where a map key is due
/// encoder.push(&Token::MapStart(Some(1))).unwrap();
/// assert!(encoder.push(&Token::Break).is_err());
/// ```
#[derive(Debug, Clone)]
pub struct StreamEncoder {
    buffer: Vec<u8>,
    nesting: Nesting,
}

impl Default for StreamEncoder {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamEncoder {
    /// Create an encoder with the decoder's default nesting limit.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: vec![],
            nesting: Nesting::new(Decoder::DEFAULT_MAX_NESTING_DEPTH),
        }
    }

    /// Set the nesting limit.
    #[must_use]
    pub fn max_nesting_depth(mut self, depth: usize) -> Self {
        self.nesting.limit = depth;
        self
    }

    /// Write a token; on error nothing is written.
    ///
    /// # Errors
    /// If the token is out of place, such as a break with nothing open to
    /// close, or opens past the nesting limit
    pub fn push(&mut self, token: &Token) -> Result<(), Error> {
        self.nesting.settle(token)?;
        let buffer = &mut self.buffer;
        match token {
            Token::Unsigned(number) => write_head(buffer, 0, *number),
            Token::Signed(number) => write_head(buffer, 1, *number),
            Token::Bytes(bytes) => {
                write_head(buffer, 2, count_of(bytes.len()));
                buffer.extend_from_slice(bytes);
            }
            Token::Text(text) => {
                write_head(buffer, 3, count_of(text.len()));
                buffer.extend_from_slice(text.as_bytes());
            }
            Token::BytesStart => buffer.push(0x5f),
            Token::TextStart => buffer.push(0x7f),
            Token::ArrayStart(Some(count)) => write_head(buffer, 4, *count),
            Token::ArrayStart(None) => buffer.push(0x9f),
            Token::MapStart(Some(count)) => write_head(buffer, 5, *count),
            Token::MapStart(None) => buffer.push(0xbf),
            Token::Tag(number) => write_head(buffer, 6, *number),
            Token::Boolean(false) => buffer.push(0xf4),
            Token::Boolean(true) => buffer.push(0xf5),
            Token::Null => buffer.push(0xf6),
            Token::Undefined => buffer.push(0xf7),
            Token::Floating(number) => write_float(buffer, 7, *number),
            Token::Simple(simple) => {
                let value = simple.value();
                if value <= 23 {
                    buffer.push(0xe0 | value);
                } else {
                    buffer.extend_from_slice(&[0xf8, value]);
                }
            }
            Token::Break => buffer.push(BREAK),
        }
        Ok(())
    }

    /// The bytes written so far.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.buffer
    }

    /// Take the bytes written so far, leaving the encoder empty.
    pub fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.buffer)
    }

    /// Items open now.
    #[must_use]
    pub fn depth(&self) -> usize {
        self.nesting.frames.len()
    }

    /// Whether every item started has ended.
    #[must_use]
    pub fn is_at_boundary(&self) -> bool {
        self.nesting.frames.is_empty()
    }
}
