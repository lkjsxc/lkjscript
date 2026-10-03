//! Copied public host, literal package authoring, owned CPU work and detached execution.
use super::*;

const WORKERS: &str = include_str!("../fixtures/parallel-library.lkjc");
const CONSUMER: &str = include_str!("../../docs/guides/examples/parallel-consumer.lkjc");
const RESULT_WORKERS: &str = include_str!("../fixtures/parallel-result-workers.lkjc");
const RESULT_CONSUMER: &str = include_str!("../fixtures/parallel-result-consumer.lkjc");
const ENTRY: &str = r#"declarations.begin
(units (module create parallel-entry
  (function create main (visibility public)
    (parameter create n (type I64)) (parameter create seed (type I64))
    (parameter create scalar (type Bool))
    (returns (record (simple (record (left I64) (right I64)))
      (aggregate (record (left I64) (right I64)))))
    (effect (task))
    (body (call parallel-consumer::main (local n) (local seed) (local scalar))))
  (component create command (visibility private)
    (port create main
      (type (task-function (I64 I64 Bool)
        (record (simple (record (left I64) (right I64)))
          (aggregate (record (left I64) (right I64)))) (row)))
      (function main))))
  (target create parallel (component parallel-entry::command)
    (runner command) (port parallel-entry::command::main)))
declarations.end
"#;

