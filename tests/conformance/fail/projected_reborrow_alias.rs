struct Slot<'a> { value: &'a mut i64 }
fn fill(value: &mut i64) { *value = 42; }
pub fn probe() -> i64 {
    let mut value = 1i64;
    let slot = Slot { value: &mut value };
    let shared = &*slot.value;
    fill(slot.value);
    *shared
}
