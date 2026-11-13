fn length(value: &str) -> usize { value.len() }
fn borrowed<'a>(value: &'a String) -> &'a str { value }
fn token(word: &str) -> i64 {
    match word { "" => 1, "水" | "deref" => 2, "nul\0" => 3, _ => 4 }
}
pub fn probe() -> i64 {
    let mut value = String::from("水");
    assert_eq!(length(&value), 3usize);
    assert_eq!(length(&format!("{}x", value)), 4usize);
    assert_eq!(borrowed(&value), "水");
    assert!(value == "水");
    assert!("水" == value);
    assert!(&value == "水");
    value.push_str(&format!("{}", 42));
    assert_eq!(value.as_str(), "水42");
    assert!("水" != "火");
    assert_eq!(token(""), 1);
    assert_eq!(token("deref"), 2);
    assert_eq!(token("nul\0"), 3);
    assert_eq!(token("another"), 4);
    let nested = &"水";
    let matched = match *nested { "水" => 9, _ => 0 };
    assert_eq!(matched, 9);
    let strings = vec![String::from("a"), String::from("b")];
    assert_eq!(strings[0], String::from("a"));
    assert_eq!(strings[1], String::from("b"));
    42
}
