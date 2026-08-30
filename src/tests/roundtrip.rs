use indexmap::IndexMap;

use crate::content::{Array, ByteString, Map, Tag, TextString};
use crate::data_item::DataItem;
use crate::decode::Decoder;
use crate::encode::Encoder;
use crate::tests::{compare_cbor_value, decode_compare, encode_compare, hex_bytes};

#[test]
fn integer() {
    compare_cbor_value("00", 0);
    compare_cbor_value("01", 1);
    compare_cbor_value("0a", 10);
    compare_cbor_value("17", 23);
    compare_cbor_value("1818", 24);
    compare_cbor_value("1819", 25);
    compare_cbor_value("1864", 100);
    compare_cbor_value("1903e8", 1000);
    compare_cbor_value("1a000f4240", 1_000_000);
    compare_cbor_value("1b000000e8d4a51000", 1_000_000_000_000u64);
    compare_cbor_value("1bffffffffffffffff", 18_446_744_073_709_551_615u64);
    compare_cbor_value(
        "3bffffffffffffffff",
        DataItem::try_from(-18_446_744_073_709_551_616_i128).unwrap(),
    );
    compare_cbor_value("20", -1);
    compare_cbor_value("29", -10);
    compare_cbor_value("3863", -100);
    compare_cbor_value("3903e7", -1000);
}

#[test]
fn float() {
    compare_cbor_value("f90000", 0.0);
    compare_cbor_value("f98000", -0.0);
    compare_cbor_value("f93c00", 1.0);
    compare_cbor_value("fb3ff199999999999a", 1.1);
    compare_cbor_value("f93e00", 1.5);
    compare_cbor_value("f97bff", 65504.0);
    compare_cbor_value("fa47c35000", 100_000.0);
    compare_cbor_value("f90400", 6.103_515_625e-05);
    compare_cbor_value("f90001", 5.960_464_477_539_063e-08);
    compare_cbor_value("fa7f7fffff", 3.402_823_466_385_288_6e+38);
    compare_cbor_value("fb7e37e43c8800759c", 1.0e+300);
    compare_cbor_value("f9c400", -4.0);
    compare_cbor_value("fbc010666666666666", -4.1);
    compare_cbor_value("f97c00", f64::INFINITY);
    compare_cbor_value("f9fc00", f64::NEG_INFINITY);
    decode_compare("fa7f800000", f64::INFINITY);
    decode_compare("faff800000", f64::NEG_INFINITY);
    decode_compare("fb7ff0000000000000", f64::INFINITY);
    decode_compare("fbfff0000000000000", f64::NEG_INFINITY);
    encode_compare("f97e00", f64::NAN);
    encode_compare("f97e00", f64::from_bits(0x7ff8_0000_0000_0000));
    // NaN in all three widths from RFC 8949 appendix A must decode to NaN
    for hex_nan in ["f97e00", "fa7fc00000", "fb7ff8000000000000"] {
        let value = DataItem::decode(&hex_bytes(hex_nan))
            .unwrap_or_else(|err| panic!("{err} failed to decode value {hex_nan}"));
        assert!(
            value.as_floating().is_some_and(f64::is_nan),
            "{hex_nan} should decode to NaN"
        );
        assert_eq!(value.encode(), hex_bytes("f97e00"), "{hex_nan}");
    }
    encode_compare("fa7fc00002", f64::from_bits(0x7ff8_0000_4000_0000));
    encode_compare("fb7ff8000000000001", f64::from_bits(0x7ff8_0000_0000_0001));
    encode_compare("f9fe00", f64::from_bits(0xfff8_0000_0000_0000));
}

#[test]
fn simple() {
    compare_cbor_value("f4", false);
    compare_cbor_value("f5", true);
    compare_cbor_value("f6", DataItem::Null);
    compare_cbor_value("f7", DataItem::Undefined);
    compare_cbor_value("f0", DataItem::simple(16).unwrap());
    compare_cbor_value("f820", DataItem::simple(32).unwrap());
    compare_cbor_value("f8ff", DataItem::simple(255).unwrap());
}

#[test]
fn tag() {
    compare_cbor_value(
        "c074323031332d30332d32315432303a30343a30305a",
        Tag::new(0, "2013-03-21T20:04:00Z"),
    );
    compare_cbor_value("c11a514b67b0", Tag::new(1, 1_363_896_240));
    compare_cbor_value("c1fb41d452d9ec200000", Tag::new(1, 1_363_896_240.5));
    compare_cbor_value(
        "d74401020304",
        Tag::new(23, DataItem::bytes(hex_bytes("01020304"))),
    );
    compare_cbor_value(
        "d818456449455446",
        Tag::new(24, DataItem::bytes(hex_bytes("6449455446"))),
    );
    compare_cbor_value(
        "d82076687474703a2f2f7777772e6578616d706c652e636f6d",
        Tag::new(32, "http://www.example.com"),
    );
    compare_cbor_value(
        "c249010000000000000000",
        Tag::new(2, DataItem::bytes(hex_bytes("010000000000000000"))),
    );
    compare_cbor_value(
        "c349010000000000000000",
        Tag::new(3, DataItem::bytes(hex_bytes("010000000000000000"))),
    );
    let nested = DataItem::tag(20, DataItem::tag(30, "abc"));
    assert_eq!(nested.tag_numbers(), vec![20, 30]);
    assert_eq!(nested.untagged(), &DataItem::from("abc"));
}

#[test]
fn byte() {
    compare_cbor_value("40", ByteString::new(vec![]));
    compare_cbor_value("4401020304", ByteString::new(hex_bytes("01020304")));
    compare_cbor_value(
        "5f42010243030405ff",
        ByteString::from_chunks([vec![0x01, 0x02], vec![0x03, 0x04, 0x05]]),
    );
    let content = ByteString::from_chunks([vec![0x01, 0x02], vec![0x03]]);
    assert_eq!(content.len(), 3);
    assert_eq!(content.to_bytes(), [0x01, 0x02, 0x03]);
    assert_eq!(content.chunks().len(), 2);
}

