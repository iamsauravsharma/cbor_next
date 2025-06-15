use std::fmt::Debug;
use std::ops::Deref;
use std::string::FromUtf8Error;

use indexmap::IndexMap;

use crate::DataItem;
use crate::error::Error;

/// Struct which holds a byte data
///
/// # Example
/// ```rust
/// use cbor_next::ByteContent
/// let mut content = ByteContent::default();
/// assert!(!content.is_indefinite());
/// content.set_indefinite(true);
/// assert!(content.is_indefinite());
/// ```
#[derive(Default, PartialEq, PartialOrd, Clone, Hash)]
pub struct ByteContent {
    is_indefinite: bool,
    bytes: Vec<Vec<u8>>,
}

impl From<Vec<u8>> for ByteContent {
    fn from(value: Vec<u8>) -> Self {
        Self {
            is_indefinite: false,
            bytes: vec![value],
        }
    }
}

impl ByteContent {
    /// Set a content as an indefinite content
    pub fn set_indefinite(&mut self, indefinite: bool) -> &mut Self {
        self.is_indefinite = indefinite;
        self
    }

    /// Set value of a content by overriding old data present inside content
    pub fn set_bytes(&mut self, byte: &[u8]) -> &mut Self {
        self.bytes = vec![byte.to_vec()];
        self
    }

    /// Append new data to a content without overriding old value
    pub fn add_bytes(&mut self, byte: &[u8]) -> &mut Self {
        self.bytes.push(byte.to_vec());
        self
    }

    /// Extend value by a byte
    pub fn extend_bytes(&mut self, byte: &[Vec<u8>]) -> &mut Self {
        self.bytes.extend(byte.to_vec());
        self
    }

    /// Get whether a byte content is indefinite or not
    #[must_use]
    pub fn is_indefinite(&self) -> bool {
        self.is_indefinite
    }

    /// Get full bytes from a byte content
    #[must_use]
    pub fn full(&self) -> Vec<u8> {
        self.bytes.concat()
    }

    /// Get chunk of  bytes from a byte content
    #[must_use]
    pub fn chunk(&self) -> &[Vec<u8>] {
        &self.bytes
    }
}

/// Struct which holds a text content
///
/// # Example
/// ```rust
/// use cbor_next::TextContent
/// let mut content = TextContent::default();
/// assert!(!content.is_indefinite());
/// content.set_indefinite(true);
/// assert!(content.is_indefinite());
/// ```
#[derive(Default, PartialEq, PartialOrd, Clone, Hash)]
pub struct TextContent {
    is_indefinite: bool,
    strings: Vec<String>,
}

impl From<String> for TextContent {
    fn from(value: String) -> Self {
        Self {
            is_indefinite: false,
            strings: vec![value],
        }
    }
}

impl From<&str> for TextContent {
    fn from(value: &str) -> Self {
        Self {
            is_indefinite: false,
            strings: vec![value.to_string()],
        }
    }
}

impl From<TextContent> for ByteContent {
    fn from(value: TextContent) -> Self {
        Self {
            is_indefinite: value.is_indefinite,
            bytes: value
                .strings
                .iter()
                .map(|m| m.as_bytes().to_vec())
                .collect(),
        }
    }
}

impl TryFrom<ByteContent> for TextContent {
    type Error = FromUtf8Error;

    fn try_from(value: ByteContent) -> Result<Self, Self::Error> {
        let mut text_content = TextContent::default();
        text_content.set_indefinite(value.is_indefinite);
        for chunk in value.chunk() {
            text_content.add_string(&String::from_utf8(chunk.clone())?);
        }
        Ok(text_content)
    }
}

impl TextContent {
    /// Set a content as an indefinite content
    pub fn set_indefinite(&mut self, indefinite: bool) -> &mut Self {
        self.is_indefinite = indefinite;
        self
    }

    /// Set value of a content by overriding old data present inside content
    pub fn set_string(&mut self, string: &str) -> &mut Self {
        self.strings = vec![string.to_string()];
        self
    }

    /// Append new data to a content without overriding old value
    pub fn add_string(&mut self, string: &str) -> &mut Self {
        self.strings.push(string.to_string());
        self
    }

