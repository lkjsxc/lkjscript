//! Consumer-first associated read views through host adapters and explicit copied candidates.
//! Every meaning change enters through literal public requests.
use super::native_byte_buffer::{Export, dependency};
use super::*;
use serde_json::json;

const CONSUMER: &str = include_str!("read_view_fixtures/01-consumer.lkjc");
const TRANSFORMER: &str = include_str!("read_view_fixtures/02-transformer.lkjc");
const FLAT: &str = include_str!("read_view_fixtures/03-flat.lkjc");
const FLAT_JOINED: &str = include_str!("read_view_fixtures/03-flat-joined.lkjc");
const CHUNKED: &str = include_str!("read_view_fixtures/04-chunked.lkjc");
const CHUNKED_JOINED: &str = include_str!("read_view_fixtures/04-chunked-joined.lkjc");
const APPLICATION: &str = include_str!("read_view_fixtures/05-application.lkjc");
const APPLICATION_JOINED: &str = include_str!("read_view_fixtures/05-application-joined.lkjc");
const CASES: &str = include_str!("read_view_fixtures/cases.json");
const MEMBER: &str = "assoc_type_d8100000000000000000000000000001";
const METHODS: [&str; 5] = [
    "method_d8100000000000000000000000000001",
    "method_d8100000000000000000000000000002",
    "method_d8100000000000000000000000000003",
    "method_d8100000000000000000000000000004",
    "method_d8100000000000000000000000000005",
];
const TARGETS: [&str; 3] = ["readview", "readview-flat", "readview-chunked"];

fn archive(public: &Native, name: &str, bytes: &[u8]) {
    let evidence = public.root.path().join("literal-evidence");
    std::fs::create_dir_all(&evidence).unwrap();
    std::fs::write(evidence.join(name), bytes).unwrap();
}

// Finite host semantics use the same public adapters as the other Native fixtures.
// Actual process routes retain owned HOME/TMPDIR, LANG and their original terminals.
fn cli(public: &Native, arguments: &[&str], success: bool) -> Vec<CompactRecord> {
    static SERIAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let evidence = public.root.path().join("cli-evidence");
    let mut command = Command::new(&public.executable);
    if !arguments.starts_with(&["package", "builtin"])
        && !arguments.starts_with(&["run", "--deployment"])
    {
        command.arg("--project").arg(&public.project);
    }
    command
        .args(arguments)
        .current_dir(public.root.path())
        .env_clear()
        .env("PATH", "")
        .env("LANG", "C")
        .env("HOME", public.root.path().join("home"))
        .env("TMPDIR", public.root.path().join("tmp"));
    let retained = std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some();
    let full_arguments = command
        .get_args()
        .map(|v| v.to_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    let semantic = (public.mode == semantic_cli::Mode::HostSemantic)
        .then(|| semantic_cli::execute(arguments, full_arguments.clone()))
        .flatten();
    let mode = if semantic.is_some() {
        semantic_cli::Mode::HostSemantic
    } else if public.mode == semantic_cli::Mode::CandidateProcess {
        public.mode
    } else {
        semantic_cli::Mode::BoundaryProcess
    };
    let retain_terminal = mode != semantic_cli::Mode::HostSemantic || retained;
    let stem = format!("{serial:08}");
    // Retain process invocations before starting them; a launch failure has an original.
    if retain_terminal {
        std::fs::create_dir_all(&evidence).unwrap();
        std::fs::write(
        evidence.join(format!("{stem}.command.json")),
        serde_json::to_vec_pretty(&json!({
            "executable": (mode != semantic_cli::Mode::HostSemantic).then_some(&public.executable),
            "directory": public.root.path(),
            "arguments": full_arguments,
            "execution_mode": mode.label(),
            "environment_cleared": mode != semantic_cli::Mode::HostSemantic,
            "environment": if mode == semantic_cli::Mode::HostSemantic { Value::Null } else { json!({"PATH":"","LANG":"C",
                "HOME":public.root.path().join("home"),"TMPDIR":public.root.path().join("tmp")}) },
            "candidate_selection": (mode == semantic_cli::Mode::CandidateProcess).then_some("copied binary() bytes"),
            "retain_product_evidence": retained,
            "expected_success": success,
        }))
        .unwrap(),
    )
    .unwrap();
    }
    let output = semantic.unwrap_or_else(|| support::output(&mut command).unwrap().into());
    if retain_terminal {
        std::fs::write(evidence.join(format!("{stem}.stdout")), &output.stdout).unwrap();
        std::fs::write(evidence.join(format!("{stem}.stderr")), &output.stderr).unwrap();
        std::fs::write(
            evidence.join(format!("{stem}.terminal.json")),
            serde_json::to_vec_pretty(&json!({"status":output.code,"success":output.success}))
                .unwrap(),
        )
        .unwrap();
    }
    assert_eq!(
        output.success,
        success,
        "{arguments:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    parse_records("associated read views public result", &output.stdout).unwrap()
}

fn native(template: &str) -> Native {
    let mut root = tempfile::tempdir().unwrap();
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        root.disable_cleanup(true);
        println!(
            "retained read-view public evidence: {}",
            root.path().display()
        );
    }
    for directory in ["home", "tmp"] {
        std::fs::create_dir_all(root.path().join(directory)).unwrap();
    }
    let selected = binary();
    let mode = semantic_cli::Mode::selected();
    let executable = semantic_cli::executable(root.path(), mode, &selected);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o555)).unwrap();
        assert_eq!(
            std::fs::metadata(&executable).unwrap().permissions().mode() & 0o777,
            0o555
        );
    }
    let public = Native {
        project: root.path().join("project"),
        root,
        executable,
        mode,
    };
    archive(&public, "candidate-selection.json", &serde_json::to_vec_pretty(&json!({
        "selected":selected,"copy":public.executable,"execution_mode":mode.label(),
        "blake3":blake3::hash(&std::fs::read(&public.executable).unwrap()).to_hex().to_string(),
        "source_tree":env!("CARGO_MANIFEST_DIR"),
        "regular_file":std::fs::symlink_metadata(&public.executable).unwrap().file_type().is_file(),
    })).unwrap());
    cli(
        &public,
        &[
            "new",
            path(&public.project),
            "--template",
            template,
            "--name",
            "associated-read-views",
        ],
        true,
    );
    public
}

