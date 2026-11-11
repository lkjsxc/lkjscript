#[derive(Clone, PartialEq)]
struct Record { name: String }
pub fn probe() -> i64 {
    let mut original = Option::Some(Record { name: "hi".to_string() });
    let copied = original.clone();
    assert!(original == copied);
    original = Option::None;
    assert!(original != copied);
    drop(original);
    copied.unwrap().name.len() as i64 + 40
}
