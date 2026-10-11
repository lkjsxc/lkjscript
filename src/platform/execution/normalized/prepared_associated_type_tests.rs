//! Private preparation boundaries, with literal canonical expected types. These
//! do not bypass or claim complete artifact/source admission of the feature.
use super::super::prepare::{
    NormalizedAssociatedTypeBinding, NormalizedBorrowedOwnedChoiceJump, NormalizedFunction,
    NormalizedImplementationApplication, NormalizedImplementationArgument,
};
use super::*;
use crate::platform::execution::ExecutionControl;
use crate::platform::kernel::{
    DeclarationReference, FunctionEffect, ImplementationParameter, Name, PackageId,
    StructuralTypeField, TypeParameterConstraints,
};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, DeclarationId, ImplementationParameterId,
};
use std::sync::Arc;

const SEED: &[u8] = b"prepared-associated-type-closure";

fn declaration(n: u64) -> DeclarationReference {
    DeclarationReference {
        package: PackageId::migrate(SEED, 0),
        declaration: DeclarationId::migrate(SEED, n),
    }
}

fn add(types: &mut BTreeMap<TypeObjectDigest, TypeObject>, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let ty = encode_type_object(&object).unwrap().0;
    types.insert(ty, object);
    ty
}

fn field(ty: TypeObjectDigest) -> StructuralTypeField {
    StructuralTypeField {
        name: Name::new("payload").unwrap(),
        ty,
    }
}

fn selected(
    identity: u32,
    ty: TypeObjectDigest,
    cell: TypeObjectDigest,
) -> NormalizedImplementationArgument {
    Arc::new(NormalizedImplementationApplication {
        identity,
        depth: 0,
        implementation: declaration(u64::from(identity) + 10),
        contract: declaration(1),
        implementation_generation: 33,
        contract_generation: 33,
        implementation_type_parameters: Arc::from([]),
        implementation_parameter_ids: Arc::from([]),
        implementation_type_arguments: Arc::from([]),
        implementation_type_parameter_constraints: Arc::from([]),
        self_type: cell,
        type_arguments: Arc::from([]),
        contract_type_parameter_constraints: Arc::from([]),
        implementations: Arc::from([]),
        prerequisites: Arc::from([]),
        associated_types: Arc::from([NormalizedAssociatedTypeBinding {
            member: AssociatedTypeMemberId::migrate(SEED, 0),
            template: ty,
            ty,
        }]),
    })
}

fn function(
    selected: NormalizedImplementationArgument,
    self_type: TypeObjectDigest,
    result: TypeObjectDigest,
) -> NormalizedFunction {
    NormalizedFunction {
        source_generation: 33,
        implementation_parameters: Arc::from([ImplementationParameter {
            id: ImplementationParameterId::migrate(SEED, 0),
            name: Name::new("reader").unwrap(),
            contract: declaration(1),
            self_type,
            type_arguments: vec![],
        }]),
        implementation_arguments: Arc::from([selected]),
        callsites: Arc::from([]),
        requirement_parameters: Arc::from([]),
        requirement_arguments: Arc::from([]),
        effect_parameters: Arc::from([]),
        effect: FunctionEffect::Pure,
        effect_arguments: Arc::from([]),
        declaration: declaration(2),
        graph_function: true,
        type_parameters: Arc::from([TypeParameterId::migrate(SEED, 0)]),
        type_arguments: Arc::from([]),
        type_parameter_constraints: Arc::from([TypeParameterConstraints::Owned]),
        parameter_count: 0,
        parameters: Arc::from([]),
        result,
        result_borrow: None,
        task_requirements: Arc::from([]),
        body: NormalizedFunctionBody::Code(NormalizedCode {
            parameter_count: 0,
            local_count: 0,
            instructions: Arc::from([NormalizedInstruction::Unit]),
        }),
    }
}

fn projection(types: &mut BTreeMap<TypeObjectDigest, TypeObject>) -> TypeObjectDigest {
    add(
        types,
        TypeForm::AssociatedWitnessProjection {
            scope: declaration(2),
            parameter: ImplementationParameterId::migrate(SEED, 0),
            contract: declaration(1),
            member: AssociatedTypeMemberId::migrate(SEED, 0),
        },
    )
}

