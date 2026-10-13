//! Private canonical compiler correspondence, never an associated admission token.
use super::*;
use crate::platform::execution::ExecutionControl;
use crate::platform::kernel::{
    AssociatedTypeBinding, AssociatedTypeBound, AssociatedTypeMember, FunctionDeclaration,
    ImplementationOperand, ImplementationParameter, KernelSnapshot, Name, OwnerHeader, OwnerKind,
    RetirementRecord, SemanticRoot, TypeParameterConstraints, TypeParameterRecord,
};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, ImplementationParameterId, RepositoryId, RevisionId,
};

const SOURCE: &str = r#"declarations.begin
(units (module create parallel-associated-compiler
  (owned-contract create Reader (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_d7000000000000000000000000000001 inspect (parameters) (returns I64)))
  (function create inspect (visibility public) (effect pure) (returns I64) (body (i64 7)))
  (owned-implementation create First (visibility public)
    (type-parameter create T (constraint owned transferable)) (contract Reader) (self T)
    (method method_d7000000000000000000000000000001 inspect))
  (owned-implementation create Second (visibility public)
    (type-parameter create T (constraint owned transferable)) (contract Reader) (self T)
    (method method_d7000000000000000000000000000001 inspect))
  (function create child (visibility public) (effect (task))
    (type-parameter create T (constraint owned transferable))
    (type-parameter create Unused (constraint owned))
    (implementation-parameter implparam_d7000000000000000000000000000001 ops Reader T)
    (parameter create payload (type T) (use consume))
    (returns T) (body (local payload)))
  (function create scalar (visibility public) (effect (task)) (returns I64) (body (i64 11)))
  (function create plain (visibility public) (effect (task)) (returns I64) (body (i64 23)))
  (function create joined (visibility public) (effect (task))
    (parameter create a (type ByteBuffer) (use consume))
    (parameter create b (type ByteBuffer) (use consume))
    (returns (owned-product (field left ByteBuffer) (field right ByteBuffer)))
    (body (parallel
      (implementation-call child (types ByteBuffer ByteBuffer)
        (implementations (implementation First (types ByteBuffer))) (local a))
      (implementation-call child (types ByteBuffer ByteBuffer)
        (implementations (implementation Second (types ByteBuffer))) (local b)))))))
declarations.end
"#;

const ORDINARY: &str = r#"declarations.begin
(units (module create unchanged-parallel
  (function create first (visibility private) (effect (task)) (returns I64) (body (i64 17)))
  (function create second (visibility private) (effect (task)) (returns I64) (body (i64 -29)))
  (function create joined (visibility public) (effect (task)) (returns (record (left I64) (right I64)))
    (body (parallel (call first) (call second))))))
declarations.end
"#;

const ORDINARY_NOMINAL: &str = r#"declarations.begin
(units (module create unchanged-nominal-cache
  (owned-record create Carrier (visibility public)
    (type-parameter create T (constraint transferable))
    (field create storage (type (owned-sequence (option T)))))
  (function create hold (visibility public) (effect pure)
    (parameter create value (type (Carrier I64)) (use consume))
    (returns (Carrier I64)) (body (local value)))))
declarations.end
"#;

fn author(literal: &str) -> KernelSnapshot {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(literal).unwrap()
}

fn named(source: &KernelSnapshot, name: &str) -> DeclarationReference {
    source
        .owners
        .iter()
        .find_map(|(key, record)| match (key, record) {
            (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record))
                if record.name.as_str() == name =>
            {
                Some(DeclarationReference {
                    package: source.root.package_id,
                    declaration: *declaration,
                })
            }
            _ => None,
        })
        .unwrap()
}

fn function(source: &KernelSnapshot, reference: DeclarationReference) -> &FunctionDeclaration {
    let OwnerRecord::Declaration(record) =
        &source.owners[&OwnerKey::Declaration(reference.declaration)]
    else {
        panic!("function owner");
    };
    let DeclarationPayload::Function(function) = &record.payload else {
        panic!("function payload");
    };
    function
}

fn function_mut(
    source: &mut KernelSnapshot,
    reference: DeclarationReference,
) -> &mut FunctionDeclaration {
    let OwnerRecord::Declaration(record) = source
        .owners
        .get_mut(&OwnerKey::Declaration(reference.declaration))
        .unwrap()
    else {
        panic!("function owner");
    };
    let DeclarationPayload::Function(function) = &mut record.payload else {
        panic!("function payload");
    };
    function
}

fn add_type(source: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    source.types.insert(digest, object);
    digest
}

struct Fixture {
    source: KernelSnapshot,
    joined: DeclarationReference,
    child: DeclarationReference,
    contract: DeclarationReference,
    implementations: [DeclarationReference; 2],
    member: AssociatedTypeMemberId,
    spare: AssociatedTypeMemberId,
    cell: TypeObjectDigest,
    buffer: TypeObjectDigest,
    pair: TypeObjectDigest,
    wrapper: ExpressionId,
    calls: [ExpressionId; 2],
}

