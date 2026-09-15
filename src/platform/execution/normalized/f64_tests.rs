//! Independently fixed binary64 observations through admitted graph functions in both evaluators.

use super::*;
use crate::platform::binary64::Binary64;
use crate::platform::execution::normalized::value::{NormalizedRecord, RecordLayoutIndex};

fn scalar(bits: u64) -> NormalizedValue {
    NormalizedValue::F64(Binary64::from_bits(bits).unwrap())
}

fn fixture() -> crate::platform::kernel::KernelSnapshot {
    let temporary = tempfile::tempdir().unwrap();
    let empty = empty_normalized_snapshot(b"binary64-runtime-vectors");
    let repository = GraphRepository::create(&temporary.path().join("numeric"), &empty, None)
        .unwrap()
        .repository;
    let base = repository.view_current().unwrap().revision();
    let mut request = format!(
        "request base={base}\ncreate.module as=$module name=numeric\n\
         type.structural-record as=@Parse\n\
         type.field parent=@Parse index=0 name=valid type=bool\n\
         type.field parent=@Parse index=1 name=value type=f64\n\
         type.structural-record as=@Integer\n\
         type.field parent=@Integer index=0 name=valid type=bool\n\
         type.field parent=@Integer index=1 name=value type=i64\n"
    );
    for (name, parameters, result) in [
        ("add", vec!["f64", "f64"], "f64"),
        ("subtract", vec!["f64", "f64"], "f64"),
        ("multiply", vec!["f64", "f64"], "f64"),
        ("divide", vec!["f64", "f64"], "f64"),
        ("negate", vec!["f64"], "f64"),
        ("abs", vec!["f64"], "f64"),
        ("sqrt", vec!["f64"], "f64"),
        ("less", vec!["f64", "f64"], "bool"),
        ("less-equal", vec!["f64", "f64"], "bool"),
        ("is-finite", vec!["f64"], "bool"),
        ("is-nan", vec!["f64"], "bool"),
        ("from-i64", vec!["i64"], "f64"),
        ("to-i64-result", vec!["f64"], "@Integer"),
        ("parse-result", vec!["text"], "@Parse"),
        ("to-text", vec!["f64"], "text"),
    ] {
        request.push_str(&format!("create.external as=${name} module=$module name={name} visibility=public result={result} implementation=core.f64.{name}\n"));
        for (index, parameter) in parameters.iter().enumerate() {
            request.push_str(&format!("add.parameter as=${name}-{index} function=${name} name=argument-{index} type={parameter}\n"));
        }
    }
    request.push_str(
        r#"expression.f64 as=$literal-value value=-0.0
create.function as=$literal module=$module name=literal visibility=public result=f64 effect=pure body=$literal-value
expression.f64 as=$ignored-result value=1
create.function as=$ignored module=$module name=ignore visibility=public result=f64 effect=pure body=$ignored-result
add.parameter as=$unused function=$ignored name=unused type=f64
type.function as=@Unary result=f64
type.argument parent=@Unary index=0 type=f64
expression.function-value as=$plus function=$add
expression.local as=$offset-value value=$offset
expression.bind as=$bound callee=$plus
expression.argument parent=$bound index=0 expression=$offset-value
create.function as=$factory module=$module name=factory visibility=public result=@Unary effect=pure body=$bound
add.parameter as=$offset function=$factory name=offset type=f64
expression.local as=$callback-value value=$callback
expression.local as=$sample-value value=$sample
expression.invoke as=$invoked function=$callback-value
expression.argument parent=$invoked index=0 expression=$sample-value
create.function as=$apply module=$module name=apply visibility=public result=f64 effect=pure body=$invoked
add.parameter as=$callback function=$apply name=callback type=@Unary
add.parameter as=$sample function=$apply name=sample type=f64
type.parameter as=@Item parameter=$item
expression.local as=$kept value=$input
create.function as=$keep module=$module name=keep visibility=public result=@Item effect=pure body=$kept
add.type-parameter as=$item declaration=$keep name=Item
add.parameter as=$input function=$keep name=input type=@Item
expression.f64 as=$generic-input value=1.25
expression.call as=$generic-result function=$keep
type.argument parent=$generic-result index=0 type=f64
expression.argument parent=$generic-result index=0 expression=$generic-input
create.function as=$generic module=$module name=generic visibility=public result=f64 effect=pure body=$generic-result
create.record as=$Box module=$module name=FloatBox visibility=public
add.field as=$number record=$Box name=number type=f64
type.named as=@Box declaration=$Box
expression.f64 as=$boxed-number value=nan
expression.record as=$boxed type=$Box
expression.record-field parent=$boxed index=0 field=$number value=$boxed-number
create.function as=$box module=$module name=box visibility=public result=@Box effect=pure body=$boxed
create.external as=$equal-f64 module=$module name=equal-f64 visibility=public result=bool implementation=core.value.equal
add.parameter as=$equal-f64-left function=$equal-f64 name=left type=f64
add.parameter as=$equal-f64-right function=$equal-f64 name=right type=f64
create.external as=$equal-box module=$module name=equal-box visibility=public result=bool implementation=core.value.equal
add.parameter as=$equal-box-left function=$equal-box name=left type=@Box
add.parameter as=$equal-box-right function=$equal-box name=right type=@Box
type.list as=@Numbers item=f64
create.external as=$equal-list module=$module name=equal-list visibility=public result=bool implementation=core.value.equal
add.parameter as=$equal-list-left function=$equal-list name=left type=@Numbers
add.parameter as=$equal-list-right function=$equal-list name=right type=@Numbers
expression.f64 as=$test-actual value=nan
expression.f64 as=$test-expected value=nan
create.test as=$test module=$module name=canonical-nan visibility=private actual=$test-actual expected=$test-expected
"#,
    );
    let decoded =
        crate::platform::control::decode_compact_change("f64-vectors", request.as_bytes()).unwrap();
    let prepared = repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap();
    assert!(matches!(
        repository.publish(&prepared.publication).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value
}

fn evaluate(
    snapshot: &crate::platform::kernel::KernelSnapshot,
    program: &NormalizedProgram,
    name: &str,
    arguments: Vec<NormalizedValue>,
) -> [NormalizedValue; 2] {
    let declaration = declaration_named(snapshot, name);
    let control = ExecutionControl::uncancelled();
    [
        NormalizedVm::new(program, NormalizedRunPolicy::foreground())
            .invoke(declaration, arguments.clone(), None, &control)
            .unwrap()
            .0,
        NormalizedReferenceInterpreter::new(snapshot, program, NormalizedRunPolicy::foreground())
            .invoke(declaration, arguments, None, &control)
            .unwrap()
            .0,
    ]
}

#[test]
fn f64_arithmetic_matches_independent_fixed_bits_and_detects_a_fault() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    // All expected bits are literal mathematical fixtures. No host computation
    // produces an expected arithmetic answer, and both evaluators meet each one.
    for (name, inputs, expected) in [
        (
            "add",
            vec![0x3ff0_0000_0000_0000, 0x3ca0_0000_0000_0000],
            0x3ff0_0000_0000_0000,
        ),
        (
            "add",
            vec![0x3ff0_0000_0000_0001, 0x3ca0_0000_0000_0000],
            0x3ff0_0000_0000_0002,
        ),
        (
            "add",
            vec![0x3fe0_0000_0000_0000, 0x3ff8_0000_0000_0000],
            0x4000_0000_0000_0000,
        ),
        (
            "subtract",
            vec![0x400c_0000_0000_0000, 0x3ff8_0000_0000_0000],
            0x4000_0000_0000_0000,
        ),
        (
            "multiply",
            vec![0x3ff8_0000_0000_0000, 0x4000_0000_0000_0000],
            0x4008_0000_0000_0000,
        ),
        (
            "divide",
            vec![0x4014_0000_0000_0000, 0x4000_0000_0000_0000],
            0x4004_0000_0000_0000,
        ),
        (
            "divide",
            vec![0x0010_0000_0000_0000, 0x4000_0000_0000_0000],
            0x0008_0000_0000_0000,
        ),
        ("divide", vec![1, 0x4000_0000_0000_0000], 0),
        ("divide", vec![3, 0x4000_0000_0000_0000], 2),
        (
            "divide",
            vec![0x8000_0000_0000_0001, 0x4000_0000_0000_0000],
            0x8000_0000_0000_0000,
        ),
        (
            "multiply",
            vec![0x7fef_ffff_ffff_ffff, 0x4000_0000_0000_0000],
            0x7ff0_0000_0000_0000,
        ),
        (
            "divide",
            vec![0x3ff0_0000_0000_0000, 0],
            0x7ff0_0000_0000_0000,
        ),
        (
            "divide",
            vec![0x3ff0_0000_0000_0000, 0x8000_0000_0000_0000],
            0xfff0_0000_0000_0000,
        ),
        ("divide", vec![0, 0], 0x7ff8_0000_0000_0000),
        (
            "subtract",
            vec![0x7ff0_0000_0000_0000, 0x7ff0_0000_0000_0000],
            0x7ff8_0000_0000_0000,
        ),
        (
            "multiply",
            vec![0x7ff0_0000_0000_0000, 0],
            0x7ff8_0000_0000_0000,
        ),
        ("negate", vec![0], 0x8000_0000_0000_0000),
        ("negate", vec![0x7ff8_0000_0000_0000], 0x7ff8_0000_0000_0000),
        ("abs", vec![0x8000_0000_0000_0000], 0),
        ("sqrt", vec![0x4000_0000_0000_0000], 0x3ff6_a09e_667f_3bcd),
        ("sqrt", vec![1], 0x1e60_0000_0000_0000),
        ("sqrt", vec![0x8000_0000_0000_0000], 0x8000_0000_0000_0000),
        ("sqrt", vec![0xbff0_0000_0000_0000], 0x7ff8_0000_0000_0000),
    ] {
        for actual in evaluate(
            &snapshot,
            &program,
            name,
            inputs.into_iter().map(scalar).collect(),
        ) {
            assert_eq!(actual, scalar(expected), "{name}");
        }
    }
    let declaration = declaration_named(&snapshot, "sqrt");
    let mut faulted = program.clone();
    let function = faulted.function(declaration).unwrap();
    Arc::make_mut(&mut faulted.functions)[function.0 as usize].body =
        NormalizedFunctionBody::External(
            ImplementationName::new("core.f64.abs".to_owned()).unwrap(),
        );
    let actual = NormalizedVm::new(&faulted, NormalizedRunPolicy::foreground())
        .invoke(
            declaration,
            vec![scalar(0x4000_0000_0000_0000)],
            None,
            &ExecutionControl::uncancelled(),
        )
        .unwrap()
        .0;
    assert_ne!(
        actual,
        scalar(0x3ff6_a09e_667f_3bcd),
        "wrong arithmetic dispatch must fail the independent sqrt(2) expectation"
    );
}

#[test]
fn f64_conversion_parsing_formatting_and_predicates_are_explicit() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let record = |valid, value| {
        NormalizedValue::Record(NormalizedRecord::Structural {
            fields: Arc::new(vec![
                (Name::new("valid").unwrap(), NormalizedValue::Bool(valid)),
                (Name::new("value").unwrap(), value),
            ]),
        })
    };
    for (number, expected) in [
        (i64::MIN, 0xc3e0_0000_0000_0000),
        (i64::MAX, 0x43e0_0000_0000_0000),
        (9_007_199_254_740_993, 0x4340_0000_0000_0000),
        (9_007_199_254_740_995, 0x4340_0000_0000_0002),
    ] {
        for actual in evaluate(
            &snapshot,
            &program,
            "from-i64",
            vec![NormalizedValue::I64(number)],
        ) {
            assert_eq!(actual, scalar(expected));
        }
    }
    for (bits, valid, expected) in [
        (0xc3e0_0000_0000_0000, true, i64::MIN),
        (0x43df_ffff_ffff_ffff, true, 9_223_372_036_854_774_784),
        (0x43e0_0000_0000_0000, false, 0),
        (0xc3e0_0000_0000_0001, false, 0),
        (0x7ff8_0000_0000_0000, false, 0),
        (0x7ff0_0000_0000_0000, false, 0),
        (0xbffc_0000_0000_0000, true, -1),
    ] {
        for actual in evaluate(&snapshot, &program, "to-i64-result", vec![scalar(bits)]) {
            assert_eq!(actual, record(valid, NormalizedValue::I64(expected)));
        }
    }
    for (text, bits) in [
        ("0.1", 0x3fb9_9999_9999_999a),
        ("-0", 0x8000_0000_0000_0000),
        ("1e0", 0x3ff0_0000_0000_0000),
        (
            "1.00000000000000011102230246251565404236316680908203125",
            0x3ff0_0000_0000_0000,
        ),
        ("-1e-400", 0x8000_0000_0000_0000),
        ("nan", 0x7ff8_0000_0000_0000),
        ("inf", 0x7ff0_0000_0000_0000),
        ("-inf", 0xfff0_0000_0000_0000),
    ] {
        for actual in evaluate(
            &snapshot,
            &program,
            "parse-result",
            vec![NormalizedValue::text(text)],
        ) {
            assert_eq!(actual, record(true, scalar(bits)), "{text}");
        }
    }
    for text in [
        "1e400", "+1", " 1", "1 ", "0x1", "1_0", "NaN", "-nan", "01", "1.", ".5", "1e", "1e+",
    ] {
        for actual in evaluate(
            &snapshot,
            &program,
            "parse-result",
            vec![NormalizedValue::text(text)],
        ) {
            assert_eq!(actual, record(false, scalar(0)), "{text}");
        }
    }
    for (bits, expected) in [
        (0x8000_0000_0000_0000, "-0.0"),
        (0x7ff8_0000_0000_0000, "nan"),
        (0x7ff0_0000_0000_0000, "inf"),
        (0xfff0_0000_0000_0000, "-inf"),
    ] {
        for actual in evaluate(&snapshot, &program, "to-text", vec![scalar(bits)]) {
            assert_eq!(actual, NormalizedValue::text(expected));
        }
    }
    for (name, arguments, expected) in [
        (
            "less",
            vec![scalar(0x7ff8_0000_0000_0000), scalar(0)],
            false,
        ),
        (
            "less-equal",
            vec![scalar(0), scalar(0x7ff8_0000_0000_0000)],
            false,
        ),
        (
            "less",
            vec![scalar(0x8000_0000_0000_0000), scalar(0)],
            false,
        ),
        (
            "less-equal",
            vec![scalar(0x8000_0000_0000_0000), scalar(0)],
            true,
        ),
        ("is-finite", vec![scalar(1)], true),
        ("is-finite", vec![scalar(0x7ff0_0000_0000_0000)], false),
        ("is-nan", vec![scalar(0x7ff8_0000_0000_0000)], true),
        ("is-nan", vec![scalar(0x7ff0_0000_0000_0000)], false),
    ] {
        for actual in evaluate(&snapshot, &program, name, arguments) {
            assert_eq!(actual, NormalizedValue::Bool(expected), "{name}");
        }
    }
}

