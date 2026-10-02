//! Public transport admits task-owned signatures, not ambient rights or wrong witnesses.
use super::*;

#[test]
fn native_owned_tasks_four_packages_keep_witnesses_and_source_free_execution() {
    let library = Native::new();
    author(
        &library,
        &[
            include_str!("../fixtures/owned-witness-library.lkjc"),
            include_str!("../fixtures/owned-choices-outcome.lkjc"),
            include_str!("../fixtures/owned-task-library.lkjc"),
        ]
        .join("\n"),
    );
    unchanged(&library, "task-transfer");
    library.cli(&["check"], true);
    let p = export(&library);
    let carrier = Native::new();
    stage(&carrier, &p);
    author(
        &carrier,
        &format!(
            "{}declarations.begin\n(units (use abstraction {} {}))\ndeclarations.end\n{}",
            dependency(&p),
            p.package,
            p.revision,
            include_str!("../fixtures/owned-witness-cell.lkjc")
        ),
    );
    let c = export(&carrier);
    let octets = Native::new();
    stage(&octets, &p);
    author(
        &octets,
        &format!(
            "{}declarations.begin\n(units (use abstraction {} {}))\ndeclarations.end\n{}",
            dependency(&p),
            p.package,
            p.revision,
            include_str!("../fixtures/owned-witness-buffer.lkjc")
        ),
    );
    let b = export(&octets);
    let consumer = Native::new();
    for package in [&p, &c, &b] {
        stage(&consumer, package);
    }
    let input = format!(
        "{}{}{}declarations.begin\n(units (use task-transfer {} {}) (use cell {} {}) (use buffer {} {}))\ndeclarations.end\n{}",
        dependency(&p),
        dependency(&c),
        dependency(&b),
        p.package,
        p.revision,
        c.package,
        c.revision,
        b.package,
        b.revision,
        include_str!("../fixtures/owned-task-consumer.lkjc")
    );
    let before = consumer.revision();
    for (name, bad, code) in [
        (
            "wrong-witness",
            input.replacen("concrete@cell::Scalar", "concrete@buffer::Octets", 1),
            "kernel_owned_contract",
        ),
        (
            "pure-task",
            input.replacen(
                "(function create task-main (visibility public) (effect (task))",
                "(function create task-main (visibility public) (effect pure)",
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
    unchanged(&consumer, "task-consumer");
    consumer.cli(&["check"], true);
    let artifact = consumer.root.path().join("tasks.lkja");
    consumer.cli(&["build", "--output", path(&artifact)], true);
    let deployment = consumer.input(
        "tasks.deployment.json",
        &json!({
            "artifact":"tasks.lkja", "target":"owned-tasks", "listen":null,
            "http":null, "session":null, "worker":null,
            "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,
                "maximum_total_bytes":1048576,"maximum_live_streams":1024},
            "grants":[],"secrets":[],"configuration":{}
        })
        .to_string(),
    );
    for detached in [false, true] {
        if detached {
            for project in [
                &library.project,
                &carrier.project,
                &octets.project,
                &consumer.project,
            ] {
                std::fs::remove_dir_all(project).unwrap();
            }
            for package in [&p.path, &c.path, &b.path] {
                std::fs::remove_file(package).unwrap();
            }
        }
        for n in [i64::MIN, -257, i64::MAX] {
            for accepted in [false, true] {
                let arguments = consumer.input(
                    &format!("args-{detached}-{n}-{accepted}.json"),
                    &json!([n, accepted]).to_string(),
                );
                let output = consumer
                    .root
                    .path()
                    .join(format!("result-{detached}-{n}-{accepted}.json"));
                let mut args = if detached {
                    vec!["run", "--deployment", path(&deployment)]
                } else {
                    vec!["run", "owned-tasks"]
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
                    json!({"value":n,"alternate":if accepted {99} else {n},"bytes":1,"product":n})
                );
            }
        }
    }
}
