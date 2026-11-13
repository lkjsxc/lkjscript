pub fn probe() -> i64 {
    let values = vec![
        String::from("|"),
        String::from("a"),
        String::from("|"),
        String::from("|"),
        String::from("水"),
        String::from("|"),
    ];
    let mut visited = 0usize;
    {
        let mut groups = values[..].split(|item: &String| {
            visited += 1;
            item == "|"
        });
        let first = groups.next().unwrap();
        let second = groups.next().unwrap();
        assert!(first.len() == 0);
        assert!(second.len() == 1);
        assert!(second[0] == "a");
        assert!(groups.next().unwrap().len() == 0);
        assert!(groups.next().unwrap()[0] == "水");
        assert!(groups.next().unwrap().len() == 0);
        assert!(groups.next().is_none());
        assert!(groups.next().is_none());
        assert!(second[0] == "a");
    }
    assert!(visited == 6);
    let empty: Vec<u8> = Vec::new();
    let mut groups = empty[..].split(|item: &u8| *item == 0u8);
    assert!(groups.next().unwrap().len() == 0);
    assert!(groups.next().is_none());
    let numbers = vec![20i64, 0i64, 22i64];
    let mut sum = 0i64;
    for group in numbers[..].split(|item: &i64| *item == 0i64) {
        sum += group[0];
    }
    sum
}
