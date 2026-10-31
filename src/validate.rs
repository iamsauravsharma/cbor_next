//! Tag validity (RFC 8949 section 5.3.2).

#[cfg(feature = "validation")]
use base64::alphabet;
#[cfg(feature = "validation")]
use base64::engine::general_purpose::GeneralPurpose;
#[cfg(feature = "validation")]
use base64::engine::{DecodePaddingMode, Engine as _, GeneralPurposeConfig};

use crate::content::Tag;
use crate::data_item::DataItem;
use crate::decode::{Decoder, DuplicateKeyMode};
use crate::error::Error;
use crate::tags;

/// How strictly registered tags are checked; see [`DataItem::validate`] and
/// [`Decoder::tag_validity`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum TagValidity {
    /// No check.
    #[default]
    Off,
    /// The content has the type the tag needs, as [`Tag::is_valid`] checks.
    Type,
    /// The content's value is right too, as [`Tag::has_valid_value`] checks.
    #[cfg(feature = "validation")]
    #[cfg_attr(docsrs, doc(cfg(feature = "validation")))]
    Value,
}

/// Check every tag in an item at the level.
pub(crate) fn validate(item: &DataItem, tag_validity: TagValidity) -> Result<(), Error> {
    match item {
        DataItem::Array(array) => {
            array
                .items()
                .iter()
                .try_for_each(|item| validate(item, tag_validity))
        }
        DataItem::Map(map) => {
            map.entries().iter().try_for_each(|(key, value)| {
                validate(key, tag_validity)?;
                validate(value, tag_validity)
            })
        }
        DataItem::Tag(tag) => {
            validate(tag.content(), tag_validity)?;
            if is_valid_at(tag, tag_validity, &Decoder::new()) {
                Ok(())
            } else {
                Err(Error::InvalidTagContent(Box::new(item.clone())))
            }
        }
        _ => Ok(()),
    }
}

/// Whether a tag passes the level; `decoder` reads tags 24 and 63.
pub(crate) fn is_valid_at(
    tag: &Tag,
    tag_validity: TagValidity,
    #[cfg_attr(
        not(feature = "validation"),
        expect(unused_variables, reason = "only the value level reads with it")
    )]
    decoder: &Decoder,
) -> bool {
    match tag_validity {
        TagValidity::Off => true,
        TagValidity::Type => has_valid_type(tag),
        #[cfg(feature = "validation")]
        TagValidity::Value => has_valid_value(tag, decoder),
    }
}

/// Whether the content has the type the tag needs.
pub(crate) fn has_valid_type(tag: &Tag) -> bool {
    let content = tag.content();
    match tag.number() {
        tags::DATE_TIME_STRING
        | tags::URI
        | tags::BASE64URL
        | tags::BASE64
        | tags::MIME_MESSAGE => content.is_text(),
        tags::EPOCH_TIME => content.is_integer() || content.is_floating(),
        tags::UNSIGNED_BIGNUM | tags::NEGATIVE_BIGNUM => content.is_byte(),
        tags::DECIMAL_FRACTION | tags::BIGFLOAT => is_exponent_and_mantissa(content),
        tags::EMBEDDED_ITEM | tags::EMBEDDED_SEQUENCE => holds_cbor(tag, &well_formed_decoder()),
        _ => true,
    }
}

/// Whether the content has the type and value the tag needs; `decoder` reads
/// tags 24 and 63, so their content meets its settings.
#[cfg(feature = "validation")]
pub(crate) fn has_valid_value(tag: &Tag, decoder: &Decoder) -> bool {
    let text = || tag.content().as_text().map(crate::TextString::to_text);
    match tag.number() {
        tags::EMBEDDED_ITEM | tags::EMBEDDED_SEQUENCE => holds_cbor(tag, decoder),
        tags::DATE_TIME_STRING => text().is_some_and(|text| is_date_time(&text)),
        tags::URI => text().is_some_and(|text| fluent_uri::UriRef::parse(text.as_str()).is_ok()),
        tags::BASE64URL => text().is_some_and(|text| is_base64(&text, true)),
        tags::BASE64 => text().is_some_and(|text| is_base64(&text, false)),
        _ => has_valid_type(tag),
    }
}

/// Whether tag 24 holds one item, or tag 63 a sequence, the decoder accepts.
fn holds_cbor(tag: &Tag, decoder: &Decoder) -> bool {
    let Some(bytes) = tag.content().as_byte().map(crate::ByteString::to_bytes) else {
        return false;
    };
    if tag.number() == tags::EMBEDDED_ITEM {
        decoder.decode(&bytes).is_ok()
    } else {
        decoder.sequence(&bytes).all(|item| item.is_ok())
    }
}

/// A decoder that only checks well-formedness (RFC 8949 section 3.4.5.1).
fn well_formed_decoder() -> Decoder {
    Decoder::new().duplicate_keys(DuplicateKeyMode::Last)
}

/// Whether an item is `[exponent, mantissa]` (tags 4 and 5).
fn is_exponent_and_mantissa(content: &DataItem) -> bool {
    content.as_array().is_some_and(|array| {
        matches!(
            array.items(),
            [exponent, mantissa] if exponent.is_integer() && is_integer_or_bignum(mantissa)
        )
    })
}

/// Whether an item is an integer or a valid bignum.
fn is_integer_or_bignum(item: &DataItem) -> bool {
    item.is_integer()
        || item.as_tag().is_some_and(|tag| {
            (tag.number() == tags::UNSIGNED_BIGNUM || tag.number() == tags::NEGATIVE_BIGNUM)
                && has_valid_type(tag)
        })
}

/// An RFC 3339 date and time with uppercase `T` and `Z` (RFC 4287).
#[cfg(feature = "validation")]
fn is_date_time(text: &str) -> bool {
    use time::OffsetDateTime;
    use time::format_description::well_known::Rfc3339;

    // the parser also takes a space and lowercase letters
    let uppercase = text.as_bytes().get(10) == Some(&b'T') && !text.contains(['t', 'z']);
    uppercase && OffsetDateTime::parse(text, &Rfc3339).is_ok()
}

/// Base64url without padding, or base64 with it (section 3.4.5.3).
#[cfg(feature = "validation")]
fn is_base64(text: &str, url: bool) -> bool {
    let (alphabet, padding) = if url {
        (&alphabet::URL_SAFE, DecodePaddingMode::RequireNone)
    } else {
        (&alphabet::STANDARD, DecodePaddingMode::RequireCanonical)
    };
    let config = GeneralPurposeConfig::new().with_decode_padding_mode(padding);
    GeneralPurpose::new(alphabet, config).decode(text).is_ok()
}
