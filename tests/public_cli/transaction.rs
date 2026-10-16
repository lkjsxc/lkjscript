//! The executable boundary owns the same atomic symbolic authoring contract.

use super::*;

#[test]
fn symbolic_transaction_file_is_one_json_result_and_failed_test_keeps_head() {
    let temporary = tempfile::tempdir().unwrap();
    let project = temporary.path().join("project");
    let executable = binary();
    let created = support::output(Command::new(&executable).args([
        "new",
        project.to_str().unwrap(),
        "--template",
        "command",
    ]))
    .unwrap();
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stdout)
    );
    let input = temporary.path().join("delta.lkjx");
    let source = "(tx (base @head) (upsert-module math (upsert-function square (visibility public) (param value I64) (returns I64) (effect pure) (body (call std::multiply (local value) (local value)))) (upsert-test twelve-squared (actual (call math::square (i64 12))) (expected (i64 144)))))";
    std::fs::write(&input, source).unwrap();
    let execute = || {
        support::output(
            Command::new(&executable)
                .arg("--project")
                .arg(&project)
                .args(["tx", "--compact", "--input-file"])
                .arg(&input),
        )
        .unwrap()
    };
    let accepted = execute();
    assert!(
        accepted.status.success(),
        "{}",
        String::from_utf8_lossy(&accepted.stdout)
    );
    assert!(accepted.stderr.is_empty());
    assert_eq!(
        accepted
            .stdout
            .iter()
            .filter(|byte| **byte == b'\n')
            .count(),
        1
    );
    let result: Value = serde_json::from_slice(&accepted.stdout).unwrap();
    assert_eq!(result["status"], "accepted");
    assert_eq!(result["tests_passed"], 1);
    let head_path = project.join("HEAD");
    let head = std::fs::read(&head_path).unwrap();
    std::fs::write(&input, source.replace("(i64 144)", "(i64 143)")).unwrap();
    let rejected = execute();
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stdout).contains("normalized_test_failed"));
    assert_eq!(std::fs::read(head_path).unwrap(), head);
    let compacted = support::output(
        Command::new(&executable)
            .arg("gc")
            .arg("--project")
            .arg(&project),
    )
    .unwrap();
    assert!(
        compacted.status.success(),
        "{}",
        String::from_utf8_lossy(&compacted.stdout)
    );
    assert!(compacted.stderr.is_empty());
    let receipt: Value = serde_json::from_slice(&compacted.stdout).unwrap();
    assert_eq!(receipt["operation"], "gc");
    assert_eq!(receipt["disk"]["packs_after"], 1);
    assert_eq!(receipt["revision"], result["revision"]);
    assert_eq!(std::fs::read(project.join("HEAD")).unwrap(), head);
    let checked = support::output(
        Command::new(&executable)
            .arg("--project")
            .arg(&project)
            .arg("check"),
    )
    .unwrap();
    assert!(
        checked.status.success(),
        "{}",
        String::from_utf8_lossy(&checked.stdout)
    );
}
