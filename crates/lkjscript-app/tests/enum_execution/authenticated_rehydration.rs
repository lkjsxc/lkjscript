use lkjscript_compiler::compile_source;
use lkjscript_core::{
    SealedSemanticDagRuntime, SemanticDagKind, SemanticDagNode, SemanticDagNodeId,
    SemanticDagPayload, SemanticDagSnapshot, SemanticDagType, StructuralKind, StructuralLayoutKind,
    StructuralLimits, StructuralSliceExt, StructuralSnapshotLimits, StructuralType,
};

#[test]
fn compiler_authenticates_general_enum_rehydration() {
    let source = concat!(
        "enum/\nname/\nevent\n/name\nvariants/\nvariant/\nname/\nmessage\n/name\nfields/\n",
        "variant-field/\nname/\ntext\n/name\ntype/\nstring\n/type\n/variant-field\n",
        "/fields\n/variant\nvariant/\nname/\nidle\n/name\nfields/\n/fields\n/variant\n",
        "/variants\n/enum\nmain/\nsig/\ninputs/\n/inputs\noutput/\nevent/\n/event\n",
        "/output\n/sig\nvariant-value/\ntype/\nevent/\n/event\n/type\n",
        "variant/\nmessage\n/variant\nfields/\nvariant-field/\nname/\ntext\n/name\n",
        "string-literal/\nauthenticated-enum\n/string-literal\n/variant-field\n",
        "/fields\n/variant-value\n/main\n",
    );
    let program = compile_source(source, "authenticated-enum.lkjscript")
        .expect("compile immutable general enum");
    let chunk = program.bytecode();
    let enum_type = chunk
        .structural_types()
        .iter()
        .find(|item| item.runtime_type.kind == StructuralKind::Enum)
        .expect("enum structural type");
    let witness = chunk
        .memory_witnesses()
        .iter()
        .find(|item| item.id == enum_type.witness)
        .expect("installed enum witness");
    assert!(witness.facts.capabilities.sealed_region);
    assert!(witness.facts.capabilities.process_codec);
    let layout = chunk
        .structural_layouts()
        .get_structural(enum_type.layout)
        .expect("enum structural layout");
    let StructuralLayoutKind::Enum { variants, .. } = &layout.kind else {
        panic!("enum layout required")
    };
    let message = &variants[0];
    let string_type = chunk
        .structural_types()
        .iter()
        .find(|item| item.runtime_type.kind == StructuralKind::String)
        .expect("string structural type");
    let snapshot = SemanticDagSnapshot::new(
        vec![
            SemanticDagNode::new(
                dag_type(string_type.runtime_type),
                SemanticDagPayload::String(b"authenticated-enum".to_vec()),
            ),
            SemanticDagNode::new(
                dag_type(enum_type.runtime_type),
                SemanticDagPayload::Enum {
                    tag: message.physical_tag,
                    fields: vec![SemanticDagNodeId::new(0)],
                },
            ),
        ],
        SemanticDagNodeId::new(1),
        StructuralSnapshotLimits::DEFAULT,
    )
    .expect("general enum snapshot");
    let expected = snapshot.clone();
    let prepared = program.prepared_identity();
    let mut malformed_nodes = expected.nodes().to_vec();
    if let Some(root) = malformed_nodes.last_mut() {
        root.payload = SemanticDagPayload::Enum {
            tag: u64::MAX,
            fields: vec![SemanticDagNodeId::new(0)],
        };
    }
    let malformed = SemanticDagSnapshot::new(
        malformed_nodes,
        expected.root(),
        StructuralSnapshotLimits::DEFAULT,
    )
    .expect("bounded malformed semantic DAG");
    assert!(lkjscript_runtime::rehydrate_process_outcome(
        lkjscript_core::ExecutionOutcome::Returned(lkjscript_core::OwnedValue::from_semantic_dag(
            malformed
        ),),
        chunk,
        prepared,
    )
    .is_err());
    let (rehydrated, report) = lkjscript_runtime::rehydrate_process_outcome(
        lkjscript_core::ExecutionOutcome::Returned(lkjscript_core::OwnedValue::from_semantic_dag(
            snapshot.clone(),
        )),
        chunk,
        prepared,
    )
    .expect("fresh authenticated process rehydration");
    let report = report.expect("structural rehydration report");
    assert_eq!(
        report.input_canonical_dag_hash,
        report.output_canonical_dag_hash
    );
    assert_eq!(report.final_domains, 0);
    assert_eq!(report.final_owners, 0);
    assert_eq!(report.final_loans, 0);
    assert_eq!(report.final_dependencies, 0);
    assert_eq!(report.release_backlog, 0);
    assert!(report.bounded_release_work);
    let lkjscript_core::ExecutionOutcome::Returned(value) = rehydrated else {
        panic!("rehydrated structural return required")
    };
    assert_eq!(value.as_semantic_dag(), Some(&expected));
    let mut runtime =
        SealedSemanticDagRuntime::new(StructuralLimits::default()).expect("sealed runtime");
    let owner = runtime
        .rehydrate_authenticated_return(chunk, snapshot)
        .expect("authenticated enum import");
    let borrow = runtime.begin_borrow(&owner).expect("enum borrow");
    assert_eq!(runtime.export_snapshot(&borrow).expect("export"), expected);
    runtime.end_borrow(borrow).expect("end borrow");
    runtime.release(owner).expect("release enum");
    assert_eq!(runtime.metrics().runtime.live_domains, 0);
}

fn dag_type(value: StructuralType) -> SemanticDagType {
    let kind = match value.kind {
        StructuralKind::String => SemanticDagKind::String,
        StructuralKind::Enum => SemanticDagKind::Enum,
        other => panic!("unexpected enum DAG kind: {other:?}"),
    };
    SemanticDagType::new(value.layout, value.semantic_type, kind)
}
