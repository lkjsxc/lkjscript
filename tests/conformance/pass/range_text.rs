fn window<'a>(text: &'a str) -> &'a str {
    &text[1..7]
}
pub fn probe() -> i64 {
    let original = "aλ🦀z";
    let text = window(original);
    assert!(text.len() == 6);
    assert!((&text[2..]).len() == 4 && text[2..].as_bytes()[0] == 240);
    let owned = "aλ🦀z".to_string();
    assert!(owned[1..7].len() == 6 && owned[1..3].as_bytes()[0] == 206);
    42
}
