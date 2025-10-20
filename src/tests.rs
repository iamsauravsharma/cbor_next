#![expect(clippy::panic, reason = "allow panic in tests")]
use core::f64;
use std::vec;

use indexmap::IndexMap;
use rand::seq::SliceRandom as _;

use crate::content::{ArrayContent, ByteContent, MapContent, TagContent, TextContent};
use crate::data_item::DataItem;
use crate::deterministic::DeterministicMode;
use crate::error::Error;
use crate::index::Get as _;

fn encode_compare<I>(hex_cbor: &str, value_into: I)
where
    I: Into<DataItem>,
{
    let value = value_into.into();
    let vec_u8_cbor =
        hex::decode(hex_cbor).unwrap_or_else(|err| panic!("{err} failed to decode hex {hex_cbor}"));
    let value_to_cbor = value.encode();
    assert_eq!(value_to_cbor, vec_u8_cbor, "{hex_cbor}");
}

fn decode_compare<I>(hex_cbor: &str, value_into: I)
where
    I: Into<DataItem>,
{
    let value = value_into.into();
    let vec_u8_cbor =
        hex::decode(hex_cbor).unwrap_or_else(|_| panic!(" failed to decode hex {hex_cbor}"));
    let cbor_to_value = DataItem::decode(&vec_u8_cbor)
        .unwrap_or_else(|err: Error| panic!("{err} failed to decode value {hex_cbor}"));
    assert_eq!(&cbor_to_value, &value, "{hex_cbor}");
}

fn compare_cbor_value<I>(hex_cbor: &str, value_into: I)
where
    I: Into<DataItem>,
{
    let value = value_into.into();
    let vec_u8_cbor =
        hex::decode(hex_cbor).unwrap_or_else(|err| panic!("{err} failed to decode hex {hex_cbor}"));
    let value_to_cbor = value.encode();
    assert_eq!(value_to_cbor, vec_u8_cbor, "{hex_cbor}");
    let cbor_to_value = DataItem::decode(&vec_u8_cbor)
        .unwrap_or_else(|err| panic!("{err} failed to decode value {hex_cbor}"))
        .encode();
    assert_eq!(&cbor_to_value, &value_to_cbor, "{hex_cbor}");
}

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
    encode_compare("fb7ff8000000000000", f64::NAN);
}

#[test]
fn simple() {
    compare_cbor_value("f4", false);
    compare_cbor_value("f5", true);
    compare_cbor_value("f6", DataItem::Null);
    compare_cbor_value("f7", DataItem::Undefined);
    compare_cbor_value("f0", DataItem::GenericSimple(16.try_into().unwrap()));
    compare_cbor_value("f820", DataItem::GenericSimple(32.try_into().unwrap()));
    compare_cbor_value("f8ff", DataItem::GenericSimple(255.try_into().unwrap()));
}

#[test]
fn tag() {
    compare_cbor_value(
        "c074323031332d30332d32315432303a30343a30305a",
        TagContent::from((0, "2013-03-21T20:04:00Z")),
    );
    compare_cbor_value(
        "c074323031332d30332d32315432303a30343a30305a",
        TagContent::from((0, "2013-03-21T20:04:00Z")),
    );
    compare_cbor_value("c11a514b67b0", TagContent::from((1, 1_363_896_240)));
    compare_cbor_value(
        "c1fb41d452d9ec200000",
        TagContent::from((1, 1_363_896_240.5)),
    );
    compare_cbor_value(
        "d74401020304",
        TagContent::from((23, hex::decode("01020304").unwrap().as_slice())),
    );
    compare_cbor_value(
        "d818456449455446",
        TagContent::from((24, hex::decode("6449455446").unwrap().as_slice())),
    );
    compare_cbor_value(
        "d82076687474703a2f2f7777772e6578616d706c652e636f6d",
        TagContent::from((32, "http://www.example.com")),
    );
}

