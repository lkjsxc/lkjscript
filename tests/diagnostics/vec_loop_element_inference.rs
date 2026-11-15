// expect: reject E_TYPE
// pending: Explain that this Vec loop needs an owned String element annotation instead of reporting an unsized text element; suggest a local Vec<String> before iteration.
fn inspect(value: &str) -> bool { !value.is_empty() }
pub fn probe() {
    for value in vec!["text".to_string()] { assert!(inspect(&value)); }
}
