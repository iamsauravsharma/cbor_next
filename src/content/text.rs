use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter, Result as FmtResult, Write as _};
use std::hash::{Hash, Hasher};
use std::string::FromUtf8Error;

use crate::content::ByteString;

/// A UTF-8 text string, major type 3.
///
/// Definite, or indefinite with its chunks kept; each chunk is valid UTF-8.
///
/// # Example
/// ```rust
/// use cbor_next::TextString;
///
/// let definite = TextString::new("hello");
/// assert!(!definite.is_indefinite());
/// assert_eq!(definite.to_text(), "hello");
///
/// let indefinite = TextString::from_chunks(["hel", "lo"]);
/// assert!(indefinite.is_indefinite());
/// assert_eq!(indefinite.to_text(), "hello");
/// ```
///
/// Equality compares text only, not chunks.
#[derive(Default, Clone)]
pub struct TextString {
    is_indefinite: bool,
    chunks: Vec<String>,
}

impl TextString {
    /// Create a definite length text string.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            is_indefinite: false,
            chunks: vec![text.into()],
        }
    }

    /// Create an indefinite length text string from chunks.
    #[must_use]
    pub fn from_chunks<I, S>(chunks: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            is_indefinite: true,
            chunks: chunks.into_iter().map(Into::into).collect(),
        }
    }

    /// Set the length form, builder style.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::TextString;
    ///
    /// let content = TextString::new("hello").with_indefinite(true);
    /// assert!(content.is_indefinite());
    /// ```
    #[must_use]
    pub fn with_indefinite(mut self, indefinite: bool) -> Self {
        self.is_indefinite = indefinite;
        self
    }

    /// Set the length form.
    pub fn set_indefinite(&mut self, indefinite: bool) {
        self.is_indefinite = indefinite;
    }

    /// Whether the indefinite length form is used.
    #[must_use]
    pub fn is_indefinite(&self) -> bool {
        self.is_indefinite
    }

    /// The chunks; a definite string is normally one chunk.
    #[must_use]
    pub fn chunks(&self) -> &[String] {
        &self.chunks
    }

    /// The chunks, mutably.
    #[must_use]
    pub fn chunks_mut(&mut self) -> &mut Vec<String> {
        &mut self.chunks
    }

    /// Take the chunks.
    #[must_use]
    pub fn into_chunks(self) -> Vec<String> {
        self.chunks
    }

    /// The UTF-8 bytes of all chunks.
    fn bytes(&self) -> impl Iterator<Item = u8> + '_ {
        self.chunks.iter().flat_map(|chunk| chunk.bytes())
    }

    /// All chunks joined.
    #[must_use]
    pub fn to_text(&self) -> String {
        self.chunks.concat()
    }

    /// Number of UTF-8 bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.chunks.iter().map(String::len).sum()
    }

    /// Whether the text is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chunks.iter().all(String::is_empty)
    }
}

impl PartialEq for TextString {
    fn eq(&self, other: &Self) -> bool {
        self.bytes().eq(other.bytes())
    }
}

impl Eq for TextString {}

impl PartialOrd for TextString {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TextString {
    fn cmp(&self, other: &Self) -> Ordering {
        self.bytes().cmp(other.bytes())
    }
}

impl Hash for TextString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let text = match self.chunks.as_slice() {
            [chunk] => Cow::Borrowed(chunk.as_str()),
            chunks => Cow::Owned(chunks.concat()),
        };
        state.write(text.as_bytes());
    }
}

/// Write text as a JSON string, as diagnostic notation does.
pub(crate) fn write_json_string(f: &mut Formatter<'_>, text: &str) -> FmtResult {
    f.write_char('"')?;
    for character in text.chars() {
        match character {
            '"' => f.write_str("\\\"")?,
            '\\' => f.write_str("\\\\")?,
            '\n' => f.write_str("\\n")?,
            '\r' => f.write_str("\\r")?,
            '\t' => f.write_str("\\t")?,
            '\u{8}' => f.write_str("\\b")?,
            '\u{c}' => f.write_str("\\f")?,
            control if u32::from(control) < 0x20 => {
                write!(f, "\\u{:04x}", u32::from(control))?;
            }
            other => f.write_char(other)?,
        }
    }
    f.write_char('"')
}

impl From<String> for TextString {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for TextString {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<TextString> for ByteString {
    fn from(value: TextString) -> Self {
        Self::from_chunks(value.chunks.into_iter().map(String::into_bytes))
            .with_indefinite(value.is_indefinite)
    }
}

impl TryFrom<ByteString> for TextString {
    type Error = FromUtf8Error;

    fn try_from(value: ByteString) -> Result<Self, Self::Error> {
        let is_indefinite = value.is_indefinite();
        let mut chunks = Vec::with_capacity(value.chunks().len());
        for chunk in value.chunks() {
            chunks.push(String::from_utf8(chunk.clone())?);
        }
        Ok(Self {
            is_indefinite,
            chunks,
        })
    }
}

impl Debug for TextString {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.is_indefinite {
            // RFC 8949 section 8.1: no chunks
            if self.chunks.is_empty() {
                return f.write_str("\"\"_");
            }
            f.write_str("(_ ")?;
            for (index, chunk) in self.chunks.iter().enumerate() {
                if index > 0 {
                    f.write_str(", ")?;
                }
                write_json_string(f, chunk)?;
            }
            f.write_char(')')
        } else {
            write_json_string(f, &self.to_text())
        }
    }
}
