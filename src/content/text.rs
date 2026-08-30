use std::fmt::{Debug, Formatter, Result as FmtResult, Write as _};
use std::string::FromUtf8Error;

use crate::content::ByteString;

/// Content of a UTF-8 text string, `CBOR` major type 3.
///
/// Like [`ByteString`], a text string is either definite or a sequence of
/// definite chunks terminated by a break. Every chunk of an indefinite text
/// string is independently valid UTF-8, so the chunk boundaries can be kept
/// without breaking the string apart. They are reached through
/// [`chunks`](TextString::chunks) and its mutable and owning counterparts.
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
#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub struct TextString {
    is_indefinite: bool,
    chunks: Vec<String>,
}

impl TextString {
    /// Create a definite length text string from a single string.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            is_indefinite: false,
            chunks: vec![text.into()],
        }
    }

    /// Create an indefinite length text string from its chunks.
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

    /// Return the same content with the definite or indefinite encoding
    /// selected.
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

    /// Select the definite or indefinite encoding in place.
    pub fn set_indefinite(&mut self, indefinite: bool) {
        self.is_indefinite = indefinite;
    }

    /// Whether the text string uses the indefinite length encoding.
    #[must_use]
    pub fn is_indefinite(&self) -> bool {
        self.is_indefinite
    }

    /// The chunks the text string is made of.
    ///
    /// A definite length text string is normally a single chunk.
    #[must_use]
    pub fn chunks(&self) -> &[String] {
        &self.chunks
    }

    /// The chunks the text string is made of, mutably.
    #[must_use]
    pub fn chunks_mut(&mut self) -> &mut Vec<String> {
        &mut self.chunks
    }

    /// Take ownership of the chunks the text string is made of.
    #[must_use]
    pub fn into_chunks(self) -> Vec<String> {
        self.chunks
    }

    /// All chunks concatenated into one string.
    #[must_use]
    pub fn to_text(&self) -> String {
        self.chunks.concat()
    }

    /// Total number of UTF-8 bytes across every chunk.
    #[must_use]
    pub fn len(&self) -> usize {
        self.chunks.iter().map(String::len).sum()
    }

    /// Whether the text string carries no character at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chunks.iter().all(String::is_empty)
    }
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
            f.write_str("(_ ")?;
            for (index, chunk) in self.chunks.iter().enumerate() {
                if index > 0 {
                    f.write_str(", ")?;
                }
                write!(f, "{chunk:?}")?;
            }
            f.write_char(')')
        } else {
            write!(f, "{:?}", self.to_text())
        }
    }
}
