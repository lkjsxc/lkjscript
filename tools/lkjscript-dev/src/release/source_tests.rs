use super::*;
use std::process::Output;
use tempfile::TempDir;

const FIXTURE_TAG: &str = "v0.1.35";
const CHECKOUT_EXPRESSION: &str =
    "${{ github.event_name == 'workflow_dispatch' && inputs.publish && inputs.tag || github.sha }}";

struct GitFixture {
    temporary: TempDir,
    repository: PathBuf,
    base: String,
    candidate: String,
    reporting: String,
    later_code: String,
    divergent: String,
    disconnected: String,
}

impl GitFixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().expect("owned source-policy fixture");
        let repository = temporary.path().join("repository");
        fs::create_dir(&repository).expect("fixture checkout directory");
        git(&repository, &["init", "--initial-branch=fixture"]);
        for (key, value) in [
            ("user.name", "Release source fixture"),
            ("user.email", "release-source-fixture@example.invalid"),
            ("commit.gpgsign", "false"),
            ("tag.gpgsign", "false"),
            ("core.autocrlf", "false"),
            ("gc.auto", "0"),
        ] {
            git(&repository, &["config", key, value]);
        }
        git(
            &repository,
            &[
                "remote",
                "add",
                "origin",
                "https://github.com/lkjsxc/lkjscript.git",
            ],
        );
        let base = commit_file(
            &repository,
            "Cargo.toml",
            "[package]\nname = \"lkjscript\"\nversion = \"0.1.35\"\n",
        );
        let candidate = commit_file(&repository, "src/lib.rs", "pub fn candidate() {}\n");
        let reporting = commit_file(
            &repository,
            "docs/status.md",
            "The selected candidate is frozen while this report advances main.\n",
        );
        let later_code = commit_file(
            &repository,
            "src/lib.rs",
            "pub fn candidate() {}\npub fn subsequent_implementation() {}\n",
        );
        git(&repository, &["checkout", "--detach", &base]);
        let divergent = commit_file(&repository, "src/divergent.rs", "pub fn divergence() {}\n");
        // commit-tree without a parent creates known disconnected history; neither
        // tested ancestry implementation determines these expected relations.
        let tree = git(
            &repository,
            &["rev-parse", &format!("{candidate}^{{tree}}")],
        );
        let disconnected = git(
            &repository,
            &["commit-tree", &tree, "-m", "independent root"],
        );
        git(&repository, &["checkout", "--detach", &candidate]);
        git(
            &repository,
            &["update-ref", "refs/remotes/origin/main", &candidate],
        );
        Self {
            temporary,
            repository,
            base,
            candidate,
            reporting,
            later_code,
            divergent,
            disconnected,
        }
    }

    fn main_at(&self, commit: &str) {
        git(
            &self.repository,
            &["update-ref", "refs/remotes/origin/main", commit],
        );
    }

    fn checkout(&self, reference: &str) {
        git(&self.repository, &["checkout", "--detach", reference]);
    }

    fn annotated_tag(&self, tag: &str, commit: &str) {
        git(
            &self.repository,
            &[
                "tag",
                "--annotate",
                tag,
                commit,
                "--message",
                "fixture release",
            ],
        );
    }

    fn known_relations(&self) -> [(&str, &str, bool, bool); 6] {
        [
            ("equality", &self.candidate, true, true),
            ("main precedes candidate", &self.base, true, false),
            ("main adds reporting", &self.reporting, true, true),
            ("main adds implementation", &self.later_code, true, true),
            ("divergence", &self.divergent, false, false),
            ("disconnected history", &self.disconnected, false, false),
        ]
    }

    fn state(&self) -> (String, String, String, Vec<u8>) {
        (
            git(&self.repository, &["show-ref"]),
            git(&self.repository, &["rev-parse", "HEAD"]),
            git(
                &self.repository,
                &["status", "--porcelain=v1", "--untracked-files=all"],
            ),
            fs::read(self.repository.join(".git/config")).expect("fixture Git configuration"),
        )
    }

    fn workflow(&self, event: WorkflowEvent<'_>) -> WorkflowResult {
        self.workflow_with_path(event, None)
    }

    fn workflow_with_path(&self, event: WorkflowEvent<'_>, path: Option<&str>) -> WorkflowResult {
        let output_file = self.temporary.path().join("github-output");
        fs::write(&output_file, "").expect("new fixture workflow output");
        let mut command = Command::new("bash");
        command
            .args(["--noprofile", "--norc", "-c", &workflow_identity_script()])
            .current_dir(&self.repository)
            .env_clear()
            .envs(process::environment())
            .env("GITHUB_EVENT_NAME", event.name)
            .env("GITHUB_SHA", event.sha)
            .env("GITHUB_REF_NAME", event.reference)
            .env("GITHUB_REF_TYPE", event.reference_type)
            .env("GITHUB_REPOSITORY", REPOSITORY_IDENTITY)
            .env("REQUESTED_PUBLISH", event.publish)
            .env("REQUESTED_TAG", event.tag)
            .env("GITHUB_OUTPUT", &output_file);
        if let Some(path) = path {
            command.env("PATH", path);
        }
        let process = command
            .output()
            .expect("execute actual workflow identity Bash");
        let outputs = fs::read_to_string(output_file).expect("workflow identity outputs");
        WorkflowResult { process, outputs }
    }

    fn git_wrapper(&self, behavior: &str) -> (String, PathBuf) {
        let found = Command::new("bash")
            .args(["--noprofile", "--norc", "-c", "command -v git"])
            .env_clear()
            .envs(process::environment())
            .output()
            .expect("find fixture Git executable");
        assert!(found.status.success());
        let real_git = String::from_utf8(found.stdout).expect("Git executable path");
        assert!(Path::new(real_git.trim()).is_absolute());
        let directory = self.temporary.path().join("git-wrapper");
        fs::create_dir(&directory).expect("owned Git wrapper directory");
        let trace = self.temporary.path().join("git-trace");
        let wrapper = directory.join("git");
        fs::write(
            &wrapper,
            format!(
                "#!/bin/bash\nset -euo pipefail\nprintf '%s\\n' \"$*\" >> {}\nfixture_git={}\n{behavior}\nexec \"$fixture_git\" \"$@\"\n",
                shell_quote(trace.to_str().expect("fixture trace path")),
                shell_quote(real_git.trim()),
            ),
        )
        .expect("write scoped Git fault wrapper");
        fs::set_permissions(wrapper, fs::Permissions::from_mode(0o755))
            .expect("make owned wrapper executable");
        let inherited_path = process::environment()
            .get("PATH")
            .expect("test PATH")
            .clone();
        (format!("{}:{inherited_path}", directory.display()), trace)
    }
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn git_output(repository: &Path, arguments: &[&str]) -> Output {
    Command::new("git")
        .args(arguments)
        .current_dir(repository)
        .env_clear()
        .envs(process::environment())
        .output()
        .expect("execute fixture Git command")
}

