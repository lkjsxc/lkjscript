// expect: reject E_UNSUPPORTED
// Diagnostic: explain the str view receiver and suggest value.as_str().chars().
pub fn probe() -> usize {
    let value = "text".to_string();
    let mut count = 0usize;
    for character in value.chars() { if character.is_ascii() { count += 1usize; } }
    count
}
