#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const EXIT_DOMAIN_CONFLICT: i32 = 10;

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_lkjwork"))
}

fn invoke(binary: &Path, arguments: &[&str]) -> Output {
    Command::new(binary)
        .args(arguments)
        .output()
        .expect("run lkjwork")
}

fn machine(binary: &Path, arguments: &[&str], expected_exit: i32) -> Value {
    let output = invoke(binary, arguments);
    assert_eq!(
        output.status.code(),
        Some(expected_exit),
        "lkjwork exit mismatch for {arguments:?}\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert!(
        output.stderr.is_empty(),
        "machine operation contaminated stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).expect("machine output is UTF-8");
    assert_eq!(
        text.lines().count(),
        1,
        "one-shot output is exactly one frame"
    );
    serde_json::from_str(text.trim_end()).expect("machine output is JSON")
}

fn project_machine(binary: &Path, project: &Path, arguments: &[&str], expected_exit: i32) -> Value {
    let project = project.to_str().expect("project path is UTF-8");
    let mut complete = vec!["--json", "--project", project];
    complete.extend_from_slice(arguments);
    machine(binary, &complete, expected_exit)
}

fn revision(value: &Value) -> u64 {
    value["revision"].as_u64().expect("response revision")
}

fn result_value(value: &Value) -> &Value {
    &value["result"]["value"]
}

fn assert_publication(
    value: &Value,
    expected: &str,
    published: bool,
    previous_revision: u64,
) -> u64 {
    assert_eq!(value["result"]["publication"], expected);
    assert_eq!(value["result"]["published"], published);
    let selected = revision(value);
    assert_eq!(selected, previous_revision + u64::from(published));
    selected
}

fn snapshot_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, directory: &Path, output: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let mut entries = fs::read_dir(directory)
            .expect("read authority tree")
            .map(|entry| entry.expect("read authority entry"))
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path).expect("authority metadata");
            assert!(
                !metadata.file_type().is_symlink(),
                "authority contains a symlink"
            );
            if metadata.is_dir() {
                visit(root, &path, output);
            } else {
                output.insert(
                    path.strip_prefix(root)
                        .expect("relative authority path")
                        .to_owned(),
                    fs::read(path).expect("read authority file"),
                );
            }
        }
    }

    let mut output = BTreeMap::new();
    visit(root, root, &mut output);
    output
}

fn instance_authority(project: &Path) -> PathBuf {
    fs::read_dir(project.join(".lkjwork/instance-store"))
        .expect("instance store")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            entry.file_type().ok()?.is_dir().then_some(entry.path())
        })
        .next()
        .expect("instance authority")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    Planned,
    Active,
    Done,
}

#[derive(Clone, Debug)]
struct ModelTask {
    id: u64,
    title: String,
    priority: i64,
    phase: Phase,
    hold: bool,
    archived: bool,
    dependencies: BTreeSet<u64>,
    labels: BTreeSet<String>,
}

#[derive(Default)]
struct ReferenceModel {
    next_id: u64,
    tasks: BTreeMap<u64, ModelTask>,
}

impl ReferenceModel {
    fn create(&mut self, title: &str, priority: i64) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        self.tasks.insert(
            id,
            ModelTask {
                id,
                title: title.to_owned(),
                priority,
                phase: Phase::Planned,
                hold: false,
                archived: false,
                dependencies: BTreeSet::new(),
                labels: BTreeSet::new(),
            },
        );
        id
    }

    fn depends(&mut self, task: u64, prerequisite: u64) -> bool {
        if task == prerequisite
            || !self.tasks.contains_key(&task)
            || !self.tasks.contains_key(&prerequisite)
            || self.reachable(prerequisite, task)
        {
            return false;
        }
        self.tasks
            .get_mut(&task)
            .expect("model task")
            .dependencies
            .insert(prerequisite)
    }

    fn reachable(&self, start: u64, target: u64) -> bool {
        let mut pending = vec![start];
        let mut visited = BTreeSet::new();
        while let Some(next) = pending.pop() {
            if next == target {
                return true;
            }
            if visited.insert(next) {
                pending.extend(
                    self.tasks
                        .get(&next)
                        .into_iter()
                        .flat_map(|task| task.dependencies.iter().copied()),
                );
            }
        }
        false
    }

    fn ready(&self, id: u64) -> bool {
        let task = &self.tasks[&id];
        task.phase == Phase::Planned
            && !task.hold
            && !task.archived
            && task
                .dependencies
                .iter()
                .all(|dependency| self.tasks[dependency].phase == Phase::Done)
    }

    fn start(&mut self, id: u64) -> bool {
        if !self.ready(id) {
            return false;
        }
        self.tasks.get_mut(&id).expect("model task").phase = Phase::Active;
        true
    }

    fn finish(&mut self, id: u64) -> bool {
        let eligible = {
            let task = &self.tasks[&id];
            task.phase == Phase::Active
                && !task.hold
                && task
                    .dependencies
                    .iter()
                    .all(|dependency| self.tasks[dependency].phase == Phase::Done)
        };
        if eligible {
            self.tasks.get_mut(&id).expect("model task").phase = Phase::Done;
        }
        eligible
    }

    fn next(&self) -> Vec<u64> {
        let mut ready = self
            .tasks
            .values()
            .filter(|task| self.ready(task.id))
            .collect::<Vec<_>>();
        ready.sort_by_key(|task| (std::cmp::Reverse(task.priority), task.id));
        ready.into_iter().map(|task| task.id).collect()
    }
}