#[test]
fn byte() {
    compare_cbor_value("40", Vec::new().as_slice());
    compare_cbor_value("4401020304", hex::decode("01020304").unwrap().as_slice());
    compare_cbor_value(
        "5f42010243030405ff",
        DataItem::Byte(
            ByteContent::default()
                .set_indefinite(true)
                .push_bytes(&[0x01, 0x02])
                .push_bytes(&[0x03, 0x04, 0x05])
                .clone(),
        ),
    );
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
        DataItem::Text(
            TextContent::default()
                .set_indefinite(true)
                .push_string("strea")
                .push_string("ming")
                .clone(),
        ),
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
    decode_compare("9fff", ArrayContent::default().set_indefinite(true).clone());
    decode_compare(
        "9f018202039f0405ffff",
        ArrayContent::default()
            .set_indefinite(true)
            .set_content::<DataItem>(&[
                1.into(),
                vec![2, 3].into(),
                ArrayContent::default()
                    .set_indefinite(true)
                    .set_content(&[4, 5])
                    .clone()
                    .into(),
            ])
            .clone(),
    );
    decode_compare(
        "9f01820203820405ff",
        ArrayContent::default()
            .set_indefinite(true)
            .set_content::<DataItem>(&[1.into(), vec![2, 3].into(), vec![4, 5].into()])
            .clone(),
    );
    decode_compare::<Vec<DataItem>>(
        "83018202039f0405ff",
        vec![
            1.into(),
            vec![2, 3].into(),
            ArrayContent::default()
                .set_indefinite(true)
                .set_content(&[4, 5])
                .clone()
                .into(),
        ],
    );
    decode_compare::<Vec<DataItem>>(
        "83019f0203ff820405",
        vec![
            1.into(),
            ArrayContent::default()
                .set_indefinite(true)
                .set_content(&[2, 3])
                .clone()
                .into(),
            vec![4, 5].into(),
        ],
    );
    decode_compare::<Vec<DataItem>>(
        "826161bf61626163ff",
        vec![
            "a".into(),
            MapContent::default()
                .set_indefinite(true)
                .set_content(&[("b", "c")].into())
                .clone()
                .into(),
        ],
    );
}

#[test]
fn map() {
    compare_cbor_value(
        "a0",
        DataItem::Map(IndexMap::<DataItem, DataItem>::new().into()),
    );
    compare_cbor_value("a201020304", vec![(1, 2), (3, 4)]);
    compare_cbor_value(
        "a26161016162820203",
        vec![("a", DataItem::from(1)), ("b", vec![2, 3].into())],
    );
    compare_cbor_value(
        "a56161614161626142616361436164614461656145",
        vec![("a", "A"), ("b", "B"), ("c", "C"), ("d", "D"), ("e", "E")],
    );
    decode_compare(
        "bf61610161629f0203ffff",
        MapContent::default()
            .set_indefinite(true)
            .set_content::<DataItem, DataItem>(
                &[
                    ("a".into(), DataItem::from(1)),
                    (
                        "b".into(),
                        ArrayContent::default()
                            .set_indefinite(true)
                            .set_content(&[2, 3])
                            .clone()
                            .into(),
                    ),
                ]
                .into(),
            )
            .clone(),
    );
    decode_compare(
        "bf6346756ef563416d7421ff",
        MapContent::default()
            .set_indefinite(true)
            .set_content(&[("Fun", DataItem::from(true)), ("Amt", DataItem::from(-2))].into())
            .clone(),
    );
}

