use super::fixtures::*;
use crate::*;

#[test]
fn verifier_accepts_dense_exact_program_and_evaluator_returns_exact_value() {
    let verified = verify(one_block_program()).expect("verify exact program");
    assert_eq!(
        evaluate(&verified, &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(42))
    );
}

#[test]
fn verifier_accepts_canonical_marker_witness_and_rejects_malformed_trait_facts() {
    let canonical = verify(bounded_call_program()).expect("verify canonical marker witness");
    assert_eq!(
        evaluate(&canonical, &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(9))
    );

    let mut unknown_trait = bounded_call_program();
    unknown_trait.functions[0].signature.bounds[0].trait_id = TraitId::new(99);
    assert!(verify(unknown_trait).is_err());

    let mut bounded_function_reference = bounded_call_program();
    let bounded_signature = bounded_function_reference.functions[0].signature.clone();
    bounded_function_reference.functions[1].signature = Signature::monomorphic(
        Vec::new(),
        SsaType::Function(Box::new(bounded_signature.clone())),
    );
    bounded_function_reference.functions[1].blocks[0].instructions = vec![Instruction {
        id: ValueId::new(0),
        ty: SsaType::Function(Box::new(bounded_signature)),
        kind: InstructionKind::FunctionRef(FunctionId::new(0)),
        metadata: metadata(EffectSet::PURE),
    }];
    bounded_function_reference.functions[1].blocks[0].terminator =
        Terminator::Return(ValueId::new(0));
    assert!(verify(bounded_function_reference).is_err());

    let mut deeply_nested_type = bounded_call_program();
    let mut nested = SsaType::I64;
    for _ in 0..70 {
        nested = SsaType::List(Box::new(nested));
    }
    deeply_nested_type.products.push(ProductMetadata {
        id: crate::ProductId::new(0),
        name: "Deep".into(),
        fields: vec![crate::ProductField {
            name: "value".into(),
            ty: nested,
        }],
    });
    let deep_error = verify(deeply_nested_type).expect_err("deep type must be bounded");
    assert!(deep_error.to_string().contains("type nesting exceeds"));

    let mut unavailable_clone_bound = bounded_call_program();
    unavailable_clone_bound.functions[0].signature.bounds[0].trait_id = TraitId::new(1);
    assert!(verify(unavailable_clone_bound).is_err());

    let mut core_implementation = bounded_call_program();
    core_implementation.products.push(ProductMetadata {
        id: crate::ProductId::new(0),
        name: "Point".into(),
        fields: Vec::new(),
    });
    core_implementation.implementations.push(ImplMetadata {
        id: ImplId::new(0),
        trait_id: TraitId::new(1),
        product: crate::ProductId::new(0),
        source: 0,
    });
    assert!(verify(core_implementation).is_err());

    let mutate_instantiation =
        |program: &mut Program, mutate: &mut dyn FnMut(&mut GenericInstantiation)| {
            let InstructionKind::Call {
                instantiation: Some(instantiation),
                ..
            } = &mut program.functions[1].blocks[0].instructions[1].kind
            else {
                panic!("bounded call fixture lost instantiation");
            };
            mutate(instantiation);
        };

    let mut unknown_impl = bounded_call_program();
    mutate_instantiation(&mut unknown_impl, &mut |instantiation| {
        instantiation.witnesses[0].kind = TraitWitnessKind::Explicit(ImplId::new(99));
    });
    assert!(verify(unknown_impl).is_err());

    let mut wrong_witness_type = bounded_call_program();
    mutate_instantiation(&mut wrong_witness_type, &mut |instantiation| {
        instantiation.witnesses[0].ty = SsaType::Bool;
    });
    assert!(verify(wrong_witness_type).is_err());

    let mut duplicate_witness = bounded_call_program();
    mutate_instantiation(&mut duplicate_witness, &mut |instantiation| {
        instantiation
            .witnesses
            .push(instantiation.witnesses[0].clone());
    });
    assert!(verify(duplicate_witness).is_err());

    let mut wrong_substitution = bounded_call_program();
    mutate_instantiation(&mut wrong_substitution, &mut |instantiation| {
        instantiation.substitutions[0].parameter = "U".into();
    });
    assert!(verify(wrong_substitution).is_err());

    let mut missing_instantiation = bounded_call_program();
    if let InstructionKind::Call { instantiation, .. } =
        &mut missing_instantiation.functions[1].blocks[0].instructions[1].kind
    {
        *instantiation = None;
    }
    assert!(verify(missing_instantiation).is_err());
}
