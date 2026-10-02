//! Literal cross-package task methods: edits, exact effects, grants and detached artifacts.
use super::*;

#[test]
fn native_owned_task_methods_contract_only_exports_its_private_requirement_closure() {
    let library = Native::template("command");
    author(
        &library,
        r#"declarations.begin
(units (use std builtin) (module create isolated
  (function create available (visibility private) (returns I64) (effect pure) (body (i64 0)))
  (component create authority (visibility private)
    (requirement create clock (interface std::WallClock)
      (operations std::WallClock::utc-milliseconds) (limits (maximum_calls 1 calls)))
    (port create ready (type (function () I64)) (function available)))
  (owned-contract create Storage (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_83000000000000000000000000000001 finish
      (parameters (Self consume)) (returns I64)
      (effect (task (requirement authority::clock)))))))
declarations.end
"#,
    );
    unchanged(&library, "isolated");
    let package = export(&library);
    let consumer = Native::new();
    stage(&consumer, &package);
    author(&consumer, &dependency(&package));
    consumer.cli(&["check"], true);
    consumer.cli(
        &[
            "build",
            "--output",
            path(&consumer.root.path().join("contract-only.lkja")),
        ],
        true,
    );
}

#[test]
fn native_owned_task_methods_three_packages_keep_effects_and_need_explicit_grants() {
    let library = Native::template("command");
    author(
        &library,
        include_str!("../fixtures/owned-task-method-library.lkjc"),
    );
    let draft = unchanged(&library, "task-method");
    let draft = std::fs::read_to_string(draft).unwrap();
    assert!(draft.contains("(effect (task"));
    assert!(!draft.contains("ByteBuffer"));
    assert!(!draft.contains("OwnedI64Cell"));
    library.cli(&["check"], true);
    let p = export(&library);
    let carriers = Native::template("command");
    stage(&carriers, &p);
    author(
        &carriers,
        &format!(
            "{}declarations.begin\n(units (use task-method {} {}))\ndeclarations.end\n{}",
            dependency(&p),
            p.package,
            p.revision,
            include_str!("../fixtures/owned-task-method-carriers.lkjc"),
        ),
    );
    unchanged(&carriers, "task-carriers");
    carriers.cli(&["check"], true);
    let c = export(&carriers);
    let consumer = Native::new();
    stage(&consumer, &p);
    stage(&consumer, &c);
    let input = format!(
        "{}{}declarations.begin\n(units (use task-method {} {}) (use task-carriers {} {}))\ndeclarations.end\n{}",
        dependency(&p),
        dependency(&c),
        p.package,
        p.revision,
        c.package,
        c.revision,
        include_str!("../fixtures/owned-task-method-consumer.lkjc"),
    );
    let before = consumer.revision();
    for (name, bad, code) in [
        (
            "wrong-witness",
            input.replacen(
                "concrete@task-carriers::Scalar",
                "concrete@task-carriers::Octets",
                1,
            ),
            "kernel_owned_contract",
        ),
        (
            "missing-effect",
            input.replacen(
                "(effect (task (requirement task-method::authority::clock)))",
                "(effect (task))",
                1,
            ),
            "kernel_type_task_requirement",
        ),
        (
            "pure-caller",
            input.replacen(
                "(effect (task (requirement task-method::authority::clock)))",
                "(effect pure)",
                1,
            ),
            "kernel_type_pure_task_call",
        ),
    ] {
        let request = consumer.input(
            &format!("{name}.lkjc"),
            &format!("request base={before}\n{bad}"),
        );
        let rejected = consumer.plan(&request, false);
        assert!(
            rejected
                .iter()
                .any(|r| r.operation == "diagnostic" && compact_field(r, "code") == code),
            "{name}: {rejected:?}"
        );
        assert_eq!(consumer.revision(), before);
    }
    author(&consumer, &input);
    unchanged(&consumer, "task-method-consumer");
    consumer.cli(&["check"], true);
    let artifact = consumer.root.path().join("methods.lkja");
    consumer.cli(&["build", "--output", path(&artifact)], true);
    let descriptor = json!({
        "artifact":"methods.lkja", "target":"task-methods", "listen":null,
        "http":null,"session":null,"worker":null,
        "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,
            "maximum_total_bytes":1048576,"maximum_live_streams":1024},
        "grants":[{"requirement":"clock", "sharing_domain":"owned-task-method-clock",
            "authority_revision":"81".repeat(32), "adapter":{"kind":"wall_clock"}}],
        "secrets":[],"configuration":{}
    });
    let deployment = consumer.input("methods.deployment.json", &descriptor.to_string());
    let mut no_grants = descriptor.clone();
    no_grants["grants"] = json!([]);
    let denied = consumer.input("denied.deployment.json", &no_grants.to_string());
    for detached in [false, true] {
        if detached {
            for project in [&library.project, &carriers.project, &consumer.project] {
                std::fs::remove_dir_all(project).unwrap();
            }
            for package in [&p.path, &c.path] {
                std::fs::remove_file(package).unwrap();
            }
        }
        for n in [i64::MIN, -257, i64::MAX] {
            let arguments = consumer.input(
                &format!("args-{detached}-{n}.json"),
                &json!([n]).to_string(),
            );
            let output = consumer
                .root
                .path()
                .join(format!("result-{detached}-{n}.json"));
            consumer.cli(
                &[
                    "run",
                    "--deployment",
                    path(&denied),
                    "--arguments-file",
                    path(&arguments),
                    "--result-file",
                    path(&output),
                ],
                false,
            );
            assert!(!output.exists());
            let result = consumer.cli(
                &[
                    "run",
                    "--deployment",
                    path(&deployment),
                    "--arguments-file",
                    path(&arguments),
                    "--result-file",
                    path(&output),
                ],
                true,
            );
            let observed: Value = serde_json::from_str(compact_field(
                compact_record(&result, "execution"),
                "production-observation",
            ))
            .unwrap();
            assert_eq!(observed["capability_calls"], json!(2));
            assert_eq!(
                serde_json::from_slice::<Value>(&std::fs::read(output).unwrap()).unwrap(),
                json!({"value":n,"bytes":1})
            );
        }
    }
}
