use super::*;

#[test]
fn native_package_resources_preserve_a_failed_foreign_borrow_without_completing_the_job() {
    let original =
        "(body (call std::data-decode-or (types (list I64)) (local payload) (list I64))))";
    assert_eq!(CONSUMER.matches(original).count(), 1);
    let source = CONSUMER.replacen(
        original,
        "(body (list I64 (call std::divide (i64 1) (i64 0)))))",
        1,
    );
    let packages = Packages::stage(LIBRARY);
    packages.apply(&source);
    let data = packages.detach();
    let deployment = packages
        .consumer
        .root
        .path()
        .join("generic.deployment.json");
    write_deployment(&deployment, "numbers");
    let rejected = packages
        .consumer
        .cli(&["run", "--deployment", path(&deployment)], false);
    assert!(
        rejected
            .iter()
            .any(|record| record.operation == "diagnostic"
                && compact_field(record, "code") == "normalized_integer_division")
    );
    let failed_job = read_job(&data, "numbers");
    super::super::failure::assert_leased_job(&failed_job, b"package-worker");

    write_deployment(&deployment, "text");
    let result = packages
        .consumer
        .cli(&["run", "--deployment", path(&deployment)], true);
    let actual: Value =
        serde_json::from_str(compact_field(compact_record(&result, "execution"), "value")).unwrap();
    assert_eq!(actual, serde_json::json!("日本語 + generic"));
    assert_completed_job(&read_job(&data, "text"), "text");
    assert_eq!(read_job(&data, "numbers"), failed_job);
}
