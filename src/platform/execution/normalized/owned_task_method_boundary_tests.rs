//! Independent negative observations for task kind, effect rows and unused contracts.
use super::*;
use crate::platform::kernel::*;

#[test]
fn owned_task_method_rejects_pure_calls_borrowing_and_effect_mismatch() {
    let valid = owned_task_method_fixture::input();
    let mutations = [
        (
            "(returns T) (effect (task))",
            "(returns T) (effect pure)",
            "kernel_type_pure_task_call",
        ),
        (
            "(parameters (Self consume))",
            "(parameters (Self borrow))",
            "kernel_owned_contract",
        ),
        (
            "(parameters (Self consume))",
            "(parameters (Self unrestricted))",
            "kernel_owned_contract",
        ),
        (
            "(function create cell-create (visibility public) (effect (task))",
            "(function create cell-create (visibility public) (effect pure)",
            "kernel_owned_contract",
        ),
        (
            "(parameters (I64 unrestricted)) (returns Self) (effect (task))",
            "(parameters (I64 unrestricted)) (returns Self) (effect pure)",
            "kernel_owned_contract",
        ),
        (
            "(parameter create value (type T) (use consume)) (returns I64)\n      (effect (task (requirement authority::clock)))",
            "(parameter create value (type T) (use consume)) (returns I64) (effect (task))",
            "kernel_type_task_requirement",
        ),
    ];
    for (old, new, code) in mutations {
        assert!(valid.contains(old));
        let error = byte_buffer_tests::author_only(&valid.replacen(old, new, 1)).unwrap_err();
        assert!(error.contains(code), "{old}: {error}");
    }
}

#[test]
fn owned_task_method_unused_contracts_require_closed_rows_and_consume() {
    let literal = "declarations.begin\n(units (module create unused (owned-contract create Unused (visibility public)
      (self Self) (type-parameter create Self (constraint owned))
      (method method_81000000000000000000000000000001 move (parameters (Self consume)) (returns Self) (effect (task))))))\ndeclarations.end\n";
    let source = byte_buffer_tests::author_only(literal).unwrap();
    assert!(memory_reference::accepts(&source));
    for package in [
        source.root.package_id,
        PackageId::migrate(b"missing-method-package", 1),
    ] {
        let mut invalid = source.clone();
        for owner in invalid.owners.values_mut() {
            if let OwnerRecord::Declaration(d) = owner
                && let DeclarationPayload::OwnedContract(c) = &mut d.payload
            {
                c.methods[0].effect = FunctionEffect::Task {
                    requirements: vec![
                        RequirementReference {
                            package,
                            requirement: crate::platform::semantic_id::RequirementId::migrate(
                                b"missing-method-requirement",
                                1,
                            ),
                        }
                        .into(),
                    ],
                    effect_parameters: Vec::new(),
                };
                assert!(c.validate_local().is_ok());
            }
        }
        assert!(validate_full(&invalid).is_err());
        assert!(
            super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&invalid])
                .is_err()
        );
    }
    for mode in [ParameterUse::Borrow, ParameterUse::Unrestricted] {
        let mut invalid = source.clone();
        for owner in invalid.owners.values_mut() {
            if let OwnerRecord::Declaration(d) = owner
                && let DeclarationPayload::OwnedContract(c) = &mut d.payload
            {
                c.methods[0].parameters[0].use_mode = mode;
            }
        }
        assert!(!memory_reference::accepts(&invalid));
        assert!(validate_full(&invalid).is_err());
        assert!(
            super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&invalid])
                .is_err()
        );
    }
    let mut invalid = source.clone();
    for owner in invalid.owners.values_mut() {
        if let OwnerRecord::Declaration(d) = owner
            && let DeclarationPayload::OwnedContract(c) = &mut d.payload
        {
            c.methods[0].effect = FunctionEffect::Task {
                requirements: Vec::new(),
                effect_parameters: vec![EffectParameterReference {
                    package: invalid.root.package_id,
                    parameter: crate::platform::semantic_id::EffectParameterId::migrate(
                        b"unbound-method-effect",
                        1,
                    ),
                }],
            };
            assert!(c.validate_local().is_err());
        }
    }
    assert!(!memory_reference::accepts(&invalid));
    assert!(validate_full(&invalid).is_err());
}

#[test]
fn owned_task_method_same_kind_effect_narrowing_rejects_independently() {
    let source = owned_task_method_fixture::source();
    for name in ["cell-finish", "buffer-finish"] {
        let mut invalid = source.clone();
        let target = declaration_named(&invalid, name);
        let OwnerRecord::Declaration(d) = invalid
            .owners
            .get_mut(&OwnerKey::Declaration(target.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::Function(f) = &mut d.payload else {
            unreachable!()
        };
        let FunctionEffect::Task { requirements, .. } = &mut f.effect else {
            unreachable!()
        };
        assert!(!requirements.is_empty());
        requirements.clear();
        // No kind or body change: the memory-only and source-metadata readers
        // must reject the exact-row mismatch, not rely on body effect checking.
        assert!(!memory_reference::accepts(&invalid));
        assert!(validate_full(&invalid).is_err());
        assert!(
            super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&invalid])
                .is_err()
        );
    }
}

#[test]
fn owned_task_method_unselected_implementation_effect_mutations_reject() {
    let source = owned_task_method_fixture::source();
    for name in ["cell-create", "cell-finish", "buffer-finish"] {
        let mut invalid = source.clone();
        let target = declaration_named(&invalid, name);
        let OwnerRecord::Declaration(d) = invalid
            .owners
            .get_mut(&OwnerKey::Declaration(target.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::Function(f) = &mut d.payload else {
            unreachable!()
        };
        f.effect = FunctionEffect::Pure;
        d.header.kind = OwnerKind::PureFunction;
        assert!(!memory_reference::accepts(&invalid));
        assert!(validate_full(&invalid).is_err());
        assert!(
            super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&invalid])
                .is_err()
        );
    }
}
