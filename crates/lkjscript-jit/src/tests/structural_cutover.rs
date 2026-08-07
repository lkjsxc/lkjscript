use super::*;

#[test]
fn source_string_entry_declines_before_native_entry_until_producer_exists() {
    let program = source_string_entry_program();
    let BaselineAttempt::Declined(decline) = attempt_baseline(
        &program,
        &ExecutionPolicy::unrestricted(),
        JitConfig::default(),
    ) else {
        panic!("source string entry must decline before native entry")
    };
    let BaselineDeclineReason::Lowering(error) = decline.reason else {
        panic!("source string entry must be a lowering decline")
    };
    assert_eq!(error.code(), FailureCode::UnsupportedType);
    assert!(error
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
