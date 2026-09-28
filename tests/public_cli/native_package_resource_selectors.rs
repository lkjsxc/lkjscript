//! Exact deployment selectors are obtained through public package/draft output.
use super::*;

pub(super) fn library_selector(packages: &Packages, name: &str) -> String {
    let package = packages.imports.split_whitespace().nth(2).unwrap();
    format!("{package}/{}", draft_requirement(&packages.library, name))
}

pub(super) fn draft_requirement(public: &Native, name: &str) -> String {
    let draft = std::fs::read_to_string(public.root.path().join("draft.lkjc")).unwrap();
    let words = draft.split_whitespace().collect::<Vec<_>>();
    let requirement = words
        .windows(4)
        .find_map(|words| {
            (words[0] == "(requirement" && words[1] == "edit" && words[3] == name)
                .then_some(words[2])
        })
        .unwrap();
    requirement.to_owned()
}

fn descriptor(public: &Native, selector: &str) -> (PathBuf, Value) {
    let path = public.root.path().join("exact.deployment.json");
    write_deployment(&path, "numbers");
    let mut value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    value["grants"][0]["requirement"] = serde_json::json!(selector);
    (path, value)
}

#[test]
fn native_package_resources_select_exact_imported_authority() {
    let packages = Packages::stage(LIBRARY);
    let selector = library_selector(&packages, "jobs");
    packages.apply(CONSUMER);
    let data = packages.detach();
    let (deployment, descriptor) = descriptor(&packages.consumer, &selector);
    std::fs::write(&deployment, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    let result = packages
        .consumer
        .cli(&["run", "--deployment", path(&deployment)], true);
    let execution = compact_record(&result, "execution");
    let actual: Value = serde_json::from_str(compact_field(execution, "value")).unwrap();
    assert_eq!(actual, serde_json::json!([7, 42, -3]));
    assert_completed_job(&read_job(&data, "numbers"), "numbers");
}

#[test]
fn native_package_resources_separate_same_name_grants_and_durable_effects() {
    let source = CONSUMER.replace(
        "(component create app (visibility private)",
        "(component create app (visibility private) (requirement create jobs (interface std::DurableQueue) (operations std::DurableQueue::initialize std::DurableQueue::enqueue) (limits (maximum_calls 64 calls)))",
    ).replace(
        "(function create numbers (visibility private)\n      (returns (list I64)) (effect (task (requirement lib::queue::jobs)))\n      (body (call process",
        "(function create numbers (visibility private)\n      (returns (list I64)) (effect (task (requirement lib::queue::jobs) (requirement app::jobs)))\n      (body (sequence (capability-call app::jobs std::DurableQueue::initialize) (capability-call app::jobs std::DurableQueue::enqueue (text \"local-only\") (text \"local-only\") (call std::data-encode (types Text) (text \"local payload\")) (i64 0) (i64 0)) (call process",
    ).replace(
        "(function-value decode-numbers) (list I64))))",
        "(function-value decode-numbers) (list I64)))))",
    ).replace(
        "(type (task-function () (list I64) (row (requirement lib::queue::jobs))))",
        "(type (task-function () (list I64) (row (requirement lib::queue::jobs) (requirement jobs))))",
    );
    let packages = Packages::stage(LIBRARY);
    let imported = library_selector(&packages, "jobs");
    packages.apply(&source);
    let transport = packages.consumer.root.path().join("selected.lkjp");
    let exported = packages.consumer.cli(
        &[
            "package",
            "current",
            "export",
            "--kind",
            "transport",
            "--output",
            path(&transport),
        ],
        true,
    );
    let local = format!(
        "{}/{}",
        compact_field(compact_record(&exported, "package"), "id"),
        draft_requirement(&packages.consumer, "jobs")
    );
    let local_data = packages.consumer.root.path().join("local-queue");
    compact_success_at(
        &packages.consumer.executable,
        packages.consumer.root.path(),
        &["data", "initialize", "--root", path(&local_data)],
    );
    let data = packages.detach();
    let (deployment, mut value) = descriptor(&packages.consumer, &imported);
    let mut local_grant = value["grants"][0].clone();
    local_grant["requirement"] = serde_json::json!(local);
    local_grant["sharing_domain"] = serde_json::json!("local-resources");
    local_grant["authority_revision"] = serde_json::json!("c4".repeat(32));
    local_grant["adapter"]["root"] = serde_json::json!("local-queue");
    value["grants"].as_array_mut().unwrap().push(local_grant);
    for reverse in [false, true] {
        let mut ambiguous = value.clone();
        ambiguous["grants"][0]["requirement"] = serde_json::json!("jobs");
        if reverse {
            ambiguous["grants"].as_array_mut().unwrap().reverse();
        }
        reject_before_secrets(
            &packages.consumer,
            &deployment,
            &ambiguous,
            "deployment_grant_ambiguous",
            &[&data, &local_data],
        );
    }
    for (reverse, expected) in [
        (false, serde_json::json!([7, 42, -3])),
        (true, serde_json::json!([])),
    ] {
        if reverse {
            value["grants"].as_array_mut().unwrap().reverse();
        }
        std::fs::write(&deployment, serde_json::to_vec(&value).unwrap()).unwrap();
        let result = packages
            .consumer
            .cli(&["run", "--deployment", path(&deployment)], true);
        let cleanup: Value = serde_json::from_str(compact_field(
            compact_record(&result, "execution"),
            "cleanup",
        ))
        .unwrap();
        assert_eq!(cleanup["remaining_tasks"], 0);
        assert_eq!(cleanup["cleanup_failures"], serde_json::json!([]));
        let actual: Value =
            serde_json::from_str(compact_field(compact_record(&result, "execution"), "value"))
                .unwrap();
        assert_eq!(actual, expected);
        assert_completed_job(&read_job(&data, "numbers"), "numbers");
        assert!(!read_job(&local_data, "local-only").is_empty());
        assert!(!has_job(&data, "local-only"));
        assert!(!has_job(&local_data, "numbers"));
    }
}

fn has_job(data: &Path, job: &str) -> bool {
    let store = DataStore::open(data, "generic-resources", DataLimits::default()).unwrap();
    let key = DataKey::new(vec![DataKeyPart::Text(job.into())], store.limits()).unwrap();
    store
        .begin()
        .unwrap()
        .get("__queue.jobs", &key)
        .unwrap()
        .is_some()
}

fn reject_before_secrets(
    public: &Native,
    deployment: &Path,
    value: &Value,
    code: &str,
    roots: &[&Path],
) {
    let before = roots
        .iter()
        .map(|root| content_inventory(root))
        .collect::<Vec<_>>();
    let mut rejected = value.clone();
    rejected["secrets"] = serde_json::json!([{
        "name":"preflight-sentinel", "variable":"LKJSCRIPT_TEST_PREFLIGHT_SENTINEL"
    }]);
    std::fs::write(deployment, serde_json::to_vec(&rejected).unwrap()).unwrap();
    let result = public.cli(&["run", "--deployment", path(deployment)], false);
    assert!(result.iter().any(|record| record.operation == "diagnostic" && compact_field(record, "code") == code),
        "expected {code}, got {result:?}");
    assert_eq!(
        roots
            .iter()
            .map(|root| content_inventory(root))
            .collect::<Vec<_>>(),
        before
    );
}

#[test]
fn native_package_resources_reject_exact_selector_errors_before_secrets() {
    let packages = Packages::stage(LIBRARY);
    let selector = library_selector(&packages, "jobs");
    let unused = library_selector(&packages, "unused-jobs");
    packages.apply(CONSUMER);
    let data = packages.detach();
    let (deployment, valid) = descriptor(&packages.consumer, &selector);
    reject_before_secrets(
        &packages.consumer,
        &deployment,
        &valid,
        "secret_missing",
        &[&data],
    );
    for (selected, code) in [
        (unused, "deployment_grant_foreign"),
        (
            format!(
                "pkg_{}/{}",
                "1".repeat(32),
                selector.split_once('/').unwrap().1
            ),
            "deployment_grant_foreign",
        ),
        (
            format!(
                "{}/req_{}",
                selector.split_once('/').unwrap().0,
                "1".repeat(32)
            ),
            "deployment_grant_foreign",
        ),
        (format!("{selector}/extra"), "deployment_grant_selector"),
        (
            format!(
                "{}/req_{}",
                selector.split_once('/').unwrap().0,
                "0".repeat(32)
            ),
            "deployment_grant_selector",
        ),
    ] {
        let mut invalid = valid.clone();
        invalid["grants"][0]["requirement"] = serde_json::json!(selected);
        reject_before_secrets(&packages.consumer, &deployment, &invalid, code, &[&data]);
    }
    for name in ["jobs", selector.as_str()] {
        for reverse in [false, true] {
            let mut invalid = valid.clone();
            let mut extra = invalid["grants"][0].clone();
            extra["requirement"] = serde_json::json!(name);
            invalid["grants"].as_array_mut().unwrap().push(extra);
            if reverse {
                invalid["grants"].as_array_mut().unwrap().reverse();
            }
            reject_before_secrets(
                &packages.consumer,
                &deployment,
                &invalid,
                "deployment_grant_duplicate",
                &[&data],
            );
        }
    }
}
