use lkjscript_compiler::compile_source;
use lkjscript_core::{
    SealedSemanticDagRuntime, SemanticDagKind, SemanticDagNode, SemanticDagNodeId,
    SemanticDagPayload, SemanticDagSnapshot, SemanticDagType, StructuralKind, StructuralLayoutKind,
    StructuralLimits, StructuralSliceExt, StructuralSnapshotLimits, StructuralType,
    StructuralTypeKind,
};

#[test]
fn compiler_authenticates_option_string_rehydration() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\noption/\nstring\n/option\n/output\n",
        "/sig\nsome/\nstring-literal/\noption-value\n/string-literal\n/some\n/main\n",
    );
    exercise(
        source,
        "authenticated-option.lkjscript",
        "some",
        StructuralKind::String,
        b"option-value",
    );
}

#[test]
fn compiler_authenticates_result_path_rehydration() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nresult/\npath\nsystem-error\n/result\n",
        "/output\n/sig\nconvert-string-to-path/\nstring-literal/\n/tmp/result-path\n",
        "/string-literal\n/convert-string-to-path\n/main\n",
    );
    exercise(
        source,
        "authenticated-result.lkjscript",
        "ok",
        StructuralKind::Path,
        b"/tmp/result-path",
    );
}

fn exercise(source: &str, name: &str, active: &str, child_kind: StructuralKind, bytes: &[u8]) {
    let program = compile_source(source, name).expect("compile enum return");
    let chunk = program.bytecode();
    let returned = chunk.main().return_structural.expect("structural return");
    let representation = chunk
        .structural_representations()
        .get_structural(returned)
        .expect("return representation");
    let enum_type = chunk
        .structural_types()
        .get_structural(representation.type_id)
        .expect("return structural type");
    assert_eq!(enum_type.runtime_type.kind, StructuralKind::Enum);
    let witness = chunk
        .memory_witnesses()
        .iter()
        .find(|item| item.id == enum_type.witness)
        .expect("installed enum witness");
    assert!(
        witness.facts.capabilities.sealed_region,
        "witness facts: {:?}",
        witness.facts
    );
    assert!(witness.facts.capabilities.process_codec);
    let StructuralTypeKind::Enum(enum_id) = enum_type.kind else {
        panic!("enum identity")
    };
    let declared = chunk
        .enums()
        .iter()
        .find(|item| item.id == enum_id)
        .expect("declared enum");
    let active_id = declared
        .variants
        .iter()
        .find(|item| item.name == active)
        .expect("active variant")
        .id;
    let StructuralLayoutKind::Enum { variants, .. } = &chunk
        .structural_layouts()
        .get_structural(enum_type.layout)
        .expect("enum layout")
        .kind
    else {
        panic!("enum layout")
    };
    let active_layout = variants
        .iter()
        .find(|item| item.variant == active_id)
        .expect("active layout");
    let child = chunk
        .structural_types()
        .iter()
        .find(|item| item.runtime_type.kind == child_kind)
        .expect("child structural type");
    let payload = match child_kind {
        StructuralKind::String => SemanticDagPayload::String(bytes.to_vec()),
        StructuralKind::Path => SemanticDagPayload::Path(bytes.to_vec()),
        _ => panic!("unsupported child kind"),
    };
    let snapshot = SemanticDagSnapshot::new(
        vec![
            SemanticDagNode::new(dag_type(child.runtime_type), payload),
            SemanticDagNode::new(
                dag_type(enum_type.runtime_type),
                SemanticDagPayload::Enum {
                    tag: active_layout.physical_tag,
                    fields: vec![SemanticDagNodeId::new(0)],
                },
            ),
        ],
        SemanticDagNodeId::new(1),
        StructuralSnapshotLimits::DEFAULT,
    )
    .expect("option/result snapshot");
    let expected = snapshot.clone();
    let mut runtime =
        SealedSemanticDagRuntime::new(StructuralLimits::default()).expect("sealed runtime");
    let owner = runtime
        .rehydrate_authenticated_return(chunk, snapshot)
        .expect("authenticated option/result import");
    let borrow = runtime.begin_borrow(&owner).expect("borrow");
    assert_eq!(runtime.export_snapshot(&borrow).expect("export"), expected);
    runtime.end_borrow(borrow).expect("end borrow");
    runtime.release(owner).expect("release");
    assert_eq!(runtime.metrics().runtime.live_domains, 0);
}

fn dag_type(value: StructuralType) -> SemanticDagType {
    let kind = match value.kind {
        StructuralKind::String => SemanticDagKind::String,
        StructuralKind::Path => SemanticDagKind::Path,
        StructuralKind::Enum => SemanticDagKind::Enum,
        other => panic!("unexpected DAG kind: {other:?}"),
    };
    SemanticDagType::new(value.layout, value.semantic_type, kind)
}
