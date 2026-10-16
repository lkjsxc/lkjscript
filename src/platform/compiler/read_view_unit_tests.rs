//! Structural codec/protocol controls. These private bytecode fixtures carry no artifact seal.
use super::*;
use crate::platform::kernel::{
    CallableResult, FunctionEffect, ReadScopeArgument, ReadScopeBinder, ReadScopeParameter,
    ReadScopeReference, ReadScopeSource, ResultRelation,
};

fn reference(serial: u8) -> DeclarationReference {
    DeclarationReference {
        package: "pkg_10000000000000000000000000000001".parse().unwrap(),
        declaration: format!("decl_{serial:032x}").parse().unwrap(),
    }
}

fn source() -> CompilationSource {
    let digest = SemanticDigest::from_bytes([1; 32]);
    CompilationSource {
        package: reference(1).package,
        owner: OwnerKey::Declaration(reference(1).declaration),
        kind: OwnerKind::PureFunction,
        semantic_interface: digest,
        implementation: digest,
        type_digest: digest,
        effect: digest,
        capability: digest,
        test: None,
        validation_dependencies: digest,
    }
}

fn scope_parameter() -> ReadScopeParameter {
    ReadScopeParameter {
        id: "read_scope_00000000000000000000000000000001"
            .parse()
            .unwrap(),
        name: Name::new("source").unwrap(),
    }
}

fn formal_scope() -> ReadScopeArgument {
    ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::Declaration(reference(1)),
        parameter: scope_parameter().id,
    })
}

fn lexical(serial: u8) -> ReadScopeArgument {
    ReadScopeArgument::Lexical {
        callable: reference(1),
        binding: format!("bind_{serial:032x}").parse().unwrap(),
    }
}

fn tables() -> CompilationTables {
    CompilationTables {
        declarations: vec![reference(1)],
        fields: vec![],
        cases: vec![],
        requirements: vec![],
        operations: vec![],
        ports: vec![],
        types: vec![TypeObjectDigest::from_bytes([2; 32])],
        structural_names: vec![],
        read_view_fields: vec![],
        texts: vec![],
    }
}

fn signature() -> CompiledSignature {
    let parameter = |serial| CompiledParameter {
        parameter: format!("param_{serial:032x}").parse().unwrap(),
        name: Name::new(format!("p{serial}")).unwrap(),
        ty: 0,
        use_mode: ParameterUse::Borrow,
        resource_requirement: None,
    };
    let first = parameter(1);
    let second = parameter(2);
    CompiledSignature {
        implementation_parameters: vec![],
        requirement_parameters: vec![],
        effect_parameters: vec![],
        effect: FunctionEffect::Pure,
        type_parameters: vec![],
        type_parameter_constraints: vec![],
        result: CallableResult {
            ty: 0,
            relation: ResultRelation::ViewFrom {
                source: first.parameter,
                scope: formal_scope(),
            },
        },
        read_scope_parameters: vec![scope_parameter()],
        read_scope_sources: vec![
            ReadScopeSource {
                source: first.parameter,
                scope: formal_scope(),
            },
            ReadScopeSource {
                source: second.parameter,
                scope: formal_scope(),
            },
        ],
        parameters: vec![first, second],
        task_requirements: vec![],
    }
}

#[test]
fn foreign_unit_schema_is_rejected_before_decoding_payload() {
    let source = source();
    let mut signature = signature();
    signature.result.relation = ResultRelation::Value;
    let unit = CompilationUnit {
        contract_version: COMPILER_UNIT_CONTRACT_VERSION,
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        bytecode_contract_version: BYTECODE_CONTRACT_VERSION,
        key: CompilationUnitKey::derive(&source, OptimizationPolicy::DeterministicBaseline)
            .unwrap(),
        source,
        optimization: OptimizationPolicy::DeterministicBaseline,
        tables: tables(),
        payload: CompilationPayload::Function {
            signature,
            code: CompiledCode {
                parameter_count: 2,
                local_count: 2,
                instructions: vec![CompiledInstruction::Unit, CompiledInstruction::Return],
            },
        },
    };
    let (key, bytes) = unit.encode().unwrap();
    assert_eq!(CompilationUnit::decode(&bytes, key).unwrap(), unit);
    let mut foreign = bytes;
    foreign[8] ^= 1;
    let key = ObjectKey::for_bytes(ObjectDomain::CompilerUnit, &foreign);
    assert_eq!(
        CompilationUnit::decode(&foreign, key).unwrap_err().code,
        "packed_schema"
    );
}

