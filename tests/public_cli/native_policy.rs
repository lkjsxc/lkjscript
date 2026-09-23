//! Public authorship, native-tool embedding and maintained policy asset reproduction.
use super::*;
use lkjscript::platform::{ExecutionControl, JsonLimits, native_tool::PureTool};
use serde_json::json;

const SOURCE: &str = include_str!("../../tools/native-policy/requests/create-bytes.lkjc");

struct PolicyTool {
    root: tempfile::TempDir,
    executable: PathBuf,
    project: PathBuf,
}

impl PolicyTool {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("lkjscript");
        copy_executable(&binary(), &executable);
        let project = root.path().join("project");
        Self {
            root,
            executable,
            project,
        }
    }

    fn cli(&self, arguments: &[&str]) -> Vec<CompactRecord> {
        compact_success_output(
            support::output(
                Command::new(&self.executable)
                    .args(arguments)
                    .current_dir(self.root.path())
                    .env_clear()
                    .env("PATH", ""),
            )
            .unwrap(),
        )
    }

    fn project(&self, arguments: &[&str]) -> Vec<CompactRecord> {
        let mut args = vec!["--project", path(&self.project)];
        args.extend_from_slice(arguments);
        self.cli(&args)
    }

    fn write(&self, name: &str, bytes: impl AsRef<[u8]>) -> PathBuf {
        let file = self.root.path().join(name);
        std::fs::write(&file, bytes).unwrap();
        file
    }

    fn accept(&self, input: &Path, plan_name: &str) {
        let plan = self.project(&[
            "change",
            "plan",
            "--input-file",
            path(input),
            "--output",
            path(&self.root.path().join(plan_name)),
        ]);
        self.project(&[
            "change",
            "apply",
            "--input-file",
            path(input),
            "--plan",
            compact_field(compact_record(&plan, "plan"), "token").unwrap(),
        ]);
    }

    fn author(&self) -> PathBuf {
        self.cli(&[
            "new",
            path(&self.project),
            "--template",
            "minimal",
            "--name",
            "native-policy",
        ]);
        let transport = self.root.path().join("standard.lkjp");
        let records = self.cli(&[
            "package",
            "builtin",
            "export",
            "--kind",
            "transport",
            "--output",
            path(&transport),
        ]);
        let transport_id = records
            .iter()
            .flat_map(|record| &record.fields)
            .map(|field| field.value.as_str())
            .find(|value| value.starts_with("package_transport_"))
            .unwrap();
        self.project(&[
            "package",
            "dependency",
            "stage",
            "--transport",
            transport_id,
            "--input-file",
            path(&transport),
        ]);
        let status = self.project(&["status"]);
        let revision = compact_field(compact_record(&status, "revision"), "id").unwrap();
        let input = self.write(
            "create.lkjc",
            SOURCE.replacen("base=BASE", &format!("base={revision}"), 1),
        );
        self.accept(&input, "create.logical-plan");
        input
    }

    fn check(&self) {
        let records = self.project(&["check"]);
        assert_eq!(
            compact_field(compact_record(&records, "tests"), "passed"),
            Some("62")
        );
        assert_eq!(
            compact_field(compact_record(&records, "tests"), "differential"),
            Some("equal")
        );
    }

    fn build(&self, name: &str) -> PathBuf {
        let artifact = self.root.path().join(format!("{name}.lkja"));
        self.project(&["build", "--output", path(&artifact)]);
        artifact
    }

    fn detached(&self, name: &str, expected: &[bool]) {
        let descriptor = self.write(&format!("{name}.deployment.json"), serde_json::to_vec(&json!({
            "artifact":format!("{name}.lkja"), "target":"extensions", "listen":null,
            "http":null, "session":null, "worker":null,
            "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,"maximum_total_bytes":1048576,"maximum_live_streams":1024},
            "grants":[], "secrets":[], "configuration":{}
        })).unwrap());
        let input = self.write(&format!("{name}.input.json"), br#"[["py","rb"]]"#);
        let output = self.root.path().join(format!("{name}.result.json"));
        self.cli(&[
            "run",
            "--deployment",
            path(&descriptor),
            "--arguments-file",
            path(&input),
            "--result-file",
            path(&output),
        ]);
        assert_eq!(
            serde_json::from_slice::<Vec<bool>>(&std::fs::read(output).unwrap()).unwrap(),
            expected
        );
    }
}

#[test]
fn maintained_native_policy_rebuilds_exactly_from_copied_accepted_meaning() {
    let tool = PolicyTool::new();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    copy_regular_tree(
        &repository.join("tools/native-policy/project"),
        &tool.project,
    );
    for cache in ["derived", "catalog"] {
        let cache = tool.project.join(cache);
        if cache.exists() {
            std::fs::remove_dir_all(cache).unwrap();
        }
    }
    let head = std::fs::read(tool.project.join("HEAD")).unwrap();
    tool.check();
    assert_eq!(
        std::fs::read(tool.build("rebuilt")).unwrap(),
        std::fs::read(repository.join("tools/native-policy/generated/policy.lkja")).unwrap()
    );
    assert_eq!(std::fs::read(tool.project.join("HEAD")).unwrap(), head);
}