fn fixture(equal_outputs: bool) -> Fixture {
    let mut source = author(SOURCE);
    let joined = named(&source, "joined");
    let child = named(&source, "child");
    let contract = named(&source, "Reader");
    let implementations = [named(&source, "First"), named(&source, "Second")];
    let cell = add_type(&mut source, TypeForm::OwnedI64Cell);
    let buffer = add_type(&mut source, TypeForm::ByteBuffer);
    let member = AssociatedTypeMemberId::migrate(b"compiler-parallel-member", 0);
    let spare = AssociatedTypeMemberId::migrate(b"compiler-parallel-member", 1);
    let OwnerRecord::Declaration(record) = source
        .owners
        .get_mut(&OwnerKey::Declaration(contract.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedContract(value) = &mut record.payload else {
        unreachable!()
    };
    value.associated_types = vec![
        AssociatedTypeMember {
            id: spare,
            name: Name::new("Unused").unwrap(),
            bound: AssociatedTypeBound::Owned,
        },
        AssociatedTypeMember {
            id: member,
            name: Name::new("Block").unwrap(),
            bound: AssociatedTypeBound::Owned,
        },
    ];
    let outputs = [cell, if equal_outputs { cell } else { buffer }];
    for (implementation, output) in implementations.into_iter().zip(outputs) {
        let OwnerRecord::Declaration(record) = source
            .owners
            .get_mut(&OwnerKey::Declaration(implementation.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::OwnedImplementation(value) = &mut record.payload else {
            unreachable!()
        };
        value.associated_types = vec![
            AssociatedTypeBinding {
                member,
                value: crate::platform::kernel::AssociatedTypeBindingValue::Owned { ty: output },
            },
            AssociatedTypeBinding {
                member: spare,
                value: crate::platform::kernel::AssociatedTypeBindingValue::Owned { ty: cell },
            },
        ];
        value.associated_types.sort_by_key(|binding| binding.member);
    }
    let witness = function(&source, child).implementation_parameters[0].id;
    let projection = add_type(
        &mut source,
        TypeForm::AssociatedWitnessProjection {
            scope: child,
            parameter: witness,
            contract,
            member,
        },
    );
    let child_parameter = function(&source, child).parameters[0];
    function_mut(&mut source, child).result.ty = projection;
    let OwnerRecord::Parameter(parameter) = source
        .owners
        .get_mut(&OwnerKey::Parameter(child_parameter))
        .unwrap()
    else {
        unreachable!()
    };
    parameter.ty = projection;
    let joined_parameters = function(&source, joined).parameters.clone();
    for (parameter, output) in joined_parameters.into_iter().zip(outputs) {
        let OwnerRecord::Parameter(parameter) = source
            .owners
            .get_mut(&OwnerKey::Parameter(parameter))
            .unwrap()
        else {
            unreachable!()
        };
        parameter.ty = output;
    }
    let original_pair = function(&source, joined).result.ty;
    let pair = add_type(
        &mut source,
        TypeForm::OwnedProduct {
            fields: vec![
                StructuralTypeField {
                    name: Name::new("left").unwrap(),
                    ty: outputs[0],
                },
                StructuralTypeField {
                    name: Name::new("right").unwrap(),
                    ty: outputs[1],
                },
            ],
        },
    );
    function_mut(&mut source, joined).result.ty = pair;
    // The old ByteBuffer/ByteBuffer result has no remaining owner or type reference. Keep the
    // complete logical fixture free of this exact obsolete object; do not prune other types.
    assert_ne!(original_pair, pair);
    assert!(
        source
            .owners
            .values()
            .all(|owner| !owner.type_roots().contains(&original_pair))
    );
    assert!(
        source
            .types
            .values()
            .all(|object| !object.child_types().contains(&original_pair))
    );
    source
        .types
        .remove(&original_pair)
        .expect("exact replaced pair was present and is now unreferenced");
    let wrapper = function(&source, joined).body;
    let OwnerRecord::Expression(record) = &source.owners[&OwnerKey::Expression(wrapper)] else {
        unreachable!()
    };
    let ExpressionOperation::Parallel { left, right } = record.operation else {
        unreachable!()
    };
    Fixture {
        source,
        joined,
        child,
        contract,
        implementations,
        member,
        spare,
        cell,
        buffer,
        pair,
        wrapper,
        calls: [left, right],
    }
}

/// Derive genuine source dimensions without a FullWitness or validation token.
/// This raw unit is only compiler correspondence input, never LoadedArtifact.
fn raw_compile(
    source: &KernelSnapshot,
    scope: DeclarationReference,
) -> Result<(CompilationUnit, BTreeMap<ObjectKey, Vec<u8>>), Diagnostic> {
    let owner = OwnerKey::Declaration(scope.declaration);
    let facts = crate::platform::witness::rebuild_canonical_facts(source)?;
    let summary = &facts.summaries[&owner];
    let binding = CompilationSource {
        package: scope.package,
        owner,
        kind: summary.kind,
        semantic_interface: summary.semantic_interface,
        implementation: summary.implementation,
        type_digest: summary.type_digest,
        effect: summary.effect,
        capability: summary.capability,
        test: summary.test,
        validation_dependencies: summary.validation_dependencies,
    };
    let mut builder = UnitBuilder {
        canonical: source,
        package: scope.package,
        scope: Some(scope.declaration),
        tables: TablesBuilder::default(),
        derived_type_objects: BTreeMap::new(),
        derived_type_admission: DerivedTypeAdmission::default(),
        work: CompilationWork::default(),
    };
    let payload = builder.compile_payload(owner, source.owners[&owner].clone(), &[])?;
    builder.complete_owned_record_type_closure()?;
    let optimization = OptimizationPolicy::DeterministicBaseline;
    let unit = CompilationUnit {
        contract_version: COMPILER_UNIT_CONTRACT_VERSION,
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        bytecode_contract_version: BYTECODE_CONTRACT_VERSION,
        key: CompilationUnitKey::derive(&binding, optimization)?,
        source: binding,
        optimization,
        tables: builder.tables.finish(),
        payload,
    };
    Ok((unit, builder.derived_type_objects))
}

fn code(unit: &CompilationUnit) -> &CompiledCode {
    let CompilationPayload::Function { code, .. } = &unit.payload else {
        unreachable!()
    };
    code
}

fn code_mut(unit: &mut CompilationUnit) -> &mut CompiledCode {
    let CompilationPayload::Function { code, .. } = &mut unit.payload else {
        unreachable!()
    };
    code
}

fn pair(unit: &CompilationUnit) -> TypeObjectDigest {
    code(unit)
        .instructions
        .iter()
        .find_map(|instruction| match instruction {
            CompiledInstruction::Parallel { result_type, .. } => {
                Some(unit.tables.types[*result_type as usize])
            }
            _ => None,
        })
        .unwrap()
}

/// Exact canonical caller formals, without fabricating an accepted witness.
fn caller_projection(f: &mut Fixture) -> (TypeObjectDigest, ImplementationParameterId) {
    let formal = TypeParameterId::migrate(b"compiler-parallel-caller-formal", 0);
    f.source.owners.insert(
        OwnerKey::TypeParameter(formal),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(formal), OwnerKind::TypeParameter),
            declaration: f.joined.declaration,
            name: Name::new("CallerSelf").unwrap(),
            constraints: TypeParameterConstraints::OwnedTransferable,
        }),
    );
    let self_type = add_type(&mut f.source, TypeForm::TypeParameter { parameter: formal });
    let witness = ImplementationParameterId::migrate(b"compiler-parallel-caller-witness", 0);
    let joined = function_mut(&mut f.source, f.joined);
    joined.type_parameters.push(formal);
    joined
        .implementation_parameters
        .push(ImplementationParameter {
            id: witness,
            name: Name::new("CallerReader").unwrap(),
            contract: f.contract,
            self_type,
            type_arguments: vec![],
        });
    let projection = add_type(
        &mut f.source,
        TypeForm::AssociatedWitnessProjection {
            scope: f.joined,
            parameter: witness,
            contract: f.contract,
            member: f.member,
        },
    );
    (projection, witness)
}

#[test]
fn associated_parallel_exact_witnesses_select_both_outputs_and_preserve_equal_self_identity() {
    for equal_outputs in [false, true] {
        let f = fixture(equal_outputs);
        let (unit, derived) = raw_compile(&f.source, f.joined).unwrap();
        assert_eq!(pair(&unit), f.pair);
        let instruction = code(&unit)
            .instructions
            .iter()
            .find(|instruction| matches!(instruction, CompiledInstruction::Parallel { .. }))
            .unwrap();
        let CompiledInstruction::Parallel {
            left_implementations,
            right_implementations,
            ..
        } = instruction
        else {
            unreachable!()
        };
        assert_eq!(
            left_implementations,
            &[ImplementationOperand::Concrete {
                implementation: f.implementations[0],
                type_arguments: vec![f.buffer],
                implementations: vec![]
            }]
        );
        assert_eq!(
            right_implementations,
            &[ImplementationOperand::Concrete {
                implementation: f.implementations[1],
                type_arguments: vec![f.buffer],
                implementations: vec![]
            }]
        );
        assert_ne!(left_implementations, right_implementations);
        // Literal expectation, independent of producer selection arrays/substitution.
        let expected = TypeObject::new(TypeForm::OwnedProduct {
            fields: vec![
                StructuralTypeField {
                    name: Name::new("left").unwrap(),
                    ty: f.cell,
                },
                StructuralTypeField {
                    name: Name::new("right").unwrap(),
                    ty: if equal_outputs { f.cell } else { f.buffer },
                },
            ],
        })
        .unwrap();
        assert_eq!(
            derived[&ObjectKey::from_digest(
                crate::platform::storage::object::ObjectDomain::Type,
                f.pair.bytes()
            )],
            encode_type_object(&expected).unwrap().1
        );
        // The fixture changes complete logical records after real ordinary authoring. Its
        // persistent map pages still describe that baseline, so this is full semantic oracle
        // agreement, not source transport, publication or an associated admission token.
        crate::platform::kernel::validate_full(&f.source)
            .expect("both exact witness outputs have complete logical associated semantics");
    }
}

#[test]
fn associated_parallel_unused_members_and_actuals_are_admitted_without_retaining_transient_types() {
    let mut f = fixture(false);
    let scalar = add_type(&mut f.source, TypeForm::I64);
    let unused = add_type(&mut f.source, TypeForm::OwnedSequence { item: scalar });
    for implementation in f.implementations {
        let OwnerRecord::Declaration(record) = f
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(implementation.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::OwnedImplementation(value) = &mut record.payload else {
            unreachable!()
        };
        value
            .associated_types
            .iter_mut()
            .find(|binding| binding.member == f.spare)
            .unwrap()
            .value = crate::platform::kernel::AssociatedTypeBindingValue::Owned { ty: unused };
    }
    let (_, derived) = raw_compile(&f.source, f.joined).unwrap();
    assert!(!derived.contains_key(&ObjectKey::from_digest(
        crate::platform::storage::object::ObjectDomain::Type,
        unused.bytes()
    )));
    let secret = add_type(&mut f.source, TypeForm::Secret);
    let OwnerRecord::Declaration(record) = f
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(f.implementations[0].declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(value) = &mut record.payload else {
        unreachable!()
    };
    value
        .associated_types
        .iter_mut()
        .find(|binding| binding.member == f.spare)
        .unwrap()
        .value = crate::platform::kernel::AssociatedTypeBindingValue::Owned { ty: secret };
    let error = raw_compile(&f.source, f.joined).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Semantic, "{error:?}");
    let mut f = fixture(false);
    let OwnerRecord::Expression(record) = f
        .source
        .owners
        .get_mut(&OwnerKey::Expression(f.calls[1]))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall { type_arguments, .. } = &mut record.operation
    else {
        unreachable!()
    };
    type_arguments.pop();
    let error = raw_compile(&f.source, f.joined).unwrap_err();
    assert_eq!(
        error.class,
        DiagnosticClass::Semantic,
        "unused ordinary argument: {error:?}"
    );
}

#[test]
fn associated_parallel_original_child_parameter_and_wrapper_generations_are_required() {
    for target in 0..5 {
        let mut f = fixture(false);
        match target {
            0 => {
                let OwnerRecord::Expression(record) = f
                    .source
                    .owners
                    .get_mut(&OwnerKey::Expression(f.calls[0]))
                    .unwrap()
                else {
                    unreachable!()
                };
                record.contract_version = 32;
            }
            1 => {
                let OwnerRecord::Expression(record) = f
                    .source
                    .owners
                    .get_mut(&OwnerKey::Expression(f.wrapper))
                    .unwrap()
                else {
                    unreachable!()
                };
                record.contract_version = 32;
            }
            2 => {
                let OwnerRecord::Declaration(record) = f
                    .source
                    .owners
                    .get_mut(&OwnerKey::Declaration(f.joined.declaration))
                    .unwrap()
                else {
                    unreachable!()
                };
                record.header.contract_version = 32;
            }
            3 => {
                let OwnerRecord::Declaration(record) = f
                    .source
                    .owners
                    .get_mut(&OwnerKey::Declaration(f.child.declaration))
                    .unwrap()
                else {
                    unreachable!()
                };
                record.header.contract_version = 32;
            }
            _ => {
                let parameter = function(&f.source, f.child).parameters[0];
                let OwnerRecord::Parameter(record) = f
                    .source
                    .owners
                    .get_mut(&OwnerKey::Parameter(parameter))
                    .unwrap()
                else {
                    unreachable!()
                };
                record.header.contract_version = 32;
            }
        }
        let error = raw_compile(&f.source, f.joined).unwrap_err();
        assert_eq!(
            error.class,
            DiagnosticClass::Semantic,
            "target {target}: {error:?}"
        );
    }
}

#[test]
fn associated_parallel_exact_parameter_parent_and_projection_witness_scope_are_required() {
    for wrong_parent in [false, true] {
        let mut f = fixture(false);
        let (unit, _) = raw_compile(&f.source, f.joined).unwrap();
        let parameter = function(&f.source, f.child).parameters[0];
        if wrong_parent {
            let OwnerRecord::Parameter(record) = f
                .source
                .owners
                .get_mut(&OwnerKey::Parameter(parameter))
                .unwrap()
            else {
                unreachable!()
            };
            record.parent = ParameterParent::Function(f.joined.declaration);
        } else {
            let witness = function(&f.source, f.child).implementation_parameters[0].id;
            let foreign = add_type(
                &mut f.source,
                TypeForm::AssociatedWitnessProjection {
                    scope: f.joined,
                    parameter: witness,
                    contract: f.contract,
                    member: f.member,
                },
            );
            function_mut(&mut f.source, f.child).result.ty = foreign;
        }
        let error = canonical_code(
            &f.source,
            f.joined.package,
            Some(f.joined.declaration),
            &unit.tables,
            f.wrapper,
            &function(&f.source, f.joined).parameters,
        )
        .unwrap_err();
        assert_eq!(
            error.class,
            DiagnosticClass::Semantic,
            "wrong_parent={wrong_parent}: {error:?}"
        );
    }
}

#[test]
fn associated_parallel_phantom_projected_actual_keeps_wrapper_generation_provenance() {
    let mut f = fixture(false);
    let (projection, _) = caller_projection(&mut f);
    let original_arguments = f.calls.map(|call| {
        let OwnerRecord::Expression(record) = &f.source.owners[&OwnerKey::Expression(call)] else {
            unreachable!()
        };
        let ExpressionOperation::ImplementationCall { arguments, .. } = &record.operation else {
            unreachable!()
        };
        assert_eq!(arguments.len(), 1);
        arguments[0]
    });
    // This limb returns only I64: the associated operand is entirely unused.
    let scalar = named(&f.source, "scalar");
    let formal = TypeParameterId::migrate(b"compiler-parallel-phantom-formal", 0);
    f.source.owners.insert(
        OwnerKey::TypeParameter(formal),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(formal), OwnerKind::TypeParameter),
            declaration: scalar.declaration,
            name: Name::new("UnusedBlock").unwrap(),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    function_mut(&mut f.source, scalar).type_parameters = vec![formal];
    let OwnerRecord::Expression(record) = f
        .source
        .owners
        .get_mut(&OwnerKey::Expression(f.calls[1]))
        .unwrap()
    else {
        unreachable!()
    };
    record.operation = ExpressionOperation::Call {
        function: scalar,
        type_arguments: vec![projection],
        arguments: vec![],
        effect_arguments: vec![],
        requirement_arguments: vec![],
        read_scope_arguments: vec![],
    };
    // Isolate provenance from the other limb, which also becomes ordinary.
    let first = named(&f.source, "plain");
    let OwnerRecord::Expression(record) = f
        .source
        .owners
        .get_mut(&OwnerKey::Expression(f.calls[0]))
        .unwrap()
    else {
        unreachable!()
    };
    record.operation = ExpressionOperation::Call {
        function: first,
        type_arguments: vec![],
        arguments: vec![],
        effect_arguments: vec![],
        requirement_arguments: vec![],
        read_scope_arguments: vec![],
    };
    // The replaced zero-argument calls no longer own these original Local roots.
    // Remove exactly those roots; a complete canonical fixture cannot keep orphans.
    for (argument, parameter) in original_arguments
        .into_iter()
        .zip(function(&f.source, f.joined).parameters.clone())
    {
        let Some(OwnerRecord::Expression(record)) =
            f.source.owners.remove(&OwnerKey::Expression(argument))
        else {
            unreachable!()
        };
        assert_eq!(
            record.operation,
            ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(parameter),
            }
        );
    }
    let scalar_type = function(&f.source, scalar).result.ty;
    let expected = add_type(
        &mut f.source,
        TypeForm::StructuralRecord {
            fields: vec![
                StructuralTypeField {
                    name: Name::new("left").unwrap(),
                    ty: scalar_type,
                },
                StructuralTypeField {
                    name: Name::new("right").unwrap(),
                    ty: scalar_type,
                },
            ],
        },
    );
    function_mut(&mut f.source, f.joined).result.ty = expected;
    let (unit, _) = raw_compile(&f.source, f.joined).unwrap();
    assert_eq!(pair(&unit), expected);
    let OwnerRecord::Expression(record) = f
        .source
        .owners
        .get_mut(&OwnerKey::Expression(f.wrapper))
        .unwrap()
    else {
        unreachable!()
    };
    record.contract_version = 32;
    let error = canonical_code(
        &f.source,
        f.joined.package,
        Some(f.joined.declaration),
        &unit.tables,
        f.wrapper,
        &function(&f.source, f.joined).parameters,
    )
    .unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Semantic, "{error:?}");
    assert_eq!(error.code, "kernel_owned_associated_generation");
}

