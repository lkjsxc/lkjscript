//! Actual private Closing output versus an independently derived canonical proof.
//! Raw fixture projection below supplies templates/instructions, never selected
//! member arrays or expected applications. It is not an associated artifact
//! admission path, a compiler, or a memory/finite-growth proof.
use super::*;
use crate::platform::execution::normalized::reference_types::associated_preparation::{
    self as reference, Application as ExpectedApplication, Entry, Limits, Proof,
    test_support::{Fixture, make_fixture, operand},
};
use crate::platform::execution::{ExecutionControl, normalized::prepare::*};
use crate::platform::kernel::{
    DeclarationPayload, DeclarationRecord, EffectParameterRecord, EffectParameterReference,
    EffectRow, ExpressionOperation, ExpressionRecord, FunctionDeclaration, FunctionEffect,
    KernelSnapshot, LocalValueReference, Name, OwnerHeader, OwnerKind, ParameterParent,
    ParameterRecord, ParameterUse, TypeParameterRecord,
};
use crate::platform::semantic_id::{
    DeclarationId, EffectParameterId, ExpressionId, MethodId, ParameterId,
};

const SEED: &[u8] = b"actual-associated-closing-comparison";

#[derive(Clone, Copy)]
enum Site {
    Single(ExpressionId),
    Parallel(ExpressionId, ExpressionId),
}
type Sites = BTreeMap<DeclarationReference, BTreeMap<u32, Site>>;

fn declaration(source: &KernelSnapshot, reference: DeclarationReference) -> &DeclarationRecord {
    assert_eq!(reference.package, source.root.package_id);
    let OwnerRecord::Declaration(owner) =
        &source.owners[&OwnerKey::Declaration(reference.declaration)]
    else {
        panic!("fixture declaration kind")
    };
    assert_eq!(
        owner.header.owner,
        OwnerKey::Declaration(reference.declaration)
    );
    owner
}
fn function(source: &KernelSnapshot, reference: DeclarationReference) -> &FunctionDeclaration {
    let DeclarationPayload::Function(value) = &declaration(source, reference).payload else {
        panic!("fixture function kind")
    };
    value
}
fn expression(source: &KernelSnapshot, id: ExpressionId) -> &ExpressionRecord {
    let OwnerRecord::Expression(value) = &source.owners[&OwnerKey::Expression(id)] else {
        panic!("fixture expression kind")
    };
    assert_eq!(value.id, id);
    value
}
fn add_type(source: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let ty = crate::platform::kernel::encode_type_object(&object)
        .unwrap()
        .0;
    source.types.insert(ty, object);
    ty
}
fn add_expression(
    source: &mut KernelSnapshot,
    index: u64,
    operation: ExpressionOperation,
) -> ExpressionId {
    let id = ExpressionId::migrate(SEED, index);
    let record = ExpressionRecord::new(id, operation).unwrap();
    assert!(
        source
            .owners
            .insert(OwnerKey::Expression(id), OwnerRecord::Expression(record))
            .is_none()
    );
    id
}