#[test]
fn complete_mutation_vocabulary_has_exact_publication_and_lifecycle_behavior() {
    let temporary = tempfile::tempdir().expect("temporary product directory");
    let project = temporary.path().join("vocabulary");
    let binary = binary();
    let initialized = machine(
        &binary,
        &[
            "--json",
            "init",
            project.to_str().expect("project path"),
            "--name",
            "vocabulary",
        ],
        0,
    );
    let mut current = revision(&initialized);

    for title in ["first", "second"] {
        let response = project_machine(&binary, &project, &["add", title], 0);
        current = assert_publication(&response, "completed", true, current);
    }

    let renamed = project_machine(&binary, &project, &["rename", "renamed"], 0);
    current = assert_publication(&renamed, "completed", true, current);
    let same_rename = project_machine(&binary, &project, &["rename", "renamed"], 0);
    current = assert_publication(&same_rename, "unchanged", false, current);

    let edited = project_machine(
        &binary,
        &project,
        &[
            "edit",
            "#1",
            "--title",
            "first revised",
            "--description",
            "exact description",
            "--priority",
            "8",
        ],
        0,
    );
    current = assert_publication(&edited, "completed", true, current);
    let same_edit = project_machine(
        &binary,
        &project,
        &[
            "edit",
            "#1",
            "--title",
            "first revised",
            "--description",
            "exact description",
            "--priority",
            "8",
        ],
        0,
    );
    current = assert_publication(&same_edit, "unchanged", false, current);
    let same_priority = project_machine(&binary, &project, &["priority", "#1", "8"], 0);
    current = assert_publication(&same_priority, "unchanged", false, current);

    let label = project_machine(&binary, &project, &["label", "#1", "add", "runtime"], 0);
    current = assert_publication(&label, "completed", true, current);
    let duplicate_label = project_machine(&binary, &project, &["label", "#1", "add", "runtime"], 0);
    current = assert_publication(&duplicate_label, "unchanged", false, current);
    let removed_label =
        project_machine(&binary, &project, &["label", "#1", "remove", "runtime"], 0);
    current = assert_publication(&removed_label, "completed", true, current);
    let absent_label = project_machine(&binary, &project, &["label", "#1", "remove", "runtime"], 0);
    current = assert_publication(&absent_label, "unchanged", false, current);

    let dependency = project_machine(&binary, &project, &["depend", "#2", "--on", "#1"], 0);
    current = assert_publication(&dependency, "completed", true, current);
    let duplicate_dependency =
        project_machine(&binary, &project, &["depend", "#2", "--on", "#1"], 0);
    current = assert_publication(&duplicate_dependency, "unchanged", false, current);
    let removed_dependency =
        project_machine(&binary, &project, &["undepend", "#2", "--on", "#1"], 0);
    current = assert_publication(&removed_dependency, "completed", true, current);
    let absent_dependency =
        project_machine(&binary, &project, &["undepend", "#2", "--on", "#1"], 0);
    current = assert_publication(&absent_dependency, "unchanged", false, current);

    let held = project_machine(&binary, &project, &["hold", "#1", "--reason", "review"], 0);
    current = assert_publication(&held, "completed", true, current);
    let same_hold = project_machine(&binary, &project, &["hold", "#1", "--reason", "review"], 0);
    current = assert_publication(&same_hold, "unchanged", false, current);
    let released = project_machine(&binary, &project, &["release", "#1"], 0);
    current = assert_publication(&released, "completed", true, current);
    let absent_hold = project_machine(&binary, &project, &["release", "#1"], 0);
    current = assert_publication(&absent_hold, "unchanged", false, current);

    let started = project_machine(&binary, &project, &["start", "#1"], 0);
    current = assert_publication(&started, "completed", true, current);
    let stopped = project_machine(&binary, &project, &["stop", "#1"], 0);
    current = assert_publication(&stopped, "completed", true, current);
    let stopped_again = project_machine(&binary, &project, &["stop", "#1"], 0);
    current = assert_publication(&stopped_again, "unchanged", false, current);
    let cancelled = project_machine(&binary, &project, &["cancel", "#1"], 0);
    current = assert_publication(&cancelled, "completed", true, current);
    let reopened = project_machine(&binary, &project, &["reopen", "#1"], 0);
    current = assert_publication(&reopened, "completed", true, current);
    let started = project_machine(&binary, &project, &["start", "#1"], 0);
    current = assert_publication(&started, "completed", true, current);
    let finished = project_machine(&binary, &project, &["finish", "#1"], 0);
    current = assert_publication(&finished, "completed", true, current);
    let archived = project_machine(&binary, &project, &["archive", "#1"], 0);
    current = assert_publication(&archived, "completed", true, current);
    let archived_again = project_machine(&binary, &project, &["archive", "#1"], 0);
    current = assert_publication(&archived_again, "unchanged", false, current);
    let unarchived = project_machine(&binary, &project, &["unarchive", "#1"], 0);
    current = assert_publication(&unarchived, "completed", true, current);
    let unarchived_again = project_machine(&binary, &project, &["unarchive", "#1"], 0);
    current = assert_publication(&unarchived_again, "unchanged", false, current);

    let note = project_machine(
        &binary,
        &project,
        &["note", "#1", "add", "immutable note", "--actor", "oracle"],
        0,
    );
    current = assert_publication(&note, "completed", true, current);
    let missing = project_machine(&binary, &project, &["cancel", "#999"], 10);
    current = assert_publication(&missing, "declined", false, current);

    let shown = project_machine(&binary, &project, &["show", "#1"], 0);
    assert_eq!(revision(&shown), current);
    assert_eq!(result_value(&shown)["task"]["title"], "first revised");
    assert_eq!(
        result_value(&shown)["task"]["description"],
        "exact description"
    );
    assert_eq!(result_value(&shown)["task"]["phase"], "done");
    assert_eq!(result_value(&shown)["task"]["archived"], false);
    assert_eq!(
        result_value(&shown)["task"]["notes"]
            .as_array()
            .expect("notes")
            .len(),
        1
    );
}

