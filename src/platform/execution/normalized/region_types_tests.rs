//! Region kind correspondence from independently read complete canonical syntax.
use super::super::reference::read_view_admission::Proof;
use super::*;
use crate::platform::kernel::{OwnerRecord, TypeParameterConstraints, encode_type_object};

const SOURCE: &str = include_str!("../../../../tests/fixtures/region-kinds.lkjc");
const SHARED: &str = include_str!("../../../../tests/fixtures/region-shared.lkjc");
const EXPRESSIONS: &str = include_str!("../../../../tests/fixtures/region-expressions.lkjc");

#[test]
fn region_planner_witness_traverses_cycles_and_shares_frozen_stages() {
    let input = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples/dependency-plan/region-witness.lkjc"),
    )
    .unwrap();
    let source = byte_buffer_tests::author_only(&input).unwrap();
    let program = prepare_snapshot(&source);
    let declaration = declaration_named(&source, "region-plan-witness");
    let policy = NormalizedRunPolicy::foreground();
    let control = ExecutionControl::uncancelled();
    let (vm, observation) = NormalizedVm::for_test(&program, policy)
        .invoke(declaration, vec![], None, &control)
        .unwrap();
    let (reference, _) = NormalizedReferenceInterpreter::new(&source, &program, policy)
        .invoke(declaration, vec![], None, &control)
        .unwrap();
    assert_eq!(vm, reference);
    let NormalizedValue::Record(super::super::value::NormalizedRecord::Structural { fields }) = vm
    else {
        panic!("pair");
    };
    assert_eq!(
        fields
            .iter()
            .map(|(name, value)| (name.as_str(), value.clone()))
            .collect::<Vec<_>>(),
        vec![
            ("left", NormalizedValue::I64(2)),
            ("right", NormalizedValue::I64(2))
        ]
    );
    assert_eq!(observation.parallel_scopes, 1);
}

#[test]
fn region_frozen_parallel_reads_share_the_lexical_arena() {
    let input = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/region-parallel.lkjc"),
    )
    .unwrap();
    let source = byte_buffer_tests::author_only(&input).unwrap();
    let program = prepare_snapshot(&source);
    for (name, left, right) in [("frozen-pair", 57, 57), ("unique-pair", 61, 5)] {
        let declaration = declaration_named(&source, name);
        let policy = NormalizedRunPolicy::foreground();
        let (vm, observation) = NormalizedVm::for_test(&program, policy)
            .invoke(declaration, vec![], None, &ExecutionControl::uncancelled())
            .unwrap_or_else(|error| panic!("VM {name}: {error:?}"));
        let (reference, _) = NormalizedReferenceInterpreter::new(&source, &program, policy)
            .invoke(declaration, vec![], None, &ExecutionControl::uncancelled())
            .unwrap_or_else(|error| panic!("reference {name}: {error:?}"));
        assert_eq!(vm, reference);
        let NormalizedValue::Record(super::super::value::NormalizedRecord::Structural { fields }) =
            vm
        else {
            panic!("pair");
        };
        assert_eq!(
            fields
                .iter()
                .map(|(name, value)| (name.as_str(), value.clone()))
                .collect::<Vec<_>>(),
            vec![
                ("left", NormalizedValue::I64(left)),
                ("right", NormalizedValue::I64(right))
            ]
        );
        assert_eq!(observation.parallel_scopes, 1);
        assert_eq!(
            observation.parallel_worker_dispatches + observation.parallel_inline_fallbacks,
            1
        );
    }
}

#[test]
fn region_expressions_execute_from_public_authoring_in_both_evaluators() {
    let source = byte_buffer_tests::author_only(EXPRESSIONS).unwrap();
    let control = ExecutionControl::uncancelled();
    Proof::reconstruct(&[&source], &control).unwrap();
    let program = prepare_snapshot(&source);
    for (name, expected) in [("scalar", 42), ("frozen", 29), ("frozen-call", 73)] {
        let declaration = declaration_named(&source, name);
        let policy = NormalizedRunPolicy::foreground();
        let vm = NormalizedVm::for_test(&program, policy)
            .invoke(declaration, vec![], None, &control)
            .unwrap()
            .0;
        let reference = NormalizedReferenceInterpreter::new(&source, &program, policy)
            .invoke(declaration, vec![], None, &control)
            .unwrap()
            .0;
        assert_eq!(vm, NormalizedValue::I64(expected));
        assert_eq!(reference, vm);
    }
}

fn check_reference_properties(
    source: &crate::platform::kernel::KernelSnapshot,
    program: &NormalizedProgram,
) {
    let schema = super::super::NormalizedReferenceSchema::reconstruct([source]).unwrap();
    for (actual, expected) in [
        (&program.ordinary_types, &schema.ordinary_types),
        (&program.buffer_free_types, &schema.buffer_free_types),
        (&program.capture_safe_types, &schema.capture_safe_types),
        (&program.comparable_types, &schema.comparable_types),
        (
            &program.application_free_types,
            &schema.application_free_types,
        ),
    ] {
        for (ty, object) in &source.types {
            assert_eq!(
                actual.contains(ty),
                expected.contains(ty),
                "{:?}",
                object.form
            );
        }
    }
}