#[test]
fn f64_numeric_equality_and_observation_remain_distinct_recursively() {
    let nan = scalar(0x7ff8_0000_0000_0000);
    let origin = super::super::value::ValueOrigin::fresh().unwrap();
    let list = NormalizedValue::list(vec![nan.clone()]).unwrap();
    let nominal = NormalizedValue::Record(NormalizedRecord::Nominal {
        layout: RecordLayoutIndex(0, origin),
        fields: Arc::new(vec![nan.clone()]),
    });
    for value in [nan, list, nominal] {
        assert_eq!(value, value.clone());
        assert!(!super::super::vm::normalized_equal(&value, &value).unwrap());
        assert!(!super::super::reference::reference_equal(&value, &value).unwrap());
        assert!(super::super::vm::normalized_observation_equal(&value, &value).unwrap());
        assert!(super::super::reference::reference_observation_equal(&value, &value).unwrap());
    }
    let positive = NormalizedValue::list(vec![scalar(0)]).unwrap();
    let negative = NormalizedValue::list(vec![scalar(0x8000_0000_0000_0000)]).unwrap();
    assert!(super::super::vm::normalized_equal(&positive, &negative).unwrap());
    assert!(super::super::reference::reference_equal(&positive, &negative).unwrap());
    assert!(!super::super::vm::normalized_observation_equal(&positive, &negative).unwrap());
    assert!(!super::super::reference::reference_observation_equal(&positive, &negative).unwrap());
    let callable = NormalizedValue::Function {
        function: super::super::value::FunctionIndex(0, origin),
        type_arguments: Arc::from([]),
        effect_arguments: Arc::from([]),
        requirement_arguments: Arc::from([]),
        bound_arguments: None,
    };
    let nested = NormalizedValue::list(vec![callable]).unwrap();
    for other in [&nested, &NormalizedValue::list(vec![]).unwrap()] {
        assert!(super::super::vm::normalized_observation_equal(&nested, other).is_err());
        assert!(super::super::reference::reference_observation_equal(&nested, other).is_err());
    }
    assert!(NormalizedMapKey::from_value(scalar(0)).is_none());
}

