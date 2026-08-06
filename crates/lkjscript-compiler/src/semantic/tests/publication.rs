use super::*;
use crate::semantic::schema::{FilePrecondition, ResponseResult, TransactionOperation};

fn snapshot(root: &std::path::Path) -> crate::semantic::schema::SnapshotResult {
    let decoded = response(
        &crate::semantic::execute(&request(root, "{\"kind\":\"snapshot\"}"))
            .expect("publication snapshot"),
    );
    let ResponseResult::Snapshot { snapshot } = decoded.result else {
        panic!("expected snapshot");
    };
    *snapshot
}

#[test]
fn publication_failure_restores_every_original_byte() {
    let directory = case_dir("publication-failure");
    let root = directory.join("main.lkjscript");
    let source = concat!(
        "def/\nname/\nf\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\n",
        "params/\n/params\nunit\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nf/\n/f\n/main\n",
    );
    std::fs::write(&root, source).expect("write publication source");
    let snapshot = snapshot(&root);
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

#[cfg(unix)]
#[test]
fn publication_rejects_changed_ancestor_without_touching_alias() {
    use std::os::unix::fs::symlink;

    let directory = case_dir("publication-ancestor");
    let sources = directory.join("sources");
    let alias = directory.join("alias");
    std::fs::create_dir_all(&sources).expect("create source parent");
    std::fs::create_dir_all(&alias).expect("create alias parent");
    let root = sources.join("main.lkjscript");
    let alias_root = alias.join("main.lkjscript");
    let source = concat!(
        "def/\nname/\nf\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\n",
        "params/\n/params\nunit\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nf/\n/f\n/main\n",
    );
    std::fs::write(&root, source).expect("write source");
    std::fs::write(&alias_root, source).expect("write alias source");
    let snapshot = snapshot(&root);
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
    let staged = crate::semantic::transaction::stage(&tree, &[operation], &[precondition])
        .expect("stage publication");
    let moved = directory.join("moved");
    std::fs::rename(&sources, &moved).expect("move validated parent");
    symlink(&alias, &sources).expect("replace parent with alias");
    let failure = crate::semantic::transaction::publish(&staged, &root)
        .expect_err("changed ancestor must fail closed");
    assert!(failure.message.contains("symbolic-link alias"));
    assert_eq!(
        std::fs::read_to_string(alias_root).expect("alias bytes"),
        source
    );
    assert_eq!(
        std::fs::read_to_string(moved.join("main.lkjscript")).expect("original bytes"),
        source
    );
}

#[test]
fn prepared_journal_is_rolled_back_before_the_next_read() {
    let directory = case_dir("publication-recovery");
    let root = directory.join("main.lkjscript");
    let library = directory.join("lib.lkjscript");
    let root_source = concat!(
        "imports/\nimport/\nmodule/\nlib.lkjscript\n/module\ndeclarations/\nf\n/declarations\n/import\n/imports\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nf/\n/f\n/main\n"
    );
    let library_source = concat!(
        "def/\nname/\nf\n/name\npublic\nfn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\n",
        "params/\n/params\nunit\n/fn\n/def\n"
    );
    std::fs::write(&root, root_source).expect("write recovery root");
    std::fs::write(&library, library_source).expect("write recovery library");
    let snapshot = snapshot(&root);
    let declaration = snapshot
        .declarations
        .iter()
        .find(|item| item.name == "f")
        .expect("recovery function");
    let operation = TransactionOperation::RenameDeclaration {
        declaration_key: declaration.key.clone(),
        entity_fingerprint: declaration.fingerprint.clone(),
        new_name: "renamed".into(),
    };
    let preconditions: Vec<_> = snapshot
        .source_units
        .iter()
        .map(|file| FilePrecondition {
            path: file.path.clone(),
            bytes: file.bytes,
            sha256: file.sha256.clone(),
        })
        .collect();
    let tree =
        crate::source::load(&root, &lkjscript_core::Limits::default()).expect("load recovery tree");
    let staged = crate::semantic::transaction::stage(&tree, &[operation], &preconditions)
        .expect("stage recovery transaction");
    let journal_path = crate::semantic::transaction::simulate_prepared_crash(&staged, &root)
        .expect("simulate interrupted publication");
    let recovered = crate::semantic::execute(&request(&root, "{\"kind\":\"snapshot\"}"))
        .expect("recover before snapshot");
    let recovered_result = response(&recovered).result;
    assert!(
        matches!(recovered_result, ResponseResult::Snapshot { .. }),
        "unexpected recovery result: {recovered_result:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&root).expect("recovered root"),
        root_source
    );
    assert_eq!(
        std::fs::read_to_string(&library).expect("recovered library"),
        library_source
    );
    assert!(!journal_path.exists());
}