// Only the literal syntax used by these fixtures is projected. In particular,
// this code does not substitute types, select witnesses, or resolve methods.
fn raw_code(
    source: &KernelSnapshot,
    definition: &FunctionDeclaration,
    targets: &BTreeMap<DeclarationReference, FunctionIndex>,
    sites: &mut BTreeMap<u32, Site>,
) -> NormalizedCode {
    fn emit(
        source: &KernelSnapshot,
        definition: &FunctionDeclaration,
        id: ExpressionId,
        targets: &BTreeMap<DeclarationReference, FunctionIndex>,
        instructions: &mut Vec<NormalizedInstruction>,
        sites: &mut BTreeMap<u32, Site>,
    ) {
        let record = expression(source, id);
        let operation = &record.operation;
        let mut single = false;
        match operation {
            ExpressionOperation::Unit {} => instructions.push(NormalizedInstruction::Unit),
            ExpressionOperation::Bool { value } => {
                instructions.push(NormalizedInstruction::Bool(*value))
            }
            ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(parameter),
            } => {
                let local = definition
                    .parameters
                    .iter()
                    .position(|id| id == parameter)
                    .unwrap();
                let OwnerRecord::Parameter(owner) =
                    &source.owners[&OwnerKey::Parameter(*parameter)]
                else {
                    panic!("fixture parameter kind")
                };
                instructions.push(NormalizedInstruction::LoadLocal {
                    local: local as u32,
                    use_mode: owner.use_mode,
                });
            }
            ExpressionOperation::Sequence { items } => {
                for (position, child) in items.iter().enumerate() {
                    emit(source, definition, *child, targets, instructions, sites);
                    if position + 1 < items.len() {
                        instructions.push(NormalizedInstruction::Drop);
                    }
                }
            }
            ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => {
                emit(source, definition, *condition, targets, instructions, sites);
                let branch = instructions.len();
                instructions.push(NormalizedInstruction::JumpIfFalse(0));
                emit(source, definition, *when_true, targets, instructions, sites);
                let jump = instructions.len();
                instructions.push(NormalizedInstruction::Jump(0));
                instructions[branch] =
                    NormalizedInstruction::JumpIfFalse(instructions.len() as u32);
                emit(
                    source,
                    definition,
                    *when_false,
                    targets,
                    instructions,
                    sites,
                );
                instructions[jump] = NormalizedInstruction::Jump(instructions.len() as u32);
            }
            ExpressionOperation::Call {
                read_scope_arguments,
                function,
                type_arguments,
                arguments,
                effect_arguments,
                requirement_arguments,
            } => {
                for argument in arguments {
                    emit(source, definition, *argument, targets, instructions, sites);
                }
                instructions.push(NormalizedInstruction::Call {
                    read_scope_arguments: read_scope_arguments.clone().into(),
                    function: targets[function],
                    type_arguments: type_arguments.clone().into(),
                    arguments: arguments.len() as u32,
                    effect_arguments: effect_arguments.clone().into(),
                    requirement_arguments: requirement_arguments.clone().into(),
                });
                single = true;
            }
            ExpressionOperation::ImplementationCall {
                read_scope_arguments,
                function,
                type_arguments,
                implementations,
                arguments,
                effect_arguments,
                requirement_arguments,
            } => {
                for argument in arguments {
                    emit(source, definition, *argument, targets, instructions, sites);
                }
                instructions.push(NormalizedInstruction::ImplementationCall {
                    read_scope_arguments: read_scope_arguments.clone().into(),
                    function: targets[function],
                    type_arguments: type_arguments.clone().into(),
                    implementations: implementations.clone().into(),
                    arguments: arguments.len() as u32,
                    effect_arguments: effect_arguments.clone().into(),
                    requirement_arguments: requirement_arguments.clone().into(),
                });
                single = true;
            }
            ExpressionOperation::FunctionValue {
                read_scope_arguments,
                function,
                type_arguments,
                effect_arguments,
                requirement_arguments,
            } => {
                instructions.push(NormalizedInstruction::FunctionValue {
                    read_scope_arguments: read_scope_arguments.clone().into(),
                    function: targets[function],
                    type_arguments: type_arguments.clone().into(),
                    effect_arguments: effect_arguments.clone().into(),
                    requirement_arguments: requirement_arguments.clone().into(),
                });
                single = true;
            }
            ExpressionOperation::MethodCall {
                read_scope_arguments,
                witness,
                contract,
                method,
                arguments,
            } => {
                for argument in arguments {
                    emit(source, definition, *argument, targets, instructions, sites);
                }
                instructions.push(NormalizedInstruction::MethodCall {
                    read_scope_arguments: read_scope_arguments.clone().into(),
                    witness: witness.clone(),
                    contract: *contract,
                    method: *method,
                    arguments: arguments.len() as u32,
                });
                single = true;
            }
            ExpressionOperation::Parallel { left, right } => {
                let call = |id| match &expression(source, id).operation {
                    ExpressionOperation::ImplementationCall {
                        function,
                        type_arguments,
                        implementations,
                        arguments,
                        read_scope_arguments,
                        ..
                    } => (
                        *function,
                        type_arguments.clone(),
                        implementations.clone(),
                        arguments.clone(),
                        read_scope_arguments.clone(),
                    ),
                    ExpressionOperation::Call {
                        function,
                        type_arguments,
                        arguments,
                        read_scope_arguments,
                        ..
                    } => (
                        *function,
                        type_arguments.clone(),
                        Vec::new(),
                        arguments.clone(),
                        read_scope_arguments.clone(),
                    ),
                    _ => panic!("fixture parallel child must be a literal call"),
                };
                let (left_function, left_types, left_implementations, left_arguments, left_scopes) =
                    call(*left);
                let (
                    right_function,
                    right_types,
                    right_implementations,
                    right_arguments,
                    right_scopes,
                ) = call(*right);
                for argument in left_arguments.iter().chain(&right_arguments) {
                    emit(source, definition, *argument, targets, instructions, sites);
                }
                sites.insert(instructions.len() as u32, Site::Parallel(*left, *right));
                instructions.push(NormalizedInstruction::Parallel {
                    left_read_scope_arguments: left_scopes.into(),
                    right_read_scope_arguments: right_scopes.into(),
                    left: targets[&left_function],
                    left_types: left_types.into(),
                    left_implementations: left_implementations.into(),
                    right: targets[&right_function],
                    right_types: right_types.into(),
                    right_implementations: right_implementations.into(),
                    left_arguments: left_arguments.len() as u32,
                    right_arguments: right_arguments.len() as u32,
                    // This is an unexecuted preparation-only model. Its projected
                    // result root is retained, not a fabricated transfer proof.
                    result_type: definition.result.ty,
                });
            }
            _ => panic!("syntax outside the finite raw fixture projection"),
        }
        if single {
            sites.insert((instructions.len() - 1) as u32, Site::Single(id));
        }
    }
    let mut instructions = Vec::new();
    emit(
        source,
        definition,
        definition.body,
        targets,
        &mut instructions,
        sites,
    );
    instructions.push(NormalizedInstruction::Return);
    NormalizedCode {
        parameter_count: definition.parameters.len() as u32,
        local_count: definition.parameters.len() as u32,
        instructions: instructions.into(),
    }
}

