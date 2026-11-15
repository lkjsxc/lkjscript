// expect: reject E_UNSUPPORTED
// pending: Explain that chars() is provided on a str view and suggest value.as_str().chars() for an owned String, instead of an unspecified implemented method or loop.
pub fn probe() -> usize {
    let value = "text".to_string();
    let mut count = 0usize;
    for character in value.chars() { if character.is_ascii() { count += 1usize; } }
    count
}
