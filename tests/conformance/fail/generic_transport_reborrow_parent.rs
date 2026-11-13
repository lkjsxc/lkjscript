struct Pair<'a, 'b> { left: &'a i64, right: &'b i64 }
fn identity<T>(value: T) -> T { value }
pub fn probe() -> i64 {
    let mut first: i64 = 20;
    let second: i64 = 22;
    let parent = &mut first;
    let captured = identity(Pair { left: &*parent, right: &second });
    *parent = 0;
    *captured.left + *captured.right
}