fn raw_templates(
    source: &KernelSnapshot,
    origin: super::super::super::value::ValueOrigin,
) -> (Vec<NormalizedFunction>, Sites) {
    let targets: BTreeMap<_, _> = source
        .owners
        .iter()
        .filter_map(|(key, owner)| {
            let (OwnerKey::Declaration(id), OwnerRecord::Declaration(owner)) = (key, owner) else {
                return None;
            };
            matches!(owner.payload, DeclarationPayload::Function(_)).then_some(
                DeclarationReference {
                    package: source.root.package_id,
                    declaration: *id,
                },
            )
        })
        .enumerate()
        .map(|(index, reference)| (reference, FunctionIndex(index as u32, origin)))
        .collect();
    let mut functions = vec![None; targets.len()];
    let mut sites = BTreeMap::new();
    for (reference, index) in &targets {
        let owner = declaration(source, *reference);
        let definition = function(source, *reference);
        let constraints = definition
            .type_parameters
            .iter()
            .map(|id| {
                let OwnerRecord::TypeParameter(parameter) =
                    &source.owners[&OwnerKey::TypeParameter(*id)]
                else {
                    panic!("fixture formal kind")
                };
                assert_eq!(parameter.declaration, reference.declaration);
                assert_eq!(parameter.header.owner, OwnerKey::TypeParameter(*id));
                parameter.constraints
            })
            .collect::<Vec<_>>();
        let parameters = definition
            .parameters
            .iter()
            .map(|id| {
                let OwnerRecord::Parameter(parameter) = &source.owners[&OwnerKey::Parameter(*id)]
                else {
                    panic!("fixture parameter kind")
                };
                assert_eq!(
                    parameter.parent,
                    ParameterParent::Function(reference.declaration)
                );
                assert_eq!(parameter.header.owner, OwnerKey::Parameter(*id));
                NormalizedParameter {
                    parameter: *id,
                    name: parameter.name.clone(),
                    ty: parameter.ty,
                    use_mode: parameter.use_mode,
                    resource_requirement: None,
                }
            })
            .collect::<Vec<_>>();
        let mut positions = BTreeMap::new();
        let body = raw_code(source, definition, &targets, &mut positions);
        sites.insert(*reference, positions);
        functions[index.0 as usize] = Some(NormalizedFunction {
            source_generation: owner.header.contract_version,
            declaration: *reference,
            graph_function: true,
            implementation_parameters: definition.implementation_parameters.clone().into(),
            implementation_arguments: Arc::from([]),
            callsites: Arc::from([]),
            requirement_parameters: definition.requirement_parameters.clone().into(),
            requirement_arguments: Arc::from([]),
            effect_parameters: definition.effect_parameters.clone().into(),
            effect: definition.effect.clone(),
            effect_arguments: Arc::from([]),
            type_parameters: definition.type_parameters.clone().into(),
            type_arguments: Arc::from([]),
            type_parameter_constraints: constraints.into(),
            parameter_count: parameters.len() as u32,
            parameters: parameters.into(),
            result: definition
                .result
                .try_map_source(|id| {
                    Ok(definition.parameters.iter().position(|p| *p == id).unwrap() as u32)
                })
                .unwrap(),
            read_scope_parameters: definition.read_scope_parameters.clone().into(),
            read_scope_sources: definition
                .read_scope_sources
                .iter()
                .cloned()
                .map(|source| {
                    source
                        .try_map_source(|id| {
                            Ok(definition.parameters.iter().position(|p| *p == id).unwrap() as u32)
                        })
                        .unwrap()
                })
                .collect::<Vec<_>>()
                .into(),
            read_scope_arguments: Arc::from([]),
            task_requirements: Arc::from([]),
            body: NormalizedFunctionBody::Code(body),
        });
    }
    (functions.into_iter().map(Option::unwrap).collect(), sites)
}

struct Produced {
    program: NormalizedProgram,
    sites: Sites,
    applications: BTreeSet<FunctionIndex>,
    raw: BTreeMap<DeclarationReference, NormalizedFunction>,
}
fn produce(
    source: &KernelSnapshot,
    entries: &[Entry<'_>],
    control: &ExecutionControl,
) -> Result<Produced, Diagnostic> {
    // Genuine feature-free preparation supplies owned test storage only. Its
    // admission token is untouched; this mutated raw model is never loaded/run.
    let (_, mut program) = crate::platform::execution::normalized::tests::owned_implementation_scheme_tests::duplicate_dag_fixture(1);
    let (templates, sites) = raw_templates(source, program.value_origin);
    let raw = templates
        .iter()
        .map(|function| (function.declaration, function.clone()))
        .collect();
    program.functions = templates.into();
    program.types.extend(source.types.clone());
    program.ports = Arc::from([]);
    program.tests.clear();
    let mut instructions = Vec::new();
    for entry in entries {
        let index = program
            .functions
            .iter()
            .position(|f| f.declaration == entry.function)
            .unwrap();
        instructions.push(NormalizedInstruction::ImplementationCall {
            read_scope_arguments: entry.read_scope_arguments.to_vec().into(),
            function: FunctionIndex(index as u32, program.value_origin),
            type_arguments: entry.types.to_vec().into(),
            implementations: entry.implementations.to_vec().into(),
            effect_arguments: entry.effects.to_vec().into(),
            requirement_arguments: entry
                .requirements
                .iter()
                .copied()
                .map(crate::platform::kernel::RequirementOperand::Concrete)
                .collect::<Vec<_>>()
                .into(),
            arguments: 0,
        });
        instructions.push(NormalizedInstruction::Drop);
    }
    instructions.extend([NormalizedInstruction::Unit, NormalizedInstruction::Return]);
    let root = entries.first().unwrap().function;
    program.tests.insert(
        root,
        NormalizedTest {
            declaration: root,
            comparison: crate::platform::kernel::ComparisonPolicy::Exact,
            actual: NormalizedCode {
                parameter_count: 0,
                local_count: 0,
                instructions: instructions.into(),
            },
            expected: NormalizedCode {
                parameter_count: 0,
                local_count: 0,
                instructions: Arc::from([
                    NormalizedInstruction::Unit,
                    NormalizedInstruction::Return,
                ]),
            },
        },
    );
    let mut work = Budget::new(control);
    super::super::close_effect_applications(&mut program, &mut work)?;
    let runtime_owners: BTreeMap<_, _> = source
        .owners
        .iter()
        .map(|(key, owner)| ((source.root.package_id, *key), owner.clone()))
        .collect();
    let mut implementations = BTreeMap::new();
    let mut contracts = BTreeMap::new();
    for (key, owner) in &source.owners {
        let (OwnerKey::Declaration(id), OwnerRecord::Declaration(owner)) = (key, owner) else {
            continue;
        };
        let reference = DeclarationReference {
            package: source.root.package_id,
            declaration: *id,
        };
        match &owner.payload {
            DeclarationPayload::OwnedImplementation(value) => {
                implementations.insert(reference, value);
            }
            DeclarationPayload::OwnedContract(value) => {
                contracts.insert(reference, value);
            }
            _ => {}
        }
    }
    // A method target is the original source callable, rather than an arbitrary
    // already specialized clone with the same DeclarationReference.
    let mut targets = BTreeMap::new();
    for (index, template) in program.functions.iter().enumerate() {
        targets
            .entry(template.declaration)
            .or_insert(FunctionIndex(index as u32, program.value_origin));
    }
    let mut closing = Closing {
        templates: Arc::clone(&program.functions),
        functions: program.functions.to_vec(),
        implementations,
        contracts,
        runtime_owners: &runtime_owners,
        producer_profiles: None,
        owned_records: program.owned_record_declarations.clone(),
        targets,
        instances: BTreeMap::new(),
        normalized: BTreeMap::new(),
        witnesses: BTreeMap::new(),
        pending: Vec::new(),
        pending_methods: Vec::new(),
        types: &mut program.types,
        work: &mut work,
    };
    for index in 0..closing.templates.len() {
        let function = &closing.templates[index];
        if function.type_parameters.is_empty()
            && function.implementation_parameters.is_empty()
            && function.effect_parameters.is_empty()
            && function.requirement_parameters.is_empty()
        {
            closing.application(
                FunctionIndex(index as u32, program.value_origin),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            )?;
        }
    }
    closing.code(
        &mut program.tests.get_mut(&root).unwrap().actual,
        None,
        &Bindings::new(),
        &BTreeMap::new(),
        &ScopeBindings::new(),
    )?;
    closing.close_pending()?;
    let applications = closing.instances.values().copied().collect();
    let catalogue = closing.normalized.values().cloned().collect::<Vec<_>>();
    let functions = std::mem::take(&mut closing.functions);
    drop(closing);
    program.functions = functions.into();
    program.implementation_applications = catalogue.into();
    Ok(Produced {
        program,
        sites,
        applications,
        raw,
    })
}

fn expected_key(
    proof: &Proof,
    produced: &Produced,
    index: FunctionIndex,
    control: &ExecutionControl,
) -> ExpectedApplication {
    assert_eq!(index.1, produced.program.value_origin);
    let function = &produced.program.functions[index.0 as usize];
    let witnesses = function
        .implementation_arguments
        .iter()
        .map(|actual| {
            proof
                .selections
                .iter()
                .find(|expected| {
                    proof
                        .compare_selection(
                            expected,
                            actual,
                            &produced.program.implementation_applications,
                            &produced.program.types,
                            control,
                        )
                        .is_ok()
                })
                .unwrap()
                .identity
        })
        .collect();
    ExpectedApplication {
        read_scope_arguments: function.read_scope_arguments.to_vec(),
        function: function.declaration,
        types: function.type_arguments.to_vec(),
        witnesses,
        effects: function.effect_arguments.to_vec(),
        requirements: function
            .requirement_arguments
            .iter()
            .map(|r| r.concrete().unwrap())
            .collect(),
    }
}

fn catalogue_bijection(
    proof: &Proof,
    produced: &Produced,
    control: &ExecutionControl,
) -> Result<(), &'static str> {
    if proof.selections.len() != produced.program.implementation_applications.len() {
        return Err("catalogue cardinality");
    }
    let mut matched = BTreeSet::new();
    for (identity, actual) in produced
        .program
        .implementation_applications
        .iter()
        .enumerate()
    {
        if actual.identity as usize != identity {
            return Err("catalogue identity");
        }
        let expected = proof
            .selections
            .iter()
            .find(|expected| {
                proof
                    .compare_selection(
                        expected,
                        actual,
                        &produced.program.implementation_applications,
                        &produced.program.types,
                        control,
                    )
                    .is_ok()
            })
            .ok_or("canonical selection correspondence")?;
        if !matched.insert(expected.identity) {
            return Err("duplicate canonical selection");
        }
    }
    let expected = proof
        .selections
        .iter()
        .map(|selection| selection.identity)
        .collect();
    if matched != expected {
        return Err("missing canonical selection");
    }
    Ok(())
}