#[test]
fn native_policy_authors_edits_and_runs_detached_without_host_tools() {
    let tool = PolicyTool::new();
    let input = tool.author();
    tool.check();
    let old_artifact = tool.build("old");
    let bytes = std::fs::read(&old_artifact).unwrap();
    let control = ExecutionControl::uncancelled();
    let embedded = PureTool::load(&bytes, &control).unwrap();
    let limits = JsonLimits::default();
    assert_eq!(
        embedded
            .run_json("extensions", br#"[["py","rb"]]"#, limits, &control)
            .unwrap(),
        b"[true,false]"
    );
    for (target, input) in [
        ("absent", b"[]".as_slice()),
        ("extensions", b"[[true]]".as_slice()),
        ("extensions", b"[[\"py\"]] trailing".as_slice()),
    ] {
        assert!(embedded.run_json(target, input, limits, &control).is_err());
    }
    let cancelled = ExecutionControl::uncancelled();
    cancelled.cancel();
    assert_eq!(
        embedded
            .run_json("extensions", b"[[]]", limits, &cancelled)
            .unwrap_err()
            .class,
        lkjscript::platform::DiagnosticClass::Cancelled
    );
    assert!(PureTool::load(&bytes, &cancelled).is_err());
    assert!(PureTool::load(&bytes[..bytes.len() - 1], &control).is_err());
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(PureTool::load(&trailing, &control).is_err());
    // Three empty text inputs fit, but three false results exceed that byte bound.
    let bounded_input = br#"[["","",""]]"#;
    let tiny = JsonLimits {
        maximum_bytes: bounded_input.len(),
        ..limits
    };
    assert_eq!(
        embedded
            .run_json("extensions", bounded_input, tiny, &control)
            .unwrap_err()
            .code,
        "normalized_json_output_bytes"
    );
    assert_eq!(
        embedded
            .run_json("extensions", br#"[["py","rb"]]"#, limits, &control)
            .unwrap(),
        b"[true,false]"
    );

    let records = tool.project(&["query", "find", "module", "policy"]);
    let owner = compact_field(compact_record(&records, "owner"), "id").unwrap();
    let draft = tool.root.path().join("edit.lkjc");
    tool.project(&[
        "change",
        "draft",
        "--owner",
        owner,
        "--output",
        path(&draft),
    ]);
    let unchanged = tool.project(&["change", "plan", "--input-file", path(&draft)]);
    assert_eq!(
        compact_field(compact_record(&unchanged, "result"), "outcome"),
        Some("unchanged")
    );
    let original = std::fs::read_to_string(&draft).unwrap();
    assert!(original.contains("(text \"py\")"));
    std::fs::write(&draft, original.replace("(text \"py\")", "(text \"rb\")")).unwrap();
    tool.accept(&draft, "edit.logical-plan");
    tool.check();
    tool.build("new");
    std::fs::remove_dir_all(&tool.project).unwrap();
    for path in [input, draft, tool.root.path().join("standard.lkjp")] {
        std::fs::remove_file(path).unwrap();
    }
    tool.detached("old", &[true, false]);
    tool.detached("new", &[false, true]);
    // Reusing admitted code does not follow edits to a different authoring project.
    assert_eq!(
        embedded
            .run_json("extensions", br#"[["py","rb"]]"#, limits, &control)
            .unwrap(),
        b"[true,false]"
    );
}

#[test]
fn reusable_pure_embedding_rejects_task_identity_and_unused_component_grants() {
    let tool = PolicyTool::new();
    tool.cli(&["new", path(&tool.project), "--template", "command"]);
    let records = tool.project(&["status"]);
    let revision = compact_field(compact_record(&records, "revision"), "id").unwrap();
    let input = tool.write("effect-boundary.lkjc", format!(r#"request base={revision}
declarations.begin
(units
  (use std builtin)
  (module create boundary
    (function create task-trap (visibility private) (returns I64) (effect (task))
      (body (call std::divide (i64 1) (i64 0))))
    (function create pure-trap (visibility private) (returns I64) (effect pure)
      (body (call std::divide (i64 1) (i64 0))))
    (component create tasks (visibility private)
      (port create invoke (type (task-function () I64 (row))) (function task-trap)))
    (component create grants (visibility private)
      (requirement create data (interface std::DataStore)
        (operations std::DataStore::get) (limits (maximum_calls 1 calls)))
      (port create invoke (type (function () I64)) (function pure-trap))))
  (target create task (component boundary::tasks) (runner command) (port boundary::tasks::invoke))
  (target create granted (component boundary::grants) (runner command) (port boundary::grants::invoke)))
declarations.end
"#));
    tool.accept(&input, "effect.logical-plan");
    let bytes = std::fs::read(tool.build("effect-boundary")).unwrap();
    std::fs::remove_dir_all(&tool.project).unwrap();
    let control = ExecutionControl::uncancelled();
    let embedded = PureTool::load(&bytes, &control).unwrap();
    for target in ["task", "granted"] {
        let error = embedded
            .run_json(target, b"[]", JsonLimits::default(), &control)
            .unwrap_err();
        assert_eq!(
            error.class,
            lkjscript::platform::DiagnosticClass::Capability
        );
        assert_eq!(error.code, "normalized_runner_grants_required");
    }
    assert_eq!(
        embedded
            .run_json("main", b"[]", JsonLimits::default(), &control)
            .unwrap(),
        br#""hello""#
    );
}
