use super::*;

fn symbol_vm() -> Vm<'static, NullJit> {
    let mut chunk = lkjscript_core::Chunk::new();
    chunk.constants.extend([
        lkjscript_core::Constant::Symbol("same".into()),
        lkjscript_core::Constant::Symbol("same".into()),
        lkjscript_core::Constant::Symbol("different".into()),
    ]);
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    let chunk = lkjscript_core::validate_chunk(chunk, &lkjscript_core::ValidationLimits::default())
        .expect("symbol comparison chunk validates");
    Vm::new(
        Box::leak(Box::new(chunk)),
        NullJit,
        crate::ExecutionInputs::default(),
        ExecutionConfig::default(),
    )
}

fn generic_enum(layout: [u8; 32], tag: u16, payload: Vec<Value>) -> HeapObj {
    HeapObj::Enum {
        layout: lkjscript_core::RuntimeLayoutId::new(layout),
        physical_tag: tag,
        active_payload: payload,
    }
}

#[test]
fn value_equality_is_exact_and_category_checked() {
    let mut vm = test_vm();

    assert!(compare(&mut vm, Op::EqualValue, Value::UNIT, Value::UNIT));
    assert!(!compare(&mut vm, Op::EqualValue, Value::TRUE, Value::FALSE));
    let wide_left = test_i64(&mut vm, i64::MAX);
    let wide_right = test_i64(&mut vm, i64::MAX);
    assert!(compare(&mut vm, Op::EqualValue, wide_left, wide_right));

    let positive_zero = Value::from_f64_bits(0.0_f64.to_bits());
    let negative_zero = Value::from_f64_bits((-0.0_f64).to_bits());
    assert!(compare(
        &mut vm,
        Op::EqualValue,
        positive_zero,
        negative_zero
    ));
    let nan_left = Value::from_f64_bits(f64::NAN.to_bits());
    let nan_right = Value::from_f64_bits(f64::NAN.to_bits());
    assert!(!compare(&mut vm, Op::EqualValue, nan_left, nan_right));

    let text_left = test_alloc(&mut vm, HeapObj::Str("same".into()));
    let text_right = test_alloc(&mut vm, HeapObj::Str("same".into()));
    assert!(compare(&mut vm, Op::EqualValue, text_left, text_right));
    let mut symbols = symbol_vm();
    let symbol_left = symbols.chunk.symbol_value(0).expect("left symbol");
    let symbol_right = symbols.chunk.symbol_value(1).expect("right symbol");
    assert!(compare(
        &mut symbols,
        Op::EqualValue,
        symbol_left,
        symbol_right
    ));
    let different = symbols.chunk.symbol_value(2).expect("different symbol");
    assert!(!compare(
        &mut symbols,
        Op::EqualValue,
        symbol_left,
        different
    ));
    let text = test_alloc(&mut symbols, HeapObj::Str("same".into()));
    symbols.push(text);
    symbols.push(symbol_left);
    assert!(dispatch(&mut symbols, Op::EqualValue as u8).is_err());
}
#[test]
fn generic_option_and_result_value_equality_is_structural() {
    let mut vm = test_vm();
    let none_left = test_alloc(
        &mut vm,
        generic_enum(lkjscript_core::OPTION_LAYOUT, 1, Vec::new()),
    );
    let none_right = test_alloc(
        &mut vm,
        generic_enum(lkjscript_core::OPTION_LAYOUT, 1, Vec::new()),
    );
    assert!(compare(&mut vm, Op::EqualValue, none_left, none_right));

    let one_left = test_i64(&mut vm, 1);
    let one_right = test_i64(&mut vm, 1);
    let some_left = test_alloc(
        &mut vm,
        generic_enum(lkjscript_core::OPTION_LAYOUT, 0, vec![one_left]),
    );
    let some_right = test_alloc(
        &mut vm,
        generic_enum(lkjscript_core::OPTION_LAYOUT, 0, vec![one_right]),
    );
    assert!(compare(&mut vm, Op::EqualValue, some_left, some_right));
    assert!(!compare(&mut vm, Op::EqualValue, none_left, some_left));

    let ok_left = test_alloc(
        &mut vm,
        generic_enum(lkjscript_core::RESULT_LAYOUT, 0, vec![one_left]),
    );
    let ok_right = test_alloc(
        &mut vm,
        generic_enum(lkjscript_core::RESULT_LAYOUT, 0, vec![one_right]),
    );
    let err = test_alloc(
        &mut vm,
        generic_enum(lkjscript_core::RESULT_LAYOUT, 1, vec![one_right]),
    );
    assert!(compare(&mut vm, Op::EqualValue, ok_left, ok_right));
    assert!(!compare(&mut vm, Op::EqualValue, ok_left, err));

    let mut deep_left = one_left;
    let mut deep_right = one_right;
    for _ in 0..10_000 {
        deep_left = test_alloc(
            &mut vm,
            generic_enum(lkjscript_core::OPTION_LAYOUT, 0, vec![deep_left]),
        );
        deep_right = test_alloc(
            &mut vm,
            generic_enum(lkjscript_core::OPTION_LAYOUT, 0, vec![deep_right]),
        );
    }
    assert!(compare(&mut vm, Op::EqualValue, deep_left, deep_right));
}
#[test]
fn f64_bit_equality_distinguishes_signed_zero_and_accepts_equal_nan_bits() {
    let mut vm = test_vm();
    let positive_zero = Value::from_f64_bits(0.0_f64.to_bits());
    let negative_zero = Value::from_f64_bits((-0.0_f64).to_bits());
    assert!(!compare(
        &mut vm,
        Op::F64BitsEqual,
        positive_zero,
        negative_zero
    ));
    let bits = 0x7ff8_0000_0000_0042_u64;
    let nan_left = Value::from_f64_bits(bits);
    let nan_right = Value::from_f64_bits(bits);
    assert!(compare(&mut vm, Op::F64BitsEqual, nan_left, nan_right));
    let different_nan = Value::from_f64_bits(bits.wrapping_add(1));
    assert!(!compare(&mut vm, Op::F64BitsEqual, nan_left, different_nan));
}
