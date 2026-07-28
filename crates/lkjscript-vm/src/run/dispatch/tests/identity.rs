use super::*;

#[test]
fn object_identity_is_limited_to_buffers_and_resources() {
    let mut vm = test_vm();
    let buffer = test_alloc(&mut vm, HeapObj::Buf(vec![1, 2, 3]));
    let clone = test_alloc(&mut vm, HeapObj::Buf(vec![1, 2, 3]));
    assert!(compare(&mut vm, Op::SameObject, buffer, buffer));
    assert!(!compare(&mut vm, Op::SameObject, buffer, clone));
    assert!(compare(
        &mut vm,
        Op::SameObject,
        Value::from_resource(7),
        Value::from_resource(7)
    ));
    assert!(!compare(
        &mut vm,
        Op::SameObject,
        Value::from_resource(7),
        Value::from_resource(8)
    ));

    let integer = test_i64(&mut vm, 1);
    vm.push(integer);
    vm.push(integer);
    assert!(dispatch(&mut vm, Op::SameObject as u8).is_err());

    let closure = test_alloc(
        &mut vm,
        HeapObj::Closure {
            proto: 0,
            captures: Vec::new(),
        },
    );
    vm.push(closure);
    vm.push(closure);
    assert!(dispatch(&mut vm, Op::EqualValue as u8).is_err());
}
