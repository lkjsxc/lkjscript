//! Literal product authoring, identity-preserving drafts, exact transport and detached execution.
use super::native_byte_buffer::{author, dependency, export, stage};
use super::*;
use serde_json::json;

#[path = "native_owned_choices.rs"]
mod owned_choices;
#[path = "native_owned_tasks.rs"]
mod owned_tasks;

#[test]
fn native_owned_metadata_cross_package_draft_and_detached_execution() {
    let producer = Native::new();
    author(
        &producer,
        include_str!("../fixtures/owned-products-read.lkjc"),
    );
    let draft = unchanged(&producer, "product-read");
    assert!(
        std::fs::read_to_string(draft)
            .unwrap()
            .contains("(name tag)")
    );
    let package = export(&producer);
    let consumer = Native::new();
    stage(&consumer, &package);
    author(
        &consumer,
        &format!(
            "{}declarations.begin\n(units (use product-read {} {}))\ndeclarations.end\n{}",
            dependency(&package),
            package.package,
            package.revision,
            include_str!("../fixtures/owned-products-read-consumer.lkjc"),
        ),
    );
    unchanged(&consumer, "read-consumer");
    consumer.cli(&["check"], true);
    let artifact = consumer.root.path().join("metadata.lkja");
    consumer.cli(&["build", "--output", path(&artifact)], true);
    let deployment = consumer.input(
        "metadata.deployment.json",
        &json!({
            "artifact": "metadata.lkja", "target": "metadata-read", "listen": null,
            "http": null, "session": null, "worker": null,
            "streams": {"maximum_chunk_bytes": 65536, "maximum_buffered_chunks": 8,
                "maximum_total_bytes": 1048576, "maximum_live_streams": 1024},
            "grants": [], "secrets": [], "configuration": {}
        })
        .to_string(),
    );
    for detached in [false, true] {
        if detached {
            for project in [&producer.project, &consumer.project] {
                std::fs::remove_dir_all(project).unwrap();
            }
            std::fs::remove_file(&package.path).unwrap();
        }
        for n in [i64::MIN, -257, 0, i64::MAX] {
            let arguments = consumer.input(
                &format!("metadata-{detached}-{n}.json"),
                &json!([n]).to_string(),
            );
            let output = consumer
                .root
                .path()
                .join(format!("metadata-result-{detached}-{n}.json"));
            let mut args = if detached {
                vec!["run", "--deployment", path(&deployment)]
            } else {
                vec!["run", "metadata-read"]
            };
            args.extend([
                "--arguments-file",
                path(&arguments),
                "--result-file",
                path(&output),
            ]);
            consumer.cli(&args, true);
            assert_eq!(
                serde_json::from_slice::<Value>(&std::fs::read(output).unwrap()).unwrap(),
                json!({"direct": n, "forwarded": n, "payload": n})
            );
        }
    }
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        for public in [producer, consumer] {
            println!(
                "retained metadata public evidence: {}",
                public.root.keep().display()
            );
        }
    }
}

fn unchanged(public: &Native, module: &str) -> PathBuf {
    let draft = public
        .root
        .path()
        .join(format!("{module}-{}-draft.lkjc", public.revision()));
    public.cli(
        &[
            "change",
            "draft",
            "--module",
            module,
            "--output",
            path(&draft),
        ],
        true,
    );
    let plan = public.plan(&draft, true);
    assert_eq!(
        compact_field(compact_record(&plan, "result"), "outcome"),
        "unchanged"
    );
    draft
}