#[test]
fn view_result_keeps_exact_source_and_full_unused_scope_inventory() {
    let signature = signature();
    signature.validate(&tables(), &source()).unwrap();
    let mut inherited = signature.clone();
    inherited.read_scope_sources.clear();
    inherited.validate(&tables(), &source()).unwrap();
    if let ResultRelation::ViewFrom { scope, .. } = &mut inherited.result.relation {
        *scope = ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(reference(2)),
            parameter: scope_parameter().id,
        });
    }
    assert_eq!(
        inherited.validate(&tables(), &source()).unwrap_err().code,
        "kernel_read_view_scope"
    );
    let mut absent = signature.clone();
    absent.result.relation = ResultRelation::ViewFrom {
        source: "param_00000000000000000000000000000003".parse().unwrap(),
        scope: formal_scope(),
    };
    assert_eq!(
        absent.validate(&tables(), &source()).unwrap_err().code,
        "compiler_unit_result_borrow"
    );
    let mut unused = signature.clone();
    unused.read_scope_sources[1].scope = ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::Declaration(reference(2)),
        parameter: scope_parameter().id,
    });
    assert_eq!(
        unused.validate(&tables(), &source()).unwrap_err().code,
        "kernel_read_view_scope"
    );
    let mut duplicate = signature.clone();
    duplicate
        .read_scope_sources
        .push(duplicate.read_scope_sources[1].clone());
    assert_eq!(
        duplicate.validate(&tables(), &source()).unwrap_err().code,
        "kernel_read_view_source"
    );
    let mut wrong_mode = signature;
    wrong_mode.parameters[0].use_mode = ParameterUse::Unrestricted;
    assert_eq!(
        wrong_mode.validate(&tables(), &source()).unwrap_err().code,
        "compiler_unit_result_borrow"
    );
}

fn code(instructions: Vec<CompiledInstruction>) -> CompiledCode {
    CompiledCode {
        parameter_count: 1,
        local_count: 3,
        instructions,
    }
}

#[test]
fn prospective_scope_outlives_descriptor_drop_and_closes_exactly_once() {
    use CompiledInstruction::*;
    let valid = code(vec![
        BeginReadViewScope {
            read_scope: lexical(1),
        },
        BeginReadView {
            view_type: 0,
            source_local: 0,
            fields: vec![],
        },
        FinishReadView {
            view_type: 0,
            fields: vec![],
        },
        Drop,
        Unit,
        EndReadViewScope {
            read_scope: lexical(1),
        },
        Return,
    ]);
    valid.validate(&tables()).unwrap();
    for fault in [0, 1, 2] {
        let mut changed = valid.clone();
        match fault {
            0 => {
                changed.instructions[5] = EndReadViewScope {
                    read_scope: lexical(2),
                }
            }
            1 => changed.instructions[5] = Jump(6),
            _ => {
                changed.instructions[0] = EndReadViewScope {
                    read_scope: lexical(1),
                }
            }
        }
        assert_eq!(
            changed.validate(&tables()).unwrap_err().code,
            "compiler_unit_view_protocol"
        );
    }
    let retired_during_init = code(vec![
        BeginReadViewScope {
            read_scope: lexical(1),
        },
        BeginReadView {
            view_type: 0,
            source_local: 0,
            fields: vec![],
        },
        EndReadViewScope {
            read_scope: lexical(1),
        },
        FinishReadView {
            view_type: 0,
            fields: vec![],
        },
        Return,
    ]);
    assert_eq!(
        retired_during_init.validate(&tables()).unwrap_err().code,
        "compiler_unit_view_protocol"
    );
    // A temporary nested source lifetime opened after the outer construction may
    // finish first; ending any lifetime while a constructor exists would be too broad.
    let nested = code(vec![
        BeginReadViewScope {
            read_scope: lexical(1),
        },
        BeginReadView {
            view_type: 0,
            source_local: 0,
            fields: vec![0],
        },
        BeginReadViewScope {
            read_scope: lexical(2),
        },
        BeginReadView {
            view_type: 0,
            source_local: 0,
            fields: vec![],
        },
        FinishReadView {
            view_type: 0,
            fields: vec![],
        },
        Drop,
        Unit,
        EndReadViewScope {
            read_scope: lexical(2),
        },
        FinishReadView {
            view_type: 0,
            fields: vec![0],
        },
        EndReadViewScope {
            read_scope: lexical(1),
        },
        Return,
    ]);
    let depths = verify_stack(&nested).unwrap();
    verify_read_view_construction(&nested, &depths).unwrap();
}

