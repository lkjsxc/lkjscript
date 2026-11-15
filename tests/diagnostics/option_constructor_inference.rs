// expect: reject E_TYPE
// pending: Name the uninferred Option element type and suggest Option<String> instead of reporting the wrong number of type arguments on None.
pub fn probe() {
    let mut value = None;
    if value.is_some() { return; }
    value = Some("text/plain".to_string());
}