fn revision(public: &Native) -> String {
    compact_field(
        compact_record(&cli(public, &["status"], true), "revision"),
        "id",
    )
    .to_owned()
}

fn plan(public: &Native, input: &Path, success: bool) -> Vec<CompactRecord> {
    cli(
        public,
        &["change", "plan", "--input-file", path(input)],
        success,
    )
}

fn apply(public: &Native, input: &Path, reviewed: &[CompactRecord]) {
    cli(
        public,
        &[
            "change",
            "apply",
            "--input-file",
            path(input),
            "--plan",
            compact_field(compact_record(reviewed, "plan"), "token"),
        ],
        true,
    );
}

fn check(public: &Native) {
    let records = cli(public, &["check"], true);
    let tests = compact_record(&records, "tests");
    assert_eq!(compact_field(tests, "failed"), "0");
    assert_eq!(compact_field(tests, "differential"), "equal");
}

fn accept(public: &Native, label: &str, literal: &str) {
    let before = revision(public);
    let request = format!("request base={before}\n{literal}");
    archive(public, &format!("{label}-request.lkjc"), request.as_bytes());
    let input = public.input(&format!("{label}.lkjc"), &request);
    let reviewed = plan(public, &input, true);
    apply(public, &input, &reviewed);
    assert_ne!(revision(public), before);
    check(public);
}

fn draft(public: &Native, module: &str) -> String {
    static SERIAL: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let serial = SERIAL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("{module}-draft-{serial:08}.lkjc");
    let before = revision(public);
    let input = public.root.path().join(&name);
    cli(
        public,
        &[
            "change",
            "draft",
            "--module",
            module,
            "--output",
            path(&input),
        ],
        true,
    );
    let literal = std::fs::read_to_string(&input).unwrap();
    archive(public, &name, literal.as_bytes());
    // Full set/re-entry retains all unused methods, scope and field inventories.
    for _ in 0..2 {
        let reviewed = plan(public, &input, true);
        assert_eq!(
            compact_field(compact_record(&reviewed, "result"), "outcome"),
            "unchanged"
        );
        assert_eq!(revision(public), before);
    }
    literal
}

fn declaration(public: &Native, module: &str, name: &str) -> String {
    let modules = cli(public, &["query", "find", "module", module], true);
    let parent = compact_field(compact_record(&modules, "owner"), "id");
    let owners = cli(
        public,
        &["query", "find", "declaration", name, "--parent", parent],
        true,
    );
    compact_field(compact_record(&owners, "owner"), "id").to_owned()
}

fn extend(public: &Native, label: &str, module: &str, literal: &str, prefix: &str) {
    let owners = cli(public, &["query", "find", "module", module], true);
    let identity = compact_field(compact_record(&owners, "owner"), "id");
    assert!(literal.contains("MODULE_OWNER"));
    accept(
        public,
        label,
        &format!("{prefix}{}", literal.replace("MODULE_OWNER", identity)),
    );
}

fn set_unused_family(public: &Native) {
    let identity = declaration(public, "readview-flat", "Dormant");
    let before = revision(public);
    let original = "(scope-parameter read_scope_d8300000000000000000000000000022 s)";
    let changed = "(scope-parameter read_scope_d8300000000000000000000000000022 spare-s)";
    let complete = draft(public, "readview-flat");
    assert_eq!(
        complete.matches(original).count(),
        1,
        "exact unused family formal"
    );
    let source = complete.replace(original, changed);
    archive(public, "unused-family-full-set.lkjc", source.as_bytes());
    let input = public.input("unused-family-full-set.lkjc", &source);
    let reviewed = plan(public, &input, true);
    apply(public, &input, &reviewed);
    assert_ne!(revision(public), before);
    assert_eq!(declaration(public, "readview-flat", "Dormant"), identity);
    let changed_draft = draft(public, "readview-flat");
    assert_eq!(changed_draft.matches(changed).count(), 1);
    let restored = changed_draft.replace(changed, original);
    archive(public, "unused-family-restore.lkjc", restored.as_bytes());
    let input = public.input("unused-family-restore.lkjc", &restored);
    let reviewed = plan(public, &input, true);
    apply(public, &input, &reviewed);
    assert_eq!(declaration(public, "readview-flat", "Dormant"), identity);
    assert!(draft(public, "readview-flat").contains(original));
    check(public);
}

fn exported(public: &Native, name: &str, builtin: bool) -> Export {
    let output = public.root.path().join(format!("{name}.lkjp"));
    let selector = if builtin { "builtin" } else { "current" };
    let records = cli(
        public,
        &[
            "package",
            selector,
            "export",
            "--kind",
            "transport",
            "--output",
            path(&output),
        ],
        true,
    );
    let package = compact_record(&records, "package");
    let exported = Export {
        path: output,
        package: compact_field(package, "id").into(),
        semantic: compact_field(package, "revision").into(),
        revision: compact_field(package, "package-revision").into(),
        transport: compact_field(package, "transport").into(),
    };
    archive(public, &format!("{name}-export.json"), &serde_json::to_vec_pretty(&json!({
        "package":exported.package,"semantic_revision":exported.semantic,
        "package_revision":exported.revision,"transport":exported.transport,"transport_file":exported.path,
        "blake3":blake3::hash(&std::fs::read(&exported.path).unwrap()).to_hex().to_string(),
    })).unwrap());
    exported
}

fn stage(public: &Native, package: &Export) {
    cli(
        public,
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            &package.transport,
            "--input-file",
            path(&package.path),
        ],
        true,
    );
}

fn imports(packages: &[(&str, &Export)]) -> String {
    packages
        .iter()
        .map(|(name, package)| {
            format!(
                "declarations.begin\n(units (use {name} {} {}))\ndeclarations.end\n",
                package.package, package.revision,
            )
        })
        .collect()
}

fn bind_export(public: &Native, imported: &Native, package: &Export, module: &str, name: &str) {
    let local = declaration(public, module, name);
    // Interface top declarations have no module parent. Query the exact staged
    // package revision without a parent, then match its proven local identity.
    let records = cli(
        imported,
        &[
            "package",
            "dependency",
            "query",
            "owners",
            "--package-revision",
            &package.revision,
            "--name",
            name,
        ],
        true,
    );
    let owners: Vec<_> = records.iter().filter(|r| r.operation == "owner").collect();
    assert_eq!(owners.len(), 1, "exact exported owner {module}::{name}");
    assert_eq!(compact_field(owners[0], "id"), local);
    archive(public, &format!("binding-{module}-{name}.json"), &serde_json::to_vec_pretty(&json!({
        "package":package.package,"package_revision":package.revision,"local_owner":local,"name":name,
    })).unwrap());
}

