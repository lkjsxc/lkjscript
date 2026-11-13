#[derive(Copy)]
struct Number { value: i64 }
impl Clone for Number { fn clone(&self) -> Self { Number { value: self.value + 1 } } }
struct Owned { value: Box<i64> }
impl Clone for Owned { fn clone(&self) -> Self { Owned { value: Box::new(*self.value) } } }
pub fn probe() -> i64 {
    let mut number = Number { value: 20 };
    let copied = Some(&number).copied().unwrap();
    let cloned = Some(&number).cloned().unwrap();
    assert!(copied.value == 20 && cloned.value == 21);
    let copied_mut = Some(&mut number).copied().unwrap();
    let cloned_mut = Some(&mut number).cloned().unwrap();
    assert!(copied_mut.value == 20 && cloned_mut.value == 21);
    let mut owned = Owned { value: Box::new(42) };
    let from_shared = Some(&owned).cloned().unwrap();
    let from_exclusive = Some(&mut owned).cloned().unwrap();
    *owned.value = 0;
    assert!(*from_shared.value == 42 && *from_exclusive.value == 42);
    let absent: Option<&Owned> = None;
    assert!(absent.cloned().is_none());
    let absent_mut: Option<&mut Number> = None;
    assert!(absent_mut.copied().is_none());
    *from_shared.value
}
