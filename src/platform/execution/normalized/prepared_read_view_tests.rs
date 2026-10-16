//! Independent expected scope coordinates and negative closure controls.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::platform::execution::ExecutionControl;
use crate::platform::kernel::{
    CallableResult, Name, ReadScopeParameter, ReadScopeSource, ResultRelation,
};
use crate::platform::semantic_id::{BindingId, DeclarationId, ParameterId, ReadScopeParameterId};
use std::sync::Arc;

const SEED: &[u8] = b"prepared-read-view-controls";

fn declaration(n: u64) -> crate::platform::kernel::DeclarationReference {
    crate::platform::kernel::DeclarationReference {
        package: crate::platform::kernel::PackageId::from_bytes([71; 16]).unwrap(),
        declaration: DeclarationId::migrate(SEED, n),
    }
}
fn scope(n: u64) -> ReadScopeParameter {
    ReadScopeParameter {
        id: ReadScopeParameterId::migrate(SEED, n),
        name: Name::new(format!("scope{n}")).unwrap(),
    }
}
fn formal(owner: u64, parameter: &ReadScopeParameter) -> ReadScopeReference {
    ReadScopeReference {
        binder: ReadScopeBinder::Declaration(declaration(owner)),
        parameter: parameter.id,
    }
}
fn lexical(n: u64) -> ReadScopeArgument {
    ReadScopeArgument::Lexical {
        callable: declaration(1),
        binding: BindingId::migrate(SEED, n),
    }
}
fn add(types: &mut BTreeMap<TypeObjectDigest, TypeObject>, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let ty = encode_type_object(&object).unwrap().0;
    types.insert(ty, object);
    ty
}

#[test]
fn untaken_view_instructions_close_scope_and_type_actuals() {
    let mut types = BTreeMap::new();
    let parameter = TypeParameterId::migrate(SEED, 0);
    let symbolic = add(&mut types, TypeForm::TypeParameter { parameter });
    let cell = add(&mut types, TypeForm::OwnedI64Cell);
    let region = scope(0);
    let template = add(
        &mut types,
        TypeForm::ReadViewApplied {
            declaration: declaration(2),
            arguments: vec![symbolic],
            read_scope: ReadScopeArgument::Parameter(formal(1, &region)),
        },
    );
    let expected = add(
        &mut types,
        TypeForm::ReadViewApplied {
            declaration: declaration(2),
            arguments: vec![cell],
            read_scope: lexical(0),
        },
    );
    let origin = super::super::value::ValueOrigin::fresh().unwrap();
    let function = FunctionIndex(0, origin);
    let ordinary = BTreeMap::from([(parameter, cell)]);
    let scopes = BTreeMap::from([(formal(1, &region), lexical(0))]);
    let application = TypeApplication {
        bindings: &ordinary,
        associated: None,
        function: Some(function),
        scopes: Some(&scopes),
    };
    let code = NormalizedCode {
        parameter_count: 0,
        local_count: 2,
        instructions: Arc::from([
            NormalizedInstruction::Jump(6),
            NormalizedInstruction::BeginReadViewScope {
                read_scope: ReadScopeArgument::Parameter(formal(1, &region)),
            },
            NormalizedInstruction::BeginReadView {
                view_type: template,
                source_local: 0,
                fields: Arc::from([]),
            },
            NormalizedInstruction::BorrowReadViewField {
                view_type: template,
                source_local: 0,
                field: 0,
                binding_local: 1,
                binding_type: template,
            },
            NormalizedInstruction::AdoptViewResult {
                source_local: 0,
                binding_local: 1,
                binding_type: template,
            },
            NormalizedInstruction::EndReadViewScope {
                read_scope: ReadScopeArgument::Parameter(formal(1, &region)),
            },
            NormalizedInstruction::Unit,
        ]),
    };
    let control = ExecutionControl::uncancelled();
    let mut work = Budget::new(&control);
    calls(
        &code,
        &application,
        &mut types,
        &mut BTreeSet::new(),
        &mut work,
        &[],
        &[],
    )
    .unwrap();
    let lookup = work.finish_type_lookup(origin);
    assert_eq!(
        lookup.get_scoped_application_arguments(
            origin,
            function,
            declaration(1),
            template,
            &ordinary,
            (std::slice::from_ref(&region), &[lexical(0)])
        ),
        Ok(Some(expected))
    );
    assert_eq!(lookup.get(origin, template, &ordinary), Ok(None));
    let foreign = BTreeMap::from([(formal(3, &region), lexical(0))]);
    let mut work = Budget::new(&control);
    let foreign_application = TypeApplication {
        scopes: Some(&foreign),
        ..application
    };
    assert!(
        calls(
            &code,
            &foreign_application,
            &mut types,
            &mut BTreeSet::new(),
            &mut work,
            &[],
            &[]
        )
        .is_err()
    );
}

