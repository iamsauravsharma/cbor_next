use core::f64;
use std::vec;

use indexmap::IndexMap;
use rand::seq::SliceRandom;

use crate::{deterministic::DeterministicMode, value::Value};

use super::SimpleNumber;

fn encode_compare<I>(hex_cbor: &str, value_into: I)
where
    I: Into<Value>,
{
    let value = value_into.into();
    let vec_u8_cbor =
        hex::decode(hex_cbor).unwrap_or_else(|err| panic!("{err} failed to decode hex {hex_cbor}"));
    let value_to_cbor = value.encode();
    assert_eq!(value_to_cbor, vec_u8_cbor, "{hex_cbor}");
}

fn decode_compare<I>(hex_cbor: &str, value_into: I)
where
    I: Into<Value>,
{
    let value = value_into.into();
    let vec_u8_cbor =
        hex::decode(hex_cbor).unwrap_or_else(|_| panic!(" failed to decode hex {hex_cbor}"));
    let cbor_to_value = Value::decode(&vec_u8_cbor).unwrap_or_else(|err: crate::error::Error| {
        panic!("{err} failed to decode value {hex_cbor}")
    });
    assert_eq!(&cbor_to_value, &value, "{hex_cbor}");
}

fn compare_cbor_value<I>(hex_cbor: &str, value_into: I)
where
    I: Into<Value>,
{
    let value = value_into.into();
    let vec_u8_cbor =
        hex::decode(hex_cbor).unwrap_or_else(|err| panic!("{err} failed to decode hex {hex_cbor}"));
    let value_to_cbor = value.encode();
    assert_eq!(value_to_cbor, vec_u8_cbor, "{hex_cbor}");
    let cbor_to_value = Value::decode(&vec_u8_cbor)
        .unwrap_or_else(|err| panic!("{err} failed to decode value {hex_cbor}"));
    assert_eq!(&cbor_to_value, &value, "{hex_cbor}");
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
        Value::try_from(-18_446_744_073_709_551_616_i128).unwrap(),
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
    compare_cbor_value("f6", Value::Null);
    compare_cbor_value("f7", Value::Undefined);
    compare_cbor_value("f0", Value::UnknownSimple(SimpleNumber::new(16).unwrap()));
    compare_cbor_value("f820", Value::UnknownSimple(SimpleNumber::new(32).unwrap()));
    compare_cbor_value(
        "f8ff",
        Value::UnknownSimple(SimpleNumber::new(255).unwrap()),
    );
}

#[test]
fn tag() {
    compare_cbor_value(
        "c074323031332d30332d32315432303a30343a30305a",
        Value::Tag(0, Box::new("2013-03-21T20:04:00Z".into())),
    );
    compare_cbor_value(
        "c074323031332d30332d32315432303a30343a30305a",
        Value::Tag(0, Box::new("2013-03-21T20:04:00Z".into())),
    );
    compare_cbor_value(
        "c11a514b67b0",
        Value::Tag(1, Box::new(1_363_896_240.into())),
    );
    compare_cbor_value(
        "c1fb41d452d9ec200000",
        Value::Tag(1, Box::new(1_363_896_240.5.into())),
    );
    compare_cbor_value(
        "d74401020304",
        Value::Tag(23, Box::new(hex::decode("01020304").unwrap().into())),
    );
    compare_cbor_value(
        "d818456449455446",
        Value::Tag(24, Box::new(hex::decode("6449455446").unwrap().into())),
    );
    compare_cbor_value(
        "d82076687474703a2f2f7777772e6578616d706c652e636f6d",
        Value::Tag(32, Box::new("http://www.example.com".into())),
    );
}

#[test]
fn byte() {
    compare_cbor_value("40", Vec::<u8>::new());
    compare_cbor_value("4401020304", hex::decode("01020304").unwrap());
    decode_compare("5f42010243030405ff", hex::decode("0102030405").unwrap());
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
    decode_compare("7f657374726561646d696e67ff", "streaming");
}

