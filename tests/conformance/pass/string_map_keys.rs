use std::collections::BTreeMap;
pub fn probe() -> i64 {
    let mut values: BTreeMap<String, i64> = BTreeMap::new();
    values.insert(String::from("水"), 7);
    values.insert(String::from("a\0z"), 11);
    values.insert(String::from(""), 13);
    values.insert(String::from("😀"), 17);
    assert_eq!(values.insert(String::from("水"), 19).unwrap(), 7);
    assert_eq!(*values.get(&String::from("a\0z")).unwrap(), 11);
    assert_eq!(values.remove(&String::from("")).unwrap(), 13);
    assert!(!values.contains_key(&String::from("")));
    let mut labels = vec![
        String::from("水"),
        String::from("a"),
        String::from(""),
        String::from("😀"),
    ];
    labels.sort();
    assert!(labels[0] == String::from(""));
    assert!(labels[1] == String::from("a"));
    assert!(labels[2] == String::from("水"));
    assert!(labels[3] == String::from("😀"));
    let mut count = 0usize;
    for key in values.keys() {
        if count == 0usize {
            assert!(*key == String::from("a\0z"));
        }
        if count == 1usize {
            assert!(*key == String::from("水"));
        }
        if count == 2usize {
            assert!(*key == String::from("😀"));
        }
        count += 1usize;
    }
    assert_eq!(count, 3usize);
    42
}