#[test]
fn view_formal_layout_accepts_exact_owned_and_transferable_constraints() {
    use crate::platform::kernel::TypeParameterConstraints as C;
    let mut source = source();
    source.kind = OwnerKind::ReadView;
    let declaration = crate::platform::kernel::ReadViewDeclaration {
        type_parameters: vec![
            "typeparam_00000000000000000000000000000001"
                .parse()
                .unwrap(),
        ],
        scope_parameter: scope_parameter(),
        fields: vec![],
    };
    for constraints in [C::Owned, C::Transferable, C::OwnedTransferableShareable] {
        CompilationPayload::ReadView {
            declaration: declaration.clone(),
            type_parameter_constraints: vec![constraints],
        }
        .validate(&source, &tables())
        .unwrap();
    }
    assert!(
        CompilationPayload::ReadView {
            declaration,
            type_parameter_constraints: vec![C::None]
        }
        .validate(&source, &tables())
        .is_err()
    );
}

#[test]
fn view_construction_protects_selected_source_before_every_initializer() {
    use CompiledInstruction::*;
    let valid = code(vec![
        BeginReadView {
            view_type: 0,
            source_local: 0,
            fields: vec![],
        },
        LoadLocal {
            local: 0,
            use_mode: ParameterUse::Borrow,
        },
        Drop,
        FinishReadView {
            view_type: 0,
            fields: vec![],
        },
        Return,
    ]);
    valid.validate(&tables()).unwrap();
    let mut consumes = valid.clone();
    consumes.instructions[1] = LoadLocal {
        local: 0,
        use_mode: ParameterUse::Consume,
    };
    assert_eq!(
        consumes.validate(&tables()).unwrap_err().code,
        "compiler_unit_view_protocol"
    );
    let mut skips = valid;
    skips.instructions[0] = Jump(1);
    assert_eq!(
        skips.validate(&tables()).unwrap_err().code,
        "compiler_unit_view_protocol"
    );
    // A released selected read token cannot remain a constructor source. The old
    // lexical loan protocol alone accepts this shape, so construction checks it too.
    let released = code(vec![
        BorrowOwnedField {
            product_type: 0,
            source_local: 0,
            field: 0,
            binding_local: 1,
            binding_type: 0,
        },
        BeginReadView {
            view_type: 0,
            source_local: 1,
            fields: vec![],
        },
        Unit,
        EndOwnedBorrow { binding_local: 1 },
        Drop,
        FinishReadView {
            view_type: 0,
            fields: vec![],
        },
        Return,
    ]);
    let depths = verify_stack(&released).unwrap();
    verify_owned_borrow_scopes(&released, &depths).unwrap();
    assert_eq!(
        verify_read_view_construction(&released, &depths)
            .unwrap_err()
            .code,
        "compiler_unit_view_protocol"
    );
    let released_handle = code(vec![
        BeginReadView {
            view_type: 0,
            source_local: 0,
            fields: vec![],
        },
        FinishReadView {
            view_type: 0,
            fields: vec![],
        },
        StoreLocal(1),
        BeginReadView {
            view_type: 0,
            source_local: 1,
            fields: vec![],
        },
        DropReadView {
            view_type: 0,
            source_local: 1,
        },
        Drop,
        FinishReadView {
            view_type: 0,
            fields: vec![],
        },
        Return,
    ]);
    released_handle.validate(&tables()).unwrap();
}