fn compare(proof: &Proof, produced: &mut Produced, control: &ExecutionControl, shared: bool) {
    catalogue_bijection(proof, produced, control).expect("complete canonical catalogue bijection");
    let keys: BTreeSet<_> = produced
        .applications
        .iter()
        .map(|index| expected_key(proof, produced, *index, control))
        .collect();
    assert_eq!(
        keys,
        proof.applications.keys().cloned().collect(),
        "all applications including unused mapped methods"
    );
    assert_eq!(
        keys.len(),
        produced.applications.len(),
        "no aliased complete application keys"
    );
    for index in produced.applications.iter().copied() {
        let key = expected_key(proof, produced, index, control);
        let expected = &proof.applications[&key];
        let function = produced.program.functions[index.0 as usize].clone();
        let raw = &produced.raw[&function.declaration];
        assert_eq!(function.source_generation, raw.source_generation);
        assert_eq!(function.effect.row(), expected.effect);
        assert_eq!(function.result.relation, expected.result.relation);
        let bindings = function
            .type_parameters
            .iter()
            .copied()
            .zip(function.type_arguments.iter().copied())
            .collect();
        let environment = ClosedTypeEnvironment::for_function(&function).unwrap();
        let mut work = Budget::new(control);
        let scopes = function_scope_bindings(&function, &mut work).unwrap();
        let application = TypeApplication {
            bindings: &bindings,
            associated: Some(&environment),
            function: Some(index),
            scopes: Some(&scopes),
        };
        let effects = raw
            .effect_parameters
            .iter()
            .zip(function.effect_arguments.iter())
            .map(|(parameter, row)| {
                (
                    EffectParameterReference {
                        package: raw.declaration.package,
                        parameter: *parameter,
                    },
                    row.clone(),
                )
            })
            .collect();
        let requirements = raw
            .requirement_parameters
            .iter()
            .zip(function.requirement_arguments.iter())
            .map(|(parameter, actual)| {
                (
                    crate::platform::kernel::RequirementParameterReference {
                        package: raw.declaration.package,
                        parameter: *parameter,
                    },
                    *actual,
                )
            })
            .collect();
        for ((root, template), output) in &expected.roots {
            let effective = super::super::substitute_effect_type(
                &mut produced.program.types,
                *template,
                &effects,
                &requirements,
                0,
                &mut work,
            )
            .unwrap();
            let actual = substitute_in(
                &mut produced.program.types,
                effective,
                &application,
                0,
                &mut work,
            )
            .unwrap();
            assert_eq!(actual, *output, "canonical root {root:?}");
            assert_eq!(produced.program.types[&actual], proof.types[output]);
        }
        let result = substitute_in(
            &mut produced.program.types,
            function.result.ty,
            &application,
            0,
            &mut work,
        )
        .unwrap();
        assert_eq!(result, expected.result.ty);
        assert_eq!(function.parameters.len(), expected.parameters.len());
        for (parameter, (expected_type, use_mode)) in
            function.parameters.iter().zip(&expected.parameters)
        {
            let ty = substitute_in(
                &mut produced.program.types,
                parameter.ty,
                &application,
                0,
                &mut work,
            )
            .unwrap();
            assert_eq!((ty, parameter.use_mode), (*expected_type, *use_mode));
        }
        let NormalizedFunctionBody::Code(code) = &function.body else {
            panic!("fixture code")
        };
        let mut targets = BTreeMap::new();
        for (position, site) in &produced.sites[&function.declaration] {
            let call = if shared {
                function
                    .callsites
                    .iter()
                    .find(|(p, _)| p == position)
                    .unwrap()
                    .1
                    .clone()
            } else {
                match &code.instructions[*position as usize] {
                    NormalizedInstruction::Call {
                        function,
                        type_arguments,
                        ..
                    }
                    | NormalizedInstruction::FunctionValue {
                        function,
                        type_arguments,
                        ..
                    } => NormalizedCallSite::Call {
                        function: *function,
                        type_arguments: Arc::clone(type_arguments),
                    },
                    NormalizedInstruction::Parallel {
                        left,
                        left_types,
                        right,
                        right_types,
                        left_implementations,
                        right_implementations,
                        ..
                    } => {
                        assert!(
                            left_implementations.is_empty() && right_implementations.is_empty()
                        );
                        NormalizedCallSite::Parallel {
                            left: *left,
                            left_types: Arc::clone(left_types),
                            right: *right,
                            right_types: Arc::clone(right_types),
                        }
                    }
                    other => panic!("unclosed target {other:?}"),
                }
            };
            let mut insert = |expression, target, types: &[TypeObjectDigest]| {
                let application = expected_key(proof, produced, target, control);
                assert_eq!(application.types, types);
                assert!(produced.applications.contains(&target));
                targets.insert(expression, application);
            };
            match (site, call) {
                (
                    Site::Single(expression),
                    NormalizedCallSite::Call {
                        function,
                        type_arguments,
                    },
                ) => insert(*expression, function, &type_arguments),
                (
                    Site::Parallel(left_expression, right_expression),
                    NormalizedCallSite::Parallel {
                        left,
                        left_types,
                        right,
                        right_types,
                    },
                ) => {
                    insert(*left_expression, left, &left_types);
                    insert(*right_expression, right, &right_types);
                }
                _ => panic!("callsite shape changed"),
            }
        }
        assert_eq!(
            targets, expected.targets,
            "every call and untaken function value and both parallel children"
        );
    }
}