#[test]
fn associated_parallel_abstract_owned_member_does_not_invent_transfer_authority() {
    let mut f = fixture(false);
    let (projection, witness) = caller_projection(&mut f);
    let self_type = function(&f.source, f.joined).implementation_parameters[0].self_type;
    let OwnerRecord::Expression(record) = f
        .source
        .owners
        .get_mut(&OwnerKey::Expression(f.calls[1]))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall {
        type_arguments,
        implementations,
        ..
    } = &mut record.operation
    else {
        unreachable!()
    };
    *type_arguments = vec![self_type, projection];
    *implementations = vec![ImplementationOperand::Parameter {
        scope: f.joined,
        parameter: witness,
    }];
    let error = raw_compile(&f.source, f.joined).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Semantic, "{error:?}");
    assert_eq!(error.code, "kernel_parallel_call");
    assert!(error.message.contains("abstract Owned member"), "{error:?}");
}

#[test]
fn associated_parallel_task_kind_and_borrowed_result_are_not_erased_by_empty_rows() {
    for pure in [false, true] {
        let mut f = fixture(false);
        if pure {
            function_mut(&mut f.source, f.child).effect = FunctionEffect::Pure;
            let OwnerRecord::Declaration(record) = f
                .source
                .owners
                .get_mut(&OwnerKey::Declaration(f.child.declaration))
                .unwrap()
            else {
                unreachable!()
            };
            record.header.kind = OwnerKind::PureFunction;
        } else {
            let parameter = function(&f.source, f.child).parameters[0];
            function_mut(&mut f.source, f.child).result.relation =
                crate::platform::kernel::ResultRelation::ReadFrom { source: parameter };
            let OwnerRecord::Parameter(record) = f
                .source
                .owners
                .get_mut(&OwnerKey::Parameter(parameter))
                .unwrap()
            else {
                unreachable!()
            };
            record.use_mode = ParameterUse::Borrow;
        }
        let error = raw_compile(&f.source, f.joined).unwrap_err();
        // A Task borrowed result is already forbidden by canonical declaration
        // admission. A valid Pure declaration reaches the Parallel Task check.
        assert_eq!(
            error.code,
            if pure {
                "kernel_parallel_call"
            } else {
                "kernel_borrow_result"
            },
            "pure={pure}: {error:?}"
        );
        assert_eq!(error.class, DiagnosticClass::Semantic);
    }
}

