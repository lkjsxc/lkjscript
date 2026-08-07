use super::*;

#[test]
fn forced_source_string_entry_fails_closed_in_both_tiers_until_producer_exists() {
    let program = source_string_entry_program();
    let baseline = execute_forced(
        &program,
        &ExecutionPolicy::unrestricted(),
        JitConfig::default(),
    )
    .expect_err("baseline source string entry must fail before native entry");
    assert_eq!(baseline.code(), FailureCode::UnsupportedType);
    assert!(baseline
        .to_string()
        .contains("no compiler-produced native structural owner"));

    let optimizing = crate::execute_optimizing(
        &program,
        &ExecutionPolicy::unrestricted(),
        JitConfig::default(),
    )
    .expect_err("optimizing source string entry must fail before native entry");
    assert_eq!(optimizing.code(), FailureCode::UnsupportedType);
    assert!(optimizing
        .to_string()
        .contains("no compiler-produced native structural owner"));
}

fn source_string_entry_program() -> lkjscript_ir::VerifiedProgram {
    verify(Program {
        prepared_identity: lkjscript_ir::PreparedProgramIdentity::UNBOUND,
        memory: lkjscript_ir::StructuralMemoryMetadata::default(),
        sources: vec![SourceMetadata {
            id: 0,
            path: "structural-cutover.lkjscript".into(),
        }],
        products: Vec::new(),
        region_products: Vec::new(),
        enums: Vec::new(),
        traits: core_traits(),
        implementations: Vec::new(),
        functions: vec![Function {
            id: FunctionId::new(0),
            name: "main".into(),
            signature: Signature::monomorphic(Vec::new(), SsaType::Str),
            places: Vec::new(),
            failure_cleanups: Vec::new(),
            effects: EffectSet::PURE,
            entry: BlockId::new(0),
            blocks: vec![Block {
                id: BlockId::new(0),
                parameters: Vec::new(),
                instructions: vec![Instruction {
                    id: ValueId::new(0),
                    ty: SsaType::Str,
                    kind: InstructionKind::Constant(Constant::Str("unsupported".into())),
                    metadata: InstructionMetadata {
                        origin: Origin::SYNTHETIC,
                        effects: EffectSet::PURE,
                        failure: FailureBehavior::None,
                        failure_cleanup: None,
                        frame_state: None,
                    },
                }],
                terminator: Terminator::Return(ValueId::new(0)),
                metadata: BlockMetadata {
                    loop_header: false,
                    origin: Origin::SYNTHETIC,
                    failure_cleanup: None,
                    frame_state: None,
                },
            }],
            origin: Origin::SYNTHETIC,
        }],
        main: FunctionId::new(0),
    })
    .expect("verify source string entry SSA")
}