#[test]
fn scope_lookup_keeps_unused_formals_and_exact_binders() {
    let origin = super::super::value::ValueOrigin::fresh().unwrap();
    let function = FunctionIndex(0, origin);
    let used = scope(0);
    let unused = scope(1);
    let scopes = BTreeMap::from([
        (formal(1, &used), lexical(0)),
        (formal(1, &unused), lexical(1)),
    ]);
    let template = TypeObjectDigest::from_bytes([73; 32]);
    let result = TypeObjectDigest::from_bytes([79; 32]);
    let control = ExecutionControl::uncancelled();
    let mut work = Budget::new(&control);
    work.remember_scoped_application_substitution(
        function,
        template,
        result,
        &BTreeMap::new(),
        &scopes,
    )
    .unwrap();
    let lookup = work.finish_type_lookup(origin);
    let parameters = [used.clone(), unused.clone()];
    assert_eq!(
        lookup.get_scoped_application_arguments(
            origin,
            function,
            declaration(1),
            template,
            &BTreeMap::new(),
            (&parameters, &[lexical(0), lexical(1)])
        ),
        Ok(Some(result))
    );
    for (owner, parameters, arguments) in [
        (
            declaration(3),
            parameters.to_vec(),
            vec![lexical(0), lexical(1)],
        ),
        (
            declaration(1),
            parameters.to_vec(),
            vec![lexical(0), lexical(2)],
        ),
        (declaration(1), vec![used.clone()], vec![lexical(0)]),
        (
            declaration(1),
            vec![used.clone(), used],
            vec![lexical(0), lexical(1)],
        ),
    ] {
        assert_eq!(
            lookup.get_scoped_application_arguments(
                origin,
                function,
                owner,
                template,
                &BTreeMap::new(),
                (&parameters, &arguments)
            ),
            Ok(None)
        );
    }
}

#[test]
fn scope_sources_admit_the_complete_inventory_before_execution() {
    let region = scope(0);
    let unit = TypeObjectDigest::from_bytes([83; 32]);
    let parameter = ParameterId::migrate(SEED, 0);
    let mut function = super::super::prepare::NormalizedFunction {
        source_generation: crate::platform::canonical_schema::SCHEMA_EPOCH,
        declaration: declaration(1),
        graph_function: true,
        implementation_parameters: Arc::from([]),
        implementation_arguments: Arc::from([]),
        callsites: Arc::from([]),
        requirement_parameters: Arc::from([]),
        requirement_arguments: Arc::from([]),
        effect_parameters: Arc::from([]),
        effect: crate::platform::kernel::FunctionEffect::Pure,
        effect_arguments: Arc::from([]),
        type_parameters: Arc::from([]),
        type_arguments: Arc::from([]),
        type_parameter_constraints: Arc::from([]),
        parameter_count: 1,
        parameters: Arc::from([super::super::prepare::NormalizedParameter {
            parameter,
            name: Name::new("source").unwrap(),
            ty: unit,
            use_mode: crate::platform::kernel::ParameterUse::Borrow,
            resource_requirement: None,
        }]),
        result: CallableResult {
            ty: unit,
            relation: ResultRelation::Value,
        },
        read_scope_parameters: Arc::from([region.clone()]),
        read_scope_arguments: Arc::from([lexical(0)]),
        read_scope_sources: Arc::from([ReadScopeSource {
            source: 0,
            scope: ReadScopeArgument::Parameter(formal(1, &region)),
        }]),
        task_requirements: Arc::from([]),
        body: NormalizedFunctionBody::Code(NormalizedCode {
            parameter_count: 1,
            local_count: 1,
            instructions: Arc::from([NormalizedInstruction::Unit]),
        }),
    };
    let control = ExecutionControl::uncancelled();
    assert_eq!(
        function_scope_bindings(&function, &mut Budget::new(&control)).unwrap(),
        BTreeMap::from([(formal(1, &region), lexical(0))])
    );
    function.read_scope_sources = Arc::from([ReadScopeSource {
        source: 0,
        scope: ReadScopeArgument::Parameter(formal(3, &region)),
    }]);
    assert!(function_scope_bindings(&function, &mut Budget::new(&control)).is_err());
    function.read_scope_sources = Arc::from([ReadScopeSource {
        source: 1,
        scope: ReadScopeArgument::Parameter(formal(1, &region)),
    }]);
    assert!(function_scope_bindings(&function, &mut Budget::new(&control)).is_err());
}

