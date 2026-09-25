//! Native first-party tool adoption, regeneration and binary-only authoring.
use super::*;
use serde_json::json;

const SOURCE: &str = include_str!("../../tools/native-guides/requests/create.lkjc");

struct GuideTool {
    root: tempfile::TempDir,
    executable: PathBuf,
    project: PathBuf,
}

impl GuideTool {
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

    fn write(&self, filename: &str, bytes: impl AsRef<[u8]>) -> PathBuf {
        let path = self.root.path().join(filename);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn plan(&self, input: &Path, name: &str) -> Vec<CompactRecord> {
        self.project(&[
            "change",
            "plan",
            "--input-file",
            path(input),
            "--output",
            path(&self.root.path().join(name)),
        ])
    }

    fn apply(&self, input: &Path, plan: &[CompactRecord]) {
        self.project(&[
            "change",
            "apply",
            "--input-file",
            path(input),
            "--plan",
            compact_field(compact_record(plan, "plan"), "token").unwrap(),
        ]);
    }

    fn result(&self, target: &str, input: &Value, name: &str) -> String {
        let input = self.write(
            &format!("{name}.input.json"),
            serde_json::to_vec(input).unwrap(),
        );
        let output = self.root.path().join(format!("{name}.result.json"));
        self.project(&[
            "run",
            target,
            "--arguments-file",
            path(&input),
            "--result-file",
            path(&output),
        ]);
        serde_json::from_slice(&std::fs::read(output).unwrap()).unwrap()
    }
}

fn diagnostic_input() -> Value {
    json!([
        {"product":"demo", "version":"1", "capabilities":"abc"},
        [{"code":"literal|`<x>", "class":"source", "meaning":"日本語🌱 &", "retry":"never"}],
        [{"status":7, "meaning":"stop"}]
    ])
}

#[test]
fn maintained_native_guides_regenerate_the_exact_embedded_artifact() {
    let tool = GuideTool::new();
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    copy_regular_tree(
        &repository.join("tools/native-guides/project"),
        &tool.project,
    );
    // Only disposable copied caches are removed; canonical project authority is preserved.
    for cache in ["derived", "catalog"] {
        let cache = tool.project.join(cache);
        if cache.exists() {
            std::fs::remove_dir_all(cache).unwrap();
        }
    }
    let head = std::fs::read(tool.project.join("HEAD")).unwrap();
    let tests = tool.project(&["check"]);
    assert_eq!(
        compact_field(compact_record(&tests, "tests"), "passed"),
        // The maintained graph retains 63 standard tests plus 16 guide tests.
        Some("79")
    );
    assert_eq!(
        compact_field(compact_record(&tests, "tests"), "differential"),
        Some("equal")
    );
    let artifact = tool.root.path().join("rebuilt.lkja");
    tool.project(&["build", "--output", path(&artifact)]);
    assert_eq!(
        std::fs::read(&artifact).unwrap(),
        std::fs::read(repository.join("tools/native-guides/generated/guides.lkja")).unwrap()
    );
    assert_eq!(std::fs::read(tool.project.join("HEAD")).unwrap(), head);
    let generated = tool.root.path().join("generated");
    tool.cli(&["capabilities", "--generate-docs", path(&generated)]);
    tool.cli(&["capabilities", "--verify-generated", path(&generated)]);
    for name in [
        "operations.md",
        "diagnostics.md",
        "change-grammar.md",
        "function-definition.md",
        "deployment.md",
        "builtin-standard.md",
        "stateful-http-authoring.md",
        "nostr-relay-info-authoring.md",
    ] {
        assert_eq!(
            std::fs::read(generated.join(name)).unwrap(),
            std::fs::read(repository.join("docs/generated").join(name)).unwrap()
        );
    }
}

#[test]
fn native_guides_author_edit_and_run_without_compiler_checkout_or_host_tools() {
    let tool = GuideTool::new();
    tool.cli(&[
        "new",
        path(&tool.project),
        "--template",
        "minimal",
        "--name",
        "native-guide",
    ]);
    let transport = tool.root.path().join("standard.lkjp");
    let exported = tool.cli(&[
        "package",
        "builtin",
        "export",
        "--kind",
        "transport",
        "--output",
        path(&transport),
    ]);
    let standard = compact_record(&exported, "package");
    tool.project(&[
        "package",
        "dependency",
        "stage",
        "--transport",
        compact_field(standard, "transport").unwrap(),
        "--input-file",
        path(&transport),
    ]);
    let status = tool.project(&["status"]);
    let revision = compact_field(compact_record(&status, "revision"), "id").unwrap();
    let request = tool.write(
        "create.lkjc",
        SOURCE
            .replacen("base=BASE", &format!("base={revision}"), 1)
            // Fresh authorship chooses the newly exported exact supplier. The
            // maintained graph above keeps its own earlier exact standard.
            .replacen(
                SOURCE
                    .lines()
                    .find(|line| line.starts_with("add.dependency "))
                    .unwrap(),
                &format!(
                    "add.dependency package={} semantic-revision={} package-revision={}",
                    compact_field(standard, "id").unwrap(),
                    compact_field(standard, "revision").unwrap(),
                    compact_field(standard, "package-revision").unwrap(),
                ),
                1,
            ),
    );
    let plan = tool.plan(&request, "create.logical-plan");
    tool.apply(&request, &plan);
    tool.add_reference_pages();
    let check = tool.project(&["check"]);
    assert_eq!(
        compact_field(compact_record(&check, "tests"), "passed"),
        Some("91")
    );
    let original = tool.result("diagnostics", &diagnostic_input(), "original");
    let expected = concat!(
        "<!-- Generated by `lkjscript capabilities --generate-docs docs/generated`. ",
        "Do not edit. demo 1 capabilities `abc`. -->\n\n",
        "# Protocol diagnostics and exit statuses\n\n",
        "<table>\n<thead>\n<tr><th>Code</th><th>Class</th><th>Stable meaning</th><th>Retry guidance</th></tr>\n",
        "</thead>\n<tbody>\n<tr><td><code>literal|`&lt;x&gt;</code></td><td><code>source</code></td>",
        "<td>日本語🌱 &amp;</td><td>never</td></tr>\n</tbody>\n</table>\n\n",
        "<table>\n<thead>\n<tr><th>Exit</th><th>Meaning</th></tr>\n</thead>\n<tbody>\n",
        "<tr><td>7</td><td>stop</td></tr>\n</tbody>\n</table>\n",
    );
    assert_eq!(original, expected);
    let old_artifact = tool.root.path().join("old.lkja");
    tool.project(&["build", "--output", path(&old_artifact)]);
    std::fs::remove_file(request).unwrap();
    let owners = tool.project(&["query", "find", "module", "guides"]);
    let owner = compact_field(compact_record(&owners, "owner"), "id").unwrap();
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
    let source = std::fs::read_to_string(&draft).unwrap();
    assert!(source.contains("# Protocol diagnostics and exit statuses"));
    std::fs::write(
        &draft,
        source.replace(
            "# Protocol diagnostics and exit statuses",
            "# Edited native guide",
        ),
    )
    .unwrap();
    let edited = tool.plan(&draft, "edit.logical-plan");
    tool.apply(&draft, &edited);
    assert_eq!(
        tool.result("diagnostics", &diagnostic_input(), "edited"),
        expected.replace(
            "# Protocol diagnostics and exit statuses",
            "# Edited native guide"
        )
    );
    let new_artifact = tool.root.path().join("new.lkja");
    tool.project(&["build", "--output", path(&new_artifact)]);
    std::fs::remove_dir_all(&tool.project).unwrap();
    std::fs::remove_file(&draft).unwrap();
    std::fs::remove_file(&transport).unwrap();
    tool.verify_detached_reference("old");
    tool.verify_detached_reference("new");
    for (name, expected) in [
        ("old", original),
        (
            "new",
            expected.replace(
                "# Protocol diagnostics and exit statuses",
                "# Edited native guide",
            ),
        ),
    ] {
        let descriptor = tool.write(&format!("{name}.deployment.json"), serde_json::to_vec(&json!({
            "artifact": format!("{name}.lkja"), "target":"diagnostics", "listen":null,
            "http":null, "session":null, "worker":null,
            "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,"maximum_total_bytes":1048576,"maximum_live_streams":1024},
            "grants":[], "secrets":[], "configuration":{}
        })).unwrap());
        let output = tool.root.path().join(format!("{name}.detached.json"));
        let input = tool.write(
            &format!("{name}.detached-input.json"),
            serde_json::to_vec(&diagnostic_input()).unwrap(),
        );
        tool.cli(&[
            "run",
            "--deployment",
            path(&descriptor),
            "--arguments-file",
            path(&input),
            "--result-file",
            path(&output),
        ]);
        assert_eq!(
            serde_json::from_slice::<String>(&std::fs::read(output).unwrap()).unwrap(),
            expected
        );
    }
}

impl GuideTool {
    fn add_reference_pages(&self) {
        let status = self.project(&["status"]);
        let revision = compact_field(compact_record(&status, "revision"), "id").unwrap();
        let owners = self.project(&["query", "find", "module", "guides"]);
        let owner = compact_field(compact_record(&owners, "owner"), "id").unwrap();
        let source = include_str!("../../tools/native-guides/requests/reference-pages.lkjc");
        let input = self.write(
            "reference-pages.lkjc",
            source
                .replacen("base=BASE", &format!("base={revision}"), 1)
                .replacen("module edit GUIDES", &format!("module edit {owner}"), 1),
        );
        let plan = self.plan(&input, "reference-pages.logical-plan");
        self.apply(&input, &plan);
        assert_eq!(
            self.result("function-definition", &reference_input(), "reference-page"),
            reference_expected()
        );
        std::fs::remove_file(input).unwrap();
    }

