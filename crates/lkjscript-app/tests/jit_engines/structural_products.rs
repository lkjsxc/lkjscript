use crate::canonical::compile;
use lkjscript_core::{
    decode_execution_outcome, encode_execution_outcome, ExecutionConfig, ExecutionOutcome,
    OwnedValue, SealedSemanticDagRuntime, SemanticDagKind, SemanticDagNode, SemanticDagNodeId,
    SemanticDagPayload, SemanticDagSnapshot, SemanticDagType, SemanticPayload, StructuralKind,
    StructuralLimits, StructuralSnapshotLimits, StructuralType,
};
use lkjscript_jit::{execute_forced, execute_optimizing, JitConfig};
use lkjscript_vm::run_chunk;

#[test]
fn product_runtime_identity_is_content_addressed_not_declaration_order() {
    let source = |field_type: &str, value: &str| {
        format!(
            concat!(
                "product/\nname/\nbox\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\n",
                "{field_type}\n/type\n/field\n/fields\n/product\nmain/\nsig/\ninputs/\n/inputs\n",
                "output/\nproduct/\nbox\n/product\n/output\n/sig\nproduct-value/\nbox\n",
                "field/\nvalue\n{value}\n/field\n/product-value\n/main\n",
            ),
            field_type = field_type,
            value = value,
        )
    };
    let integer = compile(&source("i64", "1"), "integer-box.lkjscript");
    let boolean = compile(&source("bool", "true"), "boolean-box.lkjscript");
    let product_type = |program: &lkjscript_compiler::ExecutableProgram| {
        program
            .bytecode()
            .structural_types()
            .iter()
            .find(|ty| matches!(ty.kind, lkjscript_core::StructuralTypeKind::Product(_)))
            .expect("structural product type")
            .runtime_type
    };
    assert_ne!(product_type(&integer), product_type(&boolean));
}

#[test]
fn copy_product_returns_are_flat_key_free_and_codec_stable() {
    let source = concat!(
        "product/\nname/\npoint\n/name\nfields/\nfield/\nname/\nx\n/name\ntype/\ni64\n/type\n/field\n",
        "field/\nname/\ny\n/name\ntype/\ni64\n/type\n/field\n/fields\n/product\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\nproduct/\npoint\n/product\n/output\n/sig\n",
        "product-value/\npoint\nfield/\nx\n3\n/field\nfield/\ny\n4\n/field\n/product-value\n/main\n",
    );
    let program = compile(source, "copy-product-return.lkjscript");
    let expected = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    assert!(matches!(expected, ExecutionOutcome::Returned(_)));
    let ExecutionOutcome::Returned(value) = &expected else {
        return;
    };
    assert_eq!(value.snapshot_object_count(), 3);
    assert!(matches!(
        value.as_structural().map(|value| &value.payload),
        Some(SemanticPayload::Product(_))
    ));
    let config = JitConfig::default();
    for result in [
        execute_forced(program.ssa(), &ExecutionConfig::default(), config)
            .expect("baseline returns copy product"),
        execute_optimizing(program.ssa(), &ExecutionConfig::default(), config)
            .expect("proof returns copy product"),
    ] {
        assert!(matches!(result.outcome, ExecutionOutcome::Returned(_)));
        let ExecutionOutcome::Returned(native_value) = &result.outcome else {
            continue;
        };
        assert_eq!(native_value.as_structural(), value.as_structural());
        assert_eq!(result.outcome, expected);
        assert_eq!(result.stats.vm_fallbacks, 0);
        assert_eq!(result.stats.native_structural.live_roots, 0);
        assert_eq!(result.stats.native_structural.live_views, 0);
        assert_eq!(result.stats.native_structural.live_destinations, 0);
        assert_eq!(result.stats.native_structural.teardown_failures, 0);
    }
    let encoded = encode_execution_outcome(&expected, 64 * 1024).expect("encode copy product");
    assert_eq!(
        decode_execution_outcome(&encoded, 64 * 1024).expect("decode copy product"),
        expected
    );
}

#[test]
fn compiler_witness_authenticates_sealed_product_rehydration() {
    let source = concat!(
        "product/\nname/\nbox\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\n",
        "string\n/type\n/field\n/fields\n/product\nmain/\nsig/\ninputs/\n/inputs\n",
        "output/\nproduct/\nbox\n/product\n/output\n/sig\nproduct-value/\nbox\n",
        "field/\nvalue\nstring-literal/\nauthenticated\n/string-literal\n/field\n",
        "/product-value\n/main\n",
    );
    let program = compile(source, "authenticated-product-return.lkjscript");
    let chunk = program.bytecode();
    let product = chunk
        .structural_types()
        .iter()
        .find(|item| matches!(item.runtime_type.kind, StructuralKind::Product))
        .expect("product structural type");
    let witness = chunk
        .memory_witnesses()
        .iter()
        .find(|item| item.id == product.witness)
        .expect("installed product witness");
    assert!(witness.facts.capabilities.sealed_region);
    assert_eq!(
        witness.facts.domain,
        lkjscript_contracts::MemoryWitnessDomain::UniqueStructural
    );
    assert_eq!(
        witness.facts.copy,
        lkjscript_contracts::MemoryWitnessCopy::Structural
    );

    let string = chunk
        .structural_types()
        .iter()
        .find(|item| matches!(item.runtime_type.kind, StructuralKind::String))
        .expect("string structural type");
    let nodes = vec![
        SemanticDagNode::new(
            dag_type(string.runtime_type).expect("string DAG type"),
            SemanticDagPayload::String(b"authenticated".to_vec()),
        ),
        SemanticDagNode::new(
            dag_type(product.runtime_type).expect("product DAG type"),
            SemanticDagPayload::Product(vec![SemanticDagNodeId::new(0)]),
        ),
    ];
    let snapshot = SemanticDagSnapshot::new(
        nodes,
        SemanticDagNodeId::new(1),
        StructuralSnapshotLimits::DEFAULT,
    )
    .expect("product semantic DAG");
    let expected = snapshot.clone();
    let wire = encode_execution_outcome(
        &ExecutionOutcome::Returned(OwnedValue::from_semantic_dag(snapshot)),
        64 * 1024,
    )
    .expect("encode key-free process outcome");
    let decoded = decode_execution_outcome(&wire, 64 * 1024).expect("decode process outcome");
    let snapshot = decoded
        .returned()
        .and_then(OwnedValue::as_semantic_dag)
        .cloned()
        .expect("decoded key-free semantic DAG");
    let mut runtime =
        SealedSemanticDagRuntime::new(StructuralLimits::default()).expect("fresh sealed runtime");
    let owner = runtime
        .rehydrate_authenticated_return(chunk, snapshot)
        .expect("authenticated compiler return rehydrates");
    let borrowed = runtime.begin_borrow(&owner).expect("borrow sealed owner");
    assert_eq!(
        runtime.export_snapshot(&borrowed).expect("export"),
        expected
    );
    runtime.end_borrow(borrowed).expect("end borrow");
    let release = runtime.release(owner).expect("release sealed owner");
    assert_eq!(release.regions_released, 1);
    assert_eq!(runtime.metrics().runtime.live_domains, 0);
}

fn dag_type(value: StructuralType) -> Option<SemanticDagType> {
    let kind = match value.kind {
        StructuralKind::String => SemanticDagKind::String,
        StructuralKind::Product => SemanticDagKind::Product,
        _ => return None,
    };
    Some(SemanticDagType::new(
        value.layout,
        value.semantic_type,
        kind,
    ))
}