fn inspect_consumer(public: &Native, text: &str) {
    for token in [
        "read-view-family",
        "associated-read-view-member",
        "associated-read-view-projection",
        "scope-source",
        "view-from",
        "read-from",
        "view-borrow",
        "SourceLength",
    ] {
        assert!(text.contains(token), "consumer readback {token}");
    }
    for forbidden in [
        "FlatSpan",
        "ChunkedSpan",
        "ChunkStorage",
        "owned-implementation",
    ] {
        assert!(
            !text.contains(forbidden),
            "consumer authored before suppliers: {forbidden}"
        );
    }
    let contract = declaration(public, "readview-consumer", "RangeRead");
    let records = cli(
        public,
        &["inspect", "owner", "owned_contract", &contract],
        true,
    );
    let methods: Vec<_> = records
        .iter()
        .filter(|r| r.operation == "owned.method")
        .collect();
    assert_eq!(methods.len(), METHODS.len());
    for (method, identity) in methods.iter().zip(METHODS) {
        assert_eq!(compact_field(method, "id"), identity);
        assert_eq!(compact_field(method, "kind"), "pure");
    }
    assert!(text.contains(MEMBER));
    // The normal Owned helper and the actual generic reducer already exist here.
    for name in ["source-length", "observe-range", "observe-view"] {
        let owner = declaration(public, "readview-consumer", name);
        cli(
            public,
            &[
                "inspect",
                "owner",
                "pure_function",
                &owner,
                "--detail",
                "definition",
                "--limit",
                "1000",
            ],
            true,
        );
    }
}

struct Packages {
    authors: [Native; 4],
    application: Native,
    exports: [Export; 4],
    standards: Vec<PathBuf>,
    identities: Vec<(PathBuf, blake3::Hash)>,
    source_archive: tempfile::TempDir,
}