fn git(repository: &Path, arguments: &[&str]) -> String {
    let output = git_output(repository, arguments);
    assert!(
        output.status.success(),
        "fixture Git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("fixture Git output is UTF-8")
        .trim()
        .to_owned()
}

fn commit_file(repository: &Path, name: &str, contents: &str) -> String {
    let path = repository.join(name);
    fs::create_dir_all(path.parent().expect("fixture file parent")).expect("fixture directories");
    fs::write(path, contents).expect("fixture commit content");
    git(repository, &["add", "--", name]);
    git(repository, &["commit", "--message", name]);
    git(repository, &["rev-parse", "HEAD"])
}

fn workflow_text() -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.github/workflows/release.yml"),
    )
    .expect("read maintained release workflow")
}

fn workflow_identity_script() -> String {
    let workflow = workflow_text();
    let identity = workflow
        .split_once("      - name: Validate workflow and release identity\n")
        .expect("actual workflow identity step")
        .1;
    let block = identity
        .split_once("        run: |\n")
        .expect("identity Bash block")
        .1;
    let mut script = String::new();
    for line in block.lines() {
        if line.is_empty() {
            script.push('\n');
        } else if let Some(line) = line.strip_prefix("          ") {
            script.push_str(line);
            script.push('\n');
        } else {
            break;
        }
    }
    assert!(script.starts_with("set -euo pipefail\n"));
    assert!(script.contains("GITHUB_OUTPUT"));
    script
}

