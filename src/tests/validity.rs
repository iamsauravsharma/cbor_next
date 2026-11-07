use crate::content::Tag;
use crate::data_item::DataItem;
use crate::decode::Decoder;
#[cfg(feature = "validation")]
use crate::decode::DuplicateKeyMode;
use crate::error::Error;
use crate::tags;
use crate::tests::hex_bytes;
use crate::validate::TagValidity;

#[test]
fn registered_tags_check_their_content() {
    let text = DataItem::from("2013-03-21T20:04:00Z");
    for number in [
        tags::DATE_TIME_STRING,
        tags::URI,
        tags::BASE64URL,
        tags::BASE64,
        tags::MIME_MESSAGE,
    ] {
        assert!(Tag::new(number, text.clone()).is_valid(), "{number}");
        assert!(!Tag::new(number, 1).is_valid(), "{number}");
    }
    assert!(Tag::new(tags::EPOCH_TIME, 1).is_valid());
    assert!(Tag::new(tags::EPOCH_TIME, -1).is_valid());
    assert!(Tag::new(tags::EPOCH_TIME, 1.5).is_valid());
    assert!(!Tag::new(tags::EPOCH_TIME, text.clone()).is_valid());
    assert!(Tag::new(tags::UNSIGNED_BIGNUM, DataItem::bytes([1])).is_valid());
    assert!(!Tag::new(tags::NEGATIVE_BIGNUM, 1).is_valid());
    // an exponent and a mantissa, which may be a bignum
    let bignum = DataItem::from(1_u128 << 64);
    for number in [tags::DECIMAL_FRACTION, tags::BIGFLOAT] {
        assert!(
            Tag::new(
                number,
                DataItem::array([DataItem::from(-2), DataItem::from(27)])
            )
            .is_valid()
        );
        assert!(
            Tag::new(
                number,
                DataItem::array([DataItem::from(-2), bignum.clone()])
            )
            .is_valid()
        );
        assert!(
            !Tag::new(
                number,
                DataItem::array([bignum.clone(), DataItem::from(27)])
            )
            .is_valid()
        );
        assert!(!Tag::new(number, DataItem::array([DataItem::from(-2)])).is_valid());
        assert!(
            !Tag::new(
                number,
                DataItem::array([DataItem::from(-2), DataItem::from(1.5)])
            )
            .is_valid()
        );
        assert!(
            !Tag::new(
                number,
                DataItem::array([DataItem::from(-2), DataItem::tag(2, 1)])
            )
            .is_valid()
        );
    }
    // embedded bytes must be well formed, but need not be valid
    assert!(
        Tag::new(
            tags::EMBEDDED_ITEM,
            DataItem::bytes(hex_bytes("a201020103"))
        )
        .is_valid()
    );
    assert!(!Tag::new(tags::EMBEDDED_ITEM, DataItem::bytes(hex_bytes("0102"))).is_valid());
    assert!(!Tag::new(tags::EMBEDDED_ITEM, DataItem::bytes(hex_bytes("18"))).is_valid());
    assert!(!Tag::new(tags::EMBEDDED_ITEM, 1).is_valid());
    assert!(Tag::new(tags::EMBEDDED_SEQUENCE, DataItem::bytes(hex_bytes("0102"))).is_valid());
    assert!(Tag::new(tags::EMBEDDED_SEQUENCE, DataItem::bytes([])).is_valid());
    assert!(!Tag::new(tags::EMBEDDED_SEQUENCE, DataItem::bytes(hex_bytes("0118"))).is_valid());
    // any content goes for the rest
    assert!(Tag::new(tags::SELF_DESCRIBED, 1).is_valid());
    assert!(Tag::new(tags::EXPECTED_BASE64, 1).is_valid());
    assert!(Tag::new(1_000, DataItem::Undefined).is_valid());
}