    /// Get whether a string content is indefinite or not
    #[must_use]
    pub fn is_indefinite(&self) -> bool {
        self.is_indefinite
    }

    /// Get full strings from a string content
    #[must_use]
    pub fn full(&self) -> String {
        self.strings.join("")
    }

    /// Get chunk of  strings from a string content
    #[must_use]
    pub fn chunk(&self) -> &[String] {
        &self.strings
    }
}

/// Struct which holds a array content
///
/// # Example
/// ```rust
/// use cbor_next::ArrayContent
/// let mut content = ArrayContent::default();
/// assert!(!content.is_indefinite());
/// content.set_indefinite(true);
/// assert!(content.is_indefinite());
/// ```
#[derive(Default, PartialEq, Clone, Hash)]
pub struct ArrayContent {
    is_indefinite: bool,
    array: Vec<DataItem>,
}

impl From<Vec<DataItem>> for ArrayContent {
    fn from(value: Vec<DataItem>) -> Self {
        Self {
            is_indefinite: false,
            array: value,
        }
    }
}

impl ArrayContent {
    /// Set a content as an indefinite content
    pub fn set_indefinite(&mut self, indefinite: bool) -> &mut Self {
        self.is_indefinite = indefinite;
        self
    }

    /// Set value to a content by overriding old value
    pub fn set_content(&mut self, array: &[DataItem]) -> &mut Self {
        self.array = array.to_vec();
        self
    }

    /// Get whether a array content is indefinite or not
    #[must_use]
    pub fn is_indefinite(&self) -> bool {
        self.is_indefinite
    }

    /// Get array
    #[must_use]
    pub fn array(&self) -> &[DataItem] {
        &self.array
    }

    /// Get array as mut
    #[must_use]
    pub fn array_mut(&mut self) -> &mut [DataItem] {
        &mut self.array
    }
}

/// Struct which holds a map content
///
/// # Example
/// ```rust
/// use cbor_next::MapContent
/// let mut content = MapContent::default();
/// assert!(!content.is_indefinite());
/// content.set_indefinite(true);
/// assert!(content.is_indefinite());
/// ```
#[derive(Default, PartialEq, Clone)]
pub struct MapContent {
    is_indefinite: bool,
    map: IndexMap<DataItem, DataItem>,
}

impl From<IndexMap<DataItem, DataItem>> for MapContent {
    fn from(value: IndexMap<DataItem, DataItem>) -> Self {
        Self {
            is_indefinite: false,
            map: value,
        }
    }
}

impl MapContent {
    /// Set a content as an indefinite content
    pub fn set_indefinite(&mut self, indefinite: bool) -> &mut Self {
        self.is_indefinite = indefinite;
        self
    }

    /// Set value to a content by overriding old value
    pub fn set_content(&mut self, map: &IndexMap<DataItem, DataItem>) -> &mut Self {
        self.map.clone_from(map);
        self
    }

    /// Get whether a map content is indefinite or not
    #[must_use]
    pub fn is_indefinite(&self) -> bool {
        self.is_indefinite
    }

    /// Get map
    #[must_use]
    pub fn map(&self) -> &IndexMap<DataItem, DataItem> {
        &self.map
    }

    /// Get map as mut
    #[must_use]
    pub fn map_mut(&mut self) -> &mut IndexMap<DataItem, DataItem> {
        &mut self.map
    }
}

/// struct representing simple value which only allow number between 0-19 and
/// 32 -255
///
/// # Example
/// ```rust
/// use cbor_next::SimpleValue;
///
/// assert!(SimpleValue::try_from(10).is_ok());
/// assert!(SimpleValue::try_from(100).is_ok());
/// assert!(SimpleValue::try_from(255).is_ok());
/// assert!(SimpleValue::try_from(24).is_err());
/// assert!(SimpleValue::try_from(29).is_err());
/// ```
#[derive(PartialEq, Hash, Clone)]
pub struct SimpleValue(u8);

impl Deref for SimpleValue {
    type Target = u8;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Debug for SimpleValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "simple({})", self.0)
    }
}

impl TryFrom<u8> for SimpleValue {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0..=19 | 32..=u8::MAX => Ok(Self(value)),
            _ => Err(Error::InvalidSimple),
        }
    }
}
