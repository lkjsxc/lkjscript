//! Untaken type/lexical checks and raw entry/exit remain independent of case selection.
use super::*;
use crate::platform::kernel::{ExpressionOperation, LocalValueReference};

#[test]
fn owned_choice_same_typed_arms_cannot_share_or_reference_foreign_payload_bindings() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/owned-choices.lkjc"
    ))
    .unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let (_, arms) = source
        .owners
        .iter()
        .find_map(|(id, owner)| {
            let OwnerRecord::Expression(e) = owner else {
                return None;
            };
            let ExpressionOperation::MatchOwned {
                choice_type, arms, ..
            } = &e.operation
            else {
                return None;
            };
            let TypeForm::OwnedChoice { cases } = &source.types[choice_type].form else {
                return None;
            };
            if cases.len() == 2
                && cases[0].ty == cases[1].ty
                && matches!(
                    source.types[&cases[0].ty].form,
                    TypeForm::TypeParameter { .. }
                )
            {
                Some((*id, arms.clone()))
            } else {
                None
            }
        })
        .unwrap();
    for duplicate in [false, true] {
        let mut changed = source.clone();
        if duplicate {
            let owner = changed.owners.values_mut().find(|owner| matches!(owner,
                OwnerRecord::Expression(e) if matches!(&e.operation, ExpressionOperation::MatchOwned { arms: other, .. } if other == &arms))).unwrap();
            let OwnerRecord::Expression(e) = owner else {
                unreachable!();
            };
            let ExpressionOperation::MatchOwned { arms, .. } = &mut e.operation else {
                unreachable!();
            };
            arms[1].binding = arms[0].binding;
        } else {
            let OwnerRecord::Expression(body) = changed
                .owners
                .get_mut(&OwnerKey::Expression(arms[1].body))
                .unwrap()
            else {
                panic!("choice arm body");
            };
            assert!(matches!(body.operation, ExpressionOperation::Local { .. }));
            body.operation = ExpressionOperation::Local {
                value: LocalValueReference::LexicalBinding(arms[0].binding),
            };
        }
        let errors = crate::platform::kernel::validate_full(&changed).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.class == crate::platform::diagnostic::DiagnosticClass::Semantic),
            "{errors:?}"
        );
        assert!(!crate::platform::kernel::memory_reference::accepts(
            &changed
        ));
    }
}

#[test]
fn owned_choice_rejects_open_ordinary_unsafe_untaken_and_recursive_growing_types() {
    for (parameters, ty) in [
        (
            "",
            "(owned-choice (case accepted Unit) (case rejected I64))",
        ),
        (
            "",
            "(owned-choice (case accepted ByteBuffer) (case rejected Secret))",
        ),
        (
            "",
            "(owned-choice (case accepted ByteBuffer) (case rejected (function () I64)))",
        ),
        (
            "",
            "(owned-choice (case accepted ByteBuffer) (case rejected (option ByteBuffer)))",
        ),
        (
            "(type-parameter create T)",
            "(owned-choice (case accepted ByteBuffer) (case rejected T))",
        ),
    ] {
        let input = format!("declarations.begin\n(units (module create invalid
(function create relay (visibility private) (effect pure) {parameters}
(parameter create p (type {ty}) (use consume)) (returns {ty}) (body (local p)))))\ndeclarations.end");
        let error = byte_buffer_tests::author_only(&input).unwrap_err();
        assert!(error.contains("kernel_owned_choice"), "{error}");
    }
    let grow = r#"declarations.begin
(units (module create invalid
  (function create grow (visibility private) (effect pure)
    (type-parameter create T (constraint owned)) (parameter create p (type T) (use consume))
    (returns Unit)
    (body (let
      (binding wrapper (type (owned-choice (case p T)))
        (choose-owned (type (owned-choice (case p T))) (case p) (local p)))
      (in (call grow (types (owned-choice (case p T))) (local wrapper))))))))