fn with_entries<R>(
    fixture: &Fixture,
    indexes: &[usize],
    effects: &[EffectRow],
    run: impl FnOnce(&[Entry<'_>]) -> R,
) -> R {
    let witnesses = indexes
        .iter()
        .map(|index| vec![operand(fixture, *index)])
        .collect::<Vec<_>>();
    let types = [fixture.cell];
    let entries = witnesses
        .iter()
        .map(|implementations| Entry {
            read_scope_arguments: &[],
            function: fixture.consumer,
            types: &types,
            implementations,
            effects,
            requirements: &[],
        })
        .collect::<Vec<_>>();
    run(&entries)
}

#[test]
fn actual_closing_matches_canonical_complete_catalogue_and_unused_method_applications() {
    let fixture = make_fixture();
    let control = ExecutionControl::uncancelled();
    with_entries(&fixture, &[0, 1, 2, 0], &[], |entries| {
        let proof =
            reference::derive(&[&fixture.source], entries, Limits::default(), &control).unwrap();
        let mut produced = produce(&fixture.source, entries, &control).unwrap();
        compare(&proof, &mut produced, &control, false);
        let selected = &produced.program.implementation_applications;
        assert_eq!(selected.len(), 3);
        assert_eq!(selected[0].self_type, selected[1].self_type);
        assert_eq!(selected[0].self_type, selected[2].self_type);
        assert_ne!(selected[0].implementation, selected[2].implementation);
        assert_eq!(selected[0].associated_types, selected[2].associated_types);
        assert_ne!(selected[0].associated_types, selected[1].associated_types);
        assert_ne!(selected[0].identity, selected[2].identity);
        assert_eq!(proof.methods.len(), 3);
    });
}

#[test]
fn actual_closing_comparator_refuses_duplicate_selection_with_omitted_equal_output_identity() {
    let fixture = make_fixture();
    let control = ExecutionControl::uncancelled();
    with_entries(&fixture, &[0, 1, 2], &[], |entries| {
        let proof =
            reference::derive(&[&fixture.source], entries, Limits::default(), &control).unwrap();
        let mut produced = produce(&fixture.source, entries, &control).unwrap();
        catalogue_bijection(&proof, &produced, &control).unwrap();
        let catalogue = &produced.program.implementation_applications;
        assert_ne!(catalogue[0].implementation, catalogue[2].implementation);
        assert_eq!(catalogue[0].self_type, catalogue[2].self_type);
        assert_eq!(catalogue[0].associated_types, catalogue[2].associated_types);
        let mut duplicate = catalogue[0].as_ref().clone();
        duplicate.identity = 2;
        Arc::make_mut(&mut produced.program.implementation_applications)[2] = Arc::new(duplicate);
        // Every individual entry still matches some complete canonical selection
        // with exact catalogue pointer membership. Cardinality alone is unchanged.
        for actual in produced.program.implementation_applications.iter() {
            assert!(proof.selections.iter().any(|expected| {
                proof
                    .compare_selection(
                        expected,
                        actual,
                        &produced.program.implementation_applications,
                        &produced.program.types,
                        &control,
                    )
                    .is_ok()
            }));
        }
        assert_eq!(
            catalogue_bijection(&proof, &produced, &control),
            Err("duplicate canonical selection")
        );
    });
}

fn add_body_targets(fixture: &mut Fixture) {
    let definition = function(&fixture.source, fixture.consumer).clone();
    let parameter = definition.parameters[0];
    let witness = ImplementationOperand::Parameter {
        scope: fixture.consumer,
        parameter: definition.implementation_parameters[0].id,
    };
    let contract = declaration(&fixture.source, fixture.contract);
    let DeclarationPayload::OwnedContract(contract) = &contract.payload else {
        unreachable!()
    };
    let method = contract.methods[0].id;
    let local = add_expression(
        &mut fixture.source,
        10,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(parameter),
        },
    );
    let called = add_expression(
        &mut fixture.source,
        11,
        ExpressionOperation::MethodCall {
            read_scope_arguments: vec![],
            witness,
            contract: fixture.contract,
            method,
            arguments: vec![local],
        },
    );
    let value = add_expression(
        &mut fixture.source,
        12,
        ExpressionOperation::FunctionValue {
            read_scope_arguments: vec![],
            function: fixture.reader,
            type_arguments: vec![fixture.projection],
            effect_arguments: vec![],
            requirement_arguments: vec![],
        },
    );
    let other = add_expression(
        &mut fixture.source,
        13,
        ExpressionOperation::FunctionValue {
            read_scope_arguments: vec![],
            function: fixture.reader,
            type_arguments: vec![fixture.cell],
            effect_arguments: vec![],
            requirement_arguments: vec![],
        },
    );
    let condition = add_expression(
        &mut fixture.source,
        14,
        ExpressionOperation::Bool { value: true },
    );
    let branch = add_expression(
        &mut fixture.source,
        15,
        ExpressionOperation::If {
            condition,
            when_true: other,
            when_false: value,
        },
    );
    let mut calls = Vec::new();
    for id in [16, 17] {
        calls.push(add_expression(
            &mut fixture.source,
            id,
            ExpressionOperation::Call {
                read_scope_arguments: vec![],
                function: fixture.reader,
                type_arguments: vec![fixture.projection],
                arguments: vec![local],
                effect_arguments: vec![],
                requirement_arguments: vec![],
            },
        ));
    }
    let parallel = add_expression(
        &mut fixture.source,
        18,
        ExpressionOperation::Parallel {
            left: calls[0],
            right: calls[1],
        },
    );
    // The caller-written ordinary and concrete witness actuals both contain
    // its lexical projection. Closing must resolve both before changing scope.
    // This finite fixture stabilizes at supplier0<Cell>; it is not a growth token.
    let projected_call = add_expression(
        &mut fixture.source,
        20,
        ExpressionOperation::ImplementationCall {
            read_scope_arguments: vec![],
            function: fixture.consumer,
            type_arguments: vec![fixture.projection],
            implementations: vec![ImplementationOperand::Concrete {
                implementation: fixture.implementations[0],
                type_arguments: vec![fixture.projection],
                implementations: vec![],
            }],
            arguments: vec![local],
            effect_arguments: vec![],
            requirement_arguments: vec![],
        },
    );
    let sequence = add_expression(
        &mut fixture.source,
        19,
        ExpressionOperation::Sequence {
            items: vec![called, branch, projected_call, parallel, local],
        },
    );
    let OwnerRecord::Declaration(owner) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.consumer.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(definition) = &mut owner.payload else {
        unreachable!()
    };
    definition.body = sequence;
}

fn add_unused_method(fixture: &mut Fixture) {
    let original = declaration(&fixture.source, fixture.reader).clone();
    let mut definition = function(&fixture.source, fixture.reader).clone();
    let reference = DeclarationReference {
        package: fixture.reader.package,
        declaration: DeclarationId::migrate(SEED, 0),
    };
    let formal = TypeParameterId::migrate(SEED, 0);
    let parameter = ParameterId::migrate(SEED, 0);
    let ty = add_type(
        &mut fixture.source,
        TypeForm::TypeParameter { parameter: formal },
    );
    fixture.source.owners.insert(
        OwnerKey::TypeParameter(formal),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(formal), OwnerKind::TypeParameter),
            declaration: reference.declaration,
            name: Name::new("V").unwrap(),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    let mut parameter_record =
        match &fixture.source.owners[&OwnerKey::Parameter(definition.parameters[0])] {
            OwnerRecord::Parameter(parameter) => parameter.clone(),
            _ => unreachable!(),
        };
    parameter_record.header =
        OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter);
    parameter_record.parent = ParameterParent::Function(reference.declaration);
    parameter_record.ty = ty;
    fixture.source.owners.insert(
        OwnerKey::Parameter(parameter),
        OwnerRecord::Parameter(parameter_record),
    );
    definition.type_parameters = vec![formal];
    definition.parameters = vec![parameter];
    definition.result.ty = ty;
    definition.result.relation =
        crate::platform::kernel::ResultRelation::ReadFrom { source: parameter };
    definition.body = add_expression(
        &mut fixture.source,
        30,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(parameter),
        },
    );
    fixture.source.owners.insert(
        OwnerKey::Declaration(reference.declaration),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(reference.declaration),
                OwnerKind::PureFunction,
            ),
            module: original.module,
            visibility: original.visibility,
            name: Name::new("unused-read").unwrap(),
            payload: DeclarationPayload::Function(definition),
        }),
    );
    let abstract_member = add_type(
        &mut fixture.source,
        TypeForm::AssociatedContractMember {
            contract: fixture.contract,
            member: fixture.spare,
        },
    );
    let method = MethodId::migrate(SEED, 0);
    let OwnerRecord::Declaration(owner) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.contract.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedContract(contract) = &mut owner.payload else {
        unreachable!()
    };
    let mut signature = contract.methods[0].clone();
    signature.id = method;
    signature.name = Name::new("unused").unwrap();
    signature.parameters[0].ty = abstract_member;
    signature.result.ty = abstract_member;
    contract.methods.push(signature);
    for implementation in fixture.implementations {
        let OwnerRecord::Declaration(owner) = fixture
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(implementation.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::OwnedImplementation(scheme) = &mut owner.payload else {
            unreachable!()
        };
        scheme
            .methods
            .push(crate::platform::kernel::OwnedMethodImplementation {
                method,
                function: reference,
                type_arguments: vec![fixture.cell],
                implementations: vec![],
                read_scope_arguments: vec![],
            });
        scheme.methods.sort_by_key(|mapping| mapping.method);
    }
}