#[derive(Clone, Copy)]
struct WorkflowEvent<'a> {
    name: &'a str,
    sha: &'a str,
    reference: &'a str,
    reference_type: &'a str,
    publish: &'a str,
    tag: &'a str,
}

impl<'a> WorkflowEvent<'a> {
    fn dry_run(sha: &'a str) -> Self {
        Self {
            name: "workflow_dispatch",
            sha,
            reference: "dispatch-handle",
            reference_type: "branch",
            publish: "false",
            tag: "",
        }
    }

    fn manual_publication(workflow_sha: &'a str) -> Self {
        Self {
            publish: "true",
            tag: FIXTURE_TAG,
            ..Self::dry_run(workflow_sha)
        }
    }

    fn tag_push(sha: &'a str) -> Self {
        Self {
            name: "push",
            reference: FIXTURE_TAG,
            reference_type: "tag",
            ..Self::dry_run(sha)
        }
    }
}

struct WorkflowResult {
    process: Output,
    outputs: String,
}

impl WorkflowResult {
    fn assert_admission(&self, admitted: bool, case: &str) {
        assert_eq!(
            self.process.status.success(),
            admitted,
            "{case}: status={:?}, stderr={}, outputs={}",
            self.process.status.code(),
            String::from_utf8_lossy(&self.process.stderr),
            self.outputs
        );
        if !admitted {
            assert!(
                self.outputs.is_empty(),
                "rejection emitted accepted source: {case}"
            );
        }
    }

    fn assert_source(&self, source: &str, publication_mode: &str) {
        assert!(
            self.outputs
                .lines()
                .any(|line| line == format!("commit_sha={source}"))
        );
        assert!(
            self.outputs
                .lines()
                .any(|line| line == format!("publication_mode={publication_mode}"))
        );
        assert!(
            self.outputs
                .lines()
                .any(|line| line == format!("tag={FIXTURE_TAG}"))
        );
    }
}

#[test]
fn source_facts_admits_the_known_git_truth_table_without_retargeting() {
    let fixture = GitFixture::new();
    fixture.annotated_tag(FIXTURE_TAG, &fixture.candidate);
    for (relation, main, dry_run, publication) in fixture.known_relations() {
        fixture.main_at(main);
        for (mode, expected) in [
            (PublicationMode::DryRun, dry_run),
            (PublicationMode::Release, publication),
        ] {
            let before = fixture.state();
            let facts = source_facts(&fixture.repository, FIXTURE_TAG, mode);
            assert_eq!(facts.is_ok(), expected, "{relation} / {mode:?}: {facts:?}");
            if let Ok(facts) = facts {
                assert_eq!(facts.commit_sha, fixture.candidate, "{relation}");
                assert_eq!(
                    facts.tag_object_sha.is_some(),
                    publication && mode == PublicationMode::Release
                );
            }
            assert_eq!(
                fixture.state(),
                before,
                "source admission mutated {relation}"
            );
        }
    }
}

#[test]
fn actual_workflow_shell_admits_the_known_git_truth_table_without_retargeting() {
    let fixture = GitFixture::new();
    fixture.annotated_tag(FIXTURE_TAG, &fixture.candidate);
    for (relation, main, dry_run, publication) in fixture.known_relations() {
        fixture.main_at(main);
        let before = fixture.state();
        for (event, admitted, mode) in [
            (
                WorkflowEvent::dry_run(&fixture.candidate),
                dry_run,
                "dry-run",
            ),
            (
                WorkflowEvent::tag_push(&fixture.candidate),
                publication,
                "release",
            ),
            (
                WorkflowEvent::manual_publication(&fixture.later_code),
                publication,
                "release",
            ),
        ] {
            let result = fixture.workflow(event);
            result.assert_admission(admitted, relation);
            if admitted {
                result.assert_source(&fixture.candidate, mode);
            }
            assert_eq!(
                fixture.state(),
                before,
                "workflow preflight mutated {relation}"
            );
        }
    }
}

