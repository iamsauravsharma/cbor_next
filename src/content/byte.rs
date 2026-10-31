use std::borrow::Cow;
use std::cmp::Ordering;
use std::fmt::{Debug, Formatter, Result as FmtResult, Write as _};
use std::hash::{Hash, Hasher};

/// A byte string, major type 2.
///
/// Definite, or indefinite with its chunks kept so it re-encodes the same.
///
/// # Example
/// ```rust
/// use cbor_next::ByteString;
///
/// let definite = ByteString::new([0x01, 0x02]);
/// assert!(!definite.is_indefinite());
/// assert_eq!(definite.to_bytes(), [0x01, 0x02]);
///
/// let indefinite = ByteString::from_chunks([vec![0x01], vec![0x02]]);
/// assert!(indefinite.is_indefinite());
/// assert_eq!(indefinite.chunks().len(), 2);
/// assert_eq!(indefinite.to_bytes(), [0x01, 0x02]);
/// ```
///
/// Equality compares bytes only, not chunks.
#[derive(Default, Clone)]
pub struct ByteString {
    is_indefinite: bool,
    chunks: Vec<Vec<u8>>,
}

impl ByteString {
    /// Create a definite length byte string.
    #[must_use]
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            is_indefinite: false,
            chunks: vec![bytes.into()],
        }
    }

    /// Create an indefinite length byte string from chunks.
    #[must_use]
    pub fn from_chunks<I, B>(chunks: I) -> Self
    where
        I: IntoIterator<Item = B>,
        B: Into<Vec<u8>>,
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
    /// use cbor_next::ByteString;
    ///
    /// let content = ByteString::new([0x01]).with_indefinite(true);
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
    pub fn chunks(&self) -> &[Vec<u8>] {
        &self.chunks
    }

    /// The chunks, mutably.
    #[must_use]
    pub fn chunks_mut(&mut self) -> &mut Vec<Vec<u8>> {
        &mut self.chunks
    }

    /// Take the chunks.
    #[must_use]
    pub fn into_chunks(self) -> Vec<Vec<u8>> {
        self.chunks
    }

    /// All chunks joined.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.chunks.concat()
    }

    /// Number of bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.chunks.iter().map(Vec::len).sum()
    }

    /// Whether there are no bytes.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chunks.iter().all(Vec::is_empty)
    }
}

impl PartialEq for ByteString {
    fn eq(&self, other: &Self) -> bool {
        self.chunks
            .iter()
            .flatten()
            .eq(other.chunks.iter().flatten())
    }
}

impl Eq for ByteString {}

impl PartialOrd for ByteString {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ByteString {
    fn cmp(&self, other: &Self) -> Ordering {
        self.chunks
            .iter()
            .flatten()
            .cmp(other.chunks.iter().flatten())
    }
}

impl Hash for ByteString {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let bytes = match self.chunks.as_slice() {
            [chunk] => Cow::Borrowed(chunk.as_slice()),
            chunks => Cow::Owned(chunks.concat()),
        };
        state.write(&bytes);
    }
}

impl From<Vec<u8>> for ByteString {
    fn from(value: Vec<u8>) -> Self {
        Self::new(value)
    }
}

impl From<&[u8]> for ByteString {
    fn from(value: &[u8]) -> Self {
        Self::new(value)
    }
}

impl<const N: usize> From<[u8; N]> for ByteString {
    fn from(value: [u8; N]) -> Self {
        Self::new(value)
    }
}

impl Debug for ByteString {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        if self.is_indefinite {
            // RFC 8949 section 8.1: no chunks
            if self.chunks.is_empty() {
                return f.write_str("''_");
            }
            f.write_str("(_ ")?;
            for (index, chunk) in self.chunks.iter().enumerate() {
                if index > 0 {
                    f.write_str(", ")?;
                }
                write_hex(f, chunk)?;
            }
            f.write_char(')')
        } else {
            write_hex(f, &self.to_bytes())
        }
    }
}

/// Write bytes as `h'..'`.
fn write_hex(f: &mut Formatter<'_>, bytes: &[u8]) -> FmtResult {
    f.write_str("h'")?;
    for byte in bytes {
        write!(f, "{byte:02x}")?;
    }
    f.write_char('\'')
}