impl Packages {
    fn stage() -> Self {
        let consumer = native("minimal");
        let standard = exported(&consumer, "standard", true);
        stage(&consumer, &standard);
        accept(
            &consumer,
            "consumer",
            &format!("{}{CONSUMER}", dependency(&standard)),
        );
        inspect_consumer(&consumer, &draft(&consumer, "readview-consumer"));
        let generic = exported(&consumer, "consumer", false);
        let mut standards = vec![standard.path];

        let transformer = native("minimal");
        stage(&transformer, &generic);
        accept(
            &transformer,
            "transformer",
            &format!(
                "{}{}{TRANSFORMER}",
                dependency(&generic),
                imports(&[("readview-consumer", &generic)])
            ),
        );
        let text = draft(&transformer, "readview-transformer");
        for token in ["view-reborrow", "view-move", "view-drop", "view-from"] {
            assert!(text.contains(token), "independent transformer {token}");
        }
        assert!(!text.contains("FlatSpan") && !text.contains("ChunkedSpan"));
        let transformed = exported(&transformer, "transformer", false);

        let flat = native("minimal");
        let standard = exported(&flat, "standard", true);
        stage(&flat, &standard);
        stage(&flat, &generic);
        accept(
            &flat,
            "flat",
            &format!(
                "{}{}{}{FLAT}",
                dependency(&standard),
                dependency(&generic),
                imports(&[("readview-consumer", &generic)])
            ),
        );
        extend(&flat, "flat-joined", "readview-flat", FLAT_JOINED, "");
        set_unused_family(&flat);
        let text = draft(&flat, "readview-flat");
        for token in [
            "FlatSpan",
            "read-reference",
            "view-create",
            "borrow-view-field",
            "view-data-field",
            "Metadata",
            "Zero",
            "parallel",
            "Dormant",
        ] {
            assert!(text.contains(token), "Flat supplier {token}");
        }
        let flat_export = exported(&flat, "flat", false);
        standards.push(standard.path);

        let chunked = native("minimal");
        let standard = exported(&chunked, "standard", true);
        stage(&chunked, &standard);
        stage(&chunked, &generic);
        accept(
            &chunked,
            "chunked",
            &format!(
                "{}{}{}{CHUNKED}",
                dependency(&standard),
                dependency(&generic),
                imports(&[("readview-consumer", &generic)])
            ),
        );
        extend(
            &chunked,
            "chunked-joined",
            "readview-chunked",
            CHUNKED_JOINED,
            "",
        );
        let text = draft(&chunked, "readview-chunked");
        for token in [
            "ChunkedSpan",
            "ChunkBlock",
            "ChunkStorage",
            "borrow-owned-item",
            "borrow-owned-field",
            "borrow-view-field",
        ] {
            assert!(text.contains(token), "private Chunked supplier {token}");
        }
        let chunked_export = exported(&chunked, "chunked", false);
        standards.push(standard.path);
        assert_ne!(flat_export.package, chunked_export.package);
        assert!(
            !std::fs::read_to_string(flat.root.path().join("flat.lkjc"))
                .unwrap()
                .contains(&chunked_export.package)
        );
        assert!(
            !std::fs::read_to_string(chunked.root.path().join("chunked.lkjc"))
                .unwrap()
                .contains(&flat_export.package)
        );

        let application = native("command");
        let exports = [generic, transformed, flat_export, chunked_export];
        for package in &exports {
            stage(&application, package);
        }
        for (author, package, module, names) in [
            (
                &consumer,
                &exports[0],
                "readview-consumer",
                &[
                    "RangeRead",
                    "SourceLength",
                    "SourceDrain",
                    "observe-range",
                    "observe-view",
                    "source-length",
                ][..],
            ),
            (
                &transformer,
                &exports[1],
                "readview-transformer",
                &["outer", "subrange-and-forward", "forward-first"][..],
            ),
            (
                &flat,
                &exports[2],
                "readview-flat",
                &[
                    "Flat",
                    "FlatLength",
                    "FlatDrain",
                    "build",
                    "Metadata",
                    "Zero",
                    "VisibleSpan",
                    "joined-range",
                    "joined-failure",
                    "metadata-custody-join",
                ][..],
            ),
            (
                &chunked,
                &exports[3],
                "readview-chunked",
                &[
                    "Chunked",
                    "ChunkedLength",
                    "ChunkedDrain",
                    "ChunkStorage",
                    "build",
                    "joined-range",
                ][..],
            ),
        ] {
            for name in names {
                bind_export(author, &application, package, module, name);
            }
        }
        let mut source_archive = tempfile::tempdir().unwrap();
        if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
            source_archive.disable_cleanup(true);
            println!(
                "retained independent read-view source archive: {}",
                source_archive.path().display()
            );
        }
        let packages = Self {
            authors: [consumer, transformer, flat, chunked],
            application,
            exports,
            standards,
            identities: Vec::new(),
            source_archive,
        };
        accept(
            &packages.application,
            "application",
            &format!(
                "{}{}{APPLICATION}",
                packages.exports.iter().map(dependency).collect::<String>(),
                packages.imports()
            ),
        );
        extend(
            &packages.application,
            "application-joined",
            "readview-app",
            APPLICATION_JOINED,
            &packages.imports(),
        );
        let text = draft(&packages.application, "readview-app");
        assert!(text.contains("associated-read-view-projection"));
        assert!(!text.contains("FlatSpan") && !text.contains("ChunkedSpan"));
        assert!(!text.contains("borrow-view-field"));
        archive(&packages.application, "cases.json", CASES.as_bytes());
        packages
    }

    fn imports(&self) -> String {
        imports(&[
            ("readview-consumer", &self.exports[0]),
            ("readview-transformer", &self.exports[1]),
            ("readview-flat", &self.exports[2]),
            ("readview-chunked", &self.exports[3]),
        ])
    }

    fn refused(
        &self,
        public: &Native,
        label: &str,
        literal: &str,
        with_imports: bool,
        private: bool,
    ) {
        let before = revision(public);
        let head = std::fs::read(public.project.join("HEAD")).unwrap();
        let request = format!(
            "request base={before}\n{}{literal}",
            if with_imports {
                self.imports()
            } else {
                String::new()
            }
        );
        let input = public.input(&format!("invalid-{label}.lkjc"), &request);
        archive(public, &format!("invalid-{label}.lkjc"), request.as_bytes());
        let records = plan(public, &input, false);
        let error = compact_record(&records, "diagnostic");
        let code = compact_field(error, "code");
        assert!(
            !code.contains("unsupported"),
            "missing implementation cannot satisfy semantic refusal: {label}: {code}"
        );
        assert_eq!(
            compact_field(error, "class"),
            "semantic",
            "semantic boundary {label}: {records:?}"
        );
        if private {
            assert_eq!(
                code, "kernel_read_view",
                "private typed inline field authority {label}"
            );
            assert!(compact_field(error, "message").contains("private defining module"));
        } else {
            assert!(
                code.starts_with("kernel_"),
                "semantic admission must own {label}: {code}"
            );
        }
        assert_eq!(revision(public), before);
        assert_eq!(std::fs::read(public.project.join("HEAD")).unwrap(), head);
    }

    fn build(&mut self) -> [PathBuf; 3] {
        let artifact = self.application.root.path().join("readview.lkja");
        let second = self.application.root.path().join("readview-second.lkja");
        cli(
            &self.application,
            &["build", "--output", path(&artifact)],
            true,
        );
        cli(
            &self.application,
            &["build", "--output", path(&second)],
            true,
        );
        assert_eq!(
            std::fs::read(&artifact).unwrap(),
            std::fs::read(&second).unwrap()
        );
        let descriptors =
            TARGETS.map(|target| deployment(&self.application, "readview.lkja", target));
        let mut files = vec![artifact, second, self.application.executable.clone()];
        files.extend(self.authors.iter().map(|a| a.executable.clone()));
        files.extend(descriptors.iter().cloned());
        self.identities = files
            .into_iter()
            .map(|file| {
                let hash = blake3::hash(&std::fs::read(&file).unwrap());
                (file, hash)
            })
            .collect();
        descriptors
    }

    fn verify_identity(&self) {
        for (file, expected) in &self.identities {
            assert_eq!(blake3::hash(&std::fs::read(file).unwrap()), *expected);
        }
    }

    fn detach(&self) {
        // Only these disposable test projects are removed. Copy and independently
        // byte-check their complete trees in a separate root before deletion.
        for (index, public) in self
            .authors
            .iter()
            .chain(std::iter::once(&self.application))
            .enumerate()
        {
            let archive_root = self.source_archive.path().join(index.to_string());
            std::fs::create_dir_all(&archive_root).unwrap();
            let copied = archive_root.join("project");
            copy_source_tree(&public.project, &copied);
            compare_source_tree(&public.project, &copied);
            std::fs::remove_dir_all(&public.project).unwrap();
            assert!(!public.project.exists());
            for entry in std::fs::read_dir(public.root.path()).unwrap() {
                let file = entry.unwrap().path();
                if source_file(&file) {
                    let destination = archive_root.join(file.file_name().unwrap());
                    std::fs::copy(&file, &destination).unwrap();
                    assert_eq!(
                        std::fs::read(&file).unwrap(),
                        std::fs::read(&destination).unwrap()
                    );
                    std::fs::remove_file(&file).unwrap();
                }
            }
            let literals = public.root.path().join("literal-evidence");
            for entry in std::fs::read_dir(&literals).unwrap() {
                let file = entry.unwrap().path();
                if source_file(&file) {
                    let destination = archive_root.join(format!(
                        "literal-{}",
                        file.file_name().unwrap().to_str().unwrap()
                    ));
                    std::fs::copy(&file, &destination).unwrap();
                    assert_eq!(
                        std::fs::read(&file).unwrap(),
                        std::fs::read(&destination).unwrap()
                    );
                    std::fs::remove_file(&file).unwrap();
                }
            }
            assert_no_source(public.root.path());
        }
        for file in self
            .exports
            .iter()
            .map(|p| &p.path)
            .chain(self.standards.iter())
        {
            assert!(!file.exists());
        }
        self.verify_identity();
    }
}

fn source_file(path: &Path) -> bool {
    path.extension()
        .is_some_and(|suffix| suffix == "lkjc" || suffix == "lkjp")
}