#[test]
fn why_is_typed_human_machine_agreed_and_strictly_read_only() {
    let temporary = tempfile::tempdir().expect("temporary product directory");
    let project = temporary.path().join("why");
    let binary = binary();
    machine(
        &binary,
        &[
            "--json",
            "init",
            project.to_str().expect("project path"),
            "--name",
            "why",
        ],
        0,
    );
    project_machine(&binary, &project, &["add", "first"], 0);
    project_machine(&binary, &project, &["add", "second"], 0);
    project_machine(&binary, &project, &["depend", "#2", "--on", "#1"], 0);
    project_machine(&binary, &project, &["hold", "#2", "--reason", "waiting"], 0);

    let authority = project.join(".lkjwork");
    let before = snapshot_tree(&authority);
    let explanation = project_machine(&binary, &project, &["why", "#2"], 0);
    assert_eq!(explanation["operation"], "why");
    assert_eq!(explanation["result"]["published"], false);
    assert_eq!(result_value(&explanation)["kind"], "why");
    assert_eq!(result_value(&explanation)["task"], 2);
    assert_eq!(result_value(&explanation)["phase"], "planned");
    assert_eq!(result_value(&explanation)["archived"], false);
    assert_eq!(result_value(&explanation)["manual_hold"], "waiting");
    assert_eq!(result_value(&explanation)["actionable"], false);
    assert_eq!(
        result_value(&explanation)["blockers"],
        serde_json::json!([1])
    );

    let human = invoke(
        &binary,
        &[
            "--project",
            project.to_str().expect("project path"),
            "why",
            "#2",
        ],
    );
    assert!(human.status.success());
    let human = String::from_utf8(human.stdout).expect("human output");
    assert!(human.contains("Task #2: phase=planned"));
    assert!(human.contains("manual_hold=waiting"));
    assert!(human.contains("actionable=false"));
    assert!(human.contains("blockers=#1"));

    let missing = project_machine(&binary, &project, &["why", "#99"], EXIT_DOMAIN_CONFLICT);
    assert_eq!(result_value(&missing)["kind"], "not_found");
    assert_eq!(result_value(&missing)["task"], 99);
    assert_eq!(before, snapshot_tree(&authority));
}