fn composed_operand(fixture: &mut Fixture) -> ImplementationOperand {
    let DeclarationPayload::OwnedContract(contract) =
        &declaration(&fixture.source, fixture.contract).payload
    else {
        unreachable!()
    };
    let method = contract
        .methods
        .iter()
        .find(|method| method.name.as_str() == "read")
        .unwrap()
        .id;
    let original = declaration(&fixture.source, fixture.implementations[0]).clone();
    let DeclarationPayload::OwnedImplementation(mut scheme) = original.payload else {
        unreachable!()
    };
    let reference = DeclarationReference {
        package: fixture.consumer.package,
        declaration: DeclarationId::migrate(SEED, 1),
    };
    let formal = TypeParameterId::migrate(SEED, 1);
    let ty = add_type(
        &mut fixture.source,
        TypeForm::TypeParameter { parameter: formal },
    );
    fixture.source.owners.insert(
        OwnerKey::TypeParameter(formal),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(formal), OwnerKind::TypeParameter),
            declaration: reference.declaration,
            name: Name::new("T").unwrap(),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    scheme.type_parameters = vec![formal];
    scheme.self_type = ty;
    scheme.implementation_parameters = [0, 1]
        .into_iter()
        .map(|index| crate::platform::kernel::ImplementationParameter {
            id: ImplementationParameterId::migrate(SEED, index),
            name: Name::new(format!("child{index}")).unwrap(),
            contract: fixture.contract,
            self_type: ty,
            type_arguments: vec![],
        })
        .collect();
    for binding in &mut scheme.associated_types {
        if binding.member == fixture.member {
            *binding.value.ty_mut() = ty;
        }
    }
    scheme
        .methods
        .iter_mut()
        .find(|mapping| mapping.method == method)
        .unwrap()
        .type_arguments = vec![ty];
    fixture.source.owners.insert(
        OwnerKey::Declaration(reference.declaration),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(reference.declaration),
                OwnerKind::OwnedImplementation,
            ),
            module: original.module,
            visibility: original.visibility,
            name: Name::new("composed").unwrap(),
            payload: DeclarationPayload::OwnedImplementation(scheme),
        }),
    );
    ImplementationOperand::Concrete {
        implementation: reference,
        type_arguments: vec![fixture.cell],
        implementations: vec![operand(fixture, 0), operand(fixture, 0)],
    }
}

