fn pair() -> (i64, i64) { (40, 2) }
fn identity<T>(value: T) -> T { value }
fn singleton<T>(value: T) -> Vec<T> {
    let mut values = Vec::new(); values.push(value); values
}
fn vector_loop() -> i64 {
    let mut values = Vec::new(); values.push(40i64); values.push(2);
    let mut index = 0; let mut sum = 0;
    while index < values.len() { sum = sum + values[index]; index = index + 1; }
    sum
}
fn tuple_context() -> i64 { let value = pair(); value.0 + value.1 }
fn generic_context() -> i64 { let value: (i64, i64) = (identity(40), 2); value.0 + value.1 }
fn later_assignment() -> i64 { let mut value = 0; value = 42i64; value }
fn nested_constructor() -> i64 {
    let value: (Box<i64>, (u8, i64)) = (Box::new(40), (2, 0));
    let tail = value.1;
    *value.0 + tail.0 as i64 + tail.1
}
fn separate_instances() -> i64 {
    let values = singleton(42i64); let flags = singleton(true);
    if flags[0] { values[0] } else { 0 }
}
struct Measurement { value: i64 }
impl Measurement { fn len(&self) -> i64 { self.value } }
fn method_identity() -> i64 { let value = Measurement { value: 42 }; value.len() + 0i64 }
fn scoped_bindings() -> i64 {
    let mut values = Vec::new();
    { let mut values = Vec::new(); values.push(false); }
    values.push(42i64); values[0]
}
pub fn main(mut world: World) {
    let mut output = world.stdout();
    let bytes = [vector_loop() as u8, tuple_context() as u8,
        generic_context() as u8, later_assignment() as u8,
        nested_constructor() as u8, separate_instances() as u8,
        method_identity() as u8, scoped_bindings() as u8];
    output.write(&bytes);
}