#[test]
fn pagination_priority_and_context_omissions_match_the_complete_candidate_set() {
    let temporary = tempfile::tempdir().expect("temporary product directory");
    let project = temporary.path().join("pagination");
    let binary = binary();
    machine(
        &binary,
        &[
            "--json",
            "init",
            project.to_str().expect("project path"),
            "--name",
            "pagination",
        ],
        0,
    );
    let priorities = [4_i64, 12, -1, 12, 8, 3, 30, 7, 30, 0, 11, -5];
    for (index, priority) in priorities.iter().enumerate() {
        let id = index + 1;
        project_machine(
            &binary,
            &project,
            &[
                "add",
                &format!("task {id}"),
                "--priority",
                &priority.to_string(),
            ],
            0,
        );
        project_machine(
            &binary,
            &project,
            &[
                "note",
                &format!("#{id}"),
                "add",
                &format!("note {id}"),
                "--actor",
                "oracle",
            ],
            0,
        );
    }
    let mut expected = priorities
        .iter()
        .enumerate()
        .map(|(index, priority)| (index as u64 + 1, *priority))
        .collect::<Vec<_>>();
    expected.sort_by_key(|(id, priority)| (std::cmp::Reverse(*priority), *id));
    let expected_ids = expected.iter().map(|entry| entry.0).collect::<Vec<_>>();

    for (after, limit) in [(0, 5), (5, 5), (10, 5)] {
        let response = project_machine(
            &binary,
            &project,
            &[
                "list",
                "--after",
                &after.to_string(),
                "--limit",
                &limit.to_string(),
                "--order",
                "priority",
            ],
            0,
        );
        let value = result_value(&response);
        let observed = value["tasks"]
            .as_array()
            .expect("task page")
            .iter()
            .map(|task| task["id"].as_u64().expect("task ID"))
            .collect::<Vec<_>>();
        let end = usize::min(after + limit, expected_ids.len());
        assert_eq!(observed, expected_ids[after..end]);
        assert_eq!(value["total"], 12);
        assert_eq!(value["omitted"], 12 - observed.len() as u64);
        assert_eq!(value["next_after"], end as u64);
    }

    let context = project_machine(
        &binary,
        &project,
        &[
            "context",
            "--maximum-tasks",
            "3",
            "--maximum-notes",
            "100",
            "--maximum-dependencies",
            "100",
            "--maximum-text-bytes",
            "65536",
        ],
        0,
    );
    let context_value = result_value(&context);
    assert_eq!(context_value["total"], 12);
    assert_eq!(context_value["omitted"], 9);
    assert_eq!(context_value["notes_omitted"], 9);
    assert_eq!(context_value["dependencies_omitted"], 0);
    assert_eq!(context_value["text_truncated"], false);
    assert_eq!(
        context_value["tasks"]
            .as_array()
            .expect("context tasks")
            .iter()
            .map(|task| task["id"].as_u64().expect("task ID"))
            .collect::<Vec<_>>(),
        expected_ids[..3]
    );

    project_machine(&binary, &project, &["start", "#12"], 0);
    let active_first = project_machine(
        &binary,
        &project,
        &[
            "context",
            "--maximum-tasks",
            "3",
            "--maximum-notes",
            "100",
            "--maximum-dependencies",
            "100",
            "--maximum-text-bytes",
            "65536",
        ],
        0,
    );
    assert_eq!(
        result_value(&active_first)["tasks"][0]["id"],
        12,
        "active tasks must precede actionable tasks"
    );
}