#[test]
fn zero_field_layout_closes_phantom_actuals_and_never_acquires_ordinary_properties() {
    use super::super::prepare::NormalizedReadViewLayout;
    let mut program = prepared_associated_model().program;
    let region = scope(0);
    let parameter = TypeParameterId::migrate(SEED, 0);
    let cell = add(&mut program.types, TypeForm::OwnedI64Cell);
    program.read_views = Arc::from([NormalizedReadViewLayout {
        declaration: declaration(2),
        type_parameters: Arc::from([parameter]),
        type_parameter_constraints: Arc::from([
            crate::platform::kernel::TypeParameterConstraints::Owned,
        ]),
        arguments: Arc::from([]),
        scope_parameter: region.clone(),
        read_scope: ReadScopeArgument::Parameter(formal(2, &region)),
        fields: Arc::from([]),
    }]);
    let view = add(
        &mut program.types,
        TypeForm::ReadViewApplied {
            declaration: declaration(2),
            arguments: vec![cell],
            read_scope: lexical(0),
        },
    );
    let control = ExecutionControl::uncancelled();
    complete_read_view_layouts(&mut program, &mut Budget::new(&control)).unwrap();
    let layout = program.read_view_layout(view).unwrap();
    assert!(layout.fields.is_empty());
    assert_eq!(layout.arguments.as_ref(), [cell]);
    assert_eq!(layout.read_scope, lexical(0));
    for property in [
        Property::Ordinary,
        Property::Capture,
        Property::BufferFree,
        Property::Equality,
    ] {
        assert!(
            !property_types(
                &program,
                &mut Budget::new(&control),
                property,
                &unresolved_associated_types(&program.types, &mut Budget::new(&control)).unwrap()
            )
            .unwrap()
            .contains(&view)
        );
    }
    let mut missing = program.clone();
    add(
        &mut missing.types,
        TypeForm::ReadViewApplied {
            declaration: declaration(2),
            arguments: vec![TypeObjectDigest::from_bytes([89; 32])],
            read_scope: lexical(0),
        },
    );
    assert!(complete_read_view_layouts(&mut missing, &mut Budget::new(&control)).is_err());
    let scalar = add(&mut program.types, TypeForm::I64);
    add(
        &mut program.types,
        TypeForm::ReadViewApplied {
            declaration: declaration(2),
            arguments: vec![scalar],
            read_scope: lexical(0),
        },
    );
    assert!(complete_read_view_layouts(&mut program, &mut Budget::new(&control)).is_err());
}

