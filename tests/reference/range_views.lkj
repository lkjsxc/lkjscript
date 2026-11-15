// expect: value 42
pub fn probe() -> i64 {
    let values = vec![11i64, 20, 22, 99];
    let middle = &values[1..=2];
    assert!(middle.len() == 2);
    assert!(values[4..=3].len() == 0);
    let text = "é水";
    assert!(&text[..2] == "é");
    assert!(&text[2..] == "水");
    middle[0] + middle[1]
}
