//! Exact diamond imports with independent private readers and one borrowed owner.
use super::native_byte_buffer::{Export, author, dependency, export, stage};
use super::native_owned_worklists::{deployment, rejected, run_expected};
use super::*;
use serde_json::json;

fn retained(template: &str) -> Native {
    let mut public = Native::template(template);
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        public.root.disable_cleanup(true);
        println!(
            "shared interface public evidence: {}",
            public.root.path().display()
        );
    }
    public
}

fn source(imports: &[(&str, &Export)], body: &str) -> String {
    let mut result = String::new();
    for (name, package) in imports {
        result.push_str(&format!(
            "{}declarations.begin\n(units (use {name} {} {}))\ndeclarations.end\n",
            dependency(package),
            package.package,
            package.revision,
        ));
    }
    result.push_str(body);
    result
}

#[test]
fn native_owned_shared_dependency_interfaces_preserve_diamond_readers_after_source_deletion() {
    let supplier = retained("minimal");
    author(
        &supplier,
        include_str!("../../examples/demanded-callable-proof/library.lkjc"),
    );
    let library = export(&supplier);
    // The generic library is admitted before either concrete reader exists.
    let left = retained("minimal");
    stage(&left, &library);
    author(
        &left,
        &source(
            &[("compact-proof", &library)],
            include_str!("../fixtures/shared-interface-left.lkjc"),
        ),
    );
    let left_package = export(&left);
    let right = retained("minimal");
    stage(&right, &library);
    author(
        &right,
        &source(
            &[("compact-proof", &library)],
            include_str!("../fixtures/shared-interface-right.lkjc"),
        ),
    );
    let right_package = export(&right);
    let consumer = retained("command");
    for package in [&library, &left_package, &right_package] {
        stage(&consumer, package);
    }
    let body = source(
        &[
            ("compact-proof", &library),
            ("left-reader", &left_package),
            ("right-reader", &right_package),
        ],
        include_str!("../fixtures/shared-interface-consumer.lkjc"),
    );
    let before = consumer.revision();
    rejected(
        &consumer,
        &body.replacen(
            "(call right-reader::read (local value))",
            "(call right-reader::read (local value) (local value))",
            1,
        ),
        &before,
        "wrong-reader-arity",
    );
    author(&consumer, &body);
    let draft = consumer.root.path().join("shared-reader-draft.lkjc");
    consumer.cli(
        &[
            "change",
            "draft",
            "--module",
            "shared-reader-user",
            "--output",
            path(&draft),
        ],
        true,
    );
    let plan = consumer.plan(&draft, true);
    assert_eq!(
        compact_field(compact_record(&plan, "result"), "outcome"),
        "unchanged"
    );
    let artifact = consumer.root.path().join("shared-readers.lkja");
    consumer.cli(&["build", "--output", path(&artifact)], true);
    let descriptor = deployment(&consumer, "shared-readers.lkja", "shared-readers");
    for detached in [false, true] {
        if detached {
            // These are owned disposable authoring resources, not a deployed application.
            for public in [&supplier, &left, &right, &consumer] {
                std::fs::remove_dir_all(&public.project).unwrap();
            }
            for package in [&library, &left_package, &right_package] {
                std::fs::remove_file(&package.path).unwrap();
            }
        }
        for n in [i64::MIN, -17, 0, 17, i64::MAX] {
            run_expected(
                &consumer,
                "shared-readers",
                &descriptor,
                detached,
                &format!("diamond-{detached}-{n}"),
                json!([n]),
                &json!({"left": n, "right": 99, "restored": n}),
            );
        }
    }
}
