// expect: reject E_BORROW
// rewrite: This returned view retains the iterator temporary; index the canonical Vec field of immutable self directly and advance before the original body, preserving continue, order and cleanup.
struct Table { fields: Vec<(String, String)> }
impl Table {
    fn get(&self, key: &str) -> Option<&String> {
        for field in &self.fields {
            if field.0.as_str() == key { return Some(&field.1); }
        }
        None
    }
}
pub fn probe() -> usize {
    let table = Table { fields: vec![("key".to_string(), "value".to_string())] };
    table.get("key").unwrap().len()
}