#[test]
fn failure() {
    assert_eq!(
        DataItem::decode(&hex::decode("1c").unwrap()),
        Err(Error::NotWellFormed(
            "invalid additional number 28".to_string()
        ))
    );
    assert_eq!(
        DataItem::decode(&hex::decode("7f14").unwrap()),
        Err(Error::NotWellFormed(
            "contains invalid major type 0 for indefinite major type 3".to_string()
        ))
    );
    assert_eq!(
        DataItem::decode(&hex::decode("f801").unwrap()),
        Err(Error::InvalidSimple)
    );
    assert_eq!(
        DataItem::decode(&hex::decode("9fde").unwrap()),
        Err(Error::NotWellFormed(
            "invalid additional number 30".to_string()
        ))
    );
    assert_eq!(
        DataItem::decode(&hex::decode("bf3e").unwrap()),
        Err(Error::NotWellFormed(
            "invalid additional number 30".to_string()
        ))
    );
    assert_eq!(
        DataItem::decode(&hex::decode("dd").unwrap()),
        Err(Error::NotWellFormed(
            "invalid additional number 29".to_string()
        ))
    );
    assert_eq!(
        DataItem::decode(&hex::decode("5f87").unwrap()),
        Err(Error::NotWellFormed(
            "contains invalid major type 4 for indefinite major type 2".to_string()
        ))
    );
    assert_eq!(
        DataItem::decode(&hex::decode("3f").unwrap()),
        Err(Error::NotWellFormed("failed to extract number".to_string()))
    );
    assert_eq!(
        DataItem::decode(&hex::decode("5f4100").unwrap()),
        Err(Error::IncompleteIndefinite)
    );
    assert_eq!(
        DataItem::decode(&hex::decode("5fc000ff").unwrap()),
        Err(Error::NotWellFormed(
            "contains invalid major type 6 for indefinite major type 2".to_string()
        ))
    );
    assert_eq!(
        DataItem::decode(&hex::decode("9f819f819f9fffffff").unwrap()),
        Err(Error::IncompleteIndefinite)
    );
    assert_eq!(
        DataItem::decode(&hex::decode("9f829f819f9fffffffff").unwrap()),
        Err(Error::InvalidBreakStop)
    );
    assert_eq!(
        DataItem::decode(&hex::decode("1a0102").unwrap()),
        Err(Error::NotWellFormed(
            "incomplete array of byte missing 2 byte".to_string()
        ))
    );
    assert_eq!(
        DataItem::decode(&hex::decode("5affffffff00").unwrap()),
        Err(Error::NotWellFormed(
            "incomplete array of byte missing 4294967294 byte".to_string()
        ))
    );
    assert_eq!(
        DataItem::decode(&hex::decode("bf000000ff").unwrap()),
        Err(Error::InvalidBreakStop)
    );
    assert_eq!(
        DataItem::decode(&hex::decode("a2000000").unwrap()),
        Err(Error::Incomplete)
    );
    assert_eq!(
        DataItem::decode(&hex::decode("bffc").unwrap()),
        Err(Error::NotWellFormed(
            "invalid value 28 for major type 7".to_string()
        ))
    );
    assert_eq!(
        DataItem::decode(&hex::decode("ff").unwrap()),
        Err(Error::InvalidBreakStop)
    );
}

#[test]
fn core_deterministic() {
    let key_value_vec = vec![
        (10.into(), "abc".into()),
        (100.into(), "1020".into()),
        (DataItem::from(-1), 12.into()),
        (DataItem::from("z"), "a".into()),
        (DataItem::from("aa"), DataItem::from(-1)),
        (
            DataItem::from(vec![100]),
            DataItem::from(vec![
                (1_000_000.into(), DataItem::from("1020")),
                (DataItem::from("z"), "a".into()),
                (DataItem::from("aa"), 12.into()),
            ]),
        ),
        (
            DataItem::from(vec![DataItem::from(-1)]),
            DataItem::from(vec!["cbor", "nano"]),
        ),
        (false.into(), 12.into()),
    ];
    let mut random_key_value = key_value_vec.clone();
    random_key_value.shuffle(&mut rand::rng());
    assert_ne!(key_value_vec, random_key_value);
    let random_data_item = DataItem::Map(IndexMap::from_iter(random_key_value).into());
    assert!(!random_data_item.is_deterministic(&DeterministicMode::Core));
    let deterministic = random_data_item.deterministic(&DeterministicMode::Core);
    assert!(deterministic.is_deterministic(&DeterministicMode::Core));
    assert_eq!(
        DataItem::Map(IndexMap::from_iter(key_value_vec).into()),
        deterministic
    );
}

#[test]
fn length_core_deterministic() {
    let key_value_vec = vec![
        (10.into(), "abc".into()),
        (100.into(), "1020".into()),
        (DataItem::from(-1), 12.into()),
        (DataItem::from("z"), "a".into()),
        (DataItem::from("aa"), DataItem::from(-1)),
        (
            DataItem::from(vec![100]),
            DataItem::from(vec![
                (1_000_000.into(), DataItem::from("1020")),
                (DataItem::from("z"), "a".into()),
                (DataItem::from("aa"), 12.into()),
            ]),
        ),
        (
            DataItem::from(vec![DataItem::from(-1)]),
            DataItem::from(vec!["cbor", "nano"]),
        ),
        (false.into(), 12.into()),
    ];
    let mut random_key_value = key_value_vec.clone();
    random_key_value.shuffle(&mut rand::rng());
    assert_ne!(key_value_vec, random_key_value);
    let random_data_item = DataItem::Map(IndexMap::from_iter(random_key_value).into());
    assert!(!random_data_item.is_deterministic(&DeterministicMode::LengthFirst));
    let deterministic = random_data_item.deterministic(&DeterministicMode::LengthFirst);
    assert!(deterministic.is_deterministic(&DeterministicMode::LengthFirst));
    assert_eq!(
        DataItem::Map(IndexMap::from_iter(key_value_vec).into()),
        deterministic
    );
}

