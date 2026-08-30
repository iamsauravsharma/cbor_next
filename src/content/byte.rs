use std::fmt::{Debug, Formatter, Result as FmtResult, Write as _};

/// Content of a byte string, `CBOR` major type 2.
///
/// A byte string is either definite, in which case it is encoded as a single
/// run of bytes, or indefinite, in which case it is encoded as a sequence of
/// definite chunks terminated by a break. The chunk boundaries are preserved
/// so that a decoded item re-encodes to the exact same bytes, and they are
/// reached through [`chunks`](ByteString::chunks) and its mutable and owning
/// counterparts.
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
#[derive(Default, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub struct ByteString {
    is_indefinite: bool,
    chunks: Vec<Vec<u8>>,
}

impl ByteString {
    /// Create a definite length byte string from a single run of bytes.
    #[must_use]
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            is_indefinite: false,
            chunks: vec![bytes.into()],
        }
    }

    /// Create an indefinite length byte string from its chunks.
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

    /// Return the same content with the definite or indefinite encoding
    /// selected.
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

    /// Select the definite or indefinite encoding in place.
    pub fn set_indefinite(&mut self, indefinite: bool) {
        self.is_indefinite = indefinite;
    }

    /// Whether the byte string uses the indefinite length encoding.
    #[must_use]
    pub fn is_indefinite(&self) -> bool {
        self.is_indefinite
    }

    /// The chunks the byte string is made of.
    ///
    /// A definite length byte string is normally a single chunk.
    #[must_use]
    pub fn chunks(&self) -> &[Vec<u8>] {
        &self.chunks
    }

    /// The chunks the byte string is made of, mutably.
    #[must_use]
    pub fn chunks_mut(&mut self) -> &mut Vec<Vec<u8>> {
        &mut self.chunks
    }

    /// Take ownership of the chunks the byte string is made of.
    #[must_use]
    pub fn into_chunks(self) -> Vec<Vec<u8>> {
        self.chunks
    }

    /// All chunks concatenated into one buffer.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        self.chunks.concat()
    }

    /// Total number of bytes across every chunk.
    #[must_use]
    pub fn len(&self) -> usize {
        self.chunks.iter().map(Vec::len).sum()
    }

    /// Whether the byte string carries no bytes at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.chunks.iter().all(Vec::is_empty)
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

/// Write bytes using the `h'..'` byte string diagnostic notation.
fn write_hex(f: &mut Formatter<'_>, bytes: &[u8]) -> FmtResult {
    f.write_str("h'")?;
    for byte in bytes {
        write!(f, "{byte:02x}")?;
    }
    f.write_char('\'')
}
