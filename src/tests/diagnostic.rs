use crate::data_item::DataItem;
use crate::decode::Decoder;
use crate::tests::hex_bytes;

/// Check that the provided `CBOR` bytes decode to an item which is written in
/// the expected diagnostic notation
fn diagnostic_compare(diagnostic: &str, hex_cbor: &str) {
    let item = DataItem::decode(&hex_bytes(hex_cbor)).unwrap_or_else(|err| {
        panic!("{err} failed to decode value {hex_cbor}");
    });
    assert_eq!(format!("{item:?}"), diagnostic, "{hex_cbor}");
    // display and debug both write the diagnostic notation
    assert_eq!(format!("{item}"), diagnostic, "{hex_cbor}");
}

#[test]
fn diagnostic_notation() {
    diagnostic_compare("10", "0a");
    diagnostic_compare("-10", "29");
    diagnostic_compare("Infinity", "f97c00");
    diagnostic_compare("-Infinity", "f9fc00");
    diagnostic_compare("NaN", "fb7ff8000000000000");
    diagnostic_compare("true", "f5");
    diagnostic_compare("null", "f6");
    diagnostic_compare("undefined", "f7");
    diagnostic_compare("simple(255)", "f8ff");
    diagnostic_compare(
        "0(\"2013-03-21T20:04:00Z\")",
        "c074323031332d30332d32315432303a30343a30305a",
    );
    diagnostic_compare("1(1363896240.5)", "c1fb41d452d9ec200000");
    diagnostic_compare("24(h'6449455446')", "d818456449455446");
    diagnostic_compare(
        "32(\"http://www.example.com\")",
        "d82076687474703a2f2f7777772e6578616d706c652e636f6d",
    );
    diagnostic_compare("\"IETF\"", "6449455446");
    diagnostic_compare("\"𐅑\"", "64f0908591");
    diagnostic_compare("h''", "40");
    diagnostic_compare("[1, 2, 3]", "83010203");
    diagnostic_compare("[1, [2, 3], [4, 5]]", "8301820203820405");
    diagnostic_compare("{1: 2, 3: 4}", "a201020304");
    diagnostic_compare(
        "{\"a\": \"A\", \"b\": \"B\", \"c\": \"C\", \"d\": \"D\", \"e\": \"E\"}",
        "a56161614161626142616361436164614461656145",
    );
    diagnostic_compare("(_ h'0102', h'030405')", "5f42010243030405ff");
    diagnostic_compare("(_ \"strea\", \"ming\")", "7f657374726561646d696e67ff");
    diagnostic_compare("[_ ]", "9fff");
    diagnostic_compare("[_ 1, [2, 3], [_ 4, 5]]", "9f018202039f0405ffff");
    diagnostic_compare("[1, [_ 2, 3], [4, 5]]", "83019f0203ff820405");
    diagnostic_compare("{_ \"a\": 1, \"b\": [_ 2, 3]}", "bf61610161629f0203ffff");
    diagnostic_compare("[\"a\", {_ \"b\": \"c\"}]", "826161bf61626163ff");
}

#[test]
fn diagnostic_forms_outside_appendix_a() {
    // empty indefinite strings vs. one empty chunk
    assert_eq!(
        DataItem::decode(&hex_bytes("5fff")).unwrap().to_string(),
        "''_"
    );
    assert_eq!(
        DataItem::decode(&hex_bytes("7fff")).unwrap().to_string(),
        "\"\"_"
    );
    assert_eq!(
        DataItem::decode(&hex_bytes("5f40ff")).unwrap().to_string(),
        "(_ h'')"
    );
    assert_eq!(
        DataItem::decode(&hex_bytes("7f60ff")).unwrap().to_string(),
        "(_ \"\")"
    );
    // JSON escapes
    assert_eq!(DataItem::from("a\n\u{1}").to_string(), "\"a\\n\\u0001\"");
    // bignums print as integers; tag 2 around a non-byte is a plain tag
    assert_eq!(
        DataItem::decode(&hex_bytes("c2420005"))
            .unwrap()
            .to_string(),
        "5"
    );
    assert_eq!(
        DataItem::decode(&hex_bytes("c340")).unwrap().to_string(),
        "-1"
    );
    assert_eq!(
        DataItem::decode(&hex_bytes("c341ff")).unwrap().to_string(),
        "-256"
    );
    assert_eq!(
        DataItem::decode(&hex_bytes("c250ffffffffffffffffffffffffffffffff"))
            .unwrap()
            .to_string(),
        "340282366920938463463374607431768211455"
    );
    assert_eq!(DataItem::tag(2, 5).to_string(), "2(5)");
    // RFC 8742 sequence
    let items = Decoder::new()
        .decode_sequence(&hex_bytes("010203"))
        .unwrap();
    assert_eq!(DataItem::sequence_to_string(&items), "1, 2, 3");
}
