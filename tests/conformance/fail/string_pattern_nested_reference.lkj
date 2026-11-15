// expect: reject E_TYPE
// witness: {"rustAccept":false,"rustCode":"E0308"}
pub fn probe() -> i64 {
    let nested = &"text";
    match nested { "text" => 1, _ => 0 }
}
