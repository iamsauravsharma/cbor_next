use crate::content::{Array, ByteString, Map, TextString};
use crate::data_item::DataItem;
use crate::deterministic::DeterministicMode;
use crate::encode::{Encoder, FloatMode, LengthMode, NanMode};
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

#[test]
fn shortest_float_width_matches_what_is_written() {
    use crate::encode::shortest_float_additional;

    let mut numbers = vec![
        0.0,
        -0.0,
        1.5,
        -1.5,
        65504.0,
        65536.0,
        100_000.0,
        1.1,
        1e10,
        f64::MAX,
        f64::MIN_POSITIVE,
        5.960_464_477_539_063e-8,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    // NaNs: sign and payload must survive
    for bits in [
        0x7ff8_0000_0000_0000_u64, // f16
        0xfff8_0000_0000_0000,     // negative, f16
        0x7ff8_0000_2000_0000,     // f32
        0xfff8_0000_2000_0000,     // negative, f32
        0x7ff8_0000_0000_0001,     // f64
        0xfff8_0000_0000_0001,     // negative, f64
    ] {
        numbers.push(f64::from_bits(bits));
    }
    for number in numbers {
        let item = DataItem::Floating(number);
        let encoded = item.encode();
        assert_eq!(
            shortest_float_additional(number),
            encoded[0] & 0b0001_1111,
            "{number:?}"
        );
        // round trip keeps every bit
        let decoded = DataItem::decode(&encoded).unwrap();
        assert_eq!(
            decoded.as_floating().map(f64::to_bits),
            Some(number.to_bits()),
            "{number:?}"
        );
    }
}

#[test]
fn section_4_2_2_choices_are_settings() {
    // every NaN as the single representation
    let canonical = Encoder::new().nan(NanMode::Canonical);
    for hex_cbor in ["f97e01", "f9fe00", "fa7f800001", "fb7ff8000000000001"] {
        let item = DataItem::decode(&hex_bytes(hex_cbor)).unwrap();
        assert_eq!(item.encode(), hex_bytes(hex_cbor), "{hex_cbor}");
        assert_eq!(canonical.encode(&item), hex_bytes("f97e00"), "{hex_cbor}");
    }
    // zeros with a positive sign
    let positive = Encoder::new().positive_zero(true);
    assert_eq!(DataItem::from(-0.0).encode(), hex_bytes("f98000"));
    assert_eq!(positive.encode(&DataItem::from(-0.0)), hex_bytes("f90000"));
    assert_eq!(positive.encode(&DataItem::from(-1.5)), hex_bytes("f9be00"));
    // integral numbers as integers
    let integral = Encoder::new().float(FloatMode::Integral);
    assert_eq!(integral.encode(&DataItem::from(1.0)), hex_bytes("01"));
    assert_eq!(integral.encode(&DataItem::from(-1.0)), hex_bytes("20"));
    assert_eq!(integral.encode(&DataItem::from(-0.0)), hex_bytes("00"));
    assert_eq!(integral.encode(&DataItem::from(1.5)), hex_bytes("f93e00"));
    assert_eq!(
        integral.encode(&DataItem::from(f64::INFINITY)),
        hex_bytes("f97c00")
    );
    assert_eq!(
        integral.encode(&DataItem::from(1e300)),
        hex_bytes("fb7e37e43c8800759c")
    );
    // 2^64 - 1 rounds to 2^64 as a double, which is past the integers
    assert_eq!(
        integral.encode(&DataItem::from(18_446_744_073_709_551_615.0)),
        hex_bytes("fa5f800000")
    );
    assert_eq!(
        integral.encode(&DataItem::from(18_446_744_073_709_549_568.0)),
        hex_bytes("1bfffffffffffff800")
    );
    assert_eq!(
        integral.encode(&DataItem::from(-9_223_372_036_854_775_808.0)),
        hex_bytes("3b7fffffffffffffff")
    );
    assert_eq!(
        integral.encode(&DataItem::from(-18_446_744_073_709_551_616.0)),
        hex_bytes("fadf800000")
    );
    // these choices still apply under a deterministic mode
    let strict = Encoder::new()
        .float(FloatMode::Integral)
        .nan(NanMode::Canonical)
        .positive_zero(true)
        .deterministic(DeterministicMode::Core);
    assert_eq!(strict.encode(&DataItem::from(2.0)), hex_bytes("02"));
    assert_eq!(
        strict.encode(&DataItem::from(f64::from_bits(0x7ff8_0000_0000_0001))),
        hex_bytes("f97e00")
    );
    assert_eq!(strict.encode(&DataItem::from(-0.5)), hex_bytes("f9b800"));
    // self-described, once per sequence and never twice
    let described = Encoder::new().self_described(true);
    assert_eq!(described.encode(&DataItem::from(1)), hex_bytes("d9d9f701"));
    assert_eq!(
        described.encode(&DataItem::tag(55799, 1)),
        hex_bytes("d9d9f701")
    );
    assert_eq!(
        described.encode_sequence(&[DataItem::from(1), DataItem::from(2)]),
        hex_bytes("d9d9f70102")
    );
    assert_eq!(described.encode_sequence(&[]), hex_bytes(""));
}
