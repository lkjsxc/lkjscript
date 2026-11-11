enum E { Nothing }
struct T(i64);
fn identity<T>(value: T) -> T { value }
pub fn probe() -> i64 {
    let text = "scope".to_string();
    identity(37i64) + text.as_str().len() as i64
}