#[test]
fn associated_parallel_cached_relower_matches_fresh_and_rejects_same_output_witness_substitution() {
    let f = fixture(true);
    let (unit, expected) = raw_compile(&f.source, f.joined).unwrap();
    let mut remaining = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK;
    let (actual, _) = reconstruct_parallel_result_types(&f.source, &unit, &mut remaining).unwrap();
    assert_eq!(actual, expected);
    let mut changed = unit.clone();
    let instruction = code_mut(&mut changed)
        .instructions
        .iter_mut()
        .find(|instruction| matches!(instruction, CompiledInstruction::Parallel { .. }))
        .unwrap();
    let CompiledInstruction::Parallel {
        left_implementations,
        right_implementations,
        ..
    } = instruction
    else {
        unreachable!()
    };
    *left_implementations = right_implementations.clone();
    let mut remaining = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK;
    let error = reconstruct_parallel_result_types(&f.source, &changed, &mut remaining).unwrap_err();
    assert_eq!(error.code, "compiler_parallel_control_meaning", "{error:?}");
    assert_eq!(error.class, DiagnosticClass::Corrupt);
}

#[test]
fn ordinary_parallel_literal_bytes_and_cached_fresh_unit_bytes_remain_identical() {
    let source = author(ORDINARY);
    let joined = named(&source, "joined");
    let witness = crate::platform::witness::rebuild_full_witness(&source).unwrap();
    let receipt = compile_unit(
        &source,
        &witness,
        OwnerKey::Declaration(joined.declaration),
        OptimizationPolicy::DeterministicBaseline,
    )
    .unwrap();
    // Byte27 remains exact through the private frozen DTO. Byte28 adds one
    // empty read-scope argument vector per limb; ordinals/targets stay fixed.
    let frozen_literal = [0, 0, 2, 38, 0, 0, 0, 0, 0, 0, 1, 0, 0, 27];
    let frozen = super::super::wire33::Code33::try_from(code(&receipt.unit)).unwrap();
    assert_eq!(
        bincode::encode_to_vec(&frozen, bincode::config::standard()).unwrap(),
        frozen_literal
    );
    assert_eq!(CompiledCode::from(frozen), *code(&receipt.unit));
    // parameter/local counts; two instructions; Parallel38; six empty
    // type/witness/scope inventories; target0/args0,target1/args0,result0; Return27.
    let literal = [0, 0, 2, 38, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 27];
    assert_eq!(
        bincode::encode_to_vec(
            code(&receipt.unit),
            bincode::config::standard()
                .with_little_endian()
                .with_variable_int_encoding()
        )
        .unwrap(),
        literal
    );
    let (fresh, expected) = raw_compile(&source, joined).unwrap();
    assert_eq!(fresh.encode().unwrap().1, receipt.bytes);
    let mut remaining = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK;
    let (recreated, _) =
        reconstruct_parallel_result_types(&source, &receipt.unit, &mut remaining).unwrap();
    assert_eq!(recreated, expected);
    assert_eq!(receipt.unit.encode().unwrap().1, receipt.bytes);
    // Two real same-signature Task targets: a valid table index still does not
    // authorize replacing the canonical left call with the right call.
    let mut changed = receipt.unit.clone();
    let instruction = code_mut(&mut changed)
        .instructions
        .iter_mut()
        .find(|instruction| matches!(instruction, CompiledInstruction::Parallel { .. }))
        .unwrap();
    let CompiledInstruction::Parallel { left, right, .. } = instruction else {
        unreachable!()
    };
    assert_ne!(*left, *right);
    *left = *right;
    let mut remaining = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK;
    let error = reconstruct_parallel_result_types(&source, &changed, &mut remaining).unwrap_err();
    assert_eq!(error.code, "compiler_parallel_control_meaning", "{error:?}");
    assert_eq!(error.class, DiagnosticClass::Corrupt);
}

