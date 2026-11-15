// expect: reject E_TYPE
// Diagnostic: name the unknown Option element and suggest an explicit Option<String> local.
pub fn probe() {
    let mut value = None;
    if value.is_some() { return; }
    value = Some("text/plain".to_string());
}
