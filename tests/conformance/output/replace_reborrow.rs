fn replace_twice(slot: &mut i64) {
    let old = std::mem::replace(slot, 20);
    let middle = std::mem::replace(slot, old + 20);
    *slot = *slot + middle;
}
fn refill<T>(slot: &mut T, first: T, second: T) -> T {
    let old = std::mem::replace(slot, first);
    drop(old);
    std::mem::replace(slot, second)
}
fn borrow<T>(value: &mut T) -> &mut T { value }
struct Node { next: Option<Box<Node>>, value: i64 }
fn pop(head: &mut Option<Box<Node>>) -> i64 {
    let old = std::mem::replace(head, None);
    match old {
        Some(boxed) => { let node: Node = *boxed; *head = node.next; node.value }
        None => 0,
    }
}
struct Slot<'a> { value: &'a mut i64 }
fn fill(value: &mut i64) { *value = 42; }
fn inspect(value: &i64) -> i64 { *value }
fn projected() -> i64 {
    let mut value = 0i64;
    let mut slot = Slot { value: &mut value };
    fill(slot.value);
    let shared = &*slot.value;
    assert!(inspect(slot.value) == *shared);
    *slot.value = 0;
    let mut visits = 0i64;
    fill(({ visits = visits + 1; &mut slot }).value);
    assert!(visits == 1);
    *slot.value
}
pub fn main(mut world: World) {
    let mut out: Stdout = world.stdout();
    let mut scalar = 2i64;
    replace_twice(&mut scalar);
    let mut boxed = Box::new(0i64);
    let old = refill(&mut boxed, Box::new(20i64), Box::new(22i64));
    let mut head: Option<Box<Node>> = Some(Box::new(Node {
        next: Some(Box::new(Node { next: None, value: 2 })), value: 40,
    }));
    let mut context_box = Box::new(0i64);
    let reference: &mut i64 = borrow(&mut context_box);
    *reference = 42;
    let bytes: [u8; 5] = [scalar as u8, (*old + *boxed) as u8,
        (pop(&mut head) + pop(&mut head) + pop(&mut head)) as u8, *reference as u8, projected() as u8];
    out.write(bytes.as_slice());
}