#[test]
fn ordinary_parallel_reconstruction_seeds_nested_nominal_fields_before_strict_table_closure() {
    let source = author(ORDINARY_NOMINAL);
    let hold = named(&source, "hold");
    let witness = crate::platform::witness::rebuild_full_witness(&source).unwrap();
    let receipt = compile_unit(
        &source,
        &witness,
        OwnerKey::Declaration(hold.declaration),
        OptimizationPolicy::DeterministicBaseline,
    )
    .unwrap();
    let (fresh, expected) = raw_compile(&source, hold).unwrap();
    assert_eq!(fresh.encode().unwrap().1, receipt.bytes);
    assert!(
        !code(&receipt.unit)
            .instructions
            .iter()
            .any(|instruction| matches!(instruction, CompiledInstruction::Parallel { .. }))
    );
    let missing = receipt
        .unit
        .tables
        .types
        .iter()
        .filter(|ty| !source.types.contains_key(ty))
        .copied()
        .collect::<Vec<_>>();
    assert!(
        !missing.is_empty(),
        "fixture must contain derived field roots absent from canonical source"
    );
    for ty in &missing {
        assert!(expected.contains_key(&ObjectKey::from_digest(
            crate::platform::storage::object::ObjectDomain::Type,
            ty.bytes(),
        )));
    }
    let mut remaining = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK;
    let (recreated, _) =
        reconstruct_parallel_result_types(&source, &receipt.unit, &mut remaining).unwrap();
    assert_eq!(recreated, expected);
    assert_eq!(receipt.unit.encode().unwrap().1, receipt.bytes);
    // An unrelated nonexistent cached root is still rejected after seeding.
    let mut corrupt = receipt.unit.clone();
    corrupt
        .tables
        .types
        .push(TypeObjectDigest::from_bytes([0xe7; 32]));
    let mut remaining = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK;
    let error = reconstruct_parallel_result_types(&source, &corrupt, &mut remaining).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Corrupt, "{error:?}");
    assert_eq!(error.code, "compiler_owned_record_type");
}

