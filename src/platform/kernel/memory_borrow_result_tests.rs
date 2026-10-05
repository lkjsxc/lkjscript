//! Borrowed results preserve one exact input's provenance through lexical calls.
use super::*;

const SOURCE: &str = r#"declarations.begin
(units (module create borrowed-results
  (external create length (visibility public) (implementation core.buffer.length)
    (parameter create b (type ByteBuffer) (use borrow)) (returns I64))
  (external create discard (visibility private) (implementation core.buffer.discard)
    (parameter create b (type ByteBuffer) (use consume)) (returns Unit))
  (function create root (visibility public) (effect pure)
    (parameter create b (type ByteBuffer) (use borrow))
    (returns ByteBuffer (borrow-from b)) (body (local b)))
  (function create child (visibility public) (effect pure)
    (parameter create packet (type (owned-product (field payload ByteBuffer))) (use borrow))
    (returns ByteBuffer (borrow-from packet))
    (body (borrow-owned-field (type (owned-product (field payload ByteBuffer))) (local packet)
      (field payload (binding view (type ByteBuffer))) (in (local view)))))
  (function create forward (visibility public) (effect pure)
    (parameter create packet (type (owned-product (field payload ByteBuffer))) (use borrow))
    (returns ByteBuffer (borrow-from packet))
    (body (borrow-call (call child (local packet))
      (binding view (type ByteBuffer))
      (in (borrow-call (call root (local view))
        (binding forwarded (type ByteBuffer)) (in (local forwarded)))))))
  (function create choose (visibility public) (effect pure)
    (parameter create packet
      (type (owned-product (field left ByteBuffer) (field right ByteBuffer))) (use borrow))
    (returns ByteBuffer (borrow-from packet))
    (body (if (bool true)
      (borrow-owned-field (type (owned-product (field left ByteBuffer) (field right ByteBuffer)))
        (local packet) (field left (binding left (type ByteBuffer))) (in (local left)))
      (borrow-owned-field (type (owned-product (field left ByteBuffer) (field right ByteBuffer)))
        (local packet) (field right (binding right (type ByteBuffer))) (in (local right))))))
  (function create nested (visibility public) (effect pure)
    (parameter create packet
      (type (owned-product (field inner (owned-product (field payload ByteBuffer))))) (use borrow))
    (returns ByteBuffer (borrow-from packet))
    (body (borrow-owned-field
      (type (owned-product (field inner (owned-product (field payload ByteBuffer)))))
      (local packet) (field inner (binding inner (type (owned-product (field payload ByteBuffer)))))
      (in (borrow-owned-field (type (owned-product (field payload ByteBuffer)))
        (local inner) (field payload (binding view (type ByteBuffer))) (in (local view)))))))
  (function create generic-child (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create packet (type (owned-product (field payload T))) (use borrow))
    (returns T (borrow-from packet))
    (body (borrow-owned-field (type (owned-product (field payload T))) (local packet)
      (field payload (binding view (type T))) (in (local view)))))
  (function create generic-forward (visibility public) (effect pure)
    (parameter create packet (type (owned-product (field payload ByteBuffer))) (use borrow))
    (returns ByteBuffer (borrow-from packet))
    (body (borrow-call (call generic-child (types ByteBuffer) (local packet))
      (binding view (type ByteBuffer)) (in (local view)))))
  (function create generic-relay (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (parameter create packet (type (owned-product (field payload T))) (use borrow))
    (returns T (borrow-from packet))
    (body (borrow-call (call generic-child (types T) (local packet))
      (binding view (type T)) (in (local view)))))
  (function create unrelated-scoped (visibility public) (effect pure)
    (parameter create b (type ByteBuffer) (use borrow))
    (parameter create other (type (owned-product (field payload ByteBuffer))) (use consume))
    (returns ByteBuffer (borrow-from b))
    (body (let
      (binding temporary (type (owned-product (field payload ByteBuffer))) (local other))
      (in (borrow-owned-field (type (owned-product (field payload ByteBuffer)))
        (local temporary) (field payload (binding view (type ByteBuffer))) (in (local b)))))))
  (function create observe (visibility public) (effect pure)
    (parameter create b (type ByteBuffer) (use consume)) (returns Unit)
    (body (sequence
      (borrow-call (call root (local b)) (binding view (type ByteBuffer))
        (in (call length (local view))))
      (call discard (local b)))))))
declarations.end"#;

fn author(source: &str) -> Result<KernelSnapshot, String> {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(source)
}

#[test]
fn exact_input_root_descendants_and_forwarding_are_admitted() {
    let source = author(SOURCE).unwrap();
    assert!(validate_full(&source).is_ok());
    assert!(memory_reference::accepts(&source));
}

#[test]
fn borrowed_result_rejects_wrong_input_including_untaken_return() {
    for body in ["(local other)", "(if (bool false) (local other) (local b))"] {
        let candidate = SOURCE.replace(
            "(returns ByteBuffer (borrow-from b)) (body (local b))",
            &format!("(parameter create other (type ByteBuffer) (use borrow))\n    (returns ByteBuffer (borrow-from b)) (body {body})"),
        ).replace("(call root (local view))", "(call root (local view) (local view))")
         .replace("(call root (local b))", "(call root (local b) (local b))");
        let error = author(&candidate).unwrap_err();
        assert!(error.contains("kernel_buffer_ownership"), "{body}: {error}");
    }
}

#[test]
fn borrowed_result_rejects_owning_sources_and_ordinary_call_escape() {
    for candidate in [
        SOURCE.replacen("(parameter create b (type ByteBuffer) (use borrow))\n    (returns ByteBuffer (borrow-from b))", "(parameter create b (type ByteBuffer) (use consume))\n    (returns ByteBuffer (borrow-from b))", 1),
        SOURCE.replace("(borrow-call (call root (local b)) (binding view (type ByteBuffer))\n        (in (call length (local view))))", "(let (binding saved (type ByteBuffer) (call root (local b))) (in (call length (local saved))))"),
        SOURCE.replace("(in (call length (local view))))\n      (call discard", "(in (sequence (call discard (local b)) (call length (local view)))))\n      (call discard"),
        SOURCE.replace("(returns ByteBuffer (borrow-from b)) (body (local b))", "(returns ByteBuffer (borrow-from b)) (body (call root (local b)))"),
    ] {
        let error = author(&candidate).unwrap_err();
        assert!(error.contains("kernel_buffer_ownership"), "{error}");
    }
}

#[test]
fn provenance_uses_parameter_identity_even_when_both_sources_have_equal_types() {
    let source = author(SOURCE).unwrap();
    let root = source
        .owners
        .values()
        .find_map(|owner| {
            let OwnerRecord::Declaration(declaration) = owner else {
                return None;
            };
            if declaration.name.as_str() != "root" {
                return None;
            }
            let DeclarationPayload::Function(function) = &declaration.payload else {
                return None;
            };
            function.result_borrow
        })
        .unwrap();
    let wrong =
        crate::platform::semantic_id::ParameterId::migrate(b"borrowed-result-wrong-source", 0);
    let mut state = State::from([
        (
            LocalValueReference::FunctionParameter(root),
            Slot::owner(true),
        ),
        (
            LocalValueReference::FunctionParameter(wrong),
            Slot::owner(true),
        ),
    ]);
    let expression = source
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Expression(ExpressionRecord {
                id,
                operation:
                    ExpressionOperation::Local {
                        value: LocalValueReference::FunctionParameter(parameter),
                    },
                ..
            }) if *parameter == root => Some(*id),
            _ => None,
        })
        .unwrap();
    let check = Check {
        read: &source,
        scope: None,
    };
    assert!(
        check
            .eval(
                expression,
                &mut state,
                Mode::ReturnBorrow(LocalValueReference::FunctionParameter(root)),
                0
            )
            .is_ok()
    );
    assert!(
        check
            .eval(
                expression,
                &mut state,
                Mode::ReturnBorrow(LocalValueReference::FunctionParameter(wrong)),
                0
            )
            .is_err()
    );
}