#[test]
fn former_dry_run_predicate_rejects_the_exact_reporting_descendant_regression() {
    let fixture = GitFixture::new();
    fixture.main_at(&fixture.reporting);
    let former = git_output(
        &fixture.repository,
        &["merge-base", "--is-ancestor", "origin/main", "HEAD"],
    );
    assert_eq!(former.status.code(), Some(1));
    assert!(source_facts(&fixture.repository, FIXTURE_TAG, PublicationMode::DryRun).is_ok());
    fixture
        .workflow(WorkflowEvent::dry_run(&fixture.candidate))
        .assert_admission(true, "frozen candidate before reporting descendant");
}

#[test]
fn exact_workflow_checkout_survives_dispatch_branch_movement() {
    let workflow = workflow_text();
    let checkout = workflow
        .split_once("      - name: Check out the exact source\n")
        .expect("checkout step")
        .1
        .split_once("      - name: Validate workflow and release identity\n")
        .expect("identity follows checkout")
        .0;
    assert_eq!(
        checkout
            .lines()
            .find_map(|line| line.strip_prefix("          ref: ")),
        Some(CHECKOUT_EXPRESSION)
    );
    assert!(checkout.contains("          fetch-depth: 0\n"));
    let fixture = GitFixture::new();
    git(
        &fixture.repository,
        &[
            "update-ref",
            "refs/heads/dispatch-handle",
            &fixture.candidate,
        ],
    );
    let captured_event_sha = git(&fixture.repository, &["rev-parse", "dispatch-handle"]);
    git(
        &fixture.repository,
        &[
            "update-ref",
            "refs/heads/dispatch-handle",
            &fixture.later_code,
        ],
    );
    assert_ne!(
        git(&fixture.repository, &["rev-parse", "dispatch-handle"]),
        captured_event_sha
    );
    fixture.main_at(&fixture.later_code);
    fixture.checkout(&captured_event_sha);
    let result = fixture.workflow(WorkflowEvent::dry_run(&captured_event_sha));
    result.assert_admission(true, "checkout event SHA after dispatch branch moves");
    result.assert_source(&captured_event_sha, "dry-run");
    assert_eq!(
        git(&fixture.repository, &["rev-parse", "HEAD"]),
        captured_event_sha
    );
    fixture.checkout("dispatch-handle");
    fixture
        .workflow(WorkflowEvent::dry_run(&captured_event_sha))
        .assert_admission(
            false,
            "mutable branch checkout must fail exact-source assertion",
        );
}

#[test]
fn workflow_rejects_wrong_missing_or_invalid_event_source_before_emitting_identity() {
    let fixture = GitFixture::new();
    for selected in ["", "not-a-commit", &"0".repeat(40), &fixture.base] {
        fixture
            .workflow(WorkflowEvent::dry_run(selected))
            .assert_admission(false, "invalid dry-run selected commit");
    }
    fixture.annotated_tag(FIXTURE_TAG, &fixture.candidate);
    fixture
        .workflow(WorkflowEvent::tag_push(&fixture.base))
        .assert_admission(false, "tag-push event SHA differs from product HEAD");
    fixture
        .workflow(WorkflowEvent {
            reference_type: "branch",
            ..WorkflowEvent::tag_push(&fixture.candidate)
        })
        .assert_admission(false, "push publication requires a tag event");
}