#[test]
fn application_ordering_and_transitions_match_an_independent_reference_model() {
    let temporary = tempfile::tempdir().expect("temporary product directory");
    let project = temporary.path().join("model");
    let binary = binary();
    let mut model = ReferenceModel::default();

    machine(
        &binary,
        &[
            "--json",
            "init",
            project.to_str().expect("project path"),
            "--name",
            "model",
        ],
        0,
    );

    for (title, priority) in [("alpha", 5), ("beta", 10), ("gamma", 10), ("delta", -1)] {
        let id = model.create(title, priority);
        let response = project_machine(
            &binary,
            &project,
            &[
                "add",
                title,
                "--priority",
                &priority.to_string(),
                "--actor",
                "oracle",
            ],
            0,
        );
        assert_eq!(result_value(&response)["task"], id);
    }

    assert!(model.depends(2, 1));
    project_machine(&binary, &project, &["depend", "#2", "--on", "#1"], 0);
    assert!(model.depends(4, 2));
    project_machine(&binary, &project, &["depend", "#4", "--on", "#2"], 0);
    assert!(!model.depends(1, 4));
    let cycle = project_machine(
        &binary,
        &project,
        &["depend", "#1", "--on", "#4"],
        EXIT_DOMAIN_CONFLICT,
    );
    assert_eq!(cycle["result"]["published"], false);
    assert_eq!(result_value(&cycle)["code"], "dependency_cycle");

    let next = project_machine(&binary, &project, &["next", "--limit", "20"], 0);
    let observed = result_value(&next)["tasks"]
        .as_array()
        .expect("next tasks")
        .iter()
        .map(|task| task["id"].as_u64().expect("task ID"))
        .collect::<Vec<_>>();
    assert_eq!(observed, model.next());

    assert!(model.start(3));
    project_machine(&binary, &project, &["start", "#3"], 0);
    assert!(model.finish(3));
    project_machine(&binary, &project, &["finish", "#3"], 0);
    assert!(model.start(1));
    project_machine(&binary, &project, &["start", "#1"], 0);
    assert!(model.finish(1));
    project_machine(&binary, &project, &["finish", "#1"], 0);

    let next = project_machine(&binary, &project, &["next", "--limit", "20"], 0);
    let observed = result_value(&next)["tasks"]
        .as_array()
        .expect("next tasks")
        .iter()
        .map(|task| task["id"].as_u64().expect("task ID"))
        .collect::<Vec<_>>();
    assert_eq!(observed, model.next());

    let export = project_machine(&binary, &project, &["export", "--limit", "20"], 0);
    for task in result_value(&export)["tasks"]
        .as_array()
        .expect("export tasks")
    {
        let id = task["id"].as_u64().expect("export task ID");
        let expected = &model.tasks[&id];
        assert_eq!(task["title"], expected.title);
        assert_eq!(task["priority"], expected.priority);
        assert_eq!(task["archived"], expected.archived);
        assert_eq!(
            task["dependencies"]
                .as_array()
                .expect("dependencies")
                .iter()
                .map(|value| value.as_u64().expect("dependency"))
                .collect::<BTreeSet<_>>(),
            expected.dependencies
        );
        assert_eq!(
            task["labels"]
                .as_array()
                .expect("labels")
                .iter()
                .map(|value| value.as_str().expect("label").to_owned())
                .collect::<BTreeSet<_>>(),
            expected.labels
        );
    }
}