#[test]
fn region_capabilities_retain_owning_or_scoped_categories_in_prepared_functions() {
    for mode in ["unique", "frozen", "pinned"] {
        let text = format!(
            r#"declarations.begin
(units (module create region-owner
  (function create move-cap (visibility public) (effect pure)
    (type-parameter create R (constraint region))
    (parameter create cap (type (region-cap R {mode})) (use consume))
    (returns (region-cap R {mode})) (body (local cap)))))
declarations.end"#
        );
        let snapshot = byte_buffer_tests::author_only(&text).unwrap();
        Proof::reconstruct(&[&snapshot], &ExecutionControl::uncancelled()).unwrap();
        let program = prepare_snapshot(&snapshot);
        check_reference_properties(&snapshot, &program);
        assert!(
            program
                .functions
                .iter()
                .any(|f| f.parameters.iter().any(|p| program.is_owned_type(p.ty)))
        );
    }
    let snapshot = byte_buffer_tests::author_only(SHARED).unwrap();
    Proof::reconstruct(&[&snapshot], &ExecutionControl::uncancelled()).unwrap();
    for consume in [true, false] {
        let mut corrupted = snapshot.clone();
        for owner in corrupted.owners.values_mut() {
            match owner {
                OwnerRecord::Parameter(p) if consume => {
                    p.use_mode = crate::platform::kernel::ParameterUse::Consume
                }
                OwnerRecord::Declaration(d) if !consume => {
                    if let crate::platform::kernel::DeclarationPayload::Function(f) = &mut d.payload
                    {
                        f.result.relation = crate::platform::kernel::ResultRelation::Value;
                    }
                }
                _ => {}
            }
        }
        assert!(Proof::reconstruct(&[&corrupted], &ExecutionControl::uncancelled()).is_err());
    }
    let scopes = snapshot
        .types
        .iter()
        .filter_map(|(ty, object)| {
            let TypeForm::RegionCap {
                region,
                mode: crate::platform::kernel::RegionMode::Shared { scope },
            } = object.form
            else {
                return None;
            };
            Some((*ty, region, scope))
        })
        .collect::<Vec<_>>();
    assert_eq!(scopes.len(), 3);
    let mut wrong_scope = snapshot.clone();
    let (previous, region, _) = scopes[0];
    let object = crate::platform::kernel::TypeObject::new(TypeForm::RegionCap {
        region,
        mode: crate::platform::kernel::RegionMode::Shared { scope: scopes[1].2 },
    })
    .unwrap();
    let changed = encode_type_object(&object).unwrap().0;
    wrong_scope.types.remove(&previous);
    wrong_scope.types.insert(changed, object);
    for owner in wrong_scope.owners.values_mut() {
        match owner {
            OwnerRecord::Parameter(p) if p.ty == previous => p.ty = changed,
            OwnerRecord::Declaration(d) => {
                if let crate::platform::kernel::DeclarationPayload::Function(f) = &mut d.payload
                    && f.result.ty == previous
                {
                    f.result.ty = changed;
                }
            }
            _ => {}
        }
    }
    assert!(Proof::reconstruct(&[&wrong_scope], &ExecutionControl::uncancelled()).is_err());
    let program = prepare_snapshot(&snapshot);
    check_reference_properties(&snapshot, &program);
    assert!(program.functions.iter().all(|f| {
        f.parameters
            .iter()
            .all(|p| !program.is_owned_type(p.ty) && program.types[&p.ty].form.is_read_view())
    }));
}

#[test]
fn region_type_templates_compile_and_pass_independent_canonical_admission() {
    let source = byte_buffer_tests::author_only(SOURCE).unwrap();
    let control = ExecutionControl::uncancelled();
    Proof::reconstruct(&[&source], &control).unwrap();
    let program = prepare_snapshot(&source);
    check_reference_properties(&source, &program);
    assert!(
        program
            .types
            .values()
            .any(|t| matches!(t.form, TypeForm::RelPtr { .. }))
    );

    let mut corrupted = source.clone();
    for owner in corrupted.owners.values_mut() {
        if let OwnerRecord::TypeParameter(parameter) = owner
            && parameter.constraints.is_region()
        {
            parameter.constraints = TypeParameterConstraints::None;
        }
    }
    assert!(Proof::reconstruct(&[&corrupted], &control).is_err());

    // Re-key every changed type and its consuming owners. A digest mismatch is
    // not evidence for the independent value-kind exclusion.
    let mut wrong_item = source.clone();
    let pointers = source
        .types
        .iter()
        .filter_map(|(digest, object)| {
            let TypeForm::RelPtr { region, .. } = object.form else {
                return None;
            };
            let mut object = object.clone();
            object.form = TypeForm::RelPtr {
                region,
                item: region,
            };
            Some((*digest, object))
        })
        .collect::<Vec<_>>();
    for (previous, object) in pointers {
        let digest = encode_type_object(&object).unwrap().0;
        wrong_item.types.remove(&previous);
        wrong_item.types.insert(digest, object);
        for owner in wrong_item.owners.values_mut() {
            match owner {
                OwnerRecord::Parameter(p) if p.ty == previous => p.ty = digest,
                OwnerRecord::Declaration(d) => {
                    if let crate::platform::kernel::DeclarationPayload::Function(f) = &mut d.payload
                        && f.result.ty == previous
                    {
                        f.result.ty = digest;
                    }
                }
                _ => {}
            }
        }
    }
    let error = Proof::reconstruct(&[&wrong_item], &control).err().unwrap();
    assert!(
        error
            .message
            .contains("region pointee has no runtime value"),
        "{error:?}"
    );
}
