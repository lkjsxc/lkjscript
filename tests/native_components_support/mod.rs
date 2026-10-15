use crate::semantic_cli;
use lkjscript::platform::{
    Diagnostic, ExecutionControl, JsonLimits, RunPolicy, decode_deployment,
    native_tool::PureToolRunReceipt,
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

#[allow(
    dead_code,
    reason = "finite candidate batches are shared by selected matrix tests"
)]
pub mod batch;
pub mod cases;
mod fixture_pool;
pub mod oracle;
#[allow(
    dead_code,
    reason = "source differential batches are used by selected public matrices"
)]
pub mod source_batch;

pub(crate) fn copy_executable(source: &Path, destination: &Path) {
    let expected = digest(source);
    fs::copy(source, destination).unwrap();
    assert_eq!(expected, digest(source));
    assert_eq!(expected, digest(destination));
}

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
    mode: semantic_cli::Mode,
    memory: RefCell<BTreeMap<PathBuf, Arc<[u8]>>>,
    artifacts: fixture_pool::ArtifactPool,
    detached: Cell<bool>,
    detached_smokes: RefCell<BTreeSet<[u8; 32]>>,
}
impl Native {
    pub fn new() -> Self {
        if let Some(candidate) = std::env::var_os("LKJSCRIPT_COMPONENT_CANDIDATE") {
            return Self::from_candidate(Path::new(&candidate));
        }
        Self::create(
            Path::new(env!("CARGO_BIN_EXE_lkjscript")),
            semantic_cli::Mode::HostSemantic,
        )
    }
    // Explicit selection is authoritative; never consult an environment fallback here.
    pub fn from_candidate(candidate: &Path) -> Self {
        Self::create(candidate, semantic_cli::Mode::CandidateProcess)
    }
    fn create(candidate: &Path, mode: semantic_cli::Mode) -> Self {
        let expected = digest(candidate);
        let temporary = tempfile::Builder::new()
            .prefix("lkjscript-components-")
            .tempdir()
            .unwrap();
        let root = temporary.path().to_path_buf();
        fs::create_dir(root.join("home")).unwrap();

        let executable = semantic_cli::executable(&root, mode, candidate);
        if executable != root.join("lkjscript") {
            fs::hard_link(&executable, root.join("lkjscript")).unwrap();
        }
        assert_eq!(expected, digest(candidate));
        assert_eq!(expected, digest(&root.join("lkjscript")));
        Self {
            temporary: Some(temporary),
            root,
            serial: Cell::new(0),
            mode,
            memory: RefCell::new(BTreeMap::new()),
            artifacts: fixture_pool::ArtifactPool::default(),
            detached: Cell::new(false),
            detached_smokes: RefCell::new(BTreeSet::new()),
        }
    }
    fn next(&self) -> usize {
        let next = self.serial.get();
        self.serial.set(next + 1);
        next
    }
    pub fn execution_evidence(&self) -> Value {
        serde_json::json!({
            "mode": self.mode.label(),
            "artifact_pool": self.artifacts.observation(),
            "detached_binary_smokes": self.detached_smokes.borrow().len(),
        })
    }
    pub fn input(&self, name: &str, content: &str) -> PathBuf {
        let file = self.root.join(name);
        if self.mode == semantic_cli::Mode::HostSemantic && name.ends_with(".json") {
            let previous = self
                .memory
                .borrow_mut()
                .insert(file.clone(), Arc::from(content.as_bytes()));
            assert!(
                previous.is_none(),
                "proof input must be absent: {}",
                file.display()
            );
        } else {
            assert!(
                !file.exists(),
                "proof input must be absent: {}",
                file.display()
            );
            fs::write(&file, content).unwrap();
        }
        file
    }
    pub fn cli(&self, project: &Path, args: &[&str], success: bool) -> String {
        if self.mode == semantic_cli::Mode::HostSemantic {
            let response = if args.first() == Some(&"run") && args.contains(&"--deployment") {
                Some(self.run_prepared(args))
            } else {
                let mut arguments = args.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
                if !matches!(args.first(), Some(&"new" | &"capabilities"))
                    && !args.starts_with(&["package", "builtin"])
                {
                    arguments.extend(["--project".to_owned(), path(project).to_owned()]);
                }
                semantic_cli::execute(args, arguments)
            };
            if let Some(response) = response {
                if response.success && args.first() == Some(&"build") {
                    let output = option(args, "--output");
                    self.artifacts.built(Path::new(output)).unwrap();
                }
                let text = format!(
                    "{}{}",
                    String::from_utf8(response.stdout).unwrap(),
                    String::from_utf8(response.stderr).unwrap()
                );
                self.memory.borrow_mut().insert(
                    self.root.join(format!("operation-{:04}.log", self.next())),
                    format!(
                        "engine={} arguments={args:?}\nexit={:?}\n{text}",
                        self.mode.label(),
                        response.code
                    )
                    .into_bytes()
                    .into(),
                );
                assert_eq!(
                    response.success,
                    success,
                    "{args:?}\n{text}\nproof={}",
                    self.root.display()
                );
                return text;
            }
        }
        self.process_cli(project, args, success)
    }
    fn bytes(&self, file: &Path) -> Arc<[u8]> {
        self.memory
            .borrow()
            .get(file)
            .cloned()
            .unwrap_or_else(|| fs::read(file).unwrap().into())
    }
    fn run_prepared(&self, args: &[&str]) -> semantic_cli::Response {
        assert_eq!(
            args.len(),
            7,
            "native fixture supports one closed run route"
        );
        let descriptor_path = Path::new(option(args, "--deployment"));
        let arguments = self.bytes(Path::new(option(args, "--arguments-file")));
        let result_path = Path::new(option(args, "--result-file"));
        assert!(!self.memory.borrow().contains_key(result_path));
        let outcome: Result<_, Box<Diagnostic>> = (|| {
            let descriptor = decode_deployment(&self.bytes(descriptor_path))?;
            // These fixtures prove grant-free Command semantics. Other deployment behavior
            // belongs to actual process-boundary tests and must never be silently ignored.
            assert!(descriptor.grants.is_empty());
            assert!(descriptor.configuration.is_empty() && descriptor.secrets.is_empty());
            assert!(descriptor.runtime.is_none() && descriptor.listen.is_none());
            assert!(
                descriptor.http.is_none()
                    && descriptor.session.is_none()
                    && descriptor.worker.is_none()
            );
            let artifact = descriptor_path.parent().unwrap().join(&descriptor.artifact);
            let binding = self.artifacts.bind(&artifact)?;
            Ok(binding.tool.run_json_observed(
                &descriptor.target,
                &arguments,
                JsonLimits::default(),
                descriptor.execution.unwrap_or_else(RunPolicy::unmetered),
                &ExecutionControl::uncancelled(),
            )?)
        })();
        match outcome {
            Ok(receipt) => {
                let text = observed_output(&receipt);
                self.memory
                    .borrow_mut()
                    .insert(result_path.to_path_buf(), receipt.result_json.into());
                semantic_cli::Response {
                    code: Some(0),
                    success: true,
                    stdout: text.into_bytes(),
                    stderr: Vec::new(),
                }
            }
            Err(error) => semantic_cli::failure("run", &[*error]),
        }
    }
    fn process_cli(&self, project: &Path, args: &[&str], success: bool) -> String {
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
        if self.mode != semantic_cli::Mode::HostSemantic {
            assert!(!result.exists());
        }
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
            &serde_json::from_slice::<Value>(&self.bytes(&result)).unwrap(),
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
        if self.mode != semantic_cli::Mode::HostSemantic {
            let cleanup = quoted(&output, "cleanup");
            assert_eq!(cleanup["remaining_tasks"], 0);
            assert_eq!(cleanup["cleanup_failures"], serde_json::json!([]));
        }
        let executor = quoted(&output, "executor-observation");
        assert_eq!(executor["remaining_workers"], 0);
        assert_eq!(executor["active_dispatches"], 0);
        assert_eq!(executor["dispatch_open"], false);
        if self.mode == semantic_cli::Mode::HostSemantic && self.detached.get() {
            self.detached_boundary(descriptor, arguments, expected);
        }
        output
    }
    fn detached_boundary(&self, descriptor_path: &Path, arguments_path: &Path, expected: &Value) {
        let descriptor = decode_deployment(&self.bytes(descriptor_path)).unwrap();
        let artifact = descriptor_path.parent().unwrap().join(&descriptor.artifact);
        let binding = self.artifacts.bind(&artifact).unwrap();
        if self.detached_smokes.borrow().contains(&binding.digest) {
            return;
        }
        let fresh = self.artifacts.fresh_after_detachment(&artifact).unwrap();
        // Empty batches cover the detached executable boundary without duplicating the
        // entire matrix. Single targets retain their actual caller-supplied sample.
        let (arguments, expected) = if descriptor.target.contains("batch") {
            (Arc::from(b"[[]]".as_slice()), serde_json::json!([]))
        } else {
            (self.bytes(arguments_path), expected.clone())
        };
        let receipt = fresh
            .run_json_observed(
                &descriptor.target,
                &arguments,
                JsonLimits::default(),
                descriptor.execution.unwrap_or_else(RunPolicy::unmetered),
                &ExecutionControl::uncancelled(),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&receipt.result_json).unwrap(),
            expected
        );
        assert_eq!(receipt.executor.remaining_workers, 0);
        assert_eq!(receipt.executor.active_dispatches, 0);
        assert!(!receipt.executor.dispatch_open);
        let serial = self.next();
        let smoke_descriptor = self.root.join(format!("boundary-{serial}.deployment.json"));
        let smoke_arguments = self.root.join(format!("boundary-{serial}.arguments.json"));
        let smoke_result = self.root.join(format!("boundary-{serial}.result.json"));
        fs::write(&smoke_descriptor, self.bytes(descriptor_path)).unwrap();
        fs::write(&smoke_arguments, arguments).unwrap();
        let output = self.process_cli(
            &self.root,
            &[
                "run",
                "--deployment",
                path(&smoke_descriptor),
                "--arguments-file",
                path(&smoke_arguments),
                "--result-file",
                path(&smoke_result),
            ],
            true,
        );
        assert_eq!(
            serde_json::from_slice::<Value>(&fs::read(smoke_result).unwrap()).unwrap(),
            expected
        );
        let cleanup = quoted(&output, "cleanup");
        assert_eq!(cleanup["remaining_tasks"], 0);
        assert_eq!(cleanup["cleanup_failures"], serde_json::json!([]));
        assert_eq!(
            quoted(&output, "executor-observation")["remaining_workers"],
            0
        );
        assert_eq!(
            binding.digest,
            <[u8; 32]>::from(Sha256::digest(fs::read(artifact).unwrap()))
        );
        self.detached_smokes.borrow_mut().insert(binding.digest);
    }
    pub fn sources_detached(&self) {
        self.detached.set(true);
    }
    pub fn detach(&self) {
        for name in ["library", "carriers", "application"] {
            fs::remove_dir_all(self.root.join(name)).unwrap();
        }
        for name in ["standard", "library", "carriers"] {
            fs::remove_file(self.root.join(format!("{name}.lkjp"))).unwrap();
        }
        self.sources_detached();
    }
}
impl Drop for Native {
    fn drop(&mut self) {
        if std::thread::panicking()
            || std::env::var_os("LKJSCRIPT_KEEP_COMPONENT_PROOF").as_deref()
                == Some(std::ffi::OsStr::new("1"))
        {
            for (file, bytes) in self.memory.get_mut().iter() {
                if !file.exists() {
                    fs::write(file, bytes).unwrap();
                }
            }
            let root = self.temporary.take().unwrap().keep();
            eprintln!("retained native-component proof: {}", root.display());
        }
    }
}

fn option<'a>(args: &[&'a str], name: &str) -> &'a str {
    args.windows(2).find(|pair| pair[0] == name).unwrap()[1]
}

fn observed_output(receipt: &PureToolRunReceipt) -> String {
    format!(
        "run engine=host-semantic production-observation={} executor-observation={}\n",
        serde_json::to_string(&serde_json::to_string(&receipt.production).unwrap()).unwrap(),
        serde_json::to_string(&serde_json::to_string(&receipt.executor).unwrap()).unwrap(),
    )
}
