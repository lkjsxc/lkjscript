struct Held<T> { value: T }
struct Borrowed<'a, T> { value: &'a mut T }
type Ref<'a, T> = &'a mut T;
struct Aliased<'a, T> { value: Ref<'a, T> }
fn identity<T>(value: T) -> T { value }
fn option_context() -> i64 {
    let mut value = 40i64;
    let reference = &mut value;
    let held: Option<&mut i64> = Some(reference);
    drop(held);
    *reference = *reference + 2;
    *reference
}
fn struct_context() -> i64 {
    let mut value = 40i64;
    let reference = &mut value;
    let held: Held<&mut i64> = Held { value: reference };
    drop(held);
    *reference = *reference + 2;
    *reference
}
fn declared_reference() -> i64 {
    let mut value = 40i64;
    let reference = &mut value;
    let held = Borrowed { value: reference };
    drop(held);
    *reference = *reference + 2;
    *reference
}
fn aliased_reference() -> i64 {
    let mut value = 40i64;
    let reference = &mut value;
    let held = Aliased { value: reference };
    drop(held);
    *reference = *reference + 2;
    *reference
}
fn call_context() -> i64 {
    let mut value = 40i64;
    let reference = &mut value;
    let held: &mut i64 = identity(reference);
    drop(held);
    *reference = *reference + 2;
    *reference
}
fn explicit_argument() -> i64 {
    let mut value = 40i64;
    let reference = &mut value;
    let held = identity::<&mut i64>(reference);
    drop(held);
    *reference = *reference + 2;
    *reference
}
pub fn main(mut world: World) {
    let mut output = world.stdout();
    let bytes = [option_context() as u8, struct_context() as u8,
        declared_reference() as u8, aliased_reference() as u8,
        call_context() as u8, explicit_argument() as u8];
    output.write(&bytes);
}