#[test]
fn ordinary_parallel_reconstruction_walks_unindexed_nested_nominal_children() {
    // This declaration has no executable root or explicit Inner<TOuter> table
    // entry. Its child nominal's distinct formal must be substituted to TOuter.
    let source = author(
        r#"declarations.begin
(units (module create nested-nominal-cache
  (owned-record create Inner (visibility public)
    (type-parameter create T (constraint transferable))
    (field create items (type (owned-sequence T))))
  (owned-record create Outer (visibility public)
    (type-parameter create T (constraint transferable))
    (field create children (type (owned-sequence (Inner T)))))))
declarations.end
"#,
    );
    let outer = named(&source, "Outer");
    let inner = named(&source, "Inner");
    let OwnerRecord::Declaration(record) =
        &source.owners[&OwnerKey::Declaration(outer.declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedRecord { fields, .. } = &record.payload else {
        unreachable!()
    };
    let OwnerRecord::Field(field) = &source.owners[&OwnerKey::Field(fields[0])] else {
        unreachable!()
    };
    let outer_sequence = field.ty;
    let TypeForm::OwnedSequence {
        item: child_nominal,
    } = source.types[&outer_sequence].form
    else {
        unreachable!()
    };
    let TypeForm::Applied {
        declaration,
        arguments,
    } = &source.types[&child_nominal].form
    else {
        unreachable!()
    };
    assert_eq!(*declaration, inner);
    let item = arguments[0];
    let inner_sequence_object = TypeObject::new(TypeForm::OwnedSequence { item }).unwrap();
    let (inner_sequence, inner_sequence_bytes) =
        encode_type_object(&inner_sequence_object).unwrap();
    assert!(!source.types.contains_key(&inner_sequence));
    let witness = crate::platform::witness::rebuild_full_witness(&source).unwrap();
    let receipt = compile_unit(
        &source,
        &witness,
        OwnerKey::Declaration(outer.declaration),
        OptimizationPolicy::DeterministicBaseline,
    )
    .unwrap();
    assert!(matches!(
        receipt.unit.payload,
        CompilationPayload::OwnedRecord { .. }
    ));
    // Strict LIFO visits the absent derived entry before the available outer
    // sequence. The nominal needed to derive it is only a syntactic child.
    assert_eq!(receipt.unit.tables.types, [outer_sequence, inner_sequence]);
    assert!(!receipt.unit.tables.types.contains(&child_nominal));
    assert!(source.types.contains_key(&child_nominal));
    let (fresh, expected) = raw_compile(&source, outer).unwrap();
    assert_eq!(fresh.encode().unwrap().1, receipt.bytes);
    let key = ObjectKey::from_digest(
        crate::platform::storage::object::ObjectDomain::Type,
        inner_sequence.bytes(),
    );
    assert_eq!(expected[&key], inner_sequence_bytes);
    let maximum = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK;
    let mut remaining = maximum;
    let (recreated, _) =
        reconstruct_parallel_result_types(&source, &receipt.unit, &mut remaining).unwrap();
    assert_eq!(recreated, expected);
    assert_eq!(receipt.unit.encode().unwrap().1, receipt.bytes);
    let measured = maximum - remaining;
    assert!(measured > receipt.unit.tables.types.len());
    let mut exact = measured;
    assert_eq!(
        reconstruct_parallel_result_types(&source, &receipt.unit, &mut exact)
            .unwrap()
            .0,
        expected
    );
    assert_eq!(exact, 0);
    let mut below = measured - 1;
    let error = reconstruct_parallel_result_types(&source, &receipt.unit, &mut below).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource, "{error:?}");
    let healthy = Controlled {
        source: &source,
        control: ExecutionControl::uncancelled(),
        checks: Cell::new(0),
    };
    let mut remaining = maximum;
    assert_eq!(
        reconstruct_parallel_result_types(&healthy, &receipt.unit, &mut remaining)
            .unwrap()
            .0,
        expected
    );
    let checks = healthy.checks.get();
    assert!(checks > 2);
    for cutoff in [0, 1, checks / 2, checks - 1] {
        let controlled = Controlled {
            source: &source,
            control: ExecutionControl::cancel_after_checks(cutoff),
            checks: Cell::new(0),
        };
        let mut remaining = maximum;
        let error = reconstruct_parallel_result_types(&controlled, &receipt.unit, &mut remaining)
            .unwrap_err();
        assert_eq!(
            error.class,
            DiagnosticClass::Cancelled,
            "cutoff {cutoff}: {error:?}"
        );
    }
    let mut remaining = maximum;
    assert_eq!(
        reconstruct_parallel_result_types(&source, &receipt.unit, &mut remaining)
            .unwrap()
            .0,
        expected
    );
    let mut corrupt = receipt.unit.clone();
    corrupt
        .tables
        .types
        .push(TypeObjectDigest::from_bytes([0xe8; 32]));
    let mut remaining = maximum;
    let error = reconstruct_parallel_result_types(&source, &corrupt, &mut remaining).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Corrupt, "{error:?}");
    assert_eq!(error.code, "compiler_owned_record_type");
}