#[test]
fn native_owned_products_three_packages_and_detached_execution() {
    let producer = Native::new();
    author(
        &producer,
        include_str!("../fixtures/owned-products-independent.lkjc"),
    );
    unchanged(&producer, "product-independent");
    let owners = producer.cli(&["query", "owners", "--kind", "pure_function"], true);
    let selected = owners
        .iter()
        .find(|record| record.operation == "owner" && compact_field(record, "name") == "take-left")
        .unwrap();
    producer.cli(
        &[
            "inspect",
            "owner",
            "pure_function",
            compact_field(selected, "id"),
            "--detail",
            "definition",
            "--limit",
            "100",
        ],
        true,
    );
    let p = export(&producer);
    let transformer = Native::new();
    stage(&transformer, &p);
    author(
        &transformer,
        &format!(
            "{}declarations.begin\n(units (use product-independent {} {}))\ndeclarations.end\n{}",
            dependency(&p),
            p.package,
            p.revision,
            include_str!("../fixtures/owned-products-transformer.lkjc")
        ),
    );
    unchanged(&transformer, "transform");
    let t = export(&transformer);
    let consumer = Native::new();
    stage(&consumer, &p);
    stage(&consumer, &t);
    author(
        &consumer,
        &format!(
            "{}{}declarations.begin\n(units (use product-independent {} {}) (use transform {} {}))\ndeclarations.end\n{}",
            dependency(&p),
            dependency(&t),
            p.package,
            p.revision,
            t.package,
            t.revision,
            include_str!("../fixtures/owned-products-consumer.lkjc")
        ),
    );
    let draft_path = unchanged(&consumer, "consume");
    let before = consumer.revision();
    let original = std::fs::read_to_string(&draft_path).unwrap();
    let owners = consumer.cli(&["query", "owners", "--kind", "pure_function"], true);
    let main = owners
        .iter()
        .find(|r| r.operation == "owner" && compact_field(r, "name") == "main")
        .unwrap();
    let main = compact_field(main, "id");
    let definition = consumer.cli(
        &[
            "inspect",
            "owner",
            "pure_function",
            main,
            "--detail",
            "definition",
            "--limit",
            "1000",
        ],
        true,
    );
    let unpack = definition
        .iter()
        .find(|r| {
            r.operation == "definition.expression" && compact_field(r, "form") == "unpack_owned"
        })
        .unwrap();
    let extraction = consumer.input("unsupported-extraction.lkjc", &format!(
        "request base={before}\nextract.function as=$helper function={main} expression={} name=unpacked\n",
        compact_field(unpack, "id"),
    ));
    let rejected = consumer.plan(&extraction, false);
    assert!(rejected.iter().any(|r| r.operation == "diagnostic"
        && compact_field(r, "code") == "change_extract_owned_product"));
    assert_eq!(before, consumer.revision());
    assert!(original.contains("(i64 8192)"));
    assert!(original.contains("(field tag I64)"));
    let invalid = consumer.input(
        "invalid-metadata-edit.lkjc",
        &original.replacen("(field tag I64)", "(field tag Secret)", 1),
    );
    let rejected = consumer.plan(&invalid, false);
    assert!(
        rejected
            .iter()
            .any(|record| record.operation == "diagnostic"
                && compact_field(record, "code") == "kernel_owned_product")
    );
    assert_eq!(before, consumer.revision());
    let edited = consumer.input("edited.lkjc", &original.replace("(i64 8192)", "(i64 8191)"));
    let plan = consumer.plan(&edited, true);
    consumer.apply(&edited, &plan, true);
    assert_ne!(before, consumer.revision());
    consumer.cli(&["check"], true);
    let after = unchanged(&consumer, "consume");
    assert_eq!(
        std::fs::read_to_string(after).unwrap(),
        original
            .replacen(&before, &consumer.revision(), 1)
            .replace("(i64 8192)", "(i64 8191)"),
        "canonical re-entry must preserve every declaration, parameter and unpack binding identity"
    );
    let artifact = consumer.root.path().join("products.lkja");
    consumer.cli(&["build", "--output", path(&artifact)], true);
    let deployment = consumer.input(
        "products.deployment.json",
        &json!({
            "artifact": "products.lkja", "target": "products", "listen": null,
            "http": null, "session": null, "worker": null,
            "streams": {"maximum_chunk_bytes": 65536, "maximum_buffered_chunks": 8,
                "maximum_total_bytes": 1048576, "maximum_live_streams": 1024},
            "grants": [], "secrets": [], "configuration": {}
        })
        .to_string(),
    );
    let nested_deployment = consumer.input(
        "nested.deployment.json",
        &std::fs::read_to_string(&deployment)
            .unwrap()
            .replace("\"products\"", "\"products-nested\""),
    );
    for detached in [false, true] {
        if detached {
            for project in [&producer.project, &transformer.project, &consumer.project] {
                std::fs::remove_dir_all(project).unwrap();
            }
            for package in [&p.path, &t.path] {
                std::fs::remove_file(package).unwrap();
            }
        }
        for n in [i64::MIN, 128, i64::MAX] {
            let arguments = consumer.input(
                &format!("arguments-{detached}-{n}.json"),
                &json!([n]).to_string(),
            );
            let output = consumer
                .root
                .path()
                .join(format!("result-{detached}-{n}.json"));
            let mut args = if detached {
                vec!["run", "--deployment", path(&deployment)]
            } else {
                vec!["run", "products"]
            };
            args.extend([
                "--arguments-file",
                path(&arguments),
                "--result-file",
                path(&output),
            ]);
            consumer.cli(&args, true);
            assert_eq!(
                serde_json::from_slice::<Value>(&std::fs::read(output).unwrap()).unwrap(),
                json!({"bytes":{"$bytes":"AP+A"},"scalar":n,"tag":128,"opaque":99})
            );
            let nested_output = consumer
                .root
                .path()
                .join(format!("nested-{detached}-{n}.json"));
            let mut args = if detached {
                vec!["run", "--deployment", path(&nested_deployment)]
            } else {
                vec!["run", "products-nested"]
            };
            args.extend([
                "--arguments-file",
                path(&arguments),
                "--result-file",
                path(&nested_output),
            ]);
            consumer.cli(&args, true);
            assert_eq!(
                serde_json::from_slice::<Value>(&std::fs::read(nested_output).unwrap()).unwrap(),
                json!({"$bytes":"AP+A"})
            );
        }
    }
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        // Opt-in local evidence keeps the literal requests, canonical drafts,
        // executable copies and detached artifact after the source-free test.
        for public in [producer, transformer, consumer] {
            println!(
                "retained owned-product public evidence: {}",
                public.root.keep().display()
            );
        }
    }
}
