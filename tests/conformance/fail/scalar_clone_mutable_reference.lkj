// expect: reject E_TRAIT
// witness: {"rustAccept":false,"rustCode":"E0277"}
fn duplicate<T: Clone>(value: &T) -> T { value.clone() }
pub fn probe() {
    let mut value = 7u8;
    let exclusive = &mut value;
    let _duplicated = duplicate(&exclusive);
}
