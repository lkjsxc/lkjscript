pub fn probe() -> i64 {
    let text = "Aé水Bé";
    assert!(text.starts_with('A'));
    assert!(text.ends_with("é"));
    assert!(text.contains('水'));
    assert!(text.find('水').unwrap() == 3usize);
    assert!(text.rfind("é").unwrap() == 7usize);
    assert!(text.find("missing").is_none());
    assert!(text.find("").unwrap() == 0usize);
    assert!(text.rfind("").unwrap() == 9usize);
    let owned = text.to_string();
    let needle = "é".to_string();
    assert!(owned.contains(&needle));
    assert!(owned.starts_with('A'));
    assert!("ascii".is_ascii());
    assert!(!text.is_ascii());
    assert!("AAz".eq_ignore_ascii_case("aaZ"));
    assert!("aZ水".to_ascii_uppercase().as_bytes()[1] == b'Z');
    let replaced = text.replace('é', "!");
    assert!(replaced.len() == 7usize);
    let separated = "é水".replace("", "-");
    assert!(separated.len() == 8usize);
    let repeated = "水".repeat(2usize);
    assert!(repeated.len() == 6usize);
    42
}
