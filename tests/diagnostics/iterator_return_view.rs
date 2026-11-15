// expect: reject E_BORROW
// pending: Explain that the returned view is tied to the loop iterator temporary here; suggest indexing the owning input directly when returning an element view.
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