#[test]
fn prepared_view_instruction_reprocessing_preserves_exact_applied_scope_identity() {
    let control = ExecutionControl::uncancelled();
    let origin = super::super::value::ValueOrigin::fresh().unwrap();
    let function = FunctionIndex(0, origin);
    let parameter = TypeParameterId::migrate(SEED, 400);
    let first = scope(400);
    let second = scope(401);
    let formals = [formal(1, &first), formal(1, &second)];
    for collision in [false, true] {
        // Complete method preparation forwards ContractMethod actuals; a
        // same-binder permutation additionally proves actuals stay opaque even
        // when their exact references are other raw formal keys.
        let actuals = if collision {
            [
                ReadScopeArgument::Parameter(formals[1]),
                ReadScopeArgument::Parameter(formals[0]),
            ]
        } else {
            let binder = ReadScopeBinder::ContractMethod {
                contract: declaration(3),
                method: crate::platform::semantic_id::MethodId::migrate(SEED, 400),
            };
            [first.id, second.id].map(|parameter| {
                ReadScopeArgument::Parameter(ReadScopeReference { binder, parameter })
            })
        };
        let supplied = BTreeMap::from([(formals[0], actuals[0]), (formals[1], actuals[1])]);
        let mut budget = Budget::new(&control);
        let identities = applied_scope_identities(&supplied, &mut budget).unwrap();
        assert_eq!(identities.len(), 2);
        assert!(budget.work_for_test().0 >= supplied.len());
        assert!(budget.work_for_test().1 > 0);
        assert_eq!(
            resolve_scope(actuals[0], Some(&identities)).unwrap(),
            actuals[0]
        );
        assert_eq!(
            resolve_scope(ReadScopeArgument::Parameter(formals[0]), Some(&supplied)).unwrap(),
            actuals[0]
        );
        if collision {
            assert_eq!(
                resolve_scope(actuals[0], Some(&supplied)).unwrap(),
                actuals[1]
            );
        } else {
            assert_eq!(
                resolve_scope(actuals[0], Some(&supplied)).unwrap_err().code,
                "normalized_instantiation_scope"
            );
        }
        let mut types = BTreeMap::new();
        let symbolic = add(&mut types, TypeForm::TypeParameter { parameter });
        let scalar = add(&mut types, TypeForm::I64);
        let template = add(
            &mut types,
            TypeForm::ReadViewApplied {
                declaration: declaration(2),
                arguments: vec![symbolic],
                read_scope: ReadScopeArgument::Parameter(formals[0]),
            },
        );
        let already_applied = add(
            &mut types,
            TypeForm::ReadViewApplied {
                declaration: declaration(2),
                arguments: vec![symbolic],
                read_scope: actuals[0],
            },
        );
        let expected = add(
            &mut types,
            TypeForm::ReadViewApplied {
                declaration: declaration(2),
                arguments: vec![scalar],
                read_scope: actuals[0],
            },
        );
        let ordinary = BTreeMap::from([(parameter, scalar)]);
        let application = TypeApplication {
            bindings: &ordinary,
            associated: None,
            function: Some(function),
            scopes: Some(&supplied),
        };
        let code = |view_type, read_scope| NormalizedCode {
            parameter_count: 0,
            local_count: 2,
            instructions: Arc::from([
                NormalizedInstruction::Jump(6),
                NormalizedInstruction::BeginReadViewScope { read_scope },
                NormalizedInstruction::DropReadView {
                    view_type,
                    source_local: 0,
                },
                NormalizedInstruction::BorrowReadViewField {
                    view_type,
                    source_local: 0,
                    field: 0,
                    binding_local: 1,
                    binding_type: view_type,
                },
                NormalizedInstruction::AdoptViewResult {
                    source_local: 0,
                    binding_local: 1,
                    binding_type: view_type,
                },
                NormalizedInstruction::EndReadViewScope { read_scope },
                NormalizedInstruction::Unit,
            ]),
        };
        let mut raw = Budget::new(&control);
        calls(
            &code(template, ReadScopeArgument::Parameter(formals[0])),
            &application,
            &mut types,
            &mut BTreeSet::new(),
            &mut raw,
            &[],
            &[],
        )
        .unwrap();
        let raw = raw.finish_type_lookup(origin);
        assert_eq!(
            raw.get_scoped_application_arguments(
                origin,
                function,
                declaration(1),
                template,
                &ordinary,
                (&[first.clone(), second.clone()], &actuals)
            ),
            Ok(Some(expected))
        );
        // Only the already rewritten view operands select the actual identity
        // map. The ordinary substitution remains active and memoized globally.
        let mut applied = Budget::new(&control);
        calls_at(
            &code(already_applied, actuals[0]),
            (&application, Some(&identities)),
            &mut types,
            &mut BTreeSet::new(),
            &mut applied,
            &[],
            &[],
        )
        .unwrap();
        let applied = applied.finish_type_lookup(origin);
        assert_eq!(
            applied.get(origin, already_applied, &ordinary),
            Ok(Some(expected))
        );
        assert_eq!(
            applied.get_scoped_application_arguments(
                origin,
                function,
                declaration(1),
                already_applied,
                &ordinary,
                (&[first.clone(), second.clone()], &actuals)
            ),
            Ok(None)
        );
        let foreign = ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(declaration(4)),
            parameter: first.id,
        });
        let unknown = add(
            &mut types,
            TypeForm::ReadViewApplied {
                declaration: declaration(2),
                arguments: vec![scalar],
                read_scope: foreign,
            },
        );
        for bad in [code(unknown, actuals[0]), code(expected, foreign)] {
            let error = calls_at(
                &bad,
                (&application, Some(&identities)),
                &mut types,
                &mut BTreeSet::new(),
                &mut Budget::new(&control),
                &[],
                &[],
            )
            .unwrap_err();
            assert_eq!(error.class, DiagnosticClass::Corrupt);
            assert_eq!(error.code, "normalized_instantiation_scope");
        }
        for (steps, bytes, expected_code) in [
            (MAXIMUM_WORK, 0, "normalized_instantiation_work"),
            (
                0,
                MAXIMUM_METADATA_BYTES,
                "normalized_instantiation_storage",
            ),
        ] {
            let error = applied_scope_identities(
                &supplied,
                &mut Budget::with_work_for_test(&control, steps, bytes),
            )
            .unwrap_err();
            assert_eq!(error.class, DiagnosticClass::Resource);
            assert_eq!(error.code, expected_code);
        }
        let cancelled = ExecutionControl::cancel_after_checks(0);
        assert_eq!(
            applied_scope_identities(&supplied, &mut Budget::new(&cancelled))
                .unwrap_err()
                .class,
            DiagnosticClass::Cancelled
        );
    }
}