const METHOD_SOURCE: &str = r#"declarations.begin
(units (module create borrowed-methods
  (owned-contract create Reader (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_ab000000000000000000000000000001 select
      (parameters (Self borrow) (Self borrow))
      (returns Self (borrow-from 0))))
  (function create select (visibility public) (effect pure)
    (parameter create first (type ByteBuffer) (use borrow))
    (parameter create second (type ByteBuffer) (use borrow))
    (returns ByteBuffer (borrow-from first)) (body (local first)))
  (owned-implementation create BufferReader (visibility public)
    (contract Reader) (self ByteBuffer)
    (method method_ab000000000000000000000000000001 select))
  (external create length (visibility public) (implementation core.buffer.length)
    (parameter create b (type ByteBuffer) (use borrow)) (returns I64))
  (function create observe (visibility public) (effect pure)
    (parameter create b (type ByteBuffer) (use borrow)) (returns I64)
    (body (borrow-call
      (method-call concrete@BufferReader Reader method_ab000000000000000000000000000001
        (local b) (local b))
      (binding view (type ByteBuffer)) (in (call length (local view))))))))
declarations.end"#;

#[test]
fn method_result_source_must_match_exact_parameter_position() {
    let source = author(METHOD_SOURCE).unwrap();
    assert!(validate_full(&source).is_ok());
    assert!(memory_reference::accepts(&source));
    for candidate in [
        METHOD_SOURCE.replace(
            "(returns Self (borrow-from 0))",
            "(returns Self (borrow-from 1))",
        ),
        METHOD_SOURCE.replace(
            "(returns Self (borrow-from 0))",
            "(returns Self (borrow-from 2))",
        ),
        METHOD_SOURCE.replace(
            "(returns ByteBuffer (borrow-from first)) (body (local first))",
            "(returns ByteBuffer (borrow-from second)) (body (local second))",
        ),
        METHOD_SOURCE.replace(
            "(returns ByteBuffer (borrow-from first)) (body (local first))",
            "(returns ByteBuffer) (body (local first))",
        ),
    ] {
        let error = author(&candidate).unwrap_err();
        assert!(
            error.contains("kernel_owned_contract") || error.contains("kernel_buffer_ownership"),
            "{error}"
        );
    }
}

#[test]
fn borrowed_result_cannot_be_claimed_by_a_non_view_expression() {
    let mut source = author(SOURCE).unwrap();
    let unit = source
        .owners
        .values_mut()
        .find_map(|owner| match owner {
            OwnerRecord::Expression(expression) => {
                expression.operation = ExpressionOperation::Unit {};
                Some(expression.id)
            }
            _ => None,
        })
        .unwrap();
    // A low-level oracle cannot admit an ordinary result merely because it carries no owner.
    let check = Check {
        read: &source,
        scope: None,
    };
    let nominated = LocalValueReference::FunctionParameter(
        crate::platform::semantic_id::ParameterId::migrate(b"ordinary-result-nomination", 0),
    );
    assert!(
        check
            .eval(unit, &mut State::new(), Mode::ReturnBorrow(nominated), 0)
            .is_err()
    );
}