#[test]
fn associated_preparation_closes_inactive_roots_and_both_parallel_lanes_per_exact_application() {
    let mut types = BTreeMap::new();
    let parameter = TypeParameterId::migrate(SEED, 0);
    let formal = add(&mut types, TypeForm::TypeParameter { parameter });
    let cell = add(&mut types, TypeForm::OwnedI64Cell);
    let scalar = add(&mut types, TypeForm::I64);
    let block = add(&mut types, TypeForm::OwnedSequence { item: scalar });
    let projected = projection(&mut types);
    let sequence = add(&mut types, TypeForm::OwnedSequence { item: projected });
    let product = add(
        &mut types,
        TypeForm::OwnedProduct {
            fields: vec![field(projected)],
        },
    );
    let choice = add(
        &mut types,
        TypeForm::OwnedChoice {
            cases: vec![field(projected)],
        },
    );
    let origin = super::super::value::ValueOrigin::fresh().unwrap();
    let left = FunctionIndex(80, origin);
    let right = FunctionIndex(81, origin);
    let value = FunctionIndex(82, origin);
    let tail = FunctionIndex(83, origin);
    let code = NormalizedCode {
        parameter_count: 0,
        local_count: 2,
        instructions: Arc::from([
            NormalizedInstruction::Jump(8),
            NormalizedInstruction::SequenceEmpty {
                sequence_type: sequence,
            },
            NormalizedInstruction::PackOwned {
                product_type: product,
                fields: Arc::from([]),
            },
            NormalizedInstruction::AdoptBorrowResult {
                source_local: 0,
                binding_local: 1,
                binding_type: projected,
            },
            NormalizedInstruction::MatchBorrowedOwned {
                choice_type: choice,
                source_local: 0,
                cases: Arc::from([NormalizedBorrowedOwnedChoiceJump {
                    target: 8,
                    binding_local: 1,
                    binding_type: projected,
                }]),
            },
            NormalizedInstruction::Parallel {
                left,
                right,
                left_types: Arc::from([projected]),
                right_types: Arc::from([sequence]),
                left_implementations: Arc::from([]),
                right_implementations: Arc::from([]),
                left_arguments: 0,
                right_arguments: 0,
                result_type: product,
            },
            NormalizedInstruction::FunctionValue {
                function: value,
                type_arguments: Arc::from([choice]),
                effect_arguments: Arc::from([]),
                requirement_arguments: Arc::from([]),
            },
            NormalizedInstruction::TailCall {
                function: tail,
                type_arguments: Arc::from([projected]),
                arguments: 0,
                effect_arguments: Arc::from([]),
                requirement_arguments: Arc::from([]),
            },
            NormalizedInstruction::Unit,
        ]),
    };
    let ordinary = BTreeMap::from([(parameter, cell)]);
    let control = ExecutionControl::uncancelled();
    let mut work = Budget::new(&control);
    for (identity, output) in [(0, cell), (1, block), (2, block)] {
        let function = function(selected(identity, output, cell), formal, product);
        let environment = ClosedTypeEnvironment::for_function(&function).unwrap();
        let index = FunctionIndex(identity, origin);
        let application = TypeApplication {
            bindings: &ordinary,
            associated: Some(&environment),
            function: Some(index),
        };
        let expected_sequence = add(&mut types, TypeForm::OwnedSequence { item: output });
        let expected_product = add(
            &mut types,
            TypeForm::OwnedProduct {
                fields: vec![field(output)],
            },
        );
        let expected_choice = add(
            &mut types,
            TypeForm::OwnedChoice {
                cases: vec![field(output)],
            },
        );
        let mut pending = BTreeSet::new();
        calls(
            &code,
            &application,
            &mut types,
            &mut pending,
            &mut work,
            &[],
            &[],
        )
        .unwrap();
        assert_eq!(
            pending,
            BTreeSet::from([
                (left, vec![output]),
                (right, vec![expected_sequence]),
                (value, vec![expected_choice]),
                (tail, vec![output]),
            ])
        );
        let lookup = work.finish_type_lookup(origin);
        for (template, expected) in [
            (projected, output),
            (sequence, expected_sequence),
            (product, expected_product),
            (choice, expected_choice),
        ] {
            assert_eq!(
                lookup.get_application(origin, index, template, &ordinary),
                Ok(Some(expected))
            );
            assert_eq!(lookup.get(origin, template, &ordinary), Ok(None));
        }
        // This test consumes each lookup; application isolation itself is covered
        // by prepared_application_lookup_separates_witness_applications_and_full_bindings.
    }
    assert!(substitute(&mut types, sequence, &ordinary, 0, &mut work).is_err());
}

