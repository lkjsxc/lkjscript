use lkjscript_compiler::compile_source;
use lkjscript_core::{
    Limits, SealedSemanticDagRuntime, SemanticDagKind, SemanticDagNode, SemanticDagNodeId,
    SemanticDagPayload, SemanticDagSnapshot, SemanticDagType, StructuralKind, StructuralLayoutKind,
    StructuralLimits, StructuralSnapshotLimits, StructuralType,
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
    let program = compile_source(source, "authenticated-enum.lkjscript", &Limits::default())
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
    let layout = &chunk.structural_layouts()[enum_type.layout.index()];
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
