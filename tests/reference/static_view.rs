// expect: value 42
struct View<'a> {
    text: &'a str,
}
fn static_view() -> View<'static> {
    View { text: "42" }
}
fn through_carrier<'a>(value: &&'a str) -> &'a str {
    *value
}
pub fn probe() -> i64 {
    let view = static_view();
    let text = through_carrier(&view.text);
    assert!(text.as_bytes()[0] == 52u8);
    text.parse::<i64>().unwrap()
}