#[test]
fn associated_preparation_closes_effects_before_members_and_separates_mapping_lookup() {
    use crate::platform::kernel::{EffectParameterReference, EffectRow, RequirementSubstitution};
    use crate::platform::semantic_id::EffectParameterId;
    let mut types = BTreeMap::new();
    let parameter = TypeParameterId::migrate(SEED, 0);
    let formal = add(&mut types, TypeForm::TypeParameter { parameter });
    let cell = add(&mut types, TypeForm::OwnedI64Cell);
    let unit = add(&mut types, TypeForm::Unit);
    let effect_parameter = EffectParameterReference {
        package: declaration(2).package,
        parameter: EffectParameterId::migrate(SEED, 0),
    };
    let projected = projection(&mut types);
    let raw = add(
        &mut types,
        TypeForm::TaskFunction {
            parameters: vec![projected],
            result: unit,
            effect: EffectRow {
                requirements: vec![],
                parameters: vec![effect_parameter],
            },
        },
    );
    let block = add(&mut types, TypeForm::OwnedSequence { item: cell });
    let function = function(selected(0, block, cell), formal, raw);
    let environment = ClosedTypeEnvironment::for_function(&function).unwrap();
    let ordinary = BTreeMap::from([(parameter, cell)]);
    let application = TypeApplication {
        bindings: &ordinary,
        associated: Some(&environment),
        function: None,
    };
    let control = ExecutionControl::uncancelled();
    let mut work = Budget::new(&control);
    let effect_closed = substitute_effect_type(
        &mut types,
        raw,
        &BTreeMap::from([(effect_parameter, EffectRow::default())]),
        &RequirementSubstitution::new(),
        0,
        &mut work,
    )
    .unwrap();
    assert_ne!(effect_closed, raw);
    let actual = substitute_in(&mut types, effect_closed, &application, 0, &mut work).unwrap();
    let expected = add(
        &mut types,
        TypeForm::TaskFunction {
            parameters: vec![block],
            result: unit,
            effect: EffectRow::default(),
        },
    );
    assert_eq!(actual, expected);
    // An intermediate implementation context has no unrelated caller lookup.
    let origin = super::super::value::ValueOrigin::fresh().unwrap();
    let lookup = work.finish_type_lookup(origin);
    assert_eq!(lookup.get(origin, effect_closed, &ordinary), Ok(None));
}

#[test]
fn unresolved_associated_signature_templates_acquire_no_runtime_properties() {
    let artifact = crate::platform::compiler::load_artifact(include_bytes!(
        "../../../../packages/standard/generated/standard.lkja"
    ))
    .unwrap();
    let mut program = NormalizedProgram::prepare(artifact).unwrap();
    let projected = projection(&mut program.types);
    let unit = add(&mut program.types, TypeForm::Unit);
    let callable = add(
        &mut program.types,
        TypeForm::Function {
            parameters: vec![projected],
            result: unit,
        },
    );
    let concrete = add(
        &mut program.types,
        TypeForm::Function {
            parameters: vec![unit],
            result: unit,
        },
    );
    let control = ExecutionControl::uncancelled();
    let mut work = Budget::new(&control);
    let unresolved = unresolved_associated_types(&program.types, &mut work).unwrap();
    assert!(unresolved.contains(&projected) && unresolved.contains(&callable));
    assert!(!unresolved.contains(&concrete));
    for property in [
        Property::Capture,
        Property::Ordinary,
        Property::BufferFree,
        Property::Equality,
        Property::NoApplication,
    ] {
        let result = property_types(&program, &mut work, property, &unresolved).unwrap();
        assert!(!result.contains(&projected) && !result.contains(&callable));
        if property != Property::Equality {
            assert!(result.contains(&concrete));
        }
    }
}
