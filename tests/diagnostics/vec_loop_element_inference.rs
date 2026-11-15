// expect: reject E_TYPE
// Diagnostic: suggest a Vec<String> local before borrowing loop elements as str.
fn inspect(value: &str) -> bool { !value.is_empty() }
pub fn probe() {
    for value in vec!["text".to_string()] { assert!(inspect(&value)); }
}
