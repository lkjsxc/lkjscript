// expect: reject E_TRAIT
// witness: {"rustAccept":false,"rustCode":"E0277"}
pub fn contains<T>(values: &Vec<T>, value: &T) -> bool {
    values.contains(value)
}