#[test]
fn tag_validity_is_checked_when_asked() {
    let strict = Decoder::new().tag_validity(TagValidity::Type);
    // tag 1 around text, nested inside an array
    let bytes = hex_bytes("81c16161");
    assert!(DataItem::decode(&bytes).is_ok());
    assert_eq!(
        strict.decode(&bytes),
        Err(Error::InvalidTagContent(Box::new(DataItem::tag(1, "a"))))
    );
    // tag 24 whose bytes are not well formed
    assert!(matches!(
        strict.decode(&hex_bytes("d8184118")),
        Err(Error::InvalidTagContent(_))
    ));
    // valid content and unregistered numbers pass
    assert!(strict.decode(&hex_bytes("c11a514b67b0")).is_ok());
    assert!(strict.decode(&hex_bytes("d903e76161")).is_ok());

    // an item checks itself the same way, map keys included
    let item = DataItem::map([(DataItem::tag(1, "a"), 0)]);
    assert!(item.validate(TagValidity::Off).is_ok());
    assert_eq!(
        item.validate(TagValidity::Type),
        Err(Error::InvalidTagContent(Box::new(DataItem::tag(1, "a"))))
    );
    #[cfg(feature = "validation")]
    {
        let uri = DataItem::array([DataItem::tag(tags::URI, "a b")]);
        assert!(uri.validate(TagValidity::Type).is_ok());
        assert!(uri.validate(TagValidity::Value).is_err());
        // the decoder stops on a bad value like on any other error
        assert!(matches!(
            Decoder::new()
                .tag_validity(TagValidity::Value)
                .decode(&hex_bytes("82c0616101")),
            Err(Error::InvalidTagContent(_))
        ));
    }
}

#[cfg(feature = "validation")]
#[test]
fn date_time_values_follow_rfc_3339() {
    let valid = [
        "2013-03-21T20:04:00Z",
        "2013-03-21T20:04:00.5Z",
        "2013-03-21T20:04:00.123456789+01:00",
        "2013-03-21T20:04:00-23:59",
        "2016-12-31T23:59:60Z",
        "2000-02-29T00:00:00Z",
        "2024-02-29T00:00:00Z",
    ];
    for text in valid {
        assert!(
            Tag::new(tags::DATE_TIME_STRING, text).has_valid_value(),
            "{text}"
        );
    }
    let invalid = [
        "yesterday",
        "2013-03-21t20:04:00Z",
        "2013-03-21T20:04:00z",
        "2013-03-21T20:04:00",
        "2013-03-21T20:04:00.Z",
        "2013-03-21 20:04:00Z",
        "2013-13-21T20:04:00Z",
        "2013-02-29T20:04:00Z",
        "1900-02-29T20:04:00Z",
        "2013-04-31T20:04:00Z",
        "2013-03-21T24:04:00Z",
        "2013-03-21T20:60:00Z",
        "2013-03-21T20:04:61Z",
        "2013-03-21T20:04:00+24:00",
        "2013-03-21T20:04:00+01",
        "",
    ];
    for text in invalid {
        assert!(
            !Tag::new(tags::DATE_TIME_STRING, text).has_valid_value(),
            "{text}"
        );
    }
    // the value level includes the type level
    assert!(!Tag::new(tags::DATE_TIME_STRING, 1).has_valid_value());
}

#[cfg(feature = "validation")]
#[test]
fn uri_values_follow_rfc_3986() {
    for text in [
        "http://www.example.com",
        "https://a.b/c/d?e=f&g#h",
        "urn:ietf:rfc:8949",
        "mailto:a@b",
        "/relative/path?q",
        "../up",
        "#fragment",
        "",
        "a%20b",
        "sch+eme-1.x:rest",
    ] {
        assert!(Tag::new(tags::URI, text).has_valid_value(), "{text}");
    }
    for text in [
        "a b",
        "a%2",
        "a%zz",
        "1scheme:rest",
        "sch_eme:rest",
        "a\u{fc}",
        "a\"b",
        "http://[::1",
        "http://a:port/",
    ] {
        assert!(!Tag::new(tags::URI, text).has_valid_value(), "{text}");
    }
}