struct Controlled<'a> {
    source: &'a KernelSnapshot,
    control: ExecutionControl,
    checks: Cell<u64>,
}
impl CanonicalBaseRead for Controlled<'_> {
    fn validation_checkpoint(&self) -> Result<(), Diagnostic> {
        self.checks.set(self.checks.get() + 1);
        self.control
            .check()
            .map_err(|error| Diagnostic::new(DiagnosticClass::Cancelled, error.code, error.message))
    }
    fn semantic_root(&self) -> &SemanticRoot {
        &self.source.root
    }
    fn repository_id(&self) -> RepositoryId {
        self.source.repository_id()
    }
    fn package_id(&self) -> PackageId {
        CanonicalBaseRead::package_id(self.source)
    }
    fn exact_revision(&self) -> Option<RevisionId> {
        CanonicalBaseRead::exact_revision(self.source)
    }
    fn owner_count(&self) -> u64 {
        self.source.owner_count()
    }
    fn dependency_count(&self) -> u64 {
        self.source.dependency_count()
    }
    fn retirement_count(&self) -> u64 {
        self.source.retirement_count()
    }
    fn read_owner(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic> {
        self.source.read_owner(owner)
    }
    fn read_type_object(
        &self,
        ty: TypeObjectDigest,
    ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
        self.source.read_type_object(ty)
    }
    fn read_dependency(
        &self,
        package: PackageId,
    ) -> Result<CanonicalRead<Option<crate::platform::kernel::DependencyRecord>>, Diagnostic> {
        self.source.read_dependency(package)
    }
    fn read_package_interface_owner(
        &self,
        dependency: &crate::platform::kernel::DependencyRecord,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
        self.source.read_package_interface_owner(dependency, owner)
    }
    fn read_retirement(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<RetirementRecord>>, Diagnostic> {
        self.source.read_retirement(owner)
    }
}

#[test]
fn associated_parallel_reconstruction_exact_work_bound_cancellation_and_retry_are_real_walks() {
    let f = fixture(false);
    let (unit, expected) = raw_compile(&f.source, f.joined).unwrap();
    let maximum = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK;
    let mut remaining = maximum;
    reconstruct_parallel_result_types(&f.source, &unit, &mut remaining).unwrap();
    let measured = maximum - remaining;
    assert!(measured > unit.tables.types.len());
    let mut exact = measured;
    assert_eq!(
        reconstruct_parallel_result_types(&f.source, &unit, &mut exact)
            .unwrap()
            .0,
        expected
    );
    assert_eq!(exact, 0);
    let mut below = measured - 1;
    let error = reconstruct_parallel_result_types(&f.source, &unit, &mut below).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource, "{error:?}");
    let healthy = Controlled {
        source: &f.source,
        control: ExecutionControl::uncancelled(),
        checks: Cell::new(0),
    };
    let mut remaining = maximum;
    assert_eq!(
        reconstruct_parallel_result_types(&healthy, &unit, &mut remaining)
            .unwrap()
            .0,
        expected
    );
    let checks = healthy.checks.get();
    assert!(checks > 2);
    for cutoff in [0, 1, checks / 2, checks - 1] {
        let source = Controlled {
            source: &f.source,
            control: ExecutionControl::cancel_after_checks(cutoff),
            checks: Cell::new(0),
        };
        let mut remaining = maximum;
        let error = reconstruct_parallel_result_types(&source, &unit, &mut remaining).unwrap_err();
        assert_eq!(
            error.class,
            DiagnosticClass::Cancelled,
            "cutoff {cutoff}: {error:?}"
        );
    }
    let mut remaining = maximum;
    assert_eq!(
        reconstruct_parallel_result_types(&f.source, &unit, &mut remaining)
            .unwrap()
            .0,
        expected
    );
}