declarations.end"#;
    let error = byte_buffer_tests::author_only(grow).unwrap_err();
    assert!(error.contains("kernel_callable"), "{error}");
}

#[test]
fn owned_choice_raw_unused_inputs_ordinary_selected_results_and_codecs_reject_ownership() {
    use super::super::{byte_buffer::ByteBuffer, owned_choice::OwnedChoice};
    let source = byte_buffer_tests::author_only(r#"declarations.begin
(units (module create boundary
  (function create factory (visibility private) (effect pure)
    (returns (owned-choice (case accepted Unit) (case rejected ByteBuffer)))
    (body (choose-owned (type (owned-choice (case accepted Unit) (case rejected ByteBuffer))) (case accepted) (unit))))
  (function create ignore (visibility private) (effect pure)
    (parameter create p (type (owned-choice (case accepted Unit) (case rejected ByteBuffer))) (use consume))
    (returns Unit) (body (unit)))))
declarations.end"#).unwrap();
    let program = prepare_snapshot(&source);
    let factory = declaration_named(&source, "factory");
    let ignore = declaration_named(&source, "ignore");
    let OwnerRecord::Declaration(d) = &source.owners[&OwnerKey::Declaration(factory.declaration)]
    else {
        panic!("factory");
    };
    let DeclarationPayload::Function(f) = &d.payload else {
        panic!("factory function");
    };
    let ty = f.result;
    let control = ExecutionControl::uncancelled();
    for reference in [false, true] {
        let composites = super::super::owned_product::StorageObservation::start();
        let buffers = super::super::byte_buffer::StorageObservation::start();
        let run = |entry, arguments| {
            if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(entry, arguments, None, &control)
                .map(|r| r.0)
            } else {
                NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                    .invoke(entry, arguments, None, &control)
                    .map(|r| r.0)
            }
        };
        assert!(
            run(factory, vec![]).is_err(),
            "ordinary selected case must not export an affine choice"
        );
        for case in [0, 1] {
            for mode in ["live", "inert", "borrow", "stale"] {
                let origin = program.value_origin;
                let payload = if case == 0 {
                    NormalizedValue::Unit
                } else {
                    NormalizedValue::ByteBuffer(
                        ByteBuffer::create(origin, &control, &mut |_| Ok(())).unwrap(),
                    )
                };
                let owner =
                    OwnedChoice::create(origin, ty, case, payload, &control, &mut |_| Ok(()))
                        .unwrap();
                let (token, retained) = match mode {
                    "inert" => (owner.clone(), Some(owner)),
                    "borrow" => (owner.borrow().unwrap(), Some(owner)),
                    "stale" => {
                        let marker = owner.clone();
                        drop(owner);
                        (marker, None)
                    }
                    _ => (owner, None),
                };
                let raw = NormalizedValue::OwnedChoice(token);
                assert!(
                    super::super::codec::encode_typed(
                        &program,
                        &raw,
                        ty,
                        crate::platform::json::JsonLimits::default()
                    )
                    .is_err()
                );
                assert!(super::super::data_codec::encode_typed(&program, &raw, ty).is_err());
                assert!(
                    super::super::data_codec_reference::encode_typed(&program, &raw, ty).is_err()
                );
                assert!(
                    run(ignore, vec![raw]).is_err(),
                    "unused exact-typed raw input cannot confer ownership"
                );
                drop(retained);
                assert_eq!(composites.live(), (0, 0));
                assert_eq!(buffers.live(), (0, 0));
            }
        }
        for input in [b"null".as_slice(), b"{}", b"{\"accepted\":null}"] {
            assert!(
                super::super::codec::decode_typed(
                    &program,
                    input,
                    ty,
                    crate::platform::json::JsonLimits::default()
                )
                .is_err()
            );
            assert!(super::super::data_codec::decode_typed(&program, input, ty).is_err());
            assert!(super::super::data_codec_reference::decode_typed(&program, input, ty).is_err());
        }
    }
}