#[test]
fn native_parallel_three_packages_owned_reduction_edit_and_source_free_execution() {
    // Native copies the executable and runs it with an empty environment/PATH in
    // a disposable directory outside the compiler checkout. All meaning below
    // enters through public literal requests, never through Rust graph builders.
    let library = Native::template("command");
    let before = library.revision();
    let invalid_borrow = library.input(
        "rejected-borrowed-task.lkjc",
        &format!(
            "request base={before}\ndeclarations.begin\n(units (module create invalid-task\n\
            (function create borrowed (visibility public)\n\
            (parameter create value (type ByteBuffer) (use borrow))\n\
            (returns I64) (effect (task)) (body (i64 0)))))\ndeclarations.end\n"
        ),
    );
    let rejected = library.plan(&invalid_borrow, false);
    assert!(
        rejected.iter().any(|r| r.operation == "diagnostic"
            && compact_field(r, "code") == "kernel_buffer_ownership")
    );
    assert_eq!(library.revision(), before);
    author(&library, WORKERS);
    unchanged(&library, "parallel-workers");
    let p = export(&library);
    let consumer = Native::new();
    stage(&consumer, &p);
    let input = format!(
        "{}declarations.begin\n(units (use parallel-workers {} {}))\ndeclarations.end\n{CONSUMER}",
        dependency(&p),
        p.package,
        p.revision,
    );
    let left = "(call parallel-workers::buffer-job (i64 31) (local buffer))";
    let before = consumer.revision();
    for (name, bad, code) in [
        (
            "pure-parent",
            input.replacen("(effect (task))", "(effect pure)", 1),
            "kernel_parallel_context",
        ),
        (
            "pure-child",
            input.replacen("parallel-workers::buffer-job", "parallel-workers::pure-buffer", 1),
            "kernel_parallel_call",
        ),
        (
            "effectful-child",
            input.replacen("parallel-workers::buffer-job", "parallel-workers::effectful-buffer", 1),
            "kernel_parallel_call",
        ),
        (
            "borrowed-child",
            input.replacen("parallel-workers::buffer-job", "parallel-workers::borrowed-buffer", 1),
            "kernel_parallel_call",
        ),
        (
            "dynamic-child",
            input.replacen(left, "(invoke (function-value parallel-workers::ordinary-job) (i64 31))", 1),
            "kernel_parallel_call",
        ),
        (
            "duplicate-owner",
            input.replacen("(call parallel-workers::cell-job (local n) (local cell))", left, 1),
            "kernel_buffer_ownership",
        ),
        (
            "generic-child",
            input.replacen(left, "(implementation-call parallel-workers::generic-reduce (types ByteBuffer) (implementations concrete@parallel-workers::Octets) (i64 31) (local buffer))", 1),
            "kernel_parallel_call",
        ),
    ] {
        assert_ne!(bad, input, "{name} must change the literal proposal");
        let request = consumer.input(&format!("rejected-{name}.lkjc"), &format!("request base={before}\n{bad}"));
        let rejected = consumer.plan(&request, false);
        assert!(rejected.iter().any(|r| r.operation == "diagnostic" && compact_field(r, "code") == code), "{name}: {rejected:?}");
        assert_eq!(consumer.revision(), before, "{name} must not publish");
    }
    author(&consumer, &input);
    let original = std::fs::read_to_string(unchanged(&consumer, "parallel-consumer")).unwrap();
    assert_eq!(original.matches("(parallel").count(), 2);
    let before = consumer.revision();
    let edited = consumer.input(
        "edited-parallel.lkjc",
        &original.replace("(i64 31)", "(i64 32)"),
    );
    let plan = consumer.plan(&edited, true);
    consumer.apply(&edited, &plan, true);
    assert_ne!(consumer.revision(), before);
    assert_eq!(
        std::fs::read_to_string(unchanged(&consumer, "parallel-consumer")).unwrap(),
        original
            .replacen(&before, &consumer.revision(), 1)
            .replace("(i64 31)", "(i64 32)"),
        "editing a child argument must retain every accepted expression and owner identity",
    );
    consumer.cli(&["check"], true);
    let owners = consumer.cli(&["query", "owners", "--kind", "task_function"], true);
    let main = owners
        .iter()
        .find(|r| r.operation == "owner" && compact_field(r, "name") == "main")
        .unwrap();
    let definition = consumer.cli(
        &[
            "inspect",
            "owner",
            "task_function",
            compact_field(main, "id"),
            "--detail",
            "definition",
            "--limit",
            "1000",
        ],
        true,
    );
    let parallel = definition
        .iter()
        .filter(|r| {
            r.operation == "definition.expression" && compact_field(r, "form") == "parallel"
        })
        .collect::<Vec<_>>();
    assert_eq!(parallel.len(), 2);
    for node in parallel {
        let children = definition
            .iter()
            .filter(|r| {
                r.operation == "definition.expression"
                    && compact_field(r, "parent") == compact_field(node, "id")
            })
            .collect::<Vec<_>>();
        assert_eq!(children.len(), 2);
        assert_eq!(
            children
                .iter()
                .map(|r| compact_field(r, "slot"))
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["parallel_left", "parallel_right"])
        );
        assert!(children.iter().all(|r| compact_field(r, "form") == "call"));
        assert_ne!(
            compact_field(children[0], "id"),
            compact_field(children[1], "id")
        );
    }
    // Export the program containing Parallel as well as its separately authored
    // worker library; final compilation must traverse both exact transports.
    let c = export(&consumer);
    let runner = Native::new();
    stage(&runner, &p);
    stage(&runner, &c);
    author(
        &runner,
        &format!(
            "{}{}declarations.begin\n(units (use parallel-consumer {} {}))\ndeclarations.end\n{ENTRY}",
            dependency(&p),
            dependency(&c),
            c.package,
            c.revision,
        ),
    );
    unchanged(&runner, "parallel-entry");
    let artifact = runner.root.path().join("parallel.lkja");
    runner.cli(&["build", "--output", path(&artifact)], true);
    let deployment = runner.input(
        "parallel.deployment.json",
        &json!({
            "artifact": "parallel.lkja", "target": "parallel", "listen": null,
            "http": null, "session": null, "worker": null,
            "runtime": {"maximum_concurrent_tasks": 1, "maximum_queued_tasks": 0,
                "request_deadline_milliseconds": 30000, "shutdown_grace_milliseconds": 3000,
                "cancellation_grace_milliseconds": 1000},
            "streams": {"maximum_chunk_bytes": 65536, "maximum_buffered_chunks": 8,
                "maximum_total_bytes": 1048576, "maximum_live_streams": 1024},
            "grants": [], "secrets": [], "configuration": {}
        })
        .to_string(),
    );
    for detached in [false, true] {
        if detached {
            for project in [&library.project, &consumer.project, &runner.project] {
                std::fs::remove_dir_all(project).unwrap();
            }
            for package in [&p.path, &c.path] {
                std::fs::remove_file(package).unwrap();
            }
        }
        for (n, seed, scalar) in [
            (0_i64, -257_i64, false),
            (1, 73, true),
            (16, -99, false),
            (64, 8192, true),
        ] {
            let arguments = runner.input(
                &format!("arguments-{detached}-{n}.json"),
                &json!([n, seed, scalar]).to_string(),
            );
            let output = runner
                .root
                .path()
                .join(format!("result-{detached}-{n}.json"));
            if !detached && n == 0 {
                let rejected = runner.cli(
                    &[
                        "run",
                        "parallel",
                        "--arguments-file",
                        path(&arguments),
                        "--result-file",
                        path(&output),
                    ],
                    false,
                );
                assert!(rejected.iter().any(|r| r.operation == "diagnostic"
                    && compact_field(r, "code") == "normalized_runner_grants_required"));
                assert!(!output.exists());
            }
            let mut args = vec!["run", "--deployment", path(&deployment)];
            args.extend([
                "--arguments-file",
                path(&arguments),
                "--result-file",
                path(&output),
            ]);
            let records = runner.cli(&args, true);
            let execution = compact_record(&records, "execution");
            assert_eq!(compact_field(execution, "deadline-milliseconds"), "30000");
            let cleanup: Value = serde_json::from_str(compact_field(execution, "cleanup")).unwrap();
            assert_eq!(cleanup["remaining_tasks"], json!(0));
            assert_eq!(cleanup["cleanup_failures"], json!([]));
            let observation: Value =
                serde_json::from_str(compact_field(execution, "production-observation")).unwrap();
            assert_eq!(observation["parallel_scopes"], json!(2));
            assert_eq!(observation["parallel_workers_spawned"], json!(2));
            assert_eq!(observation["capability_calls"], json!(0));
            // Independent closed-form arithmetic, not outputs from a sequential
            // copy of the authored implementation, determines every result.
            let triangular = n * (n + 1) / 2;
            assert_eq!(
                serde_json::from_slice::<Value>(&std::fs::read(output).unwrap()).unwrap(),
                json!({
                    "simple": {"left": 3 + 32 * 33 / 2, "right": seed + triangular},
                    "aggregate": {"left": seed + 2 * triangular + 14,
                        "right": if scalar { seed + triangular } else { 5 + triangular }},
                })
            );
        }
    }
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        for public in [library, consumer, runner] {
            println!(
                "retained parallel public evidence: {}",
                public.root.keep().display()
            );
        }
    }
}