#[cfg(feature = "validation")]
#[test]
fn base64_values_follow_section_3_4_5_3() {
    for text in ["", "Zg", "Zm8", "Zm9v", "-_9-"] {
        assert!(Tag::new(tags::BASE64URL, text).has_valid_value(), "{text}");
    }
    for text in ["Z", "Zg==", "Zm8=", "+/9+", "Zh", "Zm9"] {
        assert!(!Tag::new(tags::BASE64URL, text).has_valid_value(), "{text}");
    }
    for text in ["", "Zg==", "Zm8=", "Zm9v", "+/9+"] {
        assert!(Tag::new(tags::BASE64, text).has_valid_value(), "{text}");
    }
    for text in ["Z", "Zg", "Zm8", "-_9-", "Zh==", "Zm9=", "Zg===", "Zg==="] {
        assert!(!Tag::new(tags::BASE64, text).has_valid_value(), "{text}");
    }
    // a MIME message is not checked
    assert!(Tag::new(tags::MIME_MESSAGE, "anything").has_valid_value());
}

#[cfg(feature = "validation")]
#[test]
fn embedded_items_are_held_to_the_decoder_at_the_value_level() {
    // tag 24 around {1: 2, 1: 3}, well formed but not valid
    let bytes = hex_bytes("d81845a201020103");
    assert!(
        Decoder::new()
            .tag_validity(TagValidity::Type)
            .decode(&bytes)
            .is_ok()
    );
    assert!(matches!(
        Decoder::new()
            .tag_validity(TagValidity::Value)
            .decode(&bytes),
        Err(Error::InvalidTagContent(_))
    ));
    assert!(
        Decoder::new()
            .tag_validity(TagValidity::Value)
            .duplicate_keys(DuplicateKeyMode::Last)
            .decode(&bytes)
            .is_ok()
    );
    // the embedded item is held to the tag validity as well: tag 0 around 1
    let nested = hex_bytes("d81843c001");
    assert!(
        Decoder::new()
            .tag_validity(TagValidity::Type)
            .decode(&nested)
            .is_err()
    );
    // a sequence of tag 63 likewise
    let sequence = hex_bytes("d83f43c00101");
    assert!(
        Decoder::new()
            .tag_validity(TagValidity::Value)
            .decode(&sequence)
            .is_err()
    );
    assert!(
        Decoder::new()
            .tag_validity(TagValidity::Value)
            .decode(&hex_bytes("d83f420102"))
            .is_ok()
    );
    // the stand alone check uses the default decoder
    let tag = DataItem::decode(&bytes).unwrap();
    assert!(tag.as_tag().unwrap().is_valid());
    assert!(!tag.as_tag().unwrap().has_valid_value());
}

#[cfg(feature = "validation")]
#[test]
fn embedded_items_share_the_nesting_limit() {
    // tag 24 around tag 24 around ... around 1
    let mut bytes = vec![0x01];
    for _ in 0..10 {
        let mut wrapped = vec![0xd8, 0x18, 0x58, u8::try_from(bytes.len()).unwrap()];
        wrapped.extend(&bytes);
        bytes = wrapped;
    }
    let decoder = Decoder::new().tag_validity(TagValidity::Value);
    // ten tags and the item, as if nested directly
    assert!(decoder.clone().max_nesting_depth(11).decode(&bytes).is_ok());
    assert!(decoder.max_nesting_depth(10).decode(&bytes).is_err());
}

#[test]
fn rfc_8943_dates_have_types() {
    assert!(Tag::new(tags::DATE_STRING, "1940-10-09").is_valid());
    assert!(!Tag::new(tags::DATE_STRING, 1).is_valid());
    assert!(Tag::new(tags::EPOCH_DAYS, -10_676).is_valid());
    assert!(!Tag::new(tags::EPOCH_DAYS, 1.5).is_valid());
}

