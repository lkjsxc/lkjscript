//! A missing test-only environment binding makes secret lookup observable without
//! reading any real secret. Static grant rejection must precede that lookup.
use super::*;

#[test]
fn native_package_resources_reject_ambiguous_grants_before_secret_lookup() {
    let source = CONSUMER.replace(
        "(component create app (visibility private)",
        "(component create app (visibility private) (requirement create jobs (interface std::DurableQueue) (operations std::DurableQueue::initialize) (limits (maximum_calls 64 calls)))",
    );
    let packages = Packages::stage(LIBRARY);
    packages.apply(&source);
    let data = packages.detach();
    for (mode, code) in [
        ("padded", "deployment_grant_ambiguous"),
        ("single", "deployment_grant_ambiguous"),
        ("duplicate", "deployment_grant_duplicate"),
    ] {
        assert_admission(&packages.consumer, &data, mode, code);
    }
}

#[test]
fn native_package_resources_keep_grant_failures_ahead_of_secret_lookup() {
    let packages = Packages::stage(LIBRARY);
    packages.apply(CONSUMER);
    let data = packages.detach();
    for (mode, code) in [
        ("single", "secret_missing"),
        ("missing", "deployment_grant_missing"),
        ("padded", "deployment_grant_foreign"),
        ("duplicate", "deployment_grant_duplicate"),
    ] {
        assert_admission(&packages.consumer, &data, mode, code);
    }
}

fn assert_admission(public: &Native, data: &Path, mode: &str, code: &str) {
    let before = content_inventory(data);
    let deployment = public.root.path().join(format!("admission-{mode}.json"));
    write_deployment(&deployment, "numbers");
    let mut descriptor: Value =
        serde_json::from_slice(&std::fs::read(&deployment).unwrap()).unwrap();
    // Native::cli clears the complete environment, so this variable is absent.
    descriptor["secrets"] = serde_json::json!([{
        "name": "preflight-sentinel",
        "variable": "LKJSCRIPT_TEST_PREFLIGHT_SENTINEL"
    }]);
    match mode {
        "missing" => descriptor["grants"] = serde_json::json!([]),
        "padded" | "duplicate" => {
            let mut extra = descriptor["grants"][0].clone();
            if mode == "padded" {
                extra["requirement"] = serde_json::json!("unused-jobs");
            }
            descriptor["grants"].as_array_mut().unwrap().push(extra);
        }
        "single" => {}
        _ => unreachable!(),
    }
    std::fs::write(&deployment, serde_json::to_vec(&descriptor).unwrap()).unwrap();
    let rejected = public.cli(&["run", "--deployment", path(&deployment)], false);
    assert!(
        rejected.iter().any(|record| record.operation == "diagnostic"
            && compact_field(record, "code") == code),
        "{mode}: expected {code}, got {rejected:?}"
    );
    assert_eq!(content_inventory(data), before, "{mode}");
}
