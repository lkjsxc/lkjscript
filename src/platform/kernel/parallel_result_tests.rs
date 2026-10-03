//! Parallel result affinity is derived from the declared closed child results.
use super::*;

const OWNED: &str = r#"declarations.begin
(units (module create returned-pair
  (function create child (visibility public)
    (parameter create payload (type ByteBuffer) (use consume))
    (returns ByteBuffer) (effect (task)) (body (local payload)))
  (function create scalar (visibility public)
    (returns I64) (effect (task)) (body (i64 19)))
  (function create pair (visibility public)
    (parameter create a (type ByteBuffer) (use consume))
    (parameter create b (type ByteBuffer) (use consume))
    (returns (owned-product (field left ByteBuffer) (field right ByteBuffer)))
    (effect (task))
    (body (parallel (call child (local a)) (call child (local b)))))))
declarations.end
"#;

#[test]
fn parallel_result_affinity_is_exact_for_both_mixed_orientations_and_owned_pairs() {
    for (body, result, expected) in [
        (
            "(parallel (call child (local a)) (call child (local b)))",
            "(owned-product (field left ByteBuffer) (field right ByteBuffer))",
            [true, true],
        ),
        (
            "(parallel (call scalar) (call child (local b)))",
            "(owned-product (field left I64) (field right ByteBuffer))",
            [false, true],
        ),
        (
            "(parallel (call child (local a)) (call scalar))",
            "(owned-product (field left ByteBuffer) (field right I64))",
            [true, false],
        ),
        (
            "(parallel (call scalar) (call scalar))",
            "(record (left I64) (right I64))",
            [false, false],
        ),
    ] {
        let literal = OWNED
            .replace(
                "(parallel (call child (local a)) (call child (local b)))",
                body,
            )
            .replace(
                "(owned-product (field left ByteBuffer) (field right ByteBuffer))",
                result,
            );
        let source = author(&literal).unwrap();
        assert!(memory_reference::accepts(&source), "{literal}");
        let (left, right) = branches(&source);
        for (expression, owned) in [left, right].into_iter().zip(expected) {
            assert_eq!(
                parallel::admit_call(&source, expression)
                    .unwrap()
                    .result_owned,
                owned
            );
        }
    }
}

#[test]
fn parallel_returned_ownership_cannot_be_erased_or_duplicate_an_input() {
    for invalid in [
        OWNED.replace(
            "(owned-product (field left ByteBuffer) (field right ByteBuffer))",
            "(record (left ByteBuffer) (right ByteBuffer))",
        ),
        OWNED.replace("(call child (local b))", "(call child (local a))"),
        OWNED.replace(
            "(body (parallel (call child (local a)) (call child (local b))))",
            "(body (sequence (parallel (call child (local a)) (call child (local b))) (local a)))",
        ),
    ] {
        assert!(
            author(&invalid).is_err(),
            "invalid owner flow accepted: {invalid}"
        );
    }
}

#[test]
fn parallel_result_admission_checks_unselected_owned_choice_cases() {
    let mut source = author(OWNED).unwrap();
    let (left, _) = branches(&source);
    let call = parallel::admit_call(&source, left).unwrap();
    let secret = TypeObject::new(TypeForm::Secret).unwrap();
    let secret_type = encode_type_object(&secret).unwrap().0;
    source.types.insert(secret_type, secret);
    let object = TypeObject::new(TypeForm::OwnedChoice {
        cases: vec![
            StructuralTypeField {
                name: Name::new("accepted").unwrap(),
                ty: call.result,
            },
            StructuralTypeField {
                name: Name::new("hidden").unwrap(),
                ty: secret_type,
            },
        ],
    })
    .unwrap();
    let result = encode_type_object(&object).unwrap().0;
    source.types.insert(result, object);
    let OwnerRecord::Declaration(declaration) = source
        .owners
        .get_mut(&OwnerKey::Declaration(call.function.declaration))
        .unwrap()
    else {
        panic!("child declaration");
    };
    let DeclarationPayload::Function(function) = &mut declaration.payload else {
        panic!("child function");
    };
    function.result = result;
    assert!(parallel::admit_call(&source, left).is_err());
    assert!(!memory_reference::accepts(&source));
}