#[test]
fn actual_closing_matches_complete_shared_prerequisites_and_an_uncalled_distinct_method() {
    let mut fixture = make_fixture();
    add_unused_method(&mut fixture);
    let selected = composed_operand(&mut fixture);
    let types = [fixture.cell];
    let implementations = [selected];
    let entries = [Entry {
        read_scope_arguments: &[],
        function: fixture.consumer,
        types: &types,
        implementations: &implementations,
        effects: &[],
        requirements: &[],
    }];
    let control = ExecutionControl::uncancelled();
    let proof =
        reference::derive(&[&fixture.source], &entries, Limits::default(), &control).unwrap();
    let mut produced = produce(&fixture.source, &entries, &control).unwrap();
    compare(&proof, &mut produced, &control, false);
    assert_eq!(produced.program.implementation_applications.len(), 2);
    let parent = produced
        .program
        .implementation_applications
        .iter()
        .find(|node| node.implementations.len() == 2)
        .unwrap();
    assert!(Arc::ptr_eq(
        &parent.implementations[0],
        &parent.implementations[1]
    ));
    assert_eq!(parent.prerequisites.len(), 2);
    assert_eq!(parent.associated_types.len(), 2);
    assert_eq!(
        proof.methods.len(),
        4,
        "both methods of both complete selected nodes"
    );
    assert!(
        produced.applications.iter().any(|index| {
            let function = &produced.program.functions[index.0 as usize];
            function.declaration.declaration == DeclarationId::migrate(SEED, 0)
                && function.type_arguments.as_ref() == types
        }),
        "an unused generic method target must be closed by real Closing"
    );
}

#[test]
fn actual_closing_and_canonical_reference_refuse_unused_binding_and_child_obligation_corruption() {
    for fault in 0..3 {
        let mut fixture = make_fixture();
        let mut selected = composed_operand(&mut fixture);
        let ImplementationOperand::Concrete {
            implementation,
            implementations,
            ..
        } = &mut selected
        else {
            unreachable!()
        };
        if fault == 0 {
            let scalar = add_type(&mut fixture.source, TypeForm::I64);
            let OwnerRecord::Declaration(owner) = fixture
                .source
                .owners
                .get_mut(&OwnerKey::Declaration(implementation.declaration))
                .unwrap()
            else {
                unreachable!()
            };
            let DeclarationPayload::OwnedImplementation(scheme) = &mut owner.payload else {
                unreachable!()
            };
            *scheme
                .associated_types
                .iter_mut()
                .find(|binding| binding.member == fixture.spare)
                .unwrap()
                .value
                .ty_mut() = scalar;
        } else if fault == 1 {
            let ImplementationOperand::Concrete { type_arguments, .. } = &mut implementations[1]
            else {
                unreachable!()
            };
            type_arguments[0] = fixture.block;
        } else {
            let OwnerRecord::Declaration(owner) = fixture
                .source
                .owners
                .get_mut(&OwnerKey::Declaration(implementation.declaration))
                .unwrap()
            else {
                unreachable!()
            };
            owner.header.contract_version = 32;
        }
        let types = [fixture.cell];
        let implementations = [selected];
        let entries = [Entry {
            read_scope_arguments: &[],
            function: fixture.consumer,
            types: &types,
            implementations: &implementations,
            effects: &[],
            requirements: &[],
        }];
        let control = ExecutionControl::uncancelled();
        assert!(
            reference::derive(&[&fixture.source], &entries, Limits::default(), &control).is_err(),
            "canonical fault {fault}"
        );
        assert!(
            produce(&fixture.source, &entries, &control).is_err(),
            "producer fault {fault}"
        );
    }
}

