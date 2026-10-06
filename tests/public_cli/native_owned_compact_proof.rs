//! Literal typed witness DAG authoring before concrete items, and bounded detached use.
use super::native_byte_buffer::{author, dependency, export, stage};
use super::native_owned_worklists::{declaration, deployment, rejected, run_expected};
use super::*;
use serde_json::json;

const LIBRARY: &str = include_str!("../../examples/compact-callable-proof/library.lkjc");
const CONSUMER: &str = include_str!("../../examples/compact-callable-proof/consumer.lkjc");

fn retained(template: &str) -> Native {
    let mut public = Native::template(template);
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        public.root.disable_cleanup(true);
        println!(
            "compact proof public evidence: {}",
            public.root.path().display()
        );
    }
    public
}

fn draft(public: &Native, module: &str, label: &str) -> String {
    let file = public.root.path().join(format!("{label}-draft.lkjc"));
    public.cli(
        &[
            "change",
            "draft",
            "--module",
            module,
            "--output",
            path(&file),
        ],
        true,
    );
    let plan = public.plan(&file, true);
    assert_eq!(
        compact_field(compact_record(&plan, "result"), "outcome"),
        "unchanged"
    );
    std::fs::read_to_string(file).unwrap()
}

#[test]
fn native_owned_compact_callable_proof_large_library_scoped_reads_and_detached_use() {
    exercise(LIBRARY, CONSUMER);
}

#[test]
fn native_owned_demanded_callable_proof_recursive_library_scoped_reads_and_detached_use() {
    exercise(
        include_str!("../../examples/demanded-callable-proof/library.lkjc"),
        CONSUMER,
    );
}

#[test]
fn native_owned_concrete_callable_proof_depth_24_scoped_reads_and_detached_use() {
    exercise(
        include_str!("../../examples/demanded-callable-proof/library.lkjc"),
        include_str!("../../examples/concrete-callable-proof/consumer.lkjc"),
    );
}

fn exercise(library_source: &str, consumer_source: &str) {
    let library = retained("minimal");
    let before = library.revision();
    let first = "parameter@read24@implparam_c1000000000000000000000000000024";
    let foreign = "parameter@read23@implparam_c1000000000000000000000000000023";
    assert!(library_source.contains(first));
    rejected(
        &library,
        &library_source.replacen(first, foreign, 1),
        &before,
        "foreign-unused-scope",
    );
    // The whole generic library is independently admitted before selecting either
    // the small consumer or the complete 24-layer concrete workload.
    author(&library, library_source);
    std::fs::copy(
        library.root.path().join("native.lkjc"),
        library.root.path().join("library.lkjc"),
    )
    .unwrap();
    let original = draft(&library, "compact-proof", "library");
    assert!(original.contains("read24"));
    assert!(!original.contains("OwnedI64Cell"));
    let deep = declaration(&library, "compact-proof", "read24");
    let package = export(&library);

    let consumer = retained("command");
    stage(&consumer, &package);
    let source = format!(
        "{}declarations.begin\n(units (use compact-proof {} {}))\ndeclarations.end\n{consumer_source}",
        dependency(&package),
        package.package,
        package.revision,
    );
    let before = consumer.revision();
    rejected(
        &consumer,
        &source.replacen(
            "(implementations Cell)",
            "(implementations Cell Alternate)",
            1,
        ),
        &before,
        "extra-unused-application-operand",
    );
    author(&consumer, &source);
    std::fs::copy(
        consumer.root.path().join("native.lkjc"),
        consumer.root.path().join("consumer.lkjc"),
    )
    .unwrap();
    let original = draft(&consumer, "compact-proof-user", "consumer");
    let alternate = declaration(&consumer, "compact-proof-user", "Alternate");
    let cell = declaration(&consumer, "compact-proof-user", "Cell");
    assert_ne!(alternate, cell);
    let artifact = consumer.root.path().join("compact-proof.lkja");
    consumer.cli(&["build", "--output", path(&artifact)], true);
    let descriptor = deployment(&consumer, "compact-proof.lkja", "compact-proof");
    for n in [i64::MIN, -17, 0, 17, i64::MAX] {
        run_expected(
            &consumer,
            "compact-proof",
            &descriptor,
            false,
            &format!("original-{n}"),
            json!([n]),
            &json!({"observed":n,"alternate":99,"restored":n}),
        );
    }
    assert!(original.contains("(i64 99)"));
    let edit = consumer.input(
        "consumer-edit.lkjc",
        &original.replacen("(i64 99)", "(i64 100)", 1),
    );
    let plan = consumer.plan(&edit, true);
    consumer.apply(&edit, &plan, true);
    consumer.cli(&["check"], true);
    assert_eq!(
        declaration(&consumer, "compact-proof-user", "Alternate"),
        alternate
    );
    assert_eq!(declaration(&consumer, "compact-proof-user", "Cell"), cell);
    run_expected(
        &consumer,
        "compact-proof",
        &descriptor,
        false,
        "edited",
        json!([17]),
        &json!({"observed":17,"alternate":100,"restored":17}),
    );
    let edited = draft(&consumer, "compact-proof-user", "edited");
    let restore = consumer.input(
        "consumer-restore.lkjc",
        &edited.replacen("(i64 100)", "(i64 99)", 1),
    );
    let plan = consumer.plan(&restore, true);
    consumer.apply(&restore, &plan, true);
    consumer.cli(&["check"], true);
    assert_eq!(declaration(&library, "compact-proof", "read24"), deep);
    draft(&consumer, "compact-proof-user", "restored");

    // Only these owned disposable projects and transport are removed. Literal
    // proposals, copied executables, the admitted artifact and observations remain.
    std::fs::remove_dir_all(&library.project).unwrap();
    std::fs::remove_dir_all(&consumer.project).unwrap();
    std::fs::remove_file(&package.path).unwrap();
    for n in [i64::MIN, -17, 0, 17, i64::MAX] {
        run_expected(
            &consumer,
            "compact-proof",
            &descriptor,
            true,
            &format!("detached-{n}"),
            json!([n]),
            &json!({"observed":n,"alternate":99,"restored":n}),
        );
    }
}
