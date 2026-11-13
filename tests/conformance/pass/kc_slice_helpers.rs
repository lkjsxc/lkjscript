pub fn probe() -> i64 {
    let mut values: Vec<i64> = vec![4, 8, 15, 16];
    let copy = {
        let slice = &values[1..3];
        assert!(!slice.is_empty());
        assert!(*slice.get(0).unwrap() == 8);
        assert!(*slice.first().unwrap() == 8);
        assert!(*slice.last().unwrap() == 15);
        assert!(slice.get(2).is_none());
        assert!(slice.contains(&15));
        assert!(!slice.contains(&16));
        let prefix: Vec<i64> = vec![8];
        assert!(slice.starts_with(&prefix));
        slice.to_vec()
    };
    assert!(copy[0] == 8 && copy[1] == 15);
    values[0..2].copy_from_slice(&copy);
    assert!(values[0] == 8 && values[1] == 15);
    *values.last_mut().unwrap() = 19;
    *values.get_mut(2).unwrap() = 23;
    assert!(values.starts_with(&copy));
    assert!(values[2] + values[3] == 42);
    let mut text: Vec<String> = vec![String::from("one"), String::from("two")];
    let names = text.as_slice().to_vec();
    text[0].clear();
    assert!(names[0].as_str() == "one");
    text.clear();
    assert!(text.is_empty());
    assert!(text.last().is_none());
    assert!(text.last_mut().is_none());
    42
}
