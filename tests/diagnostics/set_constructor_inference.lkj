// expect: reject E_TYPE
// rewrite: Name the missing BTreeSet element type and supply std::String when unambiguous later literal.to_string inserts establish it.
use std::collections::BTreeSet;
pub fn probe() -> usize {
    let mut names = BTreeSet::new();
    names.insert("key".to_string());
    names.len()
}
