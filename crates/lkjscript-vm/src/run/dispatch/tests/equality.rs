use super::*;

macro_rules! symbol_vm {
    ($name:ident) => {
        let mut symbol_chunk = lkjscript_core::Chunk::new();
        symbol_chunk.constants.extend([
            lkjscript_core::Constant::Symbol("same".into()),
            lkjscript_core::Constant::Symbol("same".into()),
            lkjscript_core::Constant::Symbol("different".into()),
        ]);
        symbol_chunk.main.emit(Op::Unit);
        symbol_chunk.main.emit(Op::Return);
        let symbol_chunk = lkjscript_core::validate_chunk(
            symbol_chunk,
            &lkjscript_core::ValidationLimits::default(),
        )
        .expect("symbol comparison chunk validates");
        let mut $name = Vm::new(
            &symbol_chunk,
            NullJit,
            crate::ExecutionInputs::default(),
            ExecutionConfig::default(),
        );
    };
}

#[test]
fn value_equality_is_exact_and_category_checked() {
    test_vm!(vm);

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

    let text_left = crate::run::structural_ops::publish_string(&mut vm, "same".into())
        .expect("left structural string");
    let text_right = crate::run::structural_ops::publish_string(&mut vm, "same".into())
        .expect("right structural string");
    assert!(compare(&mut vm, Op::EqualValue, text_left, text_right));
    symbol_vm!(symbols);
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
    let text = crate::run::structural_ops::publish_string(&mut symbols, "same".into())
        .expect("structural string");
    symbols.push(text);
    symbols.push(symbol_left);
    assert!(dispatch(&mut symbols, Op::EqualValue as u8).is_err());
}
#[test]
fn f64_bit_equality_distinguishes_signed_zero_and_accepts_equal_nan_bits() {
    test_vm!(vm);
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
