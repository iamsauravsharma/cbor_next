use crate::content::{Array, ByteString, Map, Simple, Tag, TextString};
use crate::data_item::DataItem;
use crate::decode::Decoder;
use crate::deterministic;
use crate::deterministic::DeterministicMode;
use crate::encode::Encoder;
use crate::error::Error;

impl DataItem {
    /// Create a byte string item.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::bytes([0x01, 0x02]).encode(), [0x42, 0x01, 0x02]);
    /// ```
    #[must_use]
    pub fn bytes(bytes: impl Into<ByteString>) -> Self {
        Self::Byte(bytes.into())
    }

    /// Create a text string item.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::text("a").encode(), [0x61, 0x61]);
    /// ```
    #[must_use]
    pub fn text(text: impl Into<TextString>) -> Self {
        Self::Text(text.into())
    }

    /// Create an array item from a sequence of items.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(
    ///     DataItem::array([1, 2, 3]).encode(),
    ///     [0x83, 0x01, 0x02, 0x03]
    /// );
    /// ```
    #[must_use]
    pub fn array<I, T>(items: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Self>,
    {
        Self::Array(Array::from_items(items))
    }

    /// Create a map item from a sequence of key value pairs.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::map([(1, 2)]).encode(), [0xa1, 0x01, 0x02]);
    /// ```
    #[must_use]
    pub fn map<I, K, V>(entries: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<Self>,
        V: Into<Self>,
    {
        Self::Map(Map::from_entries(entries))
    }

    /// Create a tagged item.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::tag(1, 100).encode(), [0xc1, 0x18, 0x64]);
    /// ```
    #[must_use]
    pub fn tag(number: u64, content: impl Into<Self>) -> Self {
        Self::Tag(Tag::new(number, content))
    }

    /// Create a generic simple value item.
    ///
    /// # Errors
    /// If the number is between 20 and 31, which RFC 8949 either assigns to a
    /// dedicated data item or reserves
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::simple(255).unwrap().encode(), [0xf8, 0xff]);
    /// assert!(DataItem::simple(24).is_err());
    /// ```
    pub fn simple(value: u8) -> Result<Self, Error> {
        Ok(Self::GenericSimple(Simple::try_from(value)?))
    }

    /// Is a unsigned integer value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::from(20).is_unsigned_integer());
    /// ```
    #[must_use]
    pub fn is_unsigned_integer(&self) -> bool {
        matches!(self, Self::Unsigned(_))
    }

    /// Is a signed integer value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::from(-32).is_signed_integer());
    /// ```
    #[must_use]
    pub fn is_signed_integer(&self) -> bool {
        matches!(self, Self::Signed(_))
    }

    /// Is a integer? Can be both signed as well as unsigned
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::from(-32).is_integer());
    /// assert!(DataItem::from(31).is_integer());
    /// ```
    #[must_use]
    pub fn is_integer(&self) -> bool {
        matches!(self, Self::Unsigned(_) | Self::Signed(_))
    }

    /// Is a byte value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::bytes([65, 63, 62]).is_byte());
    /// ```
    #[must_use]
    pub fn is_byte(&self) -> bool {
        matches!(self, Self::Byte(_))
    }