#[test]
fn map_index_verification() {
    let key_value_vec = DataItem::Map(
        IndexMap::from_iter(vec![
            (10.into(), "abc".into()),
            (100.into(), "1020".into()),
            (DataItem::from(-1), 12.into()),
            (DataItem::from("z"), "a".into()),
            (DataItem::from("aa"), DataItem::from(-1)),
            (
                DataItem::from(vec![100]),
                DataItem::from(vec![
                    (1_000_000.into(), DataItem::from("1020")),
                    (DataItem::from("z"), "a".into()),
                    (DataItem::from("aa"), 12.into()),
                ]),
            ),
            (
                DataItem::from(vec![DataItem::from(-1)]),
                DataItem::from(vec!["cbor", "nano"]),
            ),
            (false.into(), 12.into()),
        ])
        .into(),
    );
    assert_eq!(key_value_vec[DataItem::from(10)], "abc".into());
    assert_eq!(key_value_vec[DataItem::from(-1)], 12.into());
    assert_eq!(
        key_value_vec[DataItem::from(vec![100])][DataItem::from("z")],
        "a".into()
    );
    assert_eq!(
        key_value_vec[DataItem::from(vec![DataItem::from(-1)])].get(0),
        Some(&"cbor".into())
    );

    assert!(key_value_vec.get(DataItem::from(122)).is_none());
    assert!(
        key_value_vec[DataItem::from(vec![100])]
            .get(DataItem::from("y"))
            .is_none()
    );
    assert!(
        key_value_vec[DataItem::from(vec![DataItem::from(-1)])]
            .get(20)
            .is_none()
    );
}

fn debug_compare(diagnostic_val: &str, hex_val: &str) {
    assert_eq!(
        format!(
            "{:?}",
            DataItem::decode(&hex::decode(hex_val).unwrap()).unwrap()
        ),
        diagnostic_val
    );
}

#[test]
fn debug() {
    debug_compare("10", "0a");
    debug_compare("-10", "29");
    debug_compare("Infinity", "f97c00");
    debug_compare("-Infinity", "f9fc00");
    debug_compare("NaN", "fb7ff8000000000000");
    debug_compare("true", "f5");
    debug_compare("simple(255)", "f8ff");
    debug_compare(
        "0(\"2013-03-21T20:04:00Z\")",
        "c074323031332d30332d32315432303a30343a30305a",
    );
    debug_compare("1(1363896240.5)", "c1fb41d452d9ec200000");
    debug_compare("24(h'6449455446')", "d818456449455446");
    debug_compare(
        "32(\"http://www.example.com\")",
        "d82076687474703a2f2f7777772e6578616d706c652e636f6d",
    );
    debug_compare("\"IETF\"", "6449455446");
    debug_compare("\"𐅑\"", "64f0908591");
    debug_compare("[1, 2, 3]", "83010203");
    debug_compare("[1, [2, 3], [4, 5]]", "8301820203820405");
    debug_compare("{1: 2, 3: 4}", "a201020304");
    debug_compare(
        "{\"a\": \"A\", \"b\": \"B\", \"c\": \"C\", \"d\": \"D\", \"e\": \"E\"}",
        "a56161614161626142616361436164614461656145",
    );
    debug_compare("(_ h'0102', h'030405')", "5f42010243030405ff");
    debug_compare("(_ \"strea\", \"ming\")", "7f657374726561646d696e67ff");
    debug_compare("[_ ]", "9fff");
    debug_compare("[_ 1, [2, 3], [_ 4, 5]]", "9f018202039f0405ffff");
    debug_compare("[_ 1, [2, 3], [_ 4, 5]]", "9f018202039f0405ffff");
    debug_compare("[1, [_ 2, 3], [4, 5]]", "83019f0203ff820405");
    debug_compare("{_ \"a\": 1, \"b\": [_ 2, 3]}", "bf61610161629f0203ffff");
    debug_compare("[\"a\", {_ \"b\": \"c\"}]", "826161bf61626163ff");
}