#[test]
fn associated_parallel_both_children_share_exact_producer_work_and_metadata_bounds() {
    fn builder(f: &Fixture) -> UnitBuilder<'_, KernelSnapshot> {
        UnitBuilder {
            canonical: &f.source,
            package: f.joined.package,
            scope: Some(f.joined.declaration),
            tables: TablesBuilder::default(),
            derived_type_objects: BTreeMap::new(),
            derived_type_admission: DerivedTypeAdmission::default(),
            work: CompilationWork::default(),
        }
    }
    fn children(
        builder: &mut UnitBuilder<'_, KernelSnapshot>,
        f: &Fixture,
    ) -> Result<(), Diagnostic> {
        for expression in f.calls {
            builder.parallel_call_contract(
                expression,
                crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
                // These two literal limbs occur outside any lexical read body.
                &BTreeSet::new(),
            )?;
        }
        Ok(())
    }
    let f = fixture(false);
    let mut healthy = builder(&f);
    children(&mut healthy, &f).unwrap();
    let steps = healthy.derived_type_admission.parallel_steps.get();
    let bytes = healthy.derived_type_admission.parallel.work().1;
    assert!(steps > 2);
    assert!(bytes > 0);
    let maximum = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK;
    let mut exact = builder(&f);
    exact
        .derived_type_admission
        .parallel_steps
        .set(maximum - steps);
    children(&mut exact, &f).unwrap();
    assert_eq!(exact.derived_type_admission.parallel_steps.get(), maximum);
    let mut below = builder(&f);
    below
        .derived_type_admission
        .parallel_steps
        .set(maximum - steps + 1);
    let error = children(&mut below, &f).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource, "{error:?}");
    assert_eq!(error.code, "compiler_parallel_proof_work");
    for (limit, success) in [(bytes, true), (bytes - 1, false)] {
        let mut bounded = builder(&f);
        bounded.derived_type_admission.parallel =
            crate::platform::kernel::owned_associated::OwnedAssociatedAdmission::with_limits(
                crate::platform::kernel::owned_associated::AssociatedAdmissionLimits {
                    maximum_metadata_bytes: limit,
                    ..Default::default()
                },
            );
        match children(&mut bounded, &f) {
            Ok(()) => assert!(success),
            Err(error) => {
                assert!(!success, "{error:?}");
                assert_eq!(error.class, DiagnosticClass::Resource, "{error:?}");
            }
        }
    }
}
