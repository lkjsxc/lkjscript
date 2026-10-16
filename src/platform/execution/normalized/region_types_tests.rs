//! Region kind correspondence from independently read complete canonical syntax.
use super::super::reference::read_view_admission::Proof;
use super::*;
use crate::platform::kernel::{OwnerRecord, TypeParameterConstraints, encode_type_object};

const SOURCE: &str = include_str!("../../../../tests/fixtures/region-kinds.lkjc");
const SHARED: &str = include_str!("../../../../tests/fixtures/region-shared.lkjc");

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
