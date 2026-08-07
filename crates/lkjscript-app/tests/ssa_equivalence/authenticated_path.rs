use crate::oracle::main_source;
use lkjscript_compiler::compile_source;
use lkjscript_core::{
    SealedSemanticDagRuntime, SemanticDagKind, SemanticDagNode, SemanticDagNodeId,
    SemanticDagPayload, SemanticDagSnapshot, SemanticDagType, StructuralKind,
};

#[test]
fn compiler_authenticates_path_rehydration() {
    let source = main_source(
        "path",
        concat!(
            "unwrap-ok/\nconvert-string-to-path/\nstring-literal/\n",
            "/tmp/authenticated\n/string-literal\n/convert-string-to-path\n/unwrap-ok",
        ),
    );
    let program =
        compile_source(&source, "authenticated-path.lkjscript").expect("compile path return");
    let chunk = program.bytecode();
    let path_type = chunk
        .structural_types()
        .iter()
        .find(|item| item.runtime_type.kind == StructuralKind::Path)
        .expect("path structural type");
    let witness = chunk
        .memory_witnesses()
        .iter()
        .find(|item| item.id == path_type.witness)
        .expect("installed path witness");
    assert!(witness.facts.capabilities.sealed_region);
    assert!(witness.facts.capabilities.semantic_snapshot);
    let snapshot = SemanticDagSnapshot::new(
        vec![SemanticDagNode::new(
            SemanticDagType::new(
                path_type.runtime_type.layout,
                path_type.runtime_type.semantic_type,
                SemanticDagKind::Path,
            ),
            SemanticDagPayload::Path(b"/tmp/authenticated".to_vec()),
        )],
        SemanticDagNodeId::new(0),
    )
    .expect("path snapshot");
    let expected = snapshot.clone();
    let mut runtime = SealedSemanticDagRuntime::new().expect("sealed runtime");
    let owner = runtime
        .rehydrate_authenticated_return(chunk, snapshot)
        .expect("authenticated path import");
    let borrow = runtime.begin_borrow(&owner).expect("path borrow");
    assert_eq!(runtime.export_snapshot(&borrow).expect("export"), expected);
    runtime.end_borrow(borrow).expect("end borrow");
    runtime.release(owner).expect("release path");
    assert_eq!(runtime.metrics().runtime.live_domains, 0);
}