#[test]
fn public_product_story_is_pure_restartable_backed_up_and_source_independent() {
    let temporary = tempfile::tempdir().expect("temporary product directory");
    let project = temporary.path().join("project");
    let backup = temporary.path().join("backup");
    let restored = temporary.path().join("restored");
    let corrupt = temporary.path().join("corrupt");
    let installed = temporary.path().join("installed-lkjwork");
    let binary = binary();

    let initialized = machine(
        &binary,
        &[
            "--json",
            "init",
            project.to_str().expect("project path"),
            "--name",
            "lkjscript-next",
        ],
        0,
    );
    assert_eq!(revision(&initialized), 0);
    let version = machine(&binary, &["--json", "version"], 0);
    assert_eq!(
        initialized["result"]["application"],
        version["result"]["application_digest"]
    );

    for (title, priority, label) in [
        ("Query contract", "60", "runtime"),
        ("Text values", "50", "language"),
        ("Sequence values", "40", "language"),
        ("Storage checkpoints", "30", "storage"),
        ("Product client", "20", "product"),
        ("Verification", "10", "test"),
    ] {
        project_machine(
            &binary,
            &project,
            &[
                "add",
                title,
                "--priority",
                priority,
                "--label",
                label,
                "--actor",
                "campaign",
            ],
            0,
        );
    }
    for (task, prerequisite) in [(2, 1), (3, 1), (5, 2), (5, 3), (5, 4), (6, 5)] {
        project_machine(
            &binary,
            &project,
            &[
                "depend",
                &format!("#{task}"),
                "--on",
                &format!("#{prerequisite}"),
            ],
            0,
        );
    }
    let before_cycle = project_machine(&binary, &project, &["summary"], 0);
    let cycle = project_machine(
        &binary,
        &project,
        &["depend", "#1", "--on", "#6"],
        EXIT_DOMAIN_CONFLICT,
    );
    assert_eq!(cycle["result"]["publication"], "declined");
    assert_eq!(cycle["result"]["published"], false);
    assert_eq!(revision(&cycle), revision(&before_cycle));

    let blocked = project_machine(&binary, &project, &["start", "#5"], EXIT_DOMAIN_CONFLICT);
    assert_eq!(result_value(&blocked)["code"], "task_blocked");
    assert_eq!(blocked["result"]["published"], false);

    project_machine(&binary, &project, &["start", "#1"], 0);
    project_machine(
        &binary,
        &project,
        &[
            "note",
            "#1",
            "add",
            "Pure query boundary reproduced.",
            "--actor",
            "agent",
        ],
        0,
    );
    let evidence = temporary.path().join("evidence.txt");
    fs::write(&evidence, b"deterministic evidence\n").expect("write evidence");
    let attached = project_machine(
        &binary,
        &project,
        &[
            "attach",
            "#1",
            evidence.to_str().expect("evidence path"),
            "--name",
            "query-evidence.txt",
            "--actor",
            "agent",
        ],
        0,
    );
    assert_eq!(attached["result"]["publication"], "completed");
    assert_eq!(
        attached["result"]["host"]
            .as_array()
            .expect("host receipts")
            .len(),
        1
    );
    project_machine(&binary, &project, &["finish", "#1"], 0);

    let current = project_machine(&binary, &project, &["summary"], 0);
    let base = revision(&current).to_string();
    let first = project_machine(
        &binary,
        &project,
        &[
            "--base-revision",
            &base,
            "--idempotency-key",
            "acceptance-label-1",
            "label",
            "#2",
            "add",
            "ready",
        ],
        0,
    );
    assert_eq!(first["result"]["replayed"], false);
    project_machine(&binary, &project, &["priority", "#4", "35"], 0);
    let replay = project_machine(
        &binary,
        &project,
        &[
            "--base-revision",
            &base,
            "--idempotency-key",
            "acceptance-label-1",
            "label",
            "#2",
            "add",
            "ready",
        ],
        0,
    );
    assert_eq!(replay["result"]["replayed"], true);
    assert_eq!(replay["revision"], first["revision"]);

    let authority = project.join(".lkjwork");
    let before_queries = snapshot_tree(&authority);
    let mut query_results = Vec::new();
    for arguments in [
        vec!["show", "#1"],
        vec!["list", "--limit", "20", "--order", "priority"],
        vec!["next", "--limit", "5"],
        vec!["summary"],
        vec!["context", "--maximum-tasks", "5", "--maximum-notes", "5"],
        vec!["export", "--limit", "20"],
        vec!["history", "--limit", "20"],
        vec!["why", "#5"],
    ] {
        query_results.push(project_machine(&binary, &project, &arguments, 0));
    }
    assert_eq!(before_queries, snapshot_tree(&authority));
    assert!(
        query_results
            .iter()
            .all(|result| result["result"]["published"] == false)
    );
    assert_eq!(query_results[5]["result"]["export_version"], 1);
    assert_eq!(result_value(&query_results[3])["kind"], "summary");
    assert_eq!(result_value(&query_results[3])["blocked"], 2);
    assert_eq!(result_value(&query_results[7])["kind"], "why");
    assert_eq!(result_value(&query_results[7])["task"], 5);
    assert_eq!(result_value(&query_results[7])["actionable"], false);
    assert_eq!(
        result_value(&query_results[7])["blockers"],
        serde_json::json!([2, 3, 4])
    );
    let digest = query_results[1]["result"]["result_digest"]
        .as_str()
        .expect("query digest");
    let unchanged = project_machine(
        &binary,
        &project,
        &[
            "--known-result-digest",
            digest,
            "list",
            "--limit",
            "20",
            "--order",
            "priority",
        ],
        0,
    );
    assert_eq!(unchanged["result"]["unchanged"], true);
    assert!(unchanged["result"].get("value").is_none());
    assert_eq!(before_queries, snapshot_tree(&authority));

    let doctor = project_machine(&binary, &project, &["doctor", "--deep"], 0);
    assert_eq!(doctor["result"]["deep_audited"], true);
    machine(
        &binary,
        &[
            "--json",
            "--project",
            project.to_str().expect("project path"),
            "backup",
            "--to",
            backup.to_str().expect("backup path"),
        ],
        0,
    );
    machine(
        &binary,
        &[
            "--json",
            "restore",
            backup.to_str().expect("backup path"),
            "--to",
            restored.to_str().expect("restore path"),
        ],
        0,
    );
    let original_export = project_machine(&binary, &project, &["export", "--limit", "20"], 0);
    let restored_export = project_machine(&binary, &restored, &["export", "--limit", "20"], 0);
    assert_eq!(
        result_value(&original_export),
        result_value(&restored_export)
    );
    assert_eq!(
        original_export["result"]["state_digest"],
        restored_export["result"]["state_digest"]
    );

    machine(
        &binary,
        &[
            "--json",
            "--project",
            project.to_str().expect("project path"),
            "backup",
            "--to",
            corrupt.to_str().expect("corrupt path"),
        ],
        0,
    );
    let records = fs::read_dir(corrupt.join(".lkjwork/instance-store"))
        .expect("instance store")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            entry
                .file_type()
                .ok()?
                .is_dir()
                .then_some(entry.path().join("records"))
        })
        .next()
        .expect("instance records");
    let record = fs::read_dir(records)
        .expect("record directory")
        .map(|entry| entry.expect("record entry").path())
        .find(|path| path.extension().and_then(|value| value.to_str()) == Some("lkis"))
        .expect("record file");
    let mut bytes = fs::read(&record).expect("read record");
    let last = bytes.last_mut().expect("nonempty record");
    *last ^= 1;
    fs::write(record, bytes).expect("corrupt record copy");
    let corrupt_doctor = project_machine(&binary, &corrupt, &["doctor", "--deep"], 3);
    assert_eq!(corrupt_doctor["error"]["code"], "instance_validation");

    fs::copy(&binary, &installed).expect("copy installed product");
    fs::set_permissions(&installed, fs::Permissions::from_mode(0o700))
        .expect("make installed product executable");
    let installed_show = project_machine(&installed, &project, &["show", "#1"], 0);
    assert_eq!(result_value(&installed_show)["task"]["id"], 1);
}