#[test]
fn view_loan_end_cannot_release_an_owned_loan_or_skip_borrowed_call_handoff() {
    use CompiledInstruction::*;
    let valid = code(vec![
        BorrowReadView {
            view_type: 0,
            source_local: 0,
            binding_local: 1,
            binding_type: 0,
        },
        Unit,
        EndReadViewBorrow { binding_local: 1 },
        Return,
    ]);
    valid.validate(&tables()).unwrap();
    let mut wrong = valid;
    wrong.instructions[2] = EndOwnedBorrow { binding_local: 1 };
    assert_eq!(
        wrong.validate(&tables()).unwrap_err().code,
        "compiler_unit_borrow_end"
    );
    let call = code(vec![
        LoadLocal {
            local: 0,
            use_mode: ParameterUse::Borrow,
        },
        BeginBorrowCall {
            source_local: 0,
            source_position: 0,
        },
        Call {
            requirement_arguments: vec![],
            effect_arguments: vec![],
            read_scope_arguments: vec![],
            function: 0,
            type_arguments: vec![],
            arguments: 1,
        },
        AdoptViewResult {
            source_local: 0,
            binding_local: 1,
            binding_type: 0,
        },
        Unit,
        Return,
    ]);
    call.validate(&tables()).unwrap();
    let mut orphan = call;
    orphan.instructions[1] = Jump(2);
    assert_eq!(
        orphan.validate(&tables()).unwrap_err().code,
        "compiler_unit_borrow_call_protocol"
    );
    for action in [
        DropReadView {
            view_type: 0,
            source_local: 1,
        },
        MoveReadView {
            view_type: 0,
            source_local: 1,
        },
    ] {
        let handles = code(vec![
            BeginReadView {
                view_type: 0,
                source_local: 0,
                fields: vec![],
            },
            FinishReadView {
                view_type: 0,
                fields: vec![],
            },
            StoreLocal(1),
            BorrowReadView {
                view_type: 0,
                source_local: 1,
                binding_local: 2,
                binding_type: 0,
            },
            action.clone(),
            Drop,
            Unit,
            EndReadViewBorrow { binding_local: 2 },
            Return,
        ]);
        handles.validate(&tables()).unwrap();
        let mut destroys_loan = handles.clone();
        destroys_loan.instructions[4] = match action {
            DropReadView { .. } => DropReadView {
                view_type: 0,
                source_local: 2,
            },
            _ => MoveReadView {
                view_type: 0,
                source_local: 2,
            },
        };
        assert_eq!(
            destroys_loan.validate(&tables()).unwrap_err().code,
            "compiler_unit_borrow_custody"
        );
        let mut owned_consume = handles;
        owned_consume.instructions[4] = LoadLocal {
            local: 1,
            use_mode: ParameterUse::Consume,
        };
        assert_eq!(
            owned_consume.validate(&tables()).unwrap_err().code,
            "compiler_unit_borrow_custody"
        );
    }
    let read_from_view = code(vec![
        BeginReadView {
            view_type: 0,
            source_local: 0,
            fields: vec![],
        },
        FinishReadView {
            view_type: 0,
            fields: vec![],
        },
        StoreLocal(1),
        LoadLocal {
            local: 1,
            use_mode: ParameterUse::Borrow,
        },
        BeginBorrowCall {
            source_local: 1,
            source_position: 0,
        },
        Call {
            requirement_arguments: vec![],
            effect_arguments: vec![],
            read_scope_arguments: vec![],
            function: 0,
            type_arguments: vec![],
            arguments: 1,
        },
        AdoptBorrowResult {
            source_local: 1,
            binding_local: 2,
            binding_type: 0,
        },
        DropReadView {
            view_type: 0,
            source_local: 1,
        },
        Drop,
        Unit,
        EndOwnedBorrow { binding_local: 2 },
        Return,
    ]);
    read_from_view.validate(&tables()).unwrap();
}
