use crate::data_item::DataItem;
use crate::index::Get as _;

fn sample() -> DataItem {
    DataItem::map([
        (DataItem::from(10), DataItem::from("abc")),
        (DataItem::from(-1), DataItem::from(12)),
        (DataItem::from("name"), DataItem::from("cbor")),
        (
            DataItem::from(vec![100]),
            DataItem::map([("z", DataItem::from("a")), ("aa", DataItem::from(12))]),
        ),
        (
            DataItem::from(vec![DataItem::from(-1)]),
            DataItem::from(vec!["cbor", "nano"]),
        ),
    ])
}

#[test]
fn get_by_key() {
    let item = sample();
    assert_eq!(item.get(&DataItem::from(10)), Some(&DataItem::from("abc")));
    assert_eq!(item.get(&DataItem::from(-1)), Some(&DataItem::from(12)));
    // a text key is looked up with a plain string slice
    assert_eq!(item.get("name"), Some(&DataItem::from("cbor")));
    assert_eq!(item.get("missing"), None);
    assert_eq!(item.get(&DataItem::from(122)), None);
    // an array is not indexed by key and a map is not indexed by position
    assert_eq!(item.get(0), None);
    assert_eq!(DataItem::array([1]).get("a"), None);
}

#[test]
fn get_by_position() {
    let item = DataItem::array(["cbor", "nano"]);
    assert_eq!(item.get(0), Some(&DataItem::from("cbor")));
    assert_eq!(item.get(1), Some(&DataItem::from("nano")));
    assert_eq!(item.get(2), None);
}

#[test]
fn index_operator() {
    let item = sample();
    assert_eq!(item[&DataItem::from(10)], DataItem::from("abc"));
    assert_eq!(item[&DataItem::from(-1)], DataItem::from(12));
    assert_eq!(item["name"], DataItem::from("cbor"));
    assert_eq!(item[&DataItem::from(vec![100])]["z"], DataItem::from("a"));
    assert_eq!(
        item[&DataItem::from(vec![DataItem::from(-1)])][0],
        DataItem::from("cbor")
    );
}

#[test]
fn get_mut_updates_in_place() {
    let mut item = sample();
    *item.get_mut("name").unwrap() = DataItem::from("cbor_next");
    assert_eq!(item["name"], DataItem::from("cbor_next"));

    let mut array = DataItem::array([1, 2]);
    *array.get_mut(1).unwrap() = DataItem::from(3);
    assert_eq!(array, DataItem::array([1, 3]));
    array.as_array_mut().unwrap().extend([4]);
    assert_eq!(array, DataItem::array([1, 3, 4]));
    assert!(array.get_mut(9).is_none());
}
