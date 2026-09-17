//! Product source selection remains independent of publication. Controller/API
//! authority and resume fixtures live with the actual controller implementation.
use super::*;
use std::process::Output;
use tempfile::TempDir;

const FIXTURE_TAG: &str = "v0.1.35";

struct GitFixture {
    temporary: TempDir,
    repository: PathBuf,
    base: String,
    candidate: String,
    reporting: String,
    divergent: String,
    disconnected: String,
}
impl GitFixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().expect("owned source fixture");
        let repository = temporary.path().join("repository");
        fs::create_dir(&repository).expect("repository");
        git(&repository, &["init", "--initial-branch=fixture"]);
        for (key, value) in [
            ("user.name", "Release source fixture"),
            ("user.email", "release-source-fixture@example.invalid"),
            ("commit.gpgsign", "false"),
            ("tag.gpgsign", "false"),
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
            "Reporting main advances after selected candidate.\n",
        );
        git(&repository, &["checkout", "--detach", &base]);
        let divergent = commit_file(&repository, "src/divergent.rs", "pub fn divergent() {}\n");
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
            divergent,
            disconnected,
        }
    }
    fn main_at(&self, source: &str) {
        git(
            &self.repository,
            &["update-ref", "refs/remotes/origin/main", source],
        );
    }
    fn annotated_tag(&self, tag: &str, source: &str) {
        git(
            &self.repository,
            &[
                "tag",
                "--annotate",
                tag,
                source,
                "--message",
                "fixture release",
            ],
        );
    }
    fn state(&self) -> (String, String, String) {
        (
            git(&self.repository, &["show-ref"]),
            git(&self.repository, &["rev-parse", "HEAD"]),
            git(
                &self.repository,
                &["status", "--porcelain=v1", "--untracked-files=all"],
            ),
        )
    }
}
fn git_output(repository: &Path, arguments: &[&str]) -> Output {
    Command::new("git")
        .args(arguments)
        .current_dir(repository)
        .env_clear()
        .envs(process::environment())
        .output()
        .expect("fixture Git command")
}
fn git(repository: &Path, arguments: &[&str]) -> String {
    let output = git_output(repository, arguments);
    assert!(
        output.status.success(),
        "{arguments:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("Git output")
        .trim()
        .to_owned()
}
fn commit_file(repository: &Path, name: &str, contents: &str) -> String {
    let path = repository.join(name);
    fs::create_dir_all(path.parent().expect("parent")).expect("directories");
    fs::write(path, contents).expect("content");
    git(repository, &["add", "--", name]);
    git(repository, &["commit", "--message", name]);
    git(repository, &["rev-parse", "HEAD"])
}

#[test]
fn source_facts_admits_known_git_relations_without_selecting_later_main() {
    let fixture = GitFixture::new();
    fixture.annotated_tag(FIXTURE_TAG, &fixture.candidate);
    for (label, main, candidate_allowed) in [
        ("equal", &fixture.candidate, true),
        ("not integrated", &fixture.base, true),
        ("reporting descendant", &fixture.reporting, true),
        ("divergent", &fixture.divergent, false),
        ("disconnected", &fixture.disconnected, false),
    ] {
        fixture.main_at(main);
        {
            let expected = candidate_allowed;
            let before = fixture.state();
            let facts = source_facts(&fixture.repository, FIXTURE_TAG);
            assert_eq!(facts.is_ok(), expected, "{label}: {facts:?}");
            if let Ok(facts) = facts {
                assert_eq!(facts.commit_sha, fixture.candidate);
            }
            assert_eq!(fixture.state(), before, "source selection mutated {label}");
        }
    }
}

#[test]
fn neutral_construction_does_not_require_or_write_a_publication_tag() {
    let fixture = GitFixture::new();
    let before = fixture.state();
    assert!(ensure_clean_checkout(&fixture.repository).is_ok());
    let facts = source_facts(&fixture.repository, FIXTURE_TAG).expect("candidate source");
    assert_eq!(facts.commit_sha, fixture.candidate);
    assert!(git(&fixture.repository, &["tag", "--list"]).is_empty());
    assert_eq!(fixture.state(), before);
    fs::write(
        fixture.repository.join("src/lib.rs"),
        "pub fn edited() {}\n",
    )
    .expect("dirty source");
    assert!(ensure_clean_checkout(&fixture.repository).is_err());
}

#[test]
fn missing_history_or_wrong_origin_cannot_silently_supply_source_authority() {
    for case in [
        "missing-main",
        "missing-main-object",
        "missing-head",
        "shallow",
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
                fs::remove_file(
                    fixture
                        .repository
                        .join(".git/objects")
                        .join(&fixture.disconnected[..2])
                        .join(&fixture.disconnected[2..]),
                )
                .expect("owned object removal");
            }
            "missing-head" => fs::write(
                fixture.repository.join(".git/HEAD"),
                format!("{}\n", "0".repeat(40)),
            )
            .expect("owned head"),
            "shallow" => fs::write(
                fixture.repository.join(".git/shallow"),
                format!("{}\n", fixture.candidate),
            )
            .expect("owned shallow"),
            "wrong-origin" => {
                git(
                    &fixture.repository,
                    &[
                        "remote",
                        "set-url",
                        "origin",
                        "https://github.com/foreign/lkjscript.git",
                    ],
                );
            }
            "missing-origin" => {
                git(&fixture.repository, &["remote", "remove", "origin"]);
            }
            _ => unreachable!(),
        }
        assert!(
            source_facts(&fixture.repository, FIXTURE_TAG).is_err(),
            "{case}"
        );
    }
}

#[test]
fn git_failure_is_not_an_ordinary_false_ancestry_result() {
    let fixture = GitFixture::new();
    assert!(
        git_is_ancestor(&fixture.repository, &fixture.base, &fixture.candidate).expect("ancestor")
    );
    assert!(
        !git_is_ancestor(&fixture.repository, &fixture.candidate, &fixture.base)
            .expect("not ancestor")
    );
    for (repository, source) in [
        (fixture.repository.clone(), "0".repeat(40)),
        (
            fixture.temporary.path().join("missing"),
            fixture.base.clone(),
        ),
    ] {
        assert_eq!(
            git_is_ancestor(&repository, &source, &fixture.candidate)
                .expect_err("Git invocation failure")
                .kind(),
            "infrastructure"
        );
    }
}