#[test]
fn text() {
    compare_cbor_value("60", "");
    compare_cbor_value("6161", "a");
    compare_cbor_value("6449455446", "IETF");
    compare_cbor_value("62225c", "\"\\");
    compare_cbor_value("62c3bc", "ü");
    compare_cbor_value("63e6b0b4", "水");
    compare_cbor_value("64f0908591", "𐅑");
    compare_cbor_value(
        "7f657374726561646d696e67ff",
        TextString::from_chunks(["strea", "ming"]),
    );
}

#[test]
fn array() {
    compare_cbor_value("80", Vec::<u64>::new());
    compare_cbor_value("83010203", vec![1, 2, 3]);
    compare_cbor_value::<Vec<DataItem>>(
        "8301820203820405",
        vec![1.into(), vec![2, 3].into(), vec![4, 5].into()],
    );
    compare_cbor_value(
        "98190102030405060708090a0b0c0d0e0f101112131415161718181819",
        vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25,
        ],
    );
    compare_cbor_value::<Vec<DataItem>>(
        "826161a161626163",
        vec!["a".into(), IndexMap::from_iter(vec![("b", "c")]).into()],
    );
    compare_cbor_value("9fff", Array::new().with_indefinite(true));
    compare_cbor_value::<Array>(
        "9f018202039f0405ffff",
        Array::from_items::<_, DataItem>([
            1.into(),
            vec![2, 3].into(),
            Array::from_items([4, 5]).with_indefinite(true).into(),
        ])
        .with_indefinite(true),
    );
    compare_cbor_value::<Array>(
        "9f01820203820405ff",
        Array::from_items::<_, DataItem>([1.into(), vec![2, 3].into(), vec![4, 5].into()])
            .with_indefinite(true),
    );
    compare_cbor_value(
        "9f0102030405060708090a0b0c0d0e0f101112131415161718181819ff",
        Array::from_items([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25,
        ])
        .with_indefinite(true),
    );
    compare_cbor_value::<Vec<DataItem>>(
        "83018202039f0405ff",
        vec![
            1.into(),
            vec![2, 3].into(),
            Array::from_items([4, 5]).with_indefinite(true).into(),
        ],
    );
    compare_cbor_value::<Vec<DataItem>>(
        "83019f0203ff820405",
        vec![
            1.into(),
            Array::from_items([2, 3]).with_indefinite(true).into(),
            vec![4, 5].into(),
        ],
    );
    decode_compare::<Vec<DataItem>>(
        "826161bf61626163ff",
        vec![
            "a".into(),
            Map::from_entries([("b", "c")]).with_indefinite(true).into(),
        ],
    );
}

#[test]
fn map() {
    compare_cbor_value("a0", Map::new());
    compare_cbor_value("a201020304", vec![(1, 2), (3, 4)]);
    compare_cbor_value(
        "a26161016162820203",
        vec![("a", DataItem::from(1)), ("b", vec![2, 3].into())],
    );
    compare_cbor_value(
        "a56161614161626142616361436164614461656145",
        vec![("a", "A"), ("b", "B"), ("c", "C"), ("d", "D"), ("e", "E")],
    );
    compare_cbor_value(
        "bf61610161629f0203ffff",
        Map::from_entries::<_, DataItem, DataItem>([
            ("a".into(), 1.into()),
            (
                "b".into(),
                Array::from_items([2, 3]).with_indefinite(true).into(),
            ),
        ])
        .with_indefinite(true),
    );
    compare_cbor_value(
        "bf6346756ef563416d7421ff",
        Map::from_entries([("Fun", DataItem::from(true)), ("Amt", DataItem::from(-2))])
            .with_indefinite(true),
    );
}

#[test]
fn sequence() {
    let items = vec![DataItem::from(1), DataItem::from("a"), DataItem::Null];
    let encoded = Encoder::new().encode_sequence(&items);
    assert_eq!(encoded, hex_bytes("016161f6"));
    assert_eq!(Decoder::new().decode_sequence(&encoded).unwrap(), items);

    let (first, rest) = Decoder::new().decode_partial(&encoded).unwrap();
    assert_eq!(first, DataItem::from(1));
    assert_eq!(rest, hex_bytes("6161f6"));
    assert_eq!(Decoder::new().decode_sequence(&[]).unwrap(), Vec::new());
}

#[test]
fn hash_matches_equality() {
    use std::hash::BuildHasher as _;

    let hasher_builder = std::hash::RandomState::new();
    let first_map = DataItem::from(vec![(1, 2), (3, 4)]);
    let second_map = DataItem::from(vec![(3, 4), (1, 2)]);
    assert_eq!(first_map, second_map);
    assert_eq!(
        hasher_builder.hash_one(&first_map),
        hasher_builder.hash_one(&second_map)
    );
    let positive_zero = DataItem::from(0.0);
    let negative_zero = DataItem::from(-0.0);
    assert_eq!(positive_zero, negative_zero);
    assert_eq!(
        hasher_builder.hash_one(&positive_zero),
        hasher_builder.hash_one(&negative_zero)
    );
}

#[test]
fn signed_minimum() {
    let minimum = DataItem::decode(&hex_bytes("3bffffffffffffffff")).unwrap();
    assert_eq!(minimum.as_signed(), Some(-18_446_744_073_709_551_616_i128));
    assert_eq!(minimum.as_integer(), Some(-18_446_744_073_709_551_616_i128));
    assert_eq!(format!("{minimum}"), "-18446744073709551616");
}
