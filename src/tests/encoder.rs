use crate::content::{Array, ByteString, Map, TextString};
use crate::data_item::DataItem;
use crate::deterministic::DeterministicMode;
use crate::encode::{Encoder, FloatMode, LengthMode};
use crate::tests::hex_bytes;

#[test]
fn default_encoder_matches_encode_method() {
    let items = [
        DataItem::from(1),
        DataItem::from("cbor"),
        DataItem::Array(Array::from_items([1, 2]).with_indefinite(true)),
        DataItem::map([(1, 2)]),
    ];
    for item in items {
        assert_eq!(Encoder::new().encode(&item), item.encode());
    }
}

#[test]
fn definite_length_mode_rewrites_indefinite_items() {
    let encoder = Encoder::new().length(LengthMode::Definite);

    let array = DataItem::Array(Array::from_items([1, 2]).with_indefinite(true));
    assert_eq!(array.encode(), hex_bytes("9f0102ff"));
    assert_eq!(encoder.encode(&array), hex_bytes("820102"));

    let map = DataItem::Map(Map::from_entries([(1, 2)]).with_indefinite(true));
    assert_eq!(map.encode(), hex_bytes("bf0102ff"));
    assert_eq!(encoder.encode(&map), hex_bytes("a10102"));

    let byte_string = DataItem::bytes(ByteString::from_chunks([vec![0x01], vec![0x02]]));
    assert_eq!(byte_string.encode(), hex_bytes("5f41014102ff"));
    assert_eq!(encoder.encode(&byte_string), hex_bytes("420102"));

    let text_string = DataItem::text(TextString::from_chunks(["a", "b"]));
    assert_eq!(text_string.encode(), hex_bytes("7f61616162ff"));
    assert_eq!(encoder.encode(&text_string), hex_bytes("626162"));

    // a definite item is written the same way by both encoders
    let definite = DataItem::from(vec![1, 2]);
    assert_eq!(encoder.encode(&definite), definite.encode());
}

#[test]
fn float_mode_picks_the_width() {
    let item = DataItem::from(1.5);
    assert_eq!(Encoder::new().encode(&item), hex_bytes("f93e00"));
    assert_eq!(
        Encoder::new().float(FloatMode::Double).encode(&item),
        hex_bytes("fb3ff8000000000000")
    );
    // the shortest width is required by a deterministic encoding
    assert_eq!(
        Encoder::new()
            .float(FloatMode::Double)
            .deterministic(DeterministicMode::Core)
            .encode(&item),
        hex_bytes("f93e00")
    );
    // a wider number is unaffected by the mode
    let wide = DataItem::from(1.1);
    assert_eq!(
        Encoder::new().float(FloatMode::Double).encode(&wide),
        wide.encode()
    );
}

#[test]
fn deterministic_mode_sorts_and_forces_definite_length() {
    let item = DataItem::Map(
        Map::from_entries([
            (DataItem::from("b"), DataItem::from(2)),
            (
                DataItem::from("a"),
                DataItem::Array(Array::from_items([1]).with_indefinite(true)),
            ),
        ])
        .with_indefinite(true),
    );
    let encoded = Encoder::new()
        .deterministic(DeterministicMode::Core)
        .encode(&item);
    assert_eq!(
        encoded,
        DataItem::map([("a", DataItem::from(vec![1])), ("b", DataItem::from(2))]).encode()
    );
    // the value itself is left untouched
    assert!(item.is_indefinite());
}

#[test]
fn encode_into_reuses_a_buffer() {
    let mut buffer = vec![0xff];
    let encoder = Encoder::new();
    encoder.encode_into(&DataItem::from(1), &mut buffer);
    encoder.encode_into(&DataItem::from(2), &mut buffer);
    assert_eq!(buffer, hex_bytes("ff0102"));

    let items = [DataItem::from(1), DataItem::from(2)];
    let mut sequence = vec![];
    encoder.encode_sequence_into(&items, &mut sequence);
    assert_eq!(sequence, hex_bytes("0102"));
    assert_eq!(encoder.encode_sequence(&items), sequence);
}