    fn verify_detached_reference(&self, name: &str) {
        assert!(!self.project.exists());
        let descriptor = self.write(
            &format!("{name}.reference.deployment.json"),
            serde_json::to_vec(&json!({
                "artifact":format!("{name}.lkja"), "target":"function-definition", "listen":null,
                "http":null, "session":null, "worker":null,
                "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,
                    "maximum_total_bytes":1048576,"maximum_live_streams":1024},
                "grants":[], "secrets":[], "configuration":{}
            }))
            .unwrap(),
        );
        let input = self.write(
            &format!("{name}.reference-input.json"),
            serde_json::to_vec(&reference_input()).unwrap(),
        );
        let output = self
            .root
            .path()
            .join(format!("{name}.reference-result.json"));
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
            serde_json::from_slice::<String>(&std::fs::read(output).unwrap()).unwrap(),
            reference_expected()
        );
    }
}

fn reference_input() -> Value {
    json!([{"product":"demo", "version":"1", "capabilities":"abc"}, "literal ```<x>&日本語\n"])
}

fn reference_expected() -> &'static str {
    concat!(
        "<!-- Generated by `lkjscript capabilities --generate-docs docs/generated`. ",
        "Do not edit. demo 1 capabilities `abc`. -->\n\n",
        "# Revision-pinned function definition projection\n\n",
        "The following compact records are the executable-owned current public capability.\n\n",
        "<pre>literal ```&lt;x&gt;&amp;日本語\n</pre>\n"
    )
}