#[test]
fn unknown_attachment_visibility_reconciles_without_repeating_put() {
    let temporary = tempfile::tempdir().expect("temporary product directory");
    let project = temporary.path().join("fake");
    let evidence = temporary.path().join("evidence.bin");
    fs::write(&evidence, b"unknown visibility evidence").expect("write evidence");
    let binary = binary();

    machine(
        &binary,
        &[
            "--json",
            "init",
            project.to_str().expect("project path"),
            "--name",
            "fake",
            "--deterministic-fake",
        ],
        0,
    );
    project_machine(
        &binary,
        &project,
        &["add", "Attachment", "--actor", "agent"],
        0,
    );
    let response = project_machine(
        &binary,
        &project,
        &[
            "attach",
            "#1",
            evidence.to_str().expect("evidence path"),
            "--actor",
            "agent",
            "--fake-put",
            "unknown",
            "--fake-inspect",
            "absent",
        ],
        EXIT_DOMAIN_CONFLICT,
    );
    let host = response["result"]["host"]
        .as_array()
        .expect("host receipts");
    assert_eq!(host.len(), 2);
    assert_eq!(host[0]["operation"], "put_blob");
    assert_eq!(host[0]["class"], "outcome_unknown");
    assert_eq!(host[1]["operation"], "inspect_blob");
    assert_eq!(host[1]["class"], "reconciliation_absent");
    assert_eq!(
        host.iter()
            .filter(|receipt| receipt["operation"] == "put_blob")
            .count(),
        1
    );
    let task = project_machine(&binary, &project, &["show", "#1"], 0);
    assert!(
        result_value(&task)["task"]["attachments"]
            .as_array()
            .expect("attachments")
            .is_empty()
    );
    let doctor = project_machine(&binary, &project, &["doctor", "--deep"], 0);
    assert_eq!(doctor["result"]["pending_command"], Value::Null);
}

