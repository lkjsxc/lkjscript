// expect: reject E_TRAIT
// witness: {"rustAccept":false}
pub fn probe()->i64{let mut text=String::new();write!(&mut text,"hello").unwrap();42}
