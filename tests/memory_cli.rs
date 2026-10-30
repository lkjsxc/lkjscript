//! Public semantic adapters run without project paths, environment changes or files.
#![allow(clippy::unwrap_used, clippy::expect_used, reason = "test assertions")]

use lkjscript::platform::control::{CompactRecord, parse_records};
use lkjscript::platform::{MemProjectStore, create_memory_project, execute_in_memory};
use std::sync::OnceLock;

fn command(project: &MemProjectStore, arguments: &[&str]) -> Vec<CompactRecord> {
    let bytes = execute_in_memory(
        project,
        arguments.iter().map(|value| (*value).to_owned()).collect(),
    )
    .unwrap();
    parse_records("memory command", &bytes).unwrap()
}

fn field<'a>(records: &'a [CompactRecord], operation: &str, key: &str) -> &'a str {
    records
        .iter()
        .find(|record| record.operation == operation)
        .unwrap()
        .fields
        .iter()
        .find(|field| field.name == key)
        .unwrap()
        .value
        .as_str()
}

fn revision(project: &MemProjectStore) -> String {
    field(&command(project, &["status"]), "revision", "id").to_owned()
}

fn shared_command() -> MemProjectStore {
    static BASE: OnceLock<MemProjectStore> = OnceLock::new();
    BASE.get_or_init(|| {
        let project = MemProjectStore::new(None).unwrap();
        create_memory_project(&project, "memory-fixture", "command").unwrap();
        command(&project, &["check"]);
        project
    })
    .fork()
    .unwrap()
}

#[test]
fn shared_command_forks_reuse_accepted_standard_and_run_both_evaluators() {
    let first = shared_command();
    let before = first.used_bytes();
    let second = first.fork().unwrap();
    assert!(
        second.used_bytes() - before < 4096,
        "a fork must share stored payloads"
    );
    assert_eq!(revision(&first), revision(&second));
    for project in [&first, &second] {
        let checked = command(project, &["check"]);
        assert_eq!(field(&checked, "tests", "failed"), "0");
        assert_eq!(field(&checked, "tests", "differential"), "equal");
        let result = command(project, &["run", "main"]);
        assert_eq!(field(&result, "execution", "differential"), "equal");
    }
}

#[test]
fn inline_publication_crosses_change_worker_without_leaking_between_forks() {
    let project = MemProjectStore::new(None).unwrap();
    create_memory_project(&project, "memory", "minimal").unwrap();
    let fork = project.fork().unwrap();
    let same_authority = project.clone();
    let before = revision(&project);
    let input = format!(
        r#"request base={before}
declarations.begin
(units (module create arithmetic
  (external create add (visibility private) (implementation core.i64.add) (returns I64)
    (parameter create left (type I64)) (parameter create right (type I64)))
  (function create answer (visibility public) (returns I64) (effect pure)
    (body (call add (i64 35) (i64 7))))
  (test create forty-two (visibility private)
    (actual (call answer)) (expected (i64 42)))
  (component create command (visibility private)
    (port create main (type (function () I64)) (function answer))))
  (target create main (component arithmetic::command) (runner command) (port arithmetic::command::main)))
declarations.end
"#
    );
    let plan = command(&project, &["change", "plan", "--input", &input]);
    let token = field(&plan, "plan", "token");
    command(
        &project,
        &["change", "apply", "--input", &input, "--plan", token],
    );
    assert_ne!(revision(&project), before);
    assert_eq!(revision(&same_authority), revision(&project));
    assert_eq!(revision(&fork), before);
    let checked = command(&project, &["check"]);
    assert_eq!(field(&checked, "tests", "passed"), "1");
    let result = command(&project, &["run", "main"]);
    assert_eq!(field(&result, "execution", "value"), "42");
    assert_eq!(field(&result, "execution", "differential"), "equal");
    let invalid = format!(
        "request base={}\ndeclarations.begin\n(units (module create invalid (function create wrong (visibility private) (returns I64) (effect pure) (body (bool true)))))\ndeclarations.end\n",
        revision(&project)
    );
    let accepted = revision(&project);
    assert!(
        execute_in_memory(
            &project,
            ["change", "plan", "--input", &invalid]
                .map(str::to_owned)
                .to_vec()
        )
        .is_err()
    );
    assert_eq!(revision(&project), accepted);
    assert_eq!(revision(&fork), before);
}

#[test]
fn explicit_memory_authority_rejects_disk_selection_and_concurrent_scopes_stay_isolated() {
    let mut workers = Vec::new();
    for index in 0..4 {
        workers.push(std::thread::spawn(move || {
            let project = MemProjectStore::new(None).unwrap();
            create_memory_project(&project, &format!("project-{index}"), "minimal").unwrap();
            let before = revision(&project);
            for _ in 0..3 {
                assert!(
                    execute_in_memory(
                        &project,
                        ["--project", "/absent", "status"]
                            .map(str::to_owned)
                            .to_vec()
                    )
                    .is_err()
                );
                assert_eq!(revision(&project), before);
            }
            before
        }));
    }
    let revisions: std::collections::BTreeSet<_> = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(revisions.len(), 4);
}
