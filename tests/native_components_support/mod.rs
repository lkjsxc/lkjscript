use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub mod cases;
pub mod oracle;

pub fn path(value: &Path) -> &str {
    value.to_str().unwrap()
}
pub fn field<'a>(text: &'a str, record: &str, key: &str) -> &'a str {
    let prefix = format!("{record} ");
    let key = format!("{key}=");
    text.lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap()
        .split_whitespace()
        .find_map(|part| part.strip_prefix(&key))
        .unwrap()
}
pub fn quoted(text: &str, key: &str) -> Value {
    let rest = text.split_once(&format!(" {key}=")).unwrap().1;
    let encoded = String::deserialize(&mut serde_json::Deserializer::from_str(rest)).unwrap();
    serde_json::from_str(&encoded).unwrap()
}
pub fn digest(file: &Path) -> String {
    Sha256::digest(fs::read(file).unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub struct Dependency {
    metadata: String,
    file: PathBuf,
}
pub struct Native {
    temporary: Option<tempfile::TempDir>,
    pub root: PathBuf,
    serial: Cell<usize>,
}
impl Native {
    pub fn new() -> Self {
        let temporary = tempfile::Builder::new()
            .prefix("lkjscript-components-")
            .tempdir()
            .unwrap();
        let root = temporary.path().to_path_buf();
        fs::create_dir(root.join("home")).unwrap();
        let candidate = std::env::var_os("LKJSCRIPT_COMPONENT_CANDIDATE")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_lkjscript")));
        fs::copy(&candidate, root.join("lkjscript")).unwrap();
        assert_eq!(digest(&candidate), digest(&root.join("lkjscript")));
        Self {
            temporary: Some(temporary),
            root,
            serial: Cell::new(0),
        }
    }
    fn next(&self) -> usize {
        let next = self.serial.get();
        self.serial.set(next + 1);
        next
    }
    pub fn input(&self, name: &str, content: &str) -> PathBuf {
        let file = self.root.join(name);
        assert!(
            !file.exists(),
            "proof input must be absent: {}",
            file.display()
        );
        fs::write(&file, content).unwrap();
        file
    }
    pub fn cli(&self, project: &Path, args: &[&str], success: bool) -> String {
        let output = Command::new(self.root.join("lkjscript"))
            .current_dir(project)
            .env_clear()
            .env("HOME", self.root.join("home"))
            .env("PATH", "/usr/bin:/bin")
            .env("LC_ALL", "C")
            .args(args)
            .output()
            .unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap()
        );
        fs::write(
            self.root.join(format!("operation-{:04}.log", self.next())),
            format!("arguments={args:?}\nexit={}\n{text}", output.status),
        )
        .unwrap();
        assert_eq!(
            output.status.success(),
            success,
            "{args:?}\n{text}\nproof={}",
            self.root.display()
        );
        text
    }
    pub fn project(&self, name: &str) -> PathBuf {
        let project = self.root.join(name);
        self.cli(
            &self.root,
            &[
                "new",
                path(&project),
                "--template",
                "minimal",
                "--name",
                name,
            ],
            true,
        );
        project
    }
    pub fn revision(&self, project: &Path) -> String {
        field(&self.cli(project, &["status"], true), "revision", "id").to_owned()
    }
    pub fn export(&self, project: Option<&Path>, name: &str) -> Dependency {
        let file = self.root.join(format!("{name}.lkjp"));
        let metadata = self.cli(
            project.unwrap_or(&self.root),
            &[
                "package",
                if project.is_some() {
                    "current"
                } else {
                    "builtin"
                },
                "export",
                "--kind",
                "transport",
                "--output",
                path(&file),
            ],
            true,
        );
        Dependency { metadata, file }
    }
    pub fn stage(&self, project: &Path, dependency: &Dependency) {
        let before = self.revision(project);
        self.cli(
            project,
            &[
                "package",
                "dependency",
                "stage",
                "--transport",
                field(&dependency.metadata, "package", "transport"),
                "--input-file",
                path(&dependency.file),
            ],
            true,
        );
        assert_eq!(self.revision(project), before);
    }
    pub fn author(
        &self,
        project: &Path,
        source: &str,
        dependencies: &[(&str, &Dependency)],
        initial: bool,
    ) {
        let mut input = format!("request base={}\n", self.revision(project));
        for (alias, dependency) in dependencies {
            let package = field(&dependency.metadata, "package", "id");
            let semantic = field(&dependency.metadata, "package", "revision");
            let revision = field(&dependency.metadata, "package", "package-revision");
            if initial {
                input.push_str(&format!("add.dependency package={package} semantic-revision={semantic} package-revision={revision}\n"));
            }
            if *alias != "builtin" {
                input.push_str(&format!("declarations.begin\n(units (use {alias} {package} {revision}))\ndeclarations.end\n"));
            }
        }
        input.push_str(source);
        let file = self.input(&format!("proposal-{:04}.lkjc", self.next()), &input);
        let plan = self.cli(
            project,
            &["change", "plan", "--input-file", path(&file)],
            true,
        );
        self.cli(
            project,
            &[
                "change",
                "apply",
                "--input-file",
                path(&file),
                "--plan",
                field(&plan, "plan", "token"),
            ],
            true,
        );
    }
    pub fn check(&self, project: &Path) {
        let output = self.cli(project, &["check"], true);
        assert_eq!(field(&output, "tests", "failed"), "0");
        assert_eq!(field(&output, "tests", "differential"), "equal");
    }
    pub fn unchanged(&self, project: &Path, module: &str) {
        let before = self.revision(project);
        let file = self.root.join(format!("draft-{module}.lkjc"));
        self.cli(
            project,
            &[
                "change",
                "draft",
                "--module",
                module,
                "--output",
                path(&file),
            ],
            true,
        );
        let output = self.cli(
            project,
            &["change", "plan", "--input-file", path(&file)],
            true,
        );
        assert_eq!(field(&output, "result", "outcome"), "unchanged");
        assert_eq!(self.revision(project), before);
    }
    pub fn compare(
        &self,
        name: &str,
        descriptor: &Path,
        arguments: &Path,
        expected: &Value,
    ) -> String {
        let result = self.root.join(format!("result-{name}.json"));
        assert!(!result.exists());
        let output = self.cli(
            &self.root,
            &[
                "run",
                "--deployment",
                path(descriptor),
                "--arguments-file",
                path(arguments),
                "--result-file",
                path(&result),
            ],
            true,
        );
        assert_eq!(
            &serde_json::from_slice::<Value>(&fs::read(result).unwrap()).unwrap(),
            expected,
            "{name}"
        );
        let observation = quoted(&output, "production-observation");
        for field in [
            "live_handles_after",
            "live_locals_after",
            "live_operands_after",
            "live_call_frames_after",
            "live_transactions_after",
            "live_type_bindings_after",
        ] {
            assert_eq!(observation[field], 0, "{name}: {field}");
        }
        let cleanup = quoted(&output, "cleanup");
        assert_eq!(cleanup["remaining_tasks"], 0);
        assert_eq!(cleanup["cleanup_failures"], serde_json::json!([]));
        assert_eq!(
            quoted(&output, "executor-observation")["remaining_workers"],
            0
        );
        output
    }
    pub fn detach(&self) {
        for name in ["library", "carriers", "application"] {
            fs::remove_dir_all(self.root.join(name)).unwrap();
        }
        for name in ["standard", "library", "carriers"] {
            fs::remove_file(self.root.join(format!("{name}.lkjp"))).unwrap();
        }
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        if std::thread::panicking()
            || std::env::var_os("LKJSCRIPT_KEEP_COMPONENT_PROOF").as_deref()
                == Some(std::ffi::OsStr::new("1"))
        {
            let root = self.temporary.take().unwrap().keep();
            eprintln!("retained native-component proof: {}", root.display());
        }
    }
}
