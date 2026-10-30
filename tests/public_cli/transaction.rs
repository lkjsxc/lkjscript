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

#[test]
fn symbolic_transaction_policy_keeps_unbounded_zero_and_exact_finite_meanings() {
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
    let input = temporary.path().join("policy.lkjx");
    let head = project.join("HEAD");
    let execute = |text: &str| {
        std::fs::write(&input, text).unwrap();
        support::output(
            Command::new(&executable)
                .arg("--project")
                .arg(&project)
                .args(["tx", "--compact", "--input-file"])
                .arg(&input),
        )
        .unwrap()
    };
    for limit in ["0", "01", "18446744073709551616"] {
        let before = std::fs::read(&head).unwrap();
        let result = execute(&format!(
            "(tx (base @head) (policy operations {limit}) (upsert-module denied))"
        ));
        assert!(!result.status.success());
        assert_eq!(std::fs::read(&head).unwrap(), before);
    }
    for (index, limit) in ["1", "18446744073709551615", "unbounded"]
        .iter()
        .enumerate()
    {
        let result = execute(&format!(
            "(tx (base @head) (policy operations {limit}) (upsert-module allowed{index}))"
        ));
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stdout)
        );
    }
    let before = std::fs::read(&head).unwrap();
    let duplicate = execute(
        "(tx (base @head) (policy operations unbounded) (policy operations 1) (upsert-module duplicate))",
    );
    assert!(!duplicate.status.success());
    assert_eq!(std::fs::read(&head).unwrap(), before);
}

#[test]
fn symbolic_transaction_retires_obsolete_functions_with_their_atomic_caller_update() {
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
    let input = temporary.path().join("retire.lkjx");
    let execute = |source: &str| {
        std::fs::write(&input, source).unwrap();
        support::output(
            Command::new(&executable)
                .arg("--project")
                .arg(&project)
                .args(["tx", "--input-file"])
                .arg(&input),
        )
        .unwrap()
    };
    let initial = execute(
        "(tx (base @head) (upsert-module cleanup \
         (upsert-function obsolete (returns I64) (effect pure) (body (i64 7))) \
         (upsert-function caller (returns I64) (effect pure) \
         (body (call cleanup::obsolete)))))",
    );
    assert!(
        initial.status.success(),
        "{}",
        String::from_utf8_lossy(&initial.stdout)
    );
    let initial: Value = serde_json::from_slice(&initial.stdout).unwrap();
    let head = project.join("HEAD");
    let before = std::fs::read(&head).unwrap();
    let rejected =
        execute("(tx (base @head) (delete function cleanup::obsolete (policy owned-closure)))");
    assert!(!rejected.status.success());
    assert!(String::from_utf8_lossy(&rejected.stdout).contains("change_delete_live_reference"));
    assert_eq!(std::fs::read(&head).unwrap(), before);

    let adopted = execute(&format!(
        "(tx (base {}) (delete function cleanup::obsolete (policy owned-closure)) \
         (upsert-module cleanup \
         (upsert-function caller (returns I64) (effect pure) (body (i64 42))) \
         (upsert-test updated-caller (actual (call cleanup::caller)) (expected (i64 42)))))",
        initial["revision"].as_str().unwrap()
    ));
    assert!(
        adopted.status.success(),
        "{}",
        String::from_utf8_lossy(&adopted.stdout)
    );
    assert!(adopted.stderr.is_empty());
    let result: Value = serde_json::from_slice(&adopted.stdout).unwrap();
    assert_eq!(result["status"], "accepted");
    assert_eq!(result["tests_passed"], 1);
    assert_eq!(result["differential"], "equal");
    assert_ne!(std::fs::read(&head).unwrap(), before);
    let after = std::fs::read(&head).unwrap();
    let removed =
        execute("(tx (base @head) (delete function cleanup::obsolete (policy owned-closure)))");
    assert!(!removed.status.success());
    assert!(String::from_utf8_lossy(&removed.stdout).contains("transaction_delete_missing"));
    assert_eq!(std::fs::read(&head).unwrap(), after);
}