#[test]
fn rfc_9164_addresses_have_shapes() {
    let v4 = DataItem::bytes([192, 0, 2, 1]);
    let v6 = DataItem::bytes([0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
    assert!(Tag::new(tags::IPV4, v4.clone()).is_valid());
    assert!(Tag::new(tags::IPV6, v6.clone()).is_valid());
    assert!(!Tag::new(tags::IPV4, v6.clone()).is_valid());
    assert!(!Tag::new(tags::IPV6, v4.clone()).is_valid());
    // prefix: length then up to that many bytes
    assert!(
        Tag::new(
            tags::IPV4,
            DataItem::array([DataItem::from(24), DataItem::bytes([192, 0, 2])])
        )
        .is_valid()
    );
    assert!(
        Tag::new(
            tags::IPV6,
            DataItem::array([DataItem::from(128), DataItem::bytes([])])
        )
        .is_valid()
    );
    assert!(
        !Tag::new(
            tags::IPV4,
            DataItem::array([DataItem::from(33), DataItem::bytes([192])])
        )
        .is_valid()
    );
    assert!(
        !Tag::new(
            tags::IPV4,
            DataItem::array([DataItem::from(8), DataItem::bytes([1, 2, 3, 4, 5])])
        )
        .is_valid()
    );
    // interface: address, length or null, maybe a zone
    assert!(
        Tag::new(
            tags::IPV4,
            DataItem::array([v4.clone(), DataItem::from(24)])
        )
        .is_valid()
    );
    assert!(
        Tag::new(
            tags::IPV4,
            DataItem::array([v4.clone(), DataItem::Null, DataItem::from(42)])
        )
        .is_valid()
    );
    assert!(
        Tag::new(
            tags::IPV6,
            DataItem::array([v6.clone(), DataItem::from(64), DataItem::from("eth0")])
        )
        .is_valid()
    );
    assert!(
        !Tag::new(
            tags::IPV6,
            DataItem::array([v6.clone(), DataItem::from(129)])
        )
        .is_valid()
    );
    assert!(
        !Tag::new(
            tags::IPV4,
            DataItem::array([v4.clone(), DataItem::from(24), DataItem::from(1.5)])
        )
        .is_valid()
    );
    assert!(!Tag::new(tags::IPV4, DataItem::array([v4.clone()])).is_valid());
    assert!(!Tag::new(tags::IPV4, "192.0.2.1").is_valid());
}

#[cfg(feature = "validation")]
#[test]
fn rfc_8943_dates_have_values() {
    for text in ["1940-10-09", "2000-02-29", "9999-12-31"] {
        assert!(
            Tag::new(tags::DATE_STRING, text).has_valid_value(),
            "{text}"
        );
    }
    for text in [
        "1940-13-09",
        "1900-02-29",
        "1940-10-9",
        "1940-10-09T00:00:00Z",
        "yesterday",
    ] {
        assert!(
            !Tag::new(tags::DATE_STRING, text).has_valid_value(),
            "{text}"
        );
    }
}

#[cfg(feature = "validation")]
#[test]
fn rfc_9164_prefixes_have_clean_bits() {
    let prefix = |length: u64, bytes: &[u8]| {
        Tag::new(
            tags::IPV6,
            DataItem::array([DataItem::from(length), DataItem::bytes(bytes)]),
        )
        .has_valid_value()
    };
    assert!(prefix(44, &hex_bytes("20010db81230")));
    assert!(prefix(64, &hex_bytes("20010db8")));
    assert!(prefix(128, &[]));
    assert!(prefix(0, &[]));
    // bits past the length, a trailing zero byte, bytes past the length
    assert!(!prefix(44, &hex_bytes("20010db81233")));
    assert!(!prefix(44, &hex_bytes("20010db8123f")));
    assert!(!prefix(44, &hex_bytes("20010db8123012")));
    assert!(!prefix(64, &hex_bytes("20010db800")));
    assert!(!prefix(32, &hex_bytes("20010db80001")));
    // the other forms have no bit rule
    let address = DataItem::bytes([192, 0, 2, 1]);
    assert!(Tag::new(tags::IPV4, DataItem::array([address, DataItem::from(24)])).has_valid_value());
}
