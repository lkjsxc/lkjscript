#[derive(Clone)]
struct View<'a> { left: &'a i64, right: &'a i64, name: String }
struct Numbers { left: i64, right: i64 }
fn cloned<T: Clone>(value: &T) -> T { value.clone() }
fn forward<'a>(numbers: &'a Numbers) -> View<'a> {
    let original = View { left: &numbers.left, right: &numbers.right, name: "view".to_string() };
    cloned(&original)
}
pub fn probe() -> i64 {
    let values = Numbers { left: 20, right: 22 };
    let view = forward(&values);
    *view.left + *view.right
}
