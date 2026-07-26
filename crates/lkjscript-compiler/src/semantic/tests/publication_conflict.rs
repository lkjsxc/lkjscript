use super::*;
use crate::semantic::schema::{FilePrecondition, ResponseResult, TransactionOperation};

#[test]
fn external_leaf_created_after_backup_is_preserved() {
    let directory = case_dir("publication-leaf-conflict");
    let root = directory.join("main.lkjscript");
    let source = concat!(
        "def/\nname/\nf\n/name\nfn/\nsig/\n->\nUnit\n/sig\n",
        "params/\n/params\nunit\n/fn\n/def\n",
        "main/\nsig/\n->\nUnit\n/sig\nf/\n/f\n/main\n",
    );
    std::fs::write(&root, source).expect("write source");
    let decoded = response(
        &crate::semantic::execute(&request(&root, "{\"kind\":\"snapshot\"}")).expect("snapshot"),
    );
    let ResponseResult::Snapshot { snapshot } = decoded.result else {
        panic!("expected snapshot");
    };
    let declaration = snapshot
        .declarations
        .iter()
        .find(|declaration| declaration.name == "f")
        .expect("function declaration");
    let file = snapshot.source_units.first().expect("source unit");
    let operation = TransactionOperation::RenameDeclaration {
        declaration_key: declaration.key.clone(),
        entity_fingerprint: declaration.fingerprint.clone(),
        new_name: "renamed".into(),
    };
    let precondition = FilePrecondition {
        path: file.path.clone(),
        bytes: file.bytes,
        sha256: file.sha256.clone(),
    };
    let tree = crate::source::load(&root, &lkjscript_core::Limits::default())
        .expect("load publication tree");
    let staged = crate::semantic::transaction::stage(
        &tree,
        &[operation],
        &[precondition],
        crate::semantic::schema::ResourceProfile::Default,
    )
    .expect("stage publication");
    let external = b"external concurrent bytes\n";
    crate::semantic::transaction::simulate_external_leaf_conflict(&staged, &root, external)
        .expect("preserve external conflict");
    assert_eq!(std::fs::read(root).expect("external bytes"), external);
}