#[test]
fn actual_closing_matches_untaken_targets_and_parallel_then_shares_body_with_exact_callsite_tables()
{
    let mut fixture = make_fixture();
    add_body_targets(&mut fixture);
    let control = ExecutionControl::uncancelled();
    with_entries(&fixture, &[0, 1, 2], &[], |entries| {
        let proof =
            reference::derive(&[&fixture.source], entries, Limits::default(), &control).unwrap();
        let mut produced = produce(&fixture.source, entries, &control).unwrap();
        compare(&proof, &mut produced, &control, false);
        let mut work = Budget::new(&control);
        super::super::shared_code::share(&mut produced.program, &mut work).unwrap();
        compare(&proof, &mut produced, &control, true);
        let consumers: Vec<_> = produced
            .applications
            .iter()
            .filter(|index| {
                let function = &produced.program.functions[index.0 as usize];
                function.declaration == fixture.consumer
                    && function.type_arguments.as_ref() == [fixture.cell]
            })
            .copied()
            .collect();
        assert_eq!(consumers.len(), 3);
        let a = &produced.program.functions[consumers[0].0 as usize];
        let b = &produced.program.functions[consumers[1].0 as usize];
        let NormalizedFunctionBody::Code(a_code) = &a.body else {
            unreachable!()
        };
        let NormalizedFunctionBody::Code(b_code) = &b.body else {
            unreachable!()
        };
        assert!(Arc::ptr_eq(&a_code.instructions, &b_code.instructions));
        assert_ne!(a.callsites, b.callsites);
        assert_eq!(a.callsites.len(), 5);
        // Sharing bytes never changes the independent origin boundary.
        let foreign = super::super::super::value::ValueOrigin::fresh().unwrap();
        let ordinary = a
            .type_parameters
            .iter()
            .copied()
            .zip(a.type_arguments.iter().copied())
            .collect();
        assert!(
            PreparedApplicationContext::new(&produced.program, consumers[0], &ordinary, &control)
                .is_ok()
        );
        assert!(
            PreparedApplicationContext::new(
                &produced.program,
                FunctionIndex(consumers[0].0, foreign),
                &ordinary,
                &control
            )
            .is_err()
        );
        let selected = Arc::make_mut(&mut produced.program.implementation_applications)[0].clone();
        let mut changed = selected.as_ref().clone();
        changed.associated_types = changed
            .associated_types
            .iter()
            .cloned()
            .map(|mut binding| {
                binding.template = fixture.block;
                binding
            })
            .collect::<Vec<_>>()
            .into();
        let changed = Arc::new(changed);
        let mut catalogue = produced.program.implementation_applications.to_vec();
        catalogue[changed.identity as usize] = Arc::clone(&changed);
        let expected = &proof.selections[0];
        assert!(
            proof
                .compare_selection(
                    expected,
                    &changed,
                    &catalogue,
                    &produced.program.types,
                    &control
                )
                .is_err()
        );
    });
}

#[test]
fn actual_closing_preserves_source_effect_scope_before_witness_selected_signature_closure() {
    let mut fixture = make_fixture();
    let id = EffectParameterId::migrate(SEED, 0);
    let reference = EffectParameterReference {
        package: fixture.consumer.package,
        parameter: id,
    };
    fixture.source.owners.insert(
        OwnerKey::EffectParameter(id),
        OwnerRecord::EffectParameter(EffectParameterRecord {
            header: OwnerHeader::new(OwnerKey::EffectParameter(id), OwnerKind::EffectParameter),
            declaration: fixture.consumer.declaration,
            name: Name::new("row").unwrap(),
        }),
    );
    let callback = add_type(
        &mut fixture.source,
        TypeForm::TaskFunction {
            parameters: vec![],
            result: fixture.projection,
            effect: EffectRow {
                parameters: vec![reference],
                requirements: vec![],
            },
        },
    );
    let parameter = ParameterId::migrate(SEED, 1);
    fixture.source.owners.insert(
        OwnerKey::Parameter(parameter),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
            parent: ParameterParent::Function(fixture.consumer.declaration),
            name: Name::new("callback").unwrap(),
            ty: callback,
            use_mode: ParameterUse::Unrestricted,
            resource_requirement: None,
        }),
    );
    let primary = function(&fixture.source, fixture.consumer).parameters[0];
    let OwnerRecord::Parameter(primary) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Parameter(primary))
        .unwrap()
    else {
        unreachable!()
    };
    primary.use_mode = ParameterUse::Consume;
    let OwnerRecord::Declaration(owner) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.consumer.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(value) = &mut owner.payload else {
        unreachable!()
    };
    value.effect_parameters = vec![id];
    value.effect = FunctionEffect::Task {
        requirements: vec![],
        effect_parameters: vec![reference],
    };
    value.parameters.push(parameter);
    value.result.relation = crate::platform::kernel::ResultRelation::Value;
    let control = ExecutionControl::uncancelled();
    with_entries(&fixture, &[0, 1], &[EffectRow::default()], |entries| {
        let proof =
            reference::derive(&[&fixture.source], entries, Limits::default(), &control).unwrap();
        let mut produced = produce(&fixture.source, entries, &control).unwrap();
        compare(&proof, &mut produced, &control, false);
        for index in &produced.applications {
            let function = &produced.program.functions[index.0 as usize];
            if function.declaration == fixture.consumer {
                assert!(function.effect_parameters.is_empty());
                assert_eq!(function.effect_arguments.as_ref(), &[EffectRow::default()]);
                assert_eq!(function.declaration, fixture.consumer);
                assert!(function.effect.row().is_closed());
            }
        }
    });
}