#[test]
fn manual_existing_tag_uses_tagged_product_source_separately_from_workflow_sha() {
    let fixture = GitFixture::new();
    fixture.annotated_tag(FIXTURE_TAG, &fixture.candidate);
    fixture.main_at(&fixture.later_code);
    fixture.checkout(FIXTURE_TAG);
    assert_ne!(fixture.candidate, fixture.later_code);
    let result = fixture.workflow(WorkflowEvent::manual_publication(&fixture.later_code));
    result.assert_admission(
        true,
        "manual recovery selects annotated tag's product source",
    );
    result.assert_source(&fixture.candidate, "release");
    fixture.checkout(&fixture.later_code);
    fixture
        .workflow(WorkflowEvent::manual_publication(&fixture.later_code))
        .assert_admission(
            false,
            "manual recovery cannot substitute workflow definition source",
        );
}

#[test]
fn both_publication_owners_reject_missing_lightweight_wrong_target_and_wrong_version_tags() {
    for case in ["missing", "lightweight", "wrong-target", "wrong-version"] {
        let fixture = GitFixture::new();
        let tag = match case {
            "lightweight" => {
                git(
                    &fixture.repository,
                    &["tag", FIXTURE_TAG, &fixture.candidate],
                );
                FIXTURE_TAG
            }
            "wrong-target" => {
                fixture.annotated_tag(FIXTURE_TAG, &fixture.base);
                FIXTURE_TAG
            }
            "wrong-version" => {
                fixture.annotated_tag("v0.1.36", &fixture.candidate);
                "v0.1.36"
            }
            _ => FIXTURE_TAG,
        };
        assert!(
            source_facts(&fixture.repository, tag, PublicationMode::Release).is_err(),
            "Rust owner accepted {case} tag"
        );
        fixture
            .workflow(WorkflowEvent {
                tag,
                ..WorkflowEvent::manual_publication(&fixture.later_code)
            })
            .assert_admission(false, case);
        fixture
            .workflow(WorkflowEvent {
                reference: tag,
                ..WorkflowEvent::tag_push(&fixture.candidate)
            })
            .assert_admission(false, case);
    }
}

#[test]
fn git_ancestry_false_and_invocation_failure_have_distinct_outcomes() {
    let fixture = GitFixture::new();
    assert!(
        git_is_ancestor(&fixture.repository, &fixture.base, &fixture.candidate)
            .expect("ancestor query")
    );
    assert!(
        !git_is_ancestor(&fixture.repository, &fixture.candidate, &fixture.base)
            .expect("false ancestor query")
    );
    for (repository, ancestor, descendant) in [
        (
            fixture.repository.clone(),
            "0".repeat(40),
            fixture.candidate.clone(),
        ),
        (
            fixture.repository.clone(),
            fixture.candidate.clone(),
            "0".repeat(40),
        ),
        (
            fixture.temporary.path().join("absent-directory"),
            fixture.base.clone(),
            fixture.candidate.clone(),
        ),
    ] {
        let error = git_is_ancestor(&repository, &ancestor, &descendant)
            .expect_err("Git invocation must fail");
        assert_eq!(error.kind(), "infrastructure", "{error}");
    }
}

#[test]
fn both_source_owners_reject_missing_main_history_objects_head_and_wrong_origin() {
    for case in [
        "missing-main",
        "missing-main-object",
        "missing-head",
        "shallow-history",
        "wrong-origin",
        "missing-origin",
    ] {
        let fixture = GitFixture::new();
        match case {
            "missing-main" => {
                git(
                    &fixture.repository,
                    &["update-ref", "-d", "refs/remotes/origin/main"],
                );
            }
            "missing-main-object" => {
                fixture.main_at(&fixture.disconnected);
                let object = fixture
                    .repository
                    .join(".git/objects")
                    .join(&fixture.disconnected[..2])
                    .join(&fixture.disconnected[2..]);
                fs::remove_file(object).expect("remove owned fixture's main object");
            }
            "missing-head" => {
                fs::write(
                    fixture.repository.join(".git/HEAD"),
                    format!("{}\n", "0".repeat(40)),
                )
                .expect("invalid HEAD in disposable fixture");
            }
            "shallow-history" => {
                fs::write(
                    fixture.repository.join(".git/shallow"),
                    format!("{}\n", fixture.candidate),
                )
                .expect("owned fixture shallow boundary");
            }
            "wrong-origin" => {
                git(
                    &fixture.repository,
                    &[
                        "remote",
                        "set-url",
                        "origin",
                        "https://github.com/other/lkjscript.git",
                    ],
                );
            }
            "missing-origin" => {
                git(&fixture.repository, &["remote", "remove", "origin"]);
            }
            _ => unreachable!(),
        }
        assert!(
            source_facts(&fixture.repository, FIXTURE_TAG, PublicationMode::DryRun).is_err(),
            "Rust owner admitted {case}"
        );
        fixture
            .workflow(WorkflowEvent::dry_run(&fixture.candidate))
            .assert_admission(false, case);
    }
}

