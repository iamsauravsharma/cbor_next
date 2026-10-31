use crate::data_item::DataItem;
use crate::decode::Decoder;
use crate::tests::hex_bytes;

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