#[test]
fn f64_literals_generics_bound_captures_and_graph_observation_use_ordinary_paths() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    for actual in evaluate(&snapshot, &program, "literal", vec![]) {
        assert_eq!(actual, scalar(0x8000_0000_0000_0000));
    }
    for actual in evaluate(&snapshot, &program, "generic", vec![]) {
        assert_eq!(actual, scalar(0x3ff4_0000_0000_0000));
    }
    let captures = evaluate(
        &snapshot,
        &program,
        "factory",
        vec![scalar(0x3fe0_0000_0000_0000)],
    );
    for capture in captures {
        for actual in evaluate(
            &snapshot,
            &program,
            "apply",
            vec![capture.clone(), scalar(0x3ff8_0000_0000_0000)],
        ) {
            assert_eq!(actual, scalar(0x4000_0000_0000_0000));
        }
    }
    for boxed in evaluate(&snapshot, &program, "box", vec![]) {
        for equal in evaluate(
            &snapshot,
            &program,
            "equal-box",
            vec![boxed.clone(), boxed.clone()],
        ) {
            assert_eq!(equal, NormalizedValue::Bool(false));
        }
        assert!(!super::super::vm::normalized_equal(&boxed, &boxed).unwrap());
        assert!(!super::super::reference::reference_equal(&boxed, &boxed).unwrap());
        assert!(boxed.is_durable());
    }
    for (left, right, expected) in [
        (
            scalar(0x7ff8_0000_0000_0000),
            scalar(0x7ff8_0000_0000_0000),
            false,
        ),
        (scalar(0), scalar(0x8000_0000_0000_0000), true),
    ] {
        for equal in evaluate(&snapshot, &program, "equal-f64", vec![left, right]) {
            assert_eq!(equal, NormalizedValue::Bool(expected));
        }
    }
    let list = NormalizedValue::list(vec![scalar(0x7ff8_0000_0000_0000)]).unwrap();
    for equal in evaluate(&snapshot, &program, "equal-list", vec![list.clone(), list]) {
        assert_eq!(equal, NormalizedValue::Bool(false));
    }
    let control = ExecutionControl::uncancelled();
    let ignored = declaration_named(&snapshot, "ignore");
    assert!(
        NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
            .invoke(ignored, vec![NormalizedValue::I64(1)], None, &control)
            .is_err()
    );
    assert!(
        NormalizedReferenceInterpreter::new(&snapshot, &program, NormalizedRunPolicy::foreground())
            .invoke(ignored, vec![NormalizedValue::I64(1)], None, &control)
            .is_err()
    );
    assert_eq!(
        run_graph_tests(
            &snapshot,
            &program,
            None,
            NormalizedRunPolicy::foreground(),
            &control
        )
        .unwrap()
        .passed,
        1
    );
    let mut wrong_zero = snapshot.clone();
    let test = declaration_named(&wrong_zero, "canonical-nan");
    let OwnerRecord::Declaration(declaration) =
        &wrong_zero.owners[&OwnerKey::Declaration(test.declaration)]
    else {
        panic!("test declaration")
    };
    let DeclarationPayload::Test {
        actual, expected, ..
    } = declaration.payload
    else {
        panic!("test payload")
    };
    for (expression, bits) in [(actual, 0), (expected, 0x8000_0000_0000_0000)] {
        let OwnerRecord::Expression(record) = wrong_zero
            .owners
            .get_mut(&OwnerKey::Expression(expression))
            .unwrap()
        else {
            panic!("test expression")
        };
        record.operation = ExpressionOperation::F64 {
            value: Binary64::from_bits(bits).unwrap(),
        };
    }
    let wrong_program = prepare_snapshot(&wrong_zero);
    assert_eq!(
        run_graph_tests(
            &wrong_zero,
            &wrong_program,
            None,
            NormalizedRunPolicy::foreground(),
            &control
        )
        .unwrap_err()
        .code,
        "normalized_test_failed"
    );
}
