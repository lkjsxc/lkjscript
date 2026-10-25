use super::*;
use semantic_cli::{Mode, Response, execute};

#[test]
fn candidate_selection_never_uses_host_semantic_execution() {
    assert_eq!(Mode::for_candidate(true), Mode::CandidateProcess);
    assert_eq!(Mode::for_candidate(false), Mode::HostSemantic);
    assert_eq!(
        Mode::selected() == Mode::CandidateProcess,
        env::var_os(RELEASE_CANDIDATE_ENVIRONMENT).is_some()
    );
    let first = Native::new();
    let second = Native::new();
    assert_eq!(first.mode, Mode::selected());
    assert_eq!(second.mode, Mode::selected());
    if Mode::selected() == Mode::HostSemantic {
        assert_eq!(first.executable, binary());
        assert_eq!(first.executable, second.executable);
        assert!(!first.executable.starts_with(first.root.path()));
        assert!(!second.executable.starts_with(second.root.path()));
    } else {
        assert_ne!(first.executable, second.executable);
        assert_eq!(first.executable.parent(), Some(first.root.path()));
        assert_eq!(second.executable.parent(), Some(second.root.path()));
    }
}

#[test]
fn relative_paths_and_process_lifecycles_retain_the_process_boundary() {
    assert!(execute(&["status"], vec!["status".to_owned()]).is_none());
    assert!(
        execute(
            &["status"],
            ["--project", "relative", "status"]
                .map(str::to_owned)
                .to_vec()
        )
        .is_none()
    );
    for arguments in [
        vec!["new", "relative"],
        vec!["build", "--output", "relative.lkja"],
        vec!["change", "plan", "--input-file", "relative.lkjc"],
        vec!["run", "main", "--arguments-file", "relative.json"],
        vec!["run", "--deployment", "relative.json"],
        vec!["run", "main", "--batch-input-file", "relative.jsonl"],
        vec!["run", "main", "--batch-result-file", "relative.jsonl"],
        vec!["serve", "--deployment", "/absolute.json"],
        vec!["worker", "--deployment", "/absolute.json"],
        vec!["runtime", "list"],
    ] {
        assert!(
            execute(
                &arguments,
                arguments.iter().map(|s| (*s).to_owned()).collect()
            )
            .is_none()
        );
    }
}

#[test]
fn refusal_projection_preserves_large_diagnostics_and_empty_refusal_is_infrastructure() {
    use lkjscript::platform::contract::exit_status_for;
    use lkjscript::platform::{Diagnostic, DiagnosticClass};
    let message = "x".repeat(4 * 1024 * 1024 + 1);
    let diagnostic = Diagnostic::new(DiagnosticClass::Semantic, "large_test_diagnostic", &message);
    let output = semantic_cli::failure("change", &[diagnostic]);
    assert!(!output.success);
    assert_eq!(output.code, Some(i32::from(exit_status_for(DiagnosticClass::Semantic))));
    assert!(output.stderr.is_empty());
    let records = parse_records("large refusal", &output.stdout).unwrap();
    assert_eq!(compact_field(compact_record(&records, "diagnostic"), "message"), Some(message.as_str()));

    let empty = semantic_cli::failure("change", &[]);
    assert!(!empty.success);
    assert_eq!(empty.code, Some(i32::from(exit_status_for(DiagnosticClass::Infrastructure))));
    assert!(empty.stdout.is_empty());
    assert!(empty.stderr.is_empty());
}

