// expect: reject E_TYPE
// pending: Name the uninferred BTreeSet element type instead of an unknown constructor; suggest annotating BTreeSet<String> or spelling the generic argument.
use std::collections::BTreeSet;
pub fn probe() -> usize {
    let mut names = BTreeSet::new();
    names.insert("key".to_string());
    names.len()
}