#[test]
fn product_session_is_strict_bounded_correlated_and_recovers_per_line() {
    let temporary = tempfile::tempdir().expect("temporary product directory");
    let project = temporary.path().join("session");
    let binary = binary();
    machine(
        &binary,
        &[
            "--json",
            "init",
            project.to_str().expect("project path"),
            "--name",
            "session",
        ],
        0,
    );
    let mut child = Command::new(&binary)
        .arg("session")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("start product session");
    {
        let input = child.stdin.as_mut().expect("session stdin");
        writeln!(input, "{{").expect("malformed line");
        writeln!(
            input,
            "{{\"contract_version\":1,\"request_id\":7,\"project\":{},\"arguments\":[\"summary\"]}}",
            serde_json::to_string(project.to_str().expect("project path")).unwrap()
        )
        .expect("valid line");
        writeln!(
            input,
            "{{\"contract_version\":1,\"request_id\":7,\"project\":{},\"arguments\":[\"summary\"]}}",
            serde_json::to_string(project.to_str().expect("project path")).unwrap()
        )
        .expect("duplicate ID");
        writeln!(
            input,
            "{{\"contract_version\":1,\"request_id\":8,\"arguments\":[\"shutdown\"]}}"
        )
        .expect("shutdown");
    }
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("wait for session");
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let frames = String::from_utf8(output.stdout)
        .expect("session output UTF-8")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("session JSON"))
        .collect::<Vec<_>>();
    assert_eq!(frames.len(), 4);
    assert_eq!(frames[0]["error"]["code"], "malformed_request");
    assert_eq!(frames[1]["request_id"], 7);
    assert_eq!(frames[1]["response"]["operation"], "summary");
    assert_eq!(frames[2]["error"]["code"], "duplicate_request_id");
    assert_eq!(frames[3]["response"]["operation"], "shutdown");
}

#[test]
fn missing_or_corrupt_current_state_cache_falls_back_without_query_writes() {
    let temporary = tempfile::tempdir().expect("temporary product directory");
    let project = temporary.path().join("cache");
    let binary = binary();
    machine(
        &binary,
        &[
            "--json",
            "init",
            project.to_str().expect("project path"),
            "--name",
            "cache",
        ],
        0,
    );
    project_machine(&binary, &project, &["add", "one"], 0);
    project_machine(&binary, &project, &["add", "two"], 0);
    let current = instance_authority(&project).join("CURRENT");

    fs::remove_file(&current).expect("remove derived current state");
    let before_missing_query = snapshot_tree(&project.join(".lkjwork"));
    let task = project_machine(&binary, &project, &["show", "#2"], 0);
    assert_eq!(result_value(&task)["task"]["title"], "two");
    assert_eq!(
        before_missing_query,
        snapshot_tree(&project.join(".lkjwork"))
    );
    let missing = project_machine(&binary, &project, &["doctor"], 0);
    assert_eq!(missing["result"]["current_state_cache"], false);
    assert_eq!(missing["result"]["normal_replay_records"], 2);

    project_machine(&binary, &project, &["priority", "#2", "7"], 0);
    let mut bytes = fs::read(&current).expect("rebuilt current state");
    bytes[0] ^= 1;
    fs::write(&current, bytes).expect("corrupt derived current state");
    let before_corrupt_query = snapshot_tree(&project.join(".lkjwork"));
    let task = project_machine(&binary, &project, &["show", "#2"], 0);
    assert_eq!(result_value(&task)["task"]["priority"], 7);
    assert_eq!(
        before_corrupt_query,
        snapshot_tree(&project.join(".lkjwork"))
    );
    let corrupt = project_machine(&binary, &project, &["doctor"], 0);
    assert_eq!(corrupt["result"]["current_state_cache"], false);
    assert_eq!(corrupt["result"]["normal_replay_records"], 3);
    let deep = project_machine(&binary, &project, &["doctor", "--deep"], 0);
    assert_eq!(deep["result"]["deep_audited"], true);
}

#[test]
fn text_and_attachment_one_over_limits_reject_without_publication() {
    let temporary = tempfile::tempdir().expect("temporary product directory");
    let project = temporary.path().join("limits");
    let binary = binary();
    machine(
        &binary,
        &[
            "--json",
            "init",
            project.to_str().expect("project path"),
            "--name",
            "limits",
        ],
        0,
    );
    let before = snapshot_tree(&project.join(".lkjwork"));
    let oversized = "x".repeat(64 * 1024 + 1);
    let response = project_machine(&binary, &project, &["add", &oversized], 2);
    assert_eq!(response["error"]["code"], "input_limit");
    assert_eq!(before, snapshot_tree(&project.join(".lkjwork")));

    let attachment = temporary.path().join("oversized.bin");
    fs::write(
        &attachment,
        vec![0_u8; lkjscript::schema::MAXIMUM_BYTE_STRING_BYTES + 1],
    )
    .expect("write oversized attachment");
    let response = project_machine(
        &binary,
        &project,
        &[
            "attach",
            "#1",
            attachment.to_str().expect("attachment path"),
        ],
        2,
    );
    assert_eq!(response["error"]["code"], "input_limit");
    assert_eq!(before, snapshot_tree(&project.join(".lkjwork")));
}
