use super::*;
use crate::semantic::schema::{FilePrecondition, ResponseResult, TransactionOperation};

#[test]
fn publication_failure_restores_every_original_byte() {
    let directory = case_dir("publication-failure");
    let root = directory.join("main.lkjscript");
    let source = concat!(
        "def/\nname/\nf\n/name\nfn/\nsig/\n->\nUnit\n/sig\n",
        "params/\n/params\nunit\n/fn\n/def\n",
        "main/\nsig/\n->\nUnit\n/sig\nf/\n/f\n/main\n",
    );
    std::fs::write(&root, source).expect("write publication source");
    let decoded = response(
        &crate::semantic::execute(&request(&root, "{\"kind\":\"snapshot\"}"))
            .expect("publication snapshot"),
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
        new_name: "renamed".to_string(),
    };
    let precondition = FilePrecondition {
        path: file.path.clone(),
        bytes: file.bytes,
        sha256: file.sha256.clone(),
    };
    let tree = crate::source::load(&root, &lkjscript_core::Limits::default())
        .expect("load publication tree");
    let mut staged = crate::semantic::transaction::stage(&tree, &[operation], &[precondition])
        .expect("stage publication");
    staged.sources[0].host_path = directory.join("missing/main.lkjscript");
    let before = std::fs::read(&root).expect("bytes before publication failure");
    assert!(crate::semantic::transaction::publish(&staged, &root).is_err());
    assert_eq!(
        std::fs::read(&root).expect("bytes after publication failure"),
        before
    );
}