#[test]
fn array() {
    compare_cbor_value("80", Vec::<u64>::new());
    compare_cbor_value("83010203", vec![1u64, 2, 3]);
    compare_cbor_value::<Vec<Value>>(
        "8301820203820405",
        vec![1u64.into(), vec![2u64, 3].into(), vec![4u64, 5u64].into()],
    );
    compare_cbor_value(
        "98190102030405060708090a0b0c0d0e0f101112131415161718181819",
        vec![
            1u64, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25,
        ],
    );
    compare_cbor_value::<Vec<Value>>(
        "826161a161626163",
        vec!["a".into(), vec![("b", "c")].into()],
    );
    decode_compare("9fff", Vec::<u64>::new());
    decode_compare::<Vec<Value>>(
        "9f018202039f0405ffff",
        vec![1u64.into(), vec![2u64, 3].into(), vec![4u64, 5u64].into()],
    );
    decode_compare::<Vec<Value>>(
        "9f01820203820405ff",
        vec![1u64.into(), vec![2u64, 3].into(), vec![4u64, 5u64].into()],
    );
    decode_compare::<Vec<Value>>(
        "83018202039f0405ff",
        vec![1u64.into(), vec![2u64, 3].into(), vec![4u64, 5u64].into()],
    );
    decode_compare::<Vec<Value>>(
        "83019f0203ff820405",
        vec![1u64.into(), vec![2u64, 3].into(), vec![4u64, 5u64].into()],
    );
    decode_compare(
        "9f0102030405060708090a0b0c0d0e0f101112131415161718181819ff",
        vec![
            1u64, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
            24, 25,
        ],
    );
    decode_compare::<Vec<Value>>(
        "826161bf61626163ff",
        vec!["a".into(), vec![("b", "c")].into()],
    );
}

#[test]
fn map() {
    compare_cbor_value("a0", Value::Map(IndexMap::new()));
    compare_cbor_value("a201020304", vec![(1, 2), (3, 4)]);
    compare_cbor_value(
        "a26161016162820203",
        vec![("a", Value::from(1)), ("b", vec![2u64, 3].into())],
    );
    compare_cbor_value(
        "a56161614161626142616361436164614461656145",
        vec![("a", "A"), ("b", "B"), ("c", "C"), ("d", "D"), ("e", "E")],
    );
    decode_compare(
        "bf61610161629f0203ffff",
        vec![("a", Value::from(1)), ("b", vec![2u64, 3].into())],
    );
    decode_compare(
        "bf6346756ef563416d7421ff",
        vec![("Fun", Value::from(true)), ("Amt", Value::from(-2))],
    );
}

#[test]
fn failure() {
    assert!(Value::decode(&hex::decode("1c").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("7f14").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("f801").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("9fde").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("bf3e").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("7fbb").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("dc").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("7f42").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("5f87").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("3f").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("5d").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("bc").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("5f4100").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("5fc000ff").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("9f819f819f9fffffff").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("9f829f819f9fffffffff").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("1a0102").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("5affffffff00").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("bf000000ff").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("a2000000").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("5fd9").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("bffc").unwrap()).is_err());
    assert!(Value::decode(&hex::decode("ff").unwrap()).is_err());
}

#[test]
fn core_deterministic() {
    let key_value_vec = vec![
        (10.into(), "abc".into()),
        (100.into(), "1020".into()),
        (Value::from(-1), 12.into()),
        (Value::from("z"), "a".into()),
        (Value::from("aa"), Value::from(-1)),
        (
            Value::Array(vec![100.into()]),
            Value::Map(IndexMap::from_iter(vec![
                (1_000_000.into(), "1020".into()),
                (Value::from("z"), "a".into()),
                (Value::from("aa"), 12.into()),
            ])),
        ),
        (
            Value::Array(vec![Value::from(-1)]),
            Value::Array(vec!["cbor".into(), "nano".into()]),
        ),
        (false.into(), 12.into()),
    ];
    let mut random_key_value = key_value_vec.clone();
    random_key_value.shuffle(&mut rand::rng());
    assert_ne!(key_value_vec, random_key_value);
    let map_val =
        Value::Map(IndexMap::from_iter(random_key_value)).deterministic(&DeterministicMode::Core);
    assert_eq!(Value::Map(IndexMap::from_iter(key_value_vec)), map_val);
}

#[test]
fn length_core_deterministic() {
    let key_value_vec = vec![
        (10.into(), "abc".into()),
        (100.into(), "1020".into()),
        (Value::from(-1), 12.into()),
        (Value::from("z"), "a".into()),
        (Value::from("aa"), Value::from(-1)),
        (
            Value::Array(vec![100.into()]),
            Value::Map(IndexMap::from_iter(vec![
                (1_000_000.into(), "1020".into()),
                (Value::from("z"), "a".into()),
                (Value::from("aa"), 12.into()),
            ])),
        ),
        (
            Value::Array(vec![Value::from(-1)]),
            Value::Array(vec!["cbor".into(), "nano".into()]),
        ),
        (false.into(), 12.into()),
    ];
    let mut random_key_value = key_value_vec.clone();
    random_key_value.shuffle(&mut rand::rng());
    assert_ne!(key_value_vec, random_key_value);
    let map_val = Value::Map(IndexMap::from_iter(random_key_value))
        .deterministic(&DeterministicMode::LengthFirst);
    assert_eq!(Value::Map(IndexMap::from_iter(key_value_vec)), map_val);
}
