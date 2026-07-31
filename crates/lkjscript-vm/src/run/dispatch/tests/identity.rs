use super::*;

#[test]
fn object_identity_is_limited_to_resources() {
    test_vm!(vm);
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

    let closure = vm.chunk.function_value(0).expect("function value");
    vm.push(closure);
    vm.push(closure);
    assert!(dispatch(&mut vm, Op::EqualValue as u8).is_err());
}
