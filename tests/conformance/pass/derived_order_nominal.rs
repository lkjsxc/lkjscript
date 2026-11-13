use std::collections::BTreeMap;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Projection {
    Field(usize),
    Index(usize),
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    root: Option<usize>,
    path: Vec<Projection>,
}
fn key(index: usize) -> Key {
    Key {
        root: Some(index / 4),
        path: vec![Projection::Field(index % 4)],
    }
}
pub fn probe() -> i64 {
    assert!(Projection::Field(99) < Projection::Index(0));
    assert!(Projection::Index(3) > Projection::Index(2));
    assert!(key(3) < key(4));
    assert!(key(4) <= key(4));
    assert!(key(4) >= key(4));
    assert!(!(key(4) > key(4)));
    let boxed_left = Box::new((Some(2i64), false));
    let boxed_right = Box::new((Some(2i64), true));
    assert!(boxed_left < boxed_right);
    let mut map: BTreeMap<Key, usize> = BTreeMap::new();
    let mut index = 32usize;
    while index != 0 {
        index -= 1;
        map.insert(key(index), index);
    }
    assert_eq!(map.len(), 32usize);
    let mut expected = 0usize;
    for (stored, value) in &map {
        assert_eq!(*value, expected);
        assert!(*stored >= key(expected));
        assert!(*stored <= key(expected));
        expected += 1;
    }
    assert_eq!(expected, 32usize);
    let left = key(1);
    let right = key(2);
    assert!(left < right);
    assert_eq!(left.path.len() + right.path.len(), 2usize);
    42
}