#[test]
fn native_parallel_owned_results_three_packages_edit_borrow_and_detached_payloads() {
    let library = Native::new();
    // Literal generic and uniquely named carrier contracts form one separately
    // exported library. The witness itself never constructs semantic records.
    author(
        &library,
        &format!(
            "{}{}",
            include_str!("../fixtures/owned-witness-library.lkjc"),
            include_str!("../fixtures/parallel-result-library.lkjc"),
        ),
    );
    unchanged(&library, "abstraction");
    let p = export(&library);
    let workers = Native::new();
    stage(&workers, &p);
    let imports = format!(
        "declarations.begin\n(units (use abstraction {} {}) (use buffer {} {}) (use cell {} {}))\ndeclarations.end\n",
        p.package, p.revision, p.package, p.revision, p.package, p.revision,
    );
    author(
        &workers,
        &format!("{}{imports}{RESULT_WORKERS}", dependency(&p)),
    );
    unchanged(&workers, "result-workers");
    let w = export(&workers);
    let consumer = Native::new();
    stage(&consumer, &p);
    stage(&consumer, &w);
    let input = format!(
        "{}{}{imports}declarations.begin\n(units (use result-workers {} {}))\ndeclarations.end\n{RESULT_CONSUMER}",
        dependency(&p),
        dependency(&w),
        w.package,
        w.revision,
    );
    let before = consumer.revision();
    for (name, bad) in [
        (
            "use-after-transfer",
            input.replacen(
                "(implementations concrete@buffer::Octets) (local data)",
                "(implementations concrete@buffer::Octets) (local value)",
                1,
            ),
        ),
        (
            "consume-result-twice",
            input.replacen(
                "(in (unpack-owned (type (owned-product (field left ByteBuffer) (field right OwnedI64Cell)))",
                "(binding duplicate (type (owned-product (field left ByteBuffer) (field right OwnedI64Cell))) (local pair))\n        (in (unpack-owned (type (owned-product (field left ByteBuffer) (field right OwnedI64Cell)))",
                1,
            ),
        ),
    ] {
        assert_ne!(bad, input, "{name} must change the literal proposal");
        let request = consumer.input(
            &format!("rejected-result-{name}.lkjc"),
            &format!("request base={before}\n{bad}"),
        );
        let rejected = consumer.plan(&request, false);
        assert!(
            rejected.iter().any(|r| r.operation == "diagnostic"
                && compact_field(r, "code") == "kernel_buffer_ownership"),
            "{name}: {rejected:?}",
        );
        assert_eq!(consumer.revision(), before, "{name} must not publish");
    }
    author(&consumer, &input);
    let original = std::fs::read_to_string(unchanged(&consumer, "result-consumer")).unwrap();
    assert_eq!(original.matches("(parallel").count(), 4);
    assert!(original.contains("(owned-product"));
    let before = consumer.revision();
    let edited = consumer.input(
        "edited-result-parallel.lkjc",
        &original.replace("(i64 7)", "(i64 8)"),
    );
    let plan = consumer.plan(&edited, true);
    consumer.apply(&edited, &plan, true);
    assert_ne!(consumer.revision(), before);
    assert_eq!(
        std::fs::read_to_string(unchanged(&consumer, "result-consumer")).unwrap(),
        original
            .replacen(&before, &consumer.revision(), 1)
            .replace("(i64 7)", "(i64 8)"),
        "an owned-result child edit retains the pair and all accepted owner identities",
    );
    consumer.cli(&["check"], true);
    let artifact = consumer.root.path().join("parallel-results.lkja");
    consumer.cli(&["build", "--output", path(&artifact)], true);
    let deployment = consumer.input(
        "parallel-results.deployment.json",
        &json!({
            "artifact": "parallel-results.lkja", "target": "parallel-results", "listen": null,
            "http": null, "session": null, "worker": null,
            "runtime": {"maximum_concurrent_tasks": 1, "maximum_queued_tasks": 0,
                "request_deadline_milliseconds": 30000, "shutdown_grace_milliseconds": 3000,
                "cancellation_grace_milliseconds": 1000},
            "streams": {"maximum_chunk_bytes": 65536, "maximum_buffered_chunks": 8,
                "maximum_total_bytes": 1048576, "maximum_live_streams": 1024},
            "grants": [], "secrets": [], "configuration": {}
        })
        .to_string(),
    );
    for detached in [false, true] {
        if detached {
            for project in [&library.project, &workers.project, &consumer.project] {
                std::fs::remove_dir_all(project).unwrap();
            }
            for package in [&p.path, &w.path] {
                std::fs::remove_file(package).unwrap();
            }
        }
        for n in [i64::MIN, -257, 0, i64::MAX] {
            for accepted in [false, true] {
                let arguments = consumer.input(
                    &format!("result-arguments-{detached}-{n}-{accepted}.json"),
                    &json!([n, accepted]).to_string(),
                );
                let output = consumer
                    .root
                    .path()
                    .join(format!("owned-result-{detached}-{n}-{accepted}.json"));
                let records = consumer.cli(
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
                let execution = compact_record(&records, "execution");
                let cleanup: Value =
                    serde_json::from_str(compact_field(execution, "cleanup")).unwrap();
                assert_eq!(cleanup["remaining_tasks"], json!(0));
                assert_eq!(cleanup["cleanup_failures"], json!([]));
                let observation: Value =
                    serde_json::from_str(compact_field(execution, "production-observation"))
                        .unwrap();
                assert_eq!(observation["parallel_scopes"], json!(5));
                assert_eq!(observation["capability_calls"], json!(0));
                // Fixed full-payload literals and the input signed cell value
                // are independent of the evaluator and authored implementation.
                assert_eq!(
                    serde_json::from_slice::<Value>(&std::fs::read(output).unwrap()).unwrap(),
                    json!({
                        "all": {"bytes": {"$bytes": "AP+ACAk="}, "length": 4,
                            "before": n, "after": -81},
                        "mixed-left": {"bytes": {"$bytes": "AP+ACw=="}, "scalar": n},
                        "mixed-right": {"ordinary": n, "scalar": n},
                        "nested": {"bytes": {"$bytes": "AP+AQA=="}, "scalar": n,
                            "outcome": {"accepted": accepted, "value": n}},
                    }),
                );
            }
        }
    }
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        for public in [library, workers, consumer] {
            println!(
                "retained parallel owned-result public evidence: {}",
                public.root.keep().display(),
            );
        }
    }
}
