type Slot<'a> = &'a mut i64;
struct Pair {
    left: i64,
    right: i64,
}
struct View<'a> {
    value: &'a mut i64,
}

fn current<'a>(value: &'a mut i64) -> i64 {
    let pointer: Slot<'a> = &mut *value;
    *pointer += 1;
    *pointer
}
fn fresh(value: &mut i64) -> i64 {
    let pointer: &mut i64 = &mut *value;
    *pointer += 1;
    drop(pointer);
    *value += 1;
    *value
}
fn delayed<'a>(value: &'a mut i64) -> i64 {
    let pointer: &'a mut i64;
    pointer = &mut *value;
    *pointer += 1;
    *pointer
}
fn history<'a, 'b: 'a>(left: &'b mut i64, right: &'b mut i64) -> i64 {
    let mut pointer = &mut *left;
    pointer = &mut *right;
    let descriptor: &&'a mut i64 = &pointer;
    drop(descriptor);
    *pointer += 1;
    *pointer
}
fn stored<'a>(mut value: &'a mut i64) -> i64 {
    let descriptor: &mut &'a mut i64 = &mut value;
    **descriptor += 1;
    drop(descriptor);
    *value
}
fn disjoint<'a>(pair: &'a mut Pair) -> i64 {
    let pointer: &'a mut i64 = &mut pair.left;
    *pointer += 1;
    let left = *pointer;
    drop(pointer);
    pair.right += 1;
    left + pair.right
}
fn nominal<'a>(value: &'a mut i64) -> i64 {
    let view: View<'a> = View { value: &mut *value };
    *view.value += 1;
    *view.value
}
fn absent<'a, 'b: 'a>() {
    let longer: Option<&'b i64> = None;
    let shorter: Option<&'a i64> = longer;
    drop(shorter);
    let never: &'a i64;
}
fn captured<'a>(input: &'a i64) -> i64 {
    let first = || {
        let pointer: &'a i64 = input;
        let second = || {
            let value: &'a i64 = pointer;
            *value
        };
        second()
    };
    first()
}
fn explicit_argument<'a>(input: &'a i64) -> i64 {
    let invoke = |value: &'a i64| {
        let pointer: &'a i64 = value;
        *pointer
    };
    invoke(input)
}
fn issuing_parent<'a, 'b: 'a>(slot: &'b mut &'a mut i64) -> i64 {
    let pointer: &'a mut i64 = &mut **slot;
    *pointer += 1;
    *pointer
}
pub fn probe() -> i64 {
    let mut value = 1i64;
    assert!(current(&mut value) == 2);
    assert!(fresh(&mut value) == 4);
    assert!(delayed(&mut value) == 5);
    let mut other = 20i64;
    assert!(history(&mut value, &mut other) == 21);
    assert!(stored(&mut other) == 22);
    let mut pair = Pair {
        left: 19,
        right: 21,
    };
    assert!(disjoint(&mut pair) == 42);
    assert!(nominal(&mut other) == 23);
    absent();
    let answer = 42i64;
    assert!(captured(&answer) == 42);
    assert!(explicit_argument(&answer) == 42);
    let mut issued = 41i64;
    let mut descriptor = &mut issued;
    issuing_parent(&mut descriptor)
}