    /// Is a text value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::from("example").is_text());
    /// ```
    #[must_use]
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Is a array value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::from(vec![12]).is_array());
    /// ```
    #[must_use]
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }

    /// Is a map value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::map([(12, "a")]).is_map());
    /// ```
    #[must_use]
    pub fn is_map(&self) -> bool {
        matches!(self, Self::Map(_))
    }

    /// Is a tag value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::tag(12, 20).is_tag());
    /// ```
    #[must_use]
    pub fn is_tag(&self) -> bool {
        matches!(self, Self::Tag(_))
    }

    /// Is a boolean value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::from(false).is_boolean());
    /// ```
    #[must_use]
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Boolean(_))
    }

    /// Is a null value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::Null.is_null());
    /// ```
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Is a undefined value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::Undefined.is_undefined());
    /// ```
    #[must_use]
    pub fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }

    /// Is a floating value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::from(3.0).is_floating());
    /// ```
    #[must_use]
    pub fn is_floating(&self) -> bool {
        matches!(self, Self::Floating(_))
    }

    /// Is a simple value? Boolean, null and undefined are simple values as
    /// well
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::simple(45).unwrap().is_simple());
    /// assert!(DataItem::Null.is_simple());
    /// ```
    #[must_use]
    pub fn is_simple(&self) -> bool {
        matches!(
            self,
            Self::GenericSimple(_) | Self::Boolean(_) | Self::Null | Self::Undefined
        )
    }

    /// Is a generic simple value?
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert!(DataItem::simple(45).unwrap().is_generic_simple());
    /// assert!(!DataItem::Null.is_generic_simple());
    /// ```
    #[must_use]
    pub fn is_generic_simple(&self) -> bool {
        matches!(self, Self::GenericSimple(_))
    }

    /// Is an item which is encoded with an indefinite length?
    ///
    /// Only byte strings, text strings, arrays and maps can use the indefinite
    /// length encoding, so every other item answers `false`.
    ///
    /// # Example
    /// ```
    /// use cbor_next::{Array, DataItem};
    ///
    /// let item = DataItem::Array(Array::from_items([1]).with_indefinite(true));
    /// assert!(item.is_indefinite());
    /// assert!(!DataItem::from(vec![1]).is_indefinite());
    /// ```
    #[must_use]
    pub fn is_indefinite(&self) -> bool {
        match self {
            Self::Byte(byte_content) => byte_content.is_indefinite(),
            Self::Text(text_content) => text_content.is_indefinite(),
            Self::Array(array_content) => array_content.is_indefinite(),
            Self::Map(map_content) => map_content.is_indefinite(),
            _ => false,
        }
    }

    /// Get as unsigned number
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::from(20).as_unsigned(), Some(20));
    /// ```
    #[must_use]
    pub fn as_unsigned(&self) -> Option<u64> {
        match self {
            Self::Unsigned(num) => Some(*num),
            _ => None,
        }
    }

    /// Get as signed number. This will always return negative number
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::from(-21).as_signed(), Some(-21));
    /// ```
    #[must_use]
    pub fn as_signed(&self) -> Option<i128> {
        match self {
            Self::Signed(num) => Some(-(i128::from(*num) + 1)),
            _ => None,
        }
    }

    /// Get as integer which can be both signed or unsigned
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::from(-21).as_integer(), Some(-21));
    /// assert_eq!(DataItem::from(345).as_integer(), Some(345));
    /// ```
    #[must_use]
    pub fn as_integer(&self) -> Option<i128> {
        match self {
            Self::Unsigned(num) => Some(i128::from(*num)),
            Self::Signed(num) => Some(-(i128::from(*num) + 1)),
            _ => None,
        }
    }

    /// Get as byte string content
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// let item = DataItem::bytes([0x01, 0x02]);
    /// assert_eq!(item.as_byte().unwrap().to_bytes(), [0x01, 0x02]);
    /// ```
    #[must_use]
    pub fn as_byte(&self) -> Option<&ByteString> {
        match self {
            Self::Byte(byte_string) => Some(byte_string),
            _ => None,
        }
    }

    /// Get as mutable byte string content
    #[must_use]
    pub fn as_byte_mut(&mut self) -> Option<&mut ByteString> {
        match self {
            Self::Byte(byte_string) => Some(byte_string),
            _ => None,
        }
    }

    /// Get as text string content
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::from("cbor").as_text().unwrap().to_text(), "cbor");
    /// ```
    #[must_use]
    pub fn as_text(&self) -> Option<&TextString> {
        match self {
            Self::Text(text_string) => Some(text_string),
            _ => None,
        }
    }

    /// Get as mutable text string content
    #[must_use]
    pub fn as_text_mut(&mut self) -> Option<&mut TextString> {
        match self {
            Self::Text(text_string) => Some(text_string),
            _ => None,
        }
    }

    /// Get as array content
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// let item = DataItem::from(vec![12u64]);
    /// assert_eq!(item.as_array().unwrap().items(), [12.into()]);
    /// ```
    #[must_use]
    pub fn as_array(&self) -> Option<&Array> {
        match self {
            Self::Array(array) => Some(array),
            _ => None,
        }
    }

    /// Get as mutable array content
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// let mut item = DataItem::from(vec![12u64]);
    /// item.as_array_mut().unwrap().extend([13]);
    /// assert_eq!(item, DataItem::from(vec![12u64, 13]));
    /// ```
    #[must_use]
    pub fn as_array_mut(&mut self) -> Option<&mut Array> {
        match self {
            Self::Array(array) => Some(array),
            _ => None,
        }
    }

    /// Get as map content
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// let item = DataItem::map([("a", 1)]);
    /// assert_eq!(item.as_map().unwrap().entries().len(), 1);
    /// ```
    #[must_use]
    pub fn as_map(&self) -> Option<&Map> {
        match self {
            Self::Map(map) => Some(map),
            _ => None,
        }
    }

    /// Get as mutable map content
    #[must_use]
    pub fn as_map_mut(&mut self) -> Option<&mut Map> {
        match self {
            Self::Map(map) => Some(map),
            _ => None,
        }
    }

    /// Get as tag content
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// let item = DataItem::tag(20, -21);
    /// assert_eq!(item.as_tag().unwrap().number(), 20);
    /// assert_eq!(item.as_tag().unwrap().content(), &DataItem::Signed(20));
    /// ```
    #[must_use]
    pub fn as_tag(&self) -> Option<&Tag> {
        match self {
            Self::Tag(tag) => Some(tag),
            _ => None,
        }
    }

    /// Get as mutable tag content
    #[must_use]
    pub fn as_tag_mut(&mut self) -> Option<&mut Tag> {
        match self {
            Self::Tag(tag) => Some(tag),
            _ => None,
        }
    }

    /// Get as boolean value
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::from(true).as_boolean(), Some(true));
    /// ```
    #[must_use]
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(bool_val) => Some(*bool_val),
            _ => None,
        }
    }

    /// Get as floating number
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::from(-20.0).as_floating(), Some(-20.0));
    /// ```
    #[must_use]
    pub fn as_floating(&self) -> Option<f64> {
        match self {
            Self::Floating(num) => Some(*num),
            _ => None,
        }
    }

    /// Get as simple value number, including the numbers assigned to boolean,
    /// null and undefined
    ///
    /// # Example
    /// ```
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::simple(10).unwrap().as_simple(), Some(10));
    /// assert_eq!(DataItem::Null.as_simple(), Some(22));
    /// ```
    #[must_use]
    pub fn as_simple(&self) -> Option<u8> {
        match self {
            Self::GenericSimple(num) => Some(num.value()),
            Self::Boolean(false) => Some(20),
            Self::Boolean(true) => Some(21),
            Self::Null => Some(22),
            Self::Undefined => Some(23),
            _ => None,
        }
    }

    /// Peel away every layer of tag and get the item they apply to
    ///
    /// An item which is not tagged is returned as it is, so this is also a
    /// convenient way to inspect a value whose tagging is optional.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::DataItem;
    ///
    /// let tagged = DataItem::tag(20, DataItem::tag(30, "abc"));
    /// assert!(tagged.untagged().is_text());
    /// assert_eq!(DataItem::from(1).untagged(), &DataItem::from(1));
    /// ```
    #[must_use]
    pub fn untagged(&self) -> &Self {
        let mut item = self;
        while let Self::Tag(tag_content) = item {
            item = tag_content.content();
        }
        item
    }

    /// Peel away every layer of tag and get the item they apply to, mutably
    #[must_use]
    pub fn untagged_mut(&mut self) -> &mut Self {
        let mut item = self;
        while let Self::Tag(tag_content) = item {
            item = tag_content.content_mut();
        }
        item
    }

    /// Get the numbers of every tag applied to the item, outermost first
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::DataItem;
    ///
    /// let tagged = DataItem::tag(20, DataItem::tag(30, -21));
    /// assert_eq!(tagged.tag_numbers(), vec![20, 30]);
    /// assert!(DataItem::from(21).tag_numbers().is_empty());
    /// ```
    #[must_use]
    pub fn tag_numbers(&self) -> Vec<u64> {
        let mut numbers = vec![];
        let mut item = self;
        while let Self::Tag(tag_content) = item {
            numbers.push(tag_content.number());
            item = tag_content.content();
        }
        numbers
    }

    /// Get a major type of a value
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::DataItem;
    ///
    /// assert_eq!(DataItem::from("cbor").major_type(), 3);
    /// ```
    #[must_use]
    pub fn major_type(&self) -> u8 {
        match self {
            Self::Unsigned(_) => 0,
            Self::Signed(_) => 1,
            Self::Byte(_) => 2,
            Self::Text(_) => 3,
            Self::Array(_) => 4,
            Self::Map(_) => 5,
            Self::Tag(..) => 6,
            Self::Boolean(_)
            | Self::Null
            | Self::Undefined
            | Self::Floating(_)
            | Self::GenericSimple(_) => 7,
        }
    }

    /// Get a `CBOR` encoded representation of value
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::DataItem;
    ///
    /// let value = DataItem::Unsigned(10_000_000);
    /// let vector_data = vec![0x1a, 0x00, 0x98, 0x96, 0x80];
    /// assert_eq!(value.encode(), vector_data);
    /// ```
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        Encoder::new().encode(self)
    }

    /// Decode a `CBOR` representation to a value
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::DataItem;
    ///
    /// let vector_data = vec![0x1a, 0x00, 0x98, 0x96, 0x80];
    /// let value = DataItem::Unsigned(10_000_000);
    /// assert_eq!(DataItem::decode(&vector_data).unwrap(), value);
    /// ```
    ///
    /// # Errors
    /// If provided bytes cannot be converted to `CBOR`, if bytes remain after a
    /// complete data item, or if nesting exceeds the supported maximum depth.
    /// [`Decoder`] decodes with a different set of limits
    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        Decoder::new().decode(bytes)
    }

    /// Check current data item is in deterministic form
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::{DataItem, DeterministicMode};
    ///
    /// assert!(DataItem::map([(1, "a"), (2, "b")]).is_deterministic(DeterministicMode::Core));
    /// assert!(!DataItem::map([(2, "b"), (1, "a")]).is_deterministic(DeterministicMode::Core));
    /// ```
    #[must_use]
    pub fn is_deterministic(&self, mode: DeterministicMode) -> bool {
        deterministic::is_deterministic(self, mode)
    }

    /// Get a deterministic form of the value in the provided mode
    ///
    /// Map keys are sorted, and every indefinite length item is turned into its
    /// definite length counterpart.
    ///
    /// # Example
    /// ```rust
    /// use cbor_next::{DataItem, DeterministicMode};
    ///
    /// let value = DataItem::map([(2, "b"), (1, "a")]);
    /// assert_eq!(
    ///     value.into_deterministic(DeterministicMode::Core),
    ///     DataItem::map([(1, "a"), (2, "b")])
    /// );
    /// ```
    #[must_use]
    pub fn into_deterministic(self, mode: DeterministicMode) -> Self {
        deterministic::into_deterministic(self, mode)
    }
}
