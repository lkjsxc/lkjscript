use super::*;
use fixture::Packages;

#[path = "native_package_resource_admission.rs"]
mod admission;
#[path = "native_package_resource_failure.rs"]
mod failure;
#[path = "native_package_resource_fixture.rs"]
mod fixture;
#[path = "native_package_resource_http.rs"]
mod http;
#[path = "native_package_resource_rejections.rs"]
mod rejections;
#[path = "native_package_resource_selectors.rs"]
mod selectors;

const LIBRARY: &str = include_str!("../fixtures/package-resources-library.lkjc");
const CONSUMER: &str = include_str!("../fixtures/package-resources-consumer.lkjc");

#[test]
fn native_package_resources_preserve_exact_authority_and_detached_execution() {
    let packages = Packages::stage(LIBRARY);
    packages.apply(CONSUMER);
    let data = packages.detach();
    assert_execution(&packages.consumer, &data);
}

fn assert_execution(consumer: &Native, data: &Path) {
    let deployment = consumer.root.path().join("generic.deployment.json");
    for (target, expected) in [
        ("numbers", serde_json::json!([7, 42, -3])),
        ("numbers", serde_json::json!([])),
        ("text", serde_json::json!("日本語 + generic")),
        ("text", serde_json::json!("absent")),
    ] {
        write_deployment(&deployment, target);
        let result = consumer.cli(&["run", "--deployment", path(&deployment)], true);
        let execution = compact_record(&result, "execution");
        assert_eq!(
            serde_json::from_str::<Value>(compact_field(execution, "value")).unwrap(),
            expected
        );
        let cleanup: Value = serde_json::from_str(compact_field(execution, "cleanup")).unwrap();
        assert_eq!(cleanup["remaining_tasks"], 0);
        assert_eq!(cleanup["cleanup_failures"], serde_json::json!([]));
    }
    for job in ["numbers", "text"] {
        assert_completed_job(&read_job(data, job), job);
    }
}

#[test]
fn native_package_resources_keep_foreign_contracts_through_a_forwarding_package() {
    let packages = Packages::stage(LIBRARY);
    packages.apply(CONSUMER);
    let transport = packages.consumer.root.path().join("forwarding.lkjp");
    let export = packages.consumer.cli(
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
    let exported = compact_record(&export, "package");
    let final_consumer = Native::template("command");
    final_consumer.cli(
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            compact_field(exported, "transport"),
            "--input-file",
            path(&transport),
        ],
        true,
    );
    let source = CONSUMER
        .replace("lib::read-lease", "bridge::relay")
        .replace("lib::finish", "bridge::finish-here");
    let input = final_consumer.input("final.lkjc", &format!(
        "request base={}\n{}add.dependency package={} semantic-revision={} package-revision={}\n\
         declarations.begin\n(units {} (use bridge {} {}))\ndeclarations.end\n{source}",
        final_consumer.revision(), packages.dependency, compact_field(exported, "id"),
        compact_field(exported, "revision"), compact_field(exported, "package-revision"),
        packages.imports, compact_field(exported, "id"), compact_field(exported, "package-revision")));
    let applied = final_consumer.apply(&input, &final_consumer.plan(&input, true), true);
    final_consumer.cli(&["check"], true);
    fixture::assert_draft(&final_consumer, &identity(&applied, "$consumer"));
    let intermediate = packages.consumer;
    let packages = Packages {
        consumer: final_consumer,
        library: packages.library,
        dependency: packages.dependency,
        imports: packages.imports,
    };
    let data = packages.detach();
    std::fs::remove_dir_all(&intermediate.project).unwrap();
    assert_execution(&packages.consumer, &data);
}