fn copy_source_tree(source: &Path, destination: &Path) {
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        let kind = entry.file_type().unwrap();
        assert!(
            !kind.is_symlink(),
            "test-owned source archive contains a symlink"
        );
        if kind.is_dir() {
            copy_source_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn compare_source_tree(source: &Path, destination: &Path) {
    let mut actual = std::fs::read_dir(source)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect::<Vec<_>>();
    let mut copied = std::fs::read_dir(destination)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect::<Vec<_>>();
    actual.sort();
    copied.sort();
    assert_eq!(actual, copied);
    for name in actual {
        let source = source.join(&name);
        let destination = destination.join(name);
        if source.is_dir() {
            compare_source_tree(&source, &destination);
        } else {
            assert_eq!(
                std::fs::read(source).unwrap(),
                std::fs::read(destination).unwrap()
            );
        }
    }
}

fn assert_no_source(root: &Path) {
    for entry in std::fs::read_dir(root).unwrap() {
        let file = entry.unwrap().path();
        assert!(
            !source_file(&file),
            "detached execution root retained source: {}",
            file.display()
        );
        if file.is_dir() {
            assert_no_source(&file);
        }
    }
}

fn deployment(public: &Native, artifact: &str, target: &str) -> PathBuf {
    public.input(&format!("{target}.deployment.json"), &json!({
        "artifact":artifact,"target":target,"listen":null,"http":null,"session":null,"worker":null,
        "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,"maximum_total_bytes":1048576,"maximum_live_streams":1024},
        "grants":[],"secrets":[],"configuration":{},
    }).to_string())
}

fn run(
    public: &Native,
    target: &str,
    descriptor: &Path,
    lane: &str,
    label: &str,
    arguments: &Value,
    expected: Option<&Value>,
) -> Vec<CompactRecord> {
    let input = public.input(
        &format!("{target}-{lane}-{label}.arguments.json"),
        &arguments.to_string(),
    );
    let result = public
        .root
        .path()
        .join(format!("{target}-{lane}-{label}.result.json"));
    let mut arguments = if lane == "project" {
        vec!["run", target]
    } else {
        vec!["run", "--deployment", path(descriptor)]
    };
    arguments.extend([
        "--arguments-file",
        path(&input),
        "--result-file",
        path(&result),
    ]);
    let records = cli(public, &arguments, expected.is_some());
    if let Some(expected) = expected {
        archive(
            public,
            &format!("{target}-{lane}-{label}.expected.json"),
            &serde_json::to_vec(expected).unwrap(),
        );
        assert_eq!(
            std::fs::read(&result).unwrap(),
            serde_json::to_vec(expected).unwrap(),
            "{target}/{lane}/{label}"
        );
        let execution = compact_record(&records, "execution");
        if lane == "project" {
            assert_eq!(compact_field(execution, "differential"), "equal");
            assert!(
                compact_field(execution, "production-instructions")
                    .parse::<u64>()
                    .unwrap()
                    > 0
            );
            assert!(
                compact_field(execution, "reference-expressions")
                    .parse::<u64>()
                    .unwrap()
                    > 0
            );
            // CLI does not expose reference cleanup counters; do not invent them.
        } else {
            super::native_indexed_data::joined(execution);
        }
    } else {
        assert!(!result.exists(), "failed run published a result {label}");
        let diagnostic = compact_record(&records, "diagnostic");
        assert_ne!(
            compact_field(diagnostic, "class"),
            "source",
            "runtime failure requires admitted inputs"
        );
        if lane != "project" {
            let notes: Vec<String> =
                serde_json::from_str(compact_field(diagnostic, "notes")).unwrap();
            assert!(notes.iter().any(|n| n == "foreground cleanup: admission-stopped=true remaining-owned-tasks=0 failures=0"));
            let joined = notes.iter().find_map(|note| note.strip_prefix(
                "structured worker cleanup: dispatch-stopped=true active=0 remaining-workers=0 joined-workers="))
                .expect("failure retained its complete joined worker cleanup").parse::<u64>().unwrap();
            if target == "readview-joined-failure" {
                assert!(joined > 0, "failing child was joined");
            }
        }
    }
    records
}

struct PureCase {
    label: String,
    target: &'static str,
    arguments: Value,
    expected: Option<Value>,
}

fn run_pure_batch(public: &Native, descriptor: &Path, lane: &str, label: &str, cases: &[PureCase]) {
    let requests = cases
        .iter()
        .map(|case| {
            let id = format!("{}-{}", case.label, case.target);
            let mut request = if let Some(expected) = &case.expected {
                archive(
                    public,
                    &format!("{}-{lane}-{}.expected.json", case.target, case.label),
                    &serde_json::to_vec(expected).unwrap(),
                );
                BatchCase::success(id, &case.arguments, expected.clone())
            } else {
                BatchCase::failure(
                    id,
                    &case.arguments,
                    "semantic",
                    "normalized_integer_division",
                )
            };
            request.target = Some(case.target.to_owned());
            request
        })
        .collect::<Vec<_>>();
    // The source reader retains an ordinary differential run and source witness;
    // both readers independently check full results, authority and actual cleanup.
    let records = if lane == "project" {
        public.source_batch(label, "readview", &requests)
    } else {
        public.batch(label, descriptor, &requests)
    };
    let result_path = public.root.path().join(if lane == "project" {
        format!("source-batch-{label}.result.jsonl")
    } else {
        format!("batch-{label}.result.jsonl")
    });
    let rows = std::fs::read_to_string(result_path).unwrap();
    let rows = rows
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(records.len(), cases.len());
    assert_eq!(rows.len(), cases.len());
    for ((records, row), case) in records.iter().zip(&rows).zip(cases) {
        if let Some(expected) = &case.expected {
            // Preserve the original stronger canonical-byte check, in addition
            // to the shared reader's independent structural result comparison.
            assert_eq!(
                row["result_json"].as_str().unwrap().as_bytes(),
                serde_json::to_vec(expected).unwrap(),
                "{}/{lane}/{}",
                case.target,
                case.label
            );
            let execution = compact_record(records, "execution");
            assert_eq!(compact_field(execution, "target"), case.target);
            if lane == "project" {
                assert_eq!(compact_field(execution, "differential"), "equal");
                for field in ["production-instructions", "reference-expressions"] {
                    assert!(compact_field(execution, field).parse::<u64>().unwrap() > 0);
                }
            } else {
                super::native_indexed_data::joined(execution);
            }
        } else {
            assert!(
                row["result_json"].is_null(),
                "failed row published a result"
            );
            let diagnostic = compact_record(records, "diagnostic");
            assert_eq!(compact_field(diagnostic, "class"), "semantic");
            assert_eq!(
                compact_field(diagnostic, "code"),
                "normalized_integer_division"
            );
            let notes: Vec<String> =
                serde_json::from_str(compact_field(diagnostic, "notes")).unwrap();
            if lane == "project" {
                for observation in ["production-observation: ", "executor-observation: "] {
                    assert!(notes.iter().any(|note| note.starts_with(observation)));
                }
            } else {
                assert!(notes.iter().any(|note| note == "foreground cleanup: admission-stopped=true remaining-owned-tasks=0 failures=0"));
                notes.iter().find_map(|note| note.strip_prefix(
                    "structured worker cleanup: dispatch-stopped=true active=0 remaining-workers=0 joined-workers="))
                    .expect("failure retained its complete joined worker cleanup").parse::<u64>().unwrap();
            }
        }
    }
}

fn literal_cases() -> Vec<Value> {
    serde_json::from_str(CASES).unwrap()
}

fn expected_for(target: &str, case: &Value) -> Value {
    match target {
        "readview" => json!({"flat":case["expected"],"chunked":case["expected"]}),
        "readview-flat" | "readview-chunked" => case["expected"].clone(),
        _ => unreachable!("fixed literal result targets"),
    }
}

#[test]
fn native_associated_read_views_consumer_first_five_literals_parent_drop_and_detached_bytes() {
    let mut packages = Packages::stage();
    let descriptors = packages.build();
    let cases = literal_cases();
    let metadata_targets = ["readview-metadata", "readview-zero"];
    let metadata_descriptors =
        metadata_targets.map(|target| deployment(&packages.application, "readview.lkja", target));
    for descriptor in &metadata_descriptors {
        packages.identities.push((
            descriptor.clone(),
            blake3::hash(&std::fs::read(descriptor).unwrap()),
        ));
    }
    for lane in ["project", "attached-artifact", "detached-artifact"] {
        if lane == "detached-artifact" {
            packages.detach();
        }
        let mut requests = Vec::new();
        for case in &cases {
            for target in TARGETS {
                requests.push(PureCase {
                    label: case["name"].as_str().unwrap().to_owned(),
                    target,
                    arguments: case["arguments"].clone(),
                    expected: Some(expected_for(target, case)),
                });
            }
            for target in metadata_targets {
                requests.push(PureCase {
                    label: case["name"].as_str().unwrap().to_owned(),
                    target,
                    arguments: case["arguments"].clone(),
                    expected: Some(case["expected"]["source_length"].clone()),
                });
            }
        }
        assert_eq!(
            requests.len(),
            25,
            "all five literals and five exact targets"
        );
        run_pure_batch(
            &packages.application,
            &descriptors[0],
            lane,
            &format!("readview-values-{lane}"),
            &requests,
        );
        packages.verify_identity();
    }
    run(
        &packages.application,
        "readview",
        &descriptors[0],
        "detached-artifact",
        "ordinary-boundary",
        &cases[0]["arguments"],
        Some(&expected_for("readview", &cases[0])),
    );
    packages.verify_identity();
}

#[test]
fn native_associated_read_views_source_alias_path_custody_and_complete_category_refusals() {
    let packages = Packages::stage();
    let application = &packages.application;
    for (label, literal, private) in [
        (
            "same-scope-second-argument",
            include_str!("read_view_fixtures/negative-alias.lkjc"),
            false,
        ),
        (
            "source-field-second-argument",
            include_str!("read_view_fixtures/negative-source-field.lkjc"),
            false,
        ),
        (
            "local-owned-source-escape",
            include_str!("read_view_fixtures/negative-escape.lkjc"),
            false,
        ),
        (
            "metadata-source-protected",
            include_str!("read_view_fixtures/negative-protected.lkjc"),
            false,
        ),
        (
            "foreign-private-field",
            include_str!("read_view_fixtures/negative-private-field.lkjc"),
            true,
        ),
        (
            "abstract-family-not-sharing-evidence",
            include_str!("read_view_fixtures/negative-abstract-sharing.lkjc"),
            false,
        ),
        (
            "self-reference-requires-sharing",
            include_str!("read_view_fixtures/negative-self-sharing.lkjc"),
            false,
        ),
        (
            "first-class-view-callable",
            include_str!("read_view_fixtures/negative-function-value.lkjc"),
            false,
        ),
    ] {
        packages.refused(application, label, literal, true, private);
    }
    let flat = &packages.authors[2];
    let modules = cli(flat, &["query", "find", "module", "readview-flat"], true);
    let module = compact_field(compact_record(&modules, "owner"), "id");
    let custody = include_str!("read_view_fixtures/negative-custody-recovery.lkjc")
        .replace("MODULE_OWNER", module);
    packages.refused(
        flat,
        "custody-cannot-promote-metadata",
        &custody,
        false,
        false,
    );
    packages.refused(
        flat,
        "zero-custody-cannot-invent-reference",
        &custody.replace("Metadata", "Zero"),
        false,
        false,
    );
    let protected = include_str!("read_view_fixtures/negative-protected.lkjc");
    packages.refused(
        application,
        "zero-reference-source-protected",
        &protected
            .replace("make-metadata", "make-zero")
            .replace("readview-flat::Metadata", "readview-flat::Zero"),
        true,
        false,
    );
    packages.refused(application, "metadata-protected-mutation", &protected.replace(
        "(call readview-consumer::discard (types (owned-sequence I64)) (local owner))",
        "(sequence (call std::data-sequence-replace (types I64) (i64 0) (i64 99) (local owner)) (unit))"), true, false);
    let discard = "(call readview-consumer::discard (types (owned-sequence I64)) (local owner))";
    let read = "(call readview-flat::metadata-length (scopes (lexical-read-scope protected-owner $protected-view-scope)) (local view))";
    packages.refused(
        application,
        "view-use-after-drop",
        &protected.replace(
            discard,
            &format!("(sequence (view-drop (local view)) (sequence {read} (unit)))"),
        ),
        true,
        false,
    );
    let view_type = "(read-view-application readview-flat::Metadata (types) (lexical-read-scope protected-owner $protected-view-scope))";
    packages.refused(application, "view-use-after-move", &protected.replace(discard,
        &format!("(let (binding moved (type {view_type}) (view-move (local view))) (in (sequence {read} (unit))))")), true, false);
    packages.refused(
        application,
        "implicit-handle-copy",
        &protected.replace(
            discard,
            &format!("(let (binding copied (type {view_type}) (local view)) (in (unit)))"),
        ),
        true,
        false,
    );
    packages.refused(
        application,
        "untaken-protected-consume",
        &protected.replace(discard, &format!("(if (bool false) {discard} (unit))")),
        true,
        false,
    );
    let alias = include_str!("read_view_fixtures/negative-alias.lkjc");
    let bad_body = "(view-reborrow (local second) (read-scope (declaration first-anchor) read_scope_d8600000000000000000000000000001))";
    let good_body = bad_body.replace("(local second)", "(local first)");
    assert!(alias.contains(bad_body));
    packages.refused(
        application,
        "borrowed-view-cannot-move",
        &alias.replace(bad_body, "(view-move (local first))"),
        true,
        false,
    );
    packages.refused(
        application,
        "borrowed-view-cannot-drop",
        &alias.replace(
            bad_body,
            &format!("(sequence (view-drop (local first)) {good_body})"),
        ),
        true,
        false,
    );
    packages.refused(
        application,
        "untaken-second-argument-escape",
        &alias.replace(
            bad_body,
            &format!("(if (bool false) {bad_body} {good_body})"),
        ),
        true,
        false,
    );
    packages.refused(
        application,
        "foreign-scope-binder",
        &alias.replace(bad_body, &good_body).replacen(
            "(read-scope (declaration first-anchor)",
            "(read-scope (declaration readview-transformer::forward-first)",
            1,
        ),
        true,
        false,
    );
    packages.refused(
        application,
        "wrong-associated-member",
        &alias
            .replace(bad_body, &good_body)
            .replace(MEMBER, "assoc_type_d8100000000000000000000000000009"),
        true,
        false,
    );
    let reader = "(implementation-parameter implparam_d8600000000000000000000000000001 reader readview-consumer::RangeRead W (types I64))";
    let extra_reader = format!(
        "{reader}\n      (implementation-parameter implparam_d8600000000000000000000000000004 spare readview-consumer::RangeRead W (types I64))"
    );
    assert!(alias.contains(reader));
    let swapped = alias.replace(bad_body, &good_body).replace(reader, &extra_reader).replace(
        "(returns (associated-read-view-projection first-anchor implparam_d8600000000000000000000000000001",
        "(returns (associated-read-view-projection first-anchor implparam_d8600000000000000000000000000004");
    packages.refused(
        application,
        "unused-witness-cannot-select-result-family",
        &swapped,
        true,
        false,
    );
    let family_as_owned = alias.replace(bad_body, &good_body)
        .replace("associated-read-view-projection", "associated-witness-projection")
        .replace("\n        (read-scope (declaration first-anchor) read_scope_d8600000000000000000000000000001)", "");
    packages.refused(
        application,
        "family-kind-arity-cannot-use-owned-projection",
        &family_as_owned,
        true,
        false,
    );
    let source_field = include_str!("read_view_fixtures/negative-source-field.lkjc");
    packages.refused(
        application,
        "constructed-other-source-return",
        &source_field.replace("(source source)", "(source other)"),
        true,
        false,
    );
    let view = "(read-view-application readview-flat::Metadata (types) (read-scope (declaration ignored) read_scope_d8600000000000000000000000000004))";
    let category = include_str!("read_view_fixtures/negative-category.lkjc");
    for (label, ty) in [
        ("unrestricted-view-parameter", view.to_owned()),
        ("ordinary-record", format!("(record (hidden {view}))")),
        (
            "owned-product",
            format!("(owned-product (field hidden {view}))"),
        ),
        ("owned-sequence", format!("(owned-sequence {view})")),
        (
            "owned-choice",
            format!("(owned-choice (case hidden {view}) (case empty Unit))"),
        ),
        ("list", format!("(list {view})")),
        ("map", format!("(map I64 {view})")),
        ("option", format!("(option {view})")),
        ("result", format!("(result {view} I64)")),
        ("callable-result", format!("(function (I64) {view})")),
        ("phantom-actual", format!("(Phantom {view})")),
    ] {
        packages.refused(
            application,
            label,
            &category.replace("CATEGORY", &ty),
            true,
            false,
        );
    }
    let unused = alias
        .replace(bad_body, &good_body)
        .replace("(use borrow)", "(use consume)");
    packages.refused(application, "consume-view-parameter", &unused, true, false);
    packages.refused(
        application,
        "task-view-result",
        &alias
            .replace(bad_body, &good_body)
            .replace("(effect pure)", "(effect (task))"),
        true,
        false,
    );
    // The same authored complete source is legal when the exact first anchor is used.
    accept(
        application,
        "healthy-first-anchor",
        &format!(
            "{}{}",
            packages.imports(),
            alias.replace(bad_body, &good_body)
        ),
    );
    let capability = include_str!("read_view_fixtures/negative-capability-result.lkjc");
    packages.refused(application, "capability-operation-view-result", &capability.replace("RESULT_TYPE",
        "(read-view-application readview-flat::Metadata (types) (read-scope (declaration scope-owner) read_scope_d8600000000000000000000000000015))"), true, false);
    let self_sharing = include_str!("read_view_fixtures/negative-self-sharing.lkjc");
    let source_field = "(view-field field_d8600000000000000000000000000003 source (type W)\n        (read-reference (read-scope (declaration GenericSpan) read_scope_d8600000000000000000000000000010)))";
    assert!(self_sharing.contains(source_field));
    packages.refused(
        application,
        "phantom-owned-actual-requires-sharing",
        &self_sharing.replace(source_field, ""),
        true,
        false,
    );
    accept(
        application,
        "healthy-self-sharing",
        &format!(
            "{}{}",
            packages.imports(),
            self_sharing.replace("(constraint owned)", "(constraint owned shareable)")
        ),
    );
    accept(
        application,
        "healthy-ordinary-category",
        &format!(
            "{}{}",
            packages.imports(),
            category.replace("CATEGORY", "I64")
        ),
    );
    accept(
        application,
        "healthy-ordinary-capability-result",
        &format!(
            "{}{}",
            packages.imports(),
            capability.replace("RESULT_TYPE", "I64")
        ),
    );
    check(application);
}

#[test]
fn native_associated_read_views_closed_joined_reads_projected_sources_and_passive_metadata_custody()
{
    let mut packages = Packages::stage();
    packages.build();
    let targets = [
        "readview-joined-flat",
        "readview-joined-chunked",
        "readview-joined-metadata",
    ];
    let descriptors =
        targets.map(|target| deployment(&packages.application, "readview.lkja", target));
    for descriptor in &descriptors {
        packages.identities.push((
            descriptor.clone(),
            blake3::hash(&std::fs::read(descriptor).unwrap()),
        ));
    }
    let failure = deployment(
        &packages.application,
        "readview.lkja",
        "readview-joined-failure",
    );
    packages.identities.push((
        failure.clone(),
        blake3::hash(&std::fs::read(&failure).unwrap()),
    ));
    let cases = literal_cases();
    let expected: Vec<Value> =
        serde_json::from_str(include_str!("read_view_fixtures/joined-cases.json")).unwrap();
    assert_eq!(cases.len(), expected.len());
    archive(
        &packages.application,
        "joined-cases.json",
        include_str!("read_view_fixtures/joined-cases.json").as_bytes(),
    );
    // Task identity requires deployment admission even when its effect row is empty.
    for (target, descriptor) in [
        (targets[0], &descriptors[0]),
        (targets[1], &descriptors[1]),
        (targets[2], &descriptors[2]),
        ("readview-joined-failure", &failure),
    ] {
        let refused = run(
            &packages.application,
            target,
            descriptor,
            "project",
            "task-requires-deployment",
            &cases[0]["arguments"],
            None,
        );
        let diagnostic = compact_record(&refused, "diagnostic");
        assert_eq!(compact_field(diagnostic, "class"), "capability");
        assert_eq!(
            compact_field(diagnostic, "code"),
            "normalized_runner_grants_required"
        );
    }
    let production_only = |records: &[CompactRecord]| {
        let execution = compact_record(records, "execution");
        assert_eq!(compact_field(execution, "execution-mode"), "production");
        assert_eq!(compact_field(execution, "verification"), "not-performed");
    };
    for lane in ["attached-artifact", "detached-artifact"] {
        if lane == "detached-artifact" {
            packages.detach();
        }
        for (case, literal) in cases.iter().zip(&expected) {
            assert_eq!(case["name"], literal["name"]);
            for (target, descriptor) in targets.iter().zip(&descriptors) {
                let result = if *target == "readview-joined-metadata" {
                    &literal["metadata"]
                } else {
                    &literal["range"]
                };
                let records = run(
                    &packages.application,
                    target,
                    descriptor,
                    lane,
                    case["name"].as_str().unwrap(),
                    &case["arguments"],
                    Some(result),
                );
                production_only(&records);
            }
        }
        let failed = run(
            &packages.application,
            "readview-joined-failure",
            &failure,
            lane,
            "child-trap",
            &cases[0]["arguments"],
            None,
        );
        assert!(compact_field(compact_record(&failed, "diagnostic"), "code").contains("division"));
        let recovered = run(
            &packages.application,
            targets[0],
            &descriptors[0],
            lane,
            "healthy-after-child-trap",
            &cases[0]["arguments"],
            Some(&expected[0]["range"]),
        );
        production_only(&recovered);
        packages.verify_identity();
    }
}

#[test]
fn native_associated_read_views_checked_traps_fuel_cleanup_and_subsequent_invocation() {
    let mut packages = Packages::stage();
    let descriptors = packages.build();
    let cases = literal_cases();
    let healthy = &cases[0];
    for lane in ["project", "attached-artifact", "detached-artifact"] {
        if lane == "detached-artifact" {
            packages.detach();
        }
        let mut requests = Vec::new();
        for (label, start, end, sub_start, sub_end) in [
            ("negative-start", -1, 2, 0, 1),
            ("past-source", 2, 7, 0, 1),
            ("backward-range", 4, 3, 0, 0),
            ("past-view", 1, 5, 0, 5),
        ] {
            let arguments = json!([{"values":[11,7,23,5,19,13],"chunks":[[11,7],[23,5,19],[13]],
                "start":start,"end":end,"sub_start":sub_start,"sub_end":sub_end}]);
            for target in TARGETS {
                requests.push(PureCase {
                    label: label.to_owned(),
                    target,
                    arguments: arguments.clone(),
                    expected: None,
                });
                requests.push(PureCase {
                    label: format!("healthy-after-{label}"),
                    target,
                    arguments: healthy["arguments"].clone(),
                    expected: Some(expected_for(target, healthy)),
                });
            }
        }
        assert_eq!(
            requests.len(),
            24,
            "all checked traps and immediate recoveries"
        );
        run_pure_batch(
            &packages.application,
            &descriptors[0],
            lane,
            &format!("readview-traps-{lane}"),
            &requests,
        );
        packages.verify_identity();
    }
    let mut limited: Value =
        serde_json::from_slice(&std::fs::read(&descriptors[0]).unwrap()).unwrap();
    limited["execution"] =
        json!({"instruction_fuel":100,"maximum_call_depth":4096,"maximum_value_stack":1000000});
    let descriptor = packages
        .application
        .input("limited.deployment.json", &limited.to_string());
    let refused = run(
        &packages.application,
        "readview",
        &descriptor,
        "detached-artifact",
        "fuel-refusal",
        &healthy["arguments"],
        None,
    );
    let diagnostic = compact_record(&refused, "diagnostic");
    assert_eq!(compact_field(diagnostic, "class"), "resource");
    assert_eq!(
        compact_field(diagnostic, "code"),
        "normalized_instruction_steps"
    );
    run(
        &packages.application,
        "readview",
        &descriptors[0],
        "detached-artifact",
        "healthy-after-fuel",
        &healthy["arguments"],
        Some(&expected_for("readview", healthy)),
    );
    packages.verify_identity();
}
