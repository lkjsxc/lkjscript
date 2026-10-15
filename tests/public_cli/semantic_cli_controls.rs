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
        vec!["run", "--deployment", "/absolute.json"],
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
fn an_undeliverable_refusal_cannot_be_reported_as_success() {
    use lkjscript::platform::contract::exit_status_for;
    use lkjscript::platform::{Diagnostic, DiagnosticClass};
    let diagnostic = Diagnostic::new(
        DiagnosticClass::Semantic,
        "oversized_test_diagnostic",
        "x".repeat(MAXIMUM_CLI_RESPONSE_BYTES + 1),
    );
    for diagnostics in [vec![diagnostic], Vec::new()] {
        let output = semantic_cli::failure("change", &diagnostics);
        assert!(!output.success);
        assert_eq!(
            output.code,
            Some(i32::from(exit_status_for(DiagnosticClass::Infrastructure)))
        );
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}

#[test]
fn host_and_process_adapters_agree_on_results_and_prepublication_refusals() {
    let public = Native::process_template("minimal");
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
"#, public.revision()));
    public.apply(&source, &public.plan(&source, true), true);
    public.cli(&["check"], true);
    let head = std::fs::read(public.project.join("HEAD")).unwrap();
    let refused = public.input("refused.lkjc", &format!("request base={}\ndeclarations.begin\n(units (module create bad (function create wrong (visibility private) (returns I64) (effect pure) (body (bool true)))))\ndeclarations.end\n", public.revision()));
    for (arguments, expected_value) in [
        (vec!["status"], None),
        (vec!["check"], None),
        (vec!["run", "main", "--arguments", "[0]"], Some("42")),
        (vec!["run", "main", "--arguments", "[-7]"], Some("35")),
        (vec!["run", "main", "--arguments", "[8]"], Some("50")),
        (vec!["query", "find", "module", "missing"], None),
        (vec!["check", "--unsupported"], None),
        (vec!["change", "plan", "--input-file", path(&refused)], None),
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
            assert_eq!(compact_field(diagnostic, "class"), "semantic");
            assert_eq!(compact_field(diagnostic, "code"), "kernel_type_root");
            assert!(records.iter().all(|record| !matches!(
                record.operation.as_str(),
                "plan" | "revision" | "execution"
            )));
        }
        assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
    }
}
