struct Capture<'a, 'b> { left: &'a i64, right: &'b i64 }
struct Exclusive<'a, 'b> { left: &'a mut i64, right: &'b mut i64 }
fn identity<T>(value: T) -> T { value }
fn capture<'a, 'b>(left: &'a i64, right: &'b i64) -> Capture<'a, 'b> { Capture { left: left, right: right } }
fn exclusive<'a, 'b>(left: &'a mut i64, right: &'b mut i64) -> Exclusive<'a, 'b> { Exclusive { left: left, right: right } }
fn sum(value: Capture<'_, '_>) -> i64 { *value.left + *value.right }
pub fn probe() -> i64 {
    let mut left = 10i64;
    let mut right = 20i64;
    {
        let value = identity(exclusive(&mut left, &mut right));
        *value.left = *value.left + 5i64;
        *value.right = *value.right + 7i64;
    }
    sum(identity(capture(&left, &right)))
}