#[test]
fn host_and_process_adapters_agree_on_results_and_prepublication_refusals() {
    let public = Native::process_template("minimal");
    let base = public.revision();
    let source = public.input("arithmetic.lkjc", &format!(r#"request base={}
declarations.begin
(units
  (module create arithmetic
    (external create add (visibility private) (implementation core.i64.add) (returns I64)
      (parameter create left (type I64)) (parameter create right (type I64)))
    (function create plus-forty-two (visibility private)
      (parameter create value (type I64)) (returns I64) (effect pure)
      (body (call add (local value) (i64 42))))
    (test create negative-input (visibility private)
      (actual (call plus-forty-two (i64 -7))) (expected (i64 35)))
    (component create command (visibility private)
      (port create main (type (function (I64) I64)) (function plus-forty-two))))
  (target create main (component arithmetic::command) (runner command) (port arithmetic::command::main)))
declarations.end
"#, base));
    public.apply(&source, &public.plan(&source, true), true);
    public.cli(&["check"], true);
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    let refused = public.input("refused.lkjc", &format!("request base={}\ndeclarations.begin\n(units (module create bad (function create wrong (visibility private) (returns I64) (effect pure) (body (bool true)))))\ndeclarations.end\n", public.revision()));
    let stale_authored = public.input(
        "stale-authored.lkjc",
        &format!("request base={base}\ncreate.module as=$stale name=stale-review\n"),
    );
    for (arguments, expected_value) in [
        (vec!["status"], None),
        (vec!["check"], None),
        (vec!["run", "main", "--arguments", "[0]"], Some("42")),
        (vec!["run", "main", "--arguments", "[-7]"], Some("35")),
        (vec!["run", "main", "--arguments", "[8]"], Some("50")),
        (vec!["query", "find", "module", "missing"], None),
        (vec!["check", "--unsupported"], None),
        (vec!["change", "plan", "--input-file", path(&refused)], None),
        (vec!["change", "plan", "--input-file", path(&source)], None),
        (
            vec!["change", "plan", "--input-file", path(&stale_authored)],
            None,
        ),
    ] {
        let mut full = vec!["--project".to_owned(), path(&public.project).to_owned()];
        full.extend(arguments.iter().map(|s| (*s).to_owned()));
        let host = execute(&arguments, full.clone()).unwrap();
        let process: Response = support::output(
            Command::new(&public.executable)
                .args(&full)
                .current_dir(public.root.path())
                .env_clear()
                .env("PATH", ""),
        )
        .unwrap()
        .into();
        assert_eq!(host.code, process.code, "{arguments:?}");
        assert_eq!(host.stdout, process.stdout, "{arguments:?}");
        assert_eq!(host.stderr, process.stderr, "{arguments:?}");
        let records = parse_records("semantic parity", &host.stdout).unwrap();
        if let Some(expected) = expected_value {
            let execution = compact_record(&records, "execution");
            assert_eq!(compact_field(execution, "value"), expected);
            assert_eq!(compact_field(execution, "differential"), "equal");
        }
        if arguments == ["check"] {
            let tests = compact_record(&records, "tests");
            assert_eq!(compact_field(tests, "passed"), "1");
            assert_eq!(compact_field(tests, "failed"), "0");
            assert_eq!(compact_field(tests, "differential"), "equal");
        }
        if arguments.starts_with(&["change", "plan"]) {
            assert!(!host.success);
            let diagnostic = compact_record(&records, "diagnostic");
            let stale_declarations = arguments.last() == Some(&path(&source));
            let is_stale_authored = arguments.last() == Some(&path(&stale_authored));
            assert_eq!(
                compact_field(diagnostic, "class"),
                if stale_declarations {
                    "source"
                } else {
                    "semantic"
                }
            );
            assert_eq!(
                compact_field(diagnostic, "code"),
                if is_stale_authored {
                    "change_authored_stale_base"
                } else if stale_declarations {
                    "change_unit_base"
                } else {
                    "kernel_type_root"
                }
            );
            if is_stale_authored {
                assert_eq!(host.code, Some(7), "public stale-meaning protocol status");
            }
            if stale_declarations {
                assert_eq!(
                    host.code,
                    Some(2),
                    "native declaration source refusal status"
                );
            }
            assert!(records.iter().all(|record| !matches!(
                record.operation.as_str(),
                "plan" | "revision" | "execution"
            )));
        }
        assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn host_foreground_and_batches_join_inside_a_runtime_and_recover_without_sources() {
    // The host adapter must also work when its synchronous caller already owns a runtime.
    for public in [
        Native::template_in("minimal", Mode::HostSemantic),
        Native::memory_project_in("minimal", Mode::HostSemantic),
    ] {
        let source = public.input("host-routes.lkjc", &format!(r#"request base={}
declarations.begin
(units
  (module create host-routes
    (external create add (visibility private) (implementation core.i64.add) (returns I64)
      (parameter create left (type I64)) (parameter create right (type I64)))
    (function create plus-forty-two (visibility private)
      (parameter create value (type I64)) (returns I64) (effect pure)
      (body (call add (local value) (i64 42))))
    (function create echo (visibility private)
      (parameter create value (type Text)) (returns Text) (effect pure) (body (local value)))
    (component create command (visibility private)
      (port create main (type (function (I64) I64)) (function plus-forty-two))
      (port create echo (type (function (Text) Text)) (function echo))))
  (target create main (component host-routes::command) (runner command) (port host-routes::command::main))
  (target create echo (component host-routes::command) (runner command) (port host-routes::command::echo)))
declarations.end
"#, public.revision()));
        public.apply(&source, &public.plan(&source, true), true);
        let artifact = public.root.path().join("host-routes.lkja");
        public.cli(&["build", "--output", path(&artifact)], true);
        let deployment = public.input("host-routes.deployment.json", &serde_json::json!({
        "artifact":"host-routes.lkja", "target":"main", "listen":null,
        "http":null,"session":null,"worker":null,
        "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,"maximum_total_bytes":1048576,"maximum_live_streams":1024},
        "grants":[],"secrets":[],"configuration":{}
    }).to_string());
        let head = public.head_bytes();
        let large = "猫\"\n\\".repeat(20_000);
        let rows = || {
            let arguments = serde_json::json!([7]);
            let mut fuel = BatchCase::failure(
                "fuel",
                &arguments,
                "resource",
                "normalized_instruction_steps",
            );
            fuel.execution = Some(serde_json::json!({
                "instruction_fuel":1,"maximum_call_depth":4096,"maximum_value_stack":1000000
            }));
            let mut deadline =
                BatchCase::failure("deadline", &arguments, "cancelled", "execution_deadline");
            deadline.deadline_milliseconds = Some(0);
            let mut echo = BatchCase::success(
                "large-target-override",
                &serde_json::json!([large]),
                serde_json::json!(large),
            );
            echo.target = Some("echo".to_owned());
            vec![
                BatchCase::success("first", &arguments, serde_json::json!(49)),
                BatchCase::failure(
                    "wrong-type",
                    &serde_json::json!([true]),
                    "source",
                    "normalized_json_type",
                ),
                fuel,
                BatchCase::success("after-fuel", &arguments, serde_json::json!(49)),
                deadline,
                BatchCase::success("after-deadline", &arguments, serde_json::json!(49)),
                echo,
            ]
        };
        let source_rows = public.source_batch("host-routes", "main", &rows());
        assert_eq!(source_rows.len(), 7);
        assert_eq!(public.head_bytes(), head);
        assert!(
            !public
                .root
                .path()
                .join("source-batch-host-routes.witness.lkjp")
                .exists()
        );
        let source_command: Value = serde_json::from_slice(
            &std::fs::read(
                public
                    .root
                    .path()
                    .join("batch-source-host-routes.command.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(source_command["execution_mode"], "host-semantic");
        assert_eq!(
            source_command["adapter"],
            if public.memory.read().unwrap().is_some() {
                "public-memory"
            } else {
                "public-cli"
            }
        );
        assert!(source_command.get("executable").is_none());
        assert!(source_command.get("environment_cleared").is_none());

        // Only copied current artifact/deployment bytes remain available to detached execution.
        public.remove_project();
        std::fs::remove_file(source).unwrap();
        let result = public.root.path().join("host-foreground.result.json");
        let healthy = public.cli(
            &[
                "run",
                "--deployment",
                path(&deployment),
                "--arguments",
                "[7]",
                "--result-file",
                path(&result),
            ],
            true,
        );
        assert_eq!(std::fs::read(&result).unwrap(), b"49");
        let execution = compact_record(&healthy, "execution");
        assert_eq!(compact_field(execution, "target"), "main");
        let refused_result = public.root.path().join("host-refused.result.json");
        let refused = public.cli(
            &[
                "run",
                "--deployment",
                path(&deployment),
                "--arguments",
                "[true]",
                "--result-file",
                path(&refused_result),
            ],
            false,
        );
        assert_eq!(
            compact_field(compact_record(&refused, "diagnostic"), "code"),
            "normalized_json_type"
        );
        assert!(!refused_result.exists());
        let detached = public.batch("host-routes", &deployment, &rows());
        assert_eq!(detached.len(), 7);
        let terminal: Value = serde_json::from_slice(
            &std::fs::read(public.root.path().join("batch-host-routes.terminal.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(terminal["execution_mode"], "host-semantic");
        assert_eq!(terminal["adapter_status"], 0);
        assert!(terminal.get("status").is_none());

        // The executable-only entry remains a real, bounded process control even in a host test.
        batch::execute(
            public.root.path(),
            &public.executable,
            "explicit-process",
            &deployment,
            &[BatchCase::success(
                "selected-process",
                &serde_json::json!([8]),
                serde_json::json!(50),
            )],
        );
        let command: Value = serde_json::from_slice(
            &std::fs::read(
                public
                    .root
                    .path()
                    .join("batch-explicit-process.command.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(command["execution_mode"], "executable-batch");
        assert_eq!(command["environment_cleared"], true);
        assert_eq!(command["executable"], path(&public.executable));
        tokio::task::yield_now().await;
    }
}
