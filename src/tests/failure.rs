use crate::data_item::DataItem;
use crate::error::Error;
use crate::tests::hex_bytes;

/// Check that the provided `CBOR` bytes fail to decode with the expected error
fn failure_compare(hex_cbor: &str, error: Error) {
    assert_eq!(
        DataItem::decode(&hex_bytes(hex_cbor)),
        Err(error),
        "{hex_cbor}"
    );
}

#[test]
fn reserved_additional_information() {
    failure_compare(
        "1c",
        Error::InvalidAdditionalInfo {
            major_type: 0,
            additional: 28,
        },
    );
    failure_compare(
        "dd",
        Error::InvalidAdditionalInfo {
            major_type: 6,
            additional: 29,
        },
    );
    failure_compare(
        "9fde",
        Error::InvalidAdditionalInfo {
            major_type: 6,
            additional: 30,
        },
    );
    failure_compare(
        "bf3e",
        Error::InvalidAdditionalInfo {
            major_type: 1,
            additional: 30,
        },
    );
    failure_compare(
        "bffc",
        Error::InvalidAdditionalInfo {
            major_type: 7,
            additional: 28,
        },
    );
    // an integer cannot use the indefinite length marker
    failure_compare(
        "3f",
        Error::InvalidAdditionalInfo {
            major_type: 1,
            additional: 31,
        },
    );
}

#[test]
fn invalid_chunk() {
    failure_compare(
        "7f14",
        Error::InvalidChunkMajorType {
            expected: 3,
            found: 0,
        },
    );
    failure_compare(
        "5f87",
        Error::InvalidChunkMajorType {
            expected: 2,
            found: 4,
        },
    );
    failure_compare(
        "5fc000ff",
        Error::InvalidChunkMajorType {
            expected: 2,
            found: 6,
        },
    );
}

#[test]
fn truncated() {
    failure_compare("1a0102", Error::UnexpectedEnd { missing: 2 });
    failure_compare(
        "5affffffff00",
        Error::UnexpectedEnd {
            missing: 4_294_967_294,
        },
    );
    failure_compare("a2000000", Error::UnexpectedEnd { missing: 1 });
    failure_compare("f8", Error::UnexpectedEnd { missing: 1 });
    failure_compare("5f4100", Error::IncompleteIndefinite);
    failure_compare("9f819f819f9fffffff", Error::IncompleteIndefinite);
}

#[test]
fn invalid_break() {
    failure_compare("ff", Error::UnexpectedBreak);
    failure_compare("9f829f819f9fffffffff", Error::UnexpectedBreak);
    failure_compare("bf000000ff", Error::UnexpectedBreak);
}

#[test]
fn invalid_simple() {
    failure_compare("f801", Error::InvalidSimpleValue(1));
    assert!(DataItem::simple(24).is_err());
    assert!(DataItem::simple(31).is_err());
}

#[test]
fn trailing_and_duplicate() {
    failure_compare("0001", Error::TrailingBytes { count: 1 });
    failure_compare(
        "a201020103",
        Error::DuplicateMapKey(Box::new(DataItem::from(1))),
    );
    failure_compare(
        "bf01020103ff",
        Error::DuplicateMapKey(Box::new(DataItem::from(1))),
    );
}

#[test]
fn nesting_depth_limit() {
    let limit = crate::decode::Decoder::DEFAULT_MAX_NESTING_DEPTH;
    for head in [0x9f, 0x81, 0xc0] {
        assert_eq!(
            DataItem::decode(&vec![head; 1_000_000]),
            Err(Error::RecursionLimit { limit })
        );
    }
    let mut nested = vec![0x81; limit - 1];
    nested.push(0x01);
    assert!(DataItem::decode(&nested).is_ok());
}

#[test]
fn error_is_displayed_and_sourced() {
    use std::error::Error as _;

    let utf8_error = DataItem::decode(&hex_bytes("62c328")).unwrap_err();
    assert!(matches!(utf8_error, Error::InvalidUtf8(_)));
    let source = utf8_error.source().expect("utf8 error has a source");
    // the error displays the message of the utf8 error it wraps
    assert_eq!(utf8_error.to_string(), source.to_string());
    assert_eq!(
        Error::TrailingBytes { count: 2 }.to_string(),
        "2 extra bytes remain after a complete CBOR data item"
    );
    assert_eq!(
        Error::DuplicateMapKey(Box::new(DataItem::from("a"))).to_string(),
        "same map key \"a\" is repeated multiple times"
    );
}