#[test]
fn actual_workflow_shell_does_not_treat_git_failure_as_a_false_ancestry_branch() {
    let fixture = GitFixture::new();
    fixture.main_at(&fixture.base);
    let (path, trace) = fixture.git_wrapper(
        "if [[ \"${1:-}\" == merge-base ]]; then\n  echo 'injected ancestry command failure' >&2\n  exit 128\nfi",
    );
    let result =
        fixture.workflow_with_path(WorkflowEvent::dry_run(&fixture.candidate), Some(&path));
    result.assert_admission(
        false,
        "Git error must terminate before trying alternate ancestry",
    );
    assert_eq!(result.process.status.code(), Some(128));
    assert!(String::from_utf8_lossy(&result.process.stderr).contains("128"));
    let trace = fs::read_to_string(trace).expect("Git fault trace");
    assert_eq!(
        trace
            .lines()
            .filter(|line| line.starts_with("merge-base "))
            .count(),
        1
    );
}

#[test]
fn actual_workflow_shell_resolves_main_once_and_uses_that_immutable_commit() {
    let fixture = GitFixture::new();
    fixture.main_at(&fixture.later_code);
    let (path, trace) = fixture.git_wrapper(&format!(
        "if [[ \"${{1:-}}\" == rev-parse && \"$*\" == *origin/main* ]]; then\n  \"$fixture_git\" \"$@\"\n  \"$fixture_git\" update-ref refs/remotes/origin/main {}\n  exit 0\nfi",
        shell_quote(&fixture.disconnected),
    ));
    let result =
        fixture.workflow_with_path(WorkflowEvent::dry_run(&fixture.candidate), Some(&path));
    result.assert_admission(true, "observed main commit remains the admission context");
    result.assert_source(&fixture.candidate, "dry-run");
    assert_eq!(
        git(&fixture.repository, &["rev-parse", "origin/main"]),
        fixture.disconnected
    );
    let trace = fs::read_to_string(trace).expect("main-resolution trace");
    assert_eq!(
        trace
            .lines()
            .filter(|line| line.contains("origin/main"))
            .count(),
        1
    );
    assert!(trace.lines().any(|line| line
        == format!(
            "merge-base --is-ancestor {} {}",
            fixture.candidate, fixture.later_code
        )));
}

#[test]
fn dry_run_identity_is_read_only_and_release_preparation_still_requires_clean_source() {
    let fixture = GitFixture::new();
    let before = fixture.state();
    assert!(git(&fixture.repository, &["tag", "--list"]).is_empty());
    assert!(ensure_clean_checkout(&fixture.repository).is_ok());
    assert!(source_facts(&fixture.repository, FIXTURE_TAG, PublicationMode::DryRun).is_ok());
    let result = fixture.workflow(WorkflowEvent::dry_run(&fixture.candidate));
    result.assert_admission(true, "tagless read-only rehearsal");
    assert!(result.outputs.lines().any(|line| line == "publish=false"));
    assert_eq!(fixture.state(), before);
    assert!(git(&fixture.repository, &["tag", "--list"]).is_empty());
    fs::write(
        fixture.repository.join("src/lib.rs"),
        "pub fn edited() {}\n",
    )
    .expect("owned dirty source");
    assert!(ensure_clean_checkout(&fixture.repository).is_err());
}
