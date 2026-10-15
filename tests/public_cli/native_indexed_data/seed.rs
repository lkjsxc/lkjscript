//! Share completed authored inputs, never repositories, command observations or progress.
use super::*;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Component;
use std::sync::{Arc, Mutex, OnceLock};

const MAXIMUM_BYTES: usize = 64 * 1024 * 1024;
const MAXIMUM_FILES: usize = 512;
const MAXIMUM_DIRECTORIES: usize = 512;
const RECIPE: &str = "indexed-authored-seed-v1: four minimal suppliers before command consumer; seven accepts and unchanged drafts; two equal builds; fifteen deployments";
const AUTHOR_INPUTS: [[&str; 4]; 4] = [
    [
        "library.lkjc",
        "indexed-data-draft.lkjc",
        "package.lkjp",
        "standard.lkjp",
    ],
    [
        "indexed-flat.lkjc",
        "indexed-flat-draft.lkjc",
        "package.lkjp",
        "standard.lkjp",
    ],
    [
        "indexed-chunked.lkjc",
        "indexed-chunked-draft.lkjc",
        "package.lkjp",
        "standard.lkjp",
    ],
    [
        "indexed-forward.lkjc",
        "indexed-forward-draft.lkjc",
        "package.lkjp",
        "standard.lkjp",
    ],
];
const CONSUMER_INPUTS: [&str; 8] = [
    "application.lkjc",
    "indexed-app-draft.lkjc",
    "costs.lkjc",
    "indexed-cost-draft.lkjc",
    "blockread.lkjc",
    "indexed-blockread-draft.lkjc",
    "indexed-data.lkja",
    "indexed-data-second.lkja",
];

#[derive(Clone, Copy, Default)]
struct Budget {
    bytes: usize,
    files: usize,
    directories: usize,
}

impl Budget {
    fn payload(&mut self, bytes: usize) {
        self.bytes = self.bytes.checked_add(bytes).unwrap();
        assert!(
            self.bytes <= MAXIMUM_BYTES,
            "indexed authored seed exceeds its byte bound"
        );
    }

    fn file(&mut self, bytes: usize) {
        self.payload(bytes);
        self.files = self.files.checked_add(1).unwrap();
        assert!(
            self.files <= MAXIMUM_FILES,
            "indexed authored seed exceeds its file bound"
        );
    }

    fn directory(&mut self) {
        self.directories = self.directories.checked_add(1).unwrap();
        assert!(
            self.directories <= MAXIMUM_DIRECTORIES,
            "indexed authored seed exceeds its directory bound"
        );
    }
}

struct SnapshotFile {
    relative: PathBuf,
    bytes: Vec<u8>,
    permissions: fs::Permissions,
}

struct SnapshotDirectory {
    relative: PathBuf,
    permissions: fs::Permissions,
}

struct Snapshot {
    directories: Vec<SnapshotDirectory>,
    files: Vec<SnapshotFile>,
}

struct Exported {
    package: String,
    semantic: String,
    revision: String,
    transport: String,
}

struct Seed {
    key: blake3::Hash,
    snapshots: [Snapshot; 5],
    exports: [Exported; 4],
    budget: Budget,
}

#[derive(Default)]
struct Pool {
    seeds: BTreeMap<[u8; 32], Arc<Seed>>,
    budget: Budget,
}

fn fingerprint(file: &Path) -> blake3::Hash {
    let metadata = fs::symlink_metadata(file).unwrap();
    assert!(metadata.is_file() && !metadata.file_type().is_symlink());
    let mut input = File::open(file).unwrap();
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut bytes = 0_u64;
    loop {
        let count = input.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        bytes = bytes.checked_add(count as u64).unwrap();
        assert!(
            bytes <= metadata.len(),
            "selected executable grew during identity read"
        );
        hasher.update(&buffer[..count]);
    }
    assert_eq!(bytes, metadata.len(), "selected executable changed length");
    let after = fs::symlink_metadata(file).unwrap();
    assert!(after.is_file() && !after.file_type().is_symlink());
    assert_eq!(after.len(), metadata.len());
    hasher.finalize()
}

fn identity(
    product: blake3::Hash,
    harness: blake3::Hash,
    mode: &str,
    recipe: &str,
    literals: &[&str],
    targets: &[&str],
) -> blake3::Hash {
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.test.indexed-authored-seed.v1");
    for bytes in [
        product.as_bytes().as_slice(),
        harness.as_bytes().as_slice(),
        mode.as_bytes(),
        recipe.as_bytes(),
    ] {
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    for fields in [literals, targets] {
        hasher.update(&(fields.len() as u64).to_le_bytes());
        for field in fields {
            hasher.update(&(field.len() as u64).to_le_bytes());
            hasher.update(field.as_bytes());
        }
    }
    hasher.finalize()
}

fn relative(path: &Path) {
    assert!(!path.as_os_str().is_empty());
    assert!(
        path.components()
            .all(|part| matches!(part, Component::Normal(_))),
        "indexed seed path must stay inside its fresh root"
    );
}

impl Snapshot {
    fn capture(root: &Path, inputs: &[&str], budget: &mut Budget) -> Self {
        let mut snapshot = Self {
            directories: Vec::new(),
            files: Vec::new(),
        };
        snapshot.tree(root, Path::new("project"), budget);
        // The fixed allowlist contains inputs and artifacts, never *.json,
        // operation logs, command terminals, plan responses or progress.
        for input in inputs {
            snapshot.file(root, Path::new(input), budget);
        }
        snapshot.files.sort_by(|a, b| a.relative.cmp(&b.relative));
        snapshot
    }

    fn tree(&mut self, root: &Path, path: &Path, budget: &mut Budget) {
        relative(path);
        let directory = root.join(path);
        let metadata = fs::symlink_metadata(&directory).unwrap();
        assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
        budget.directory();
        self.directories.push(SnapshotDirectory {
            relative: path.to_path_buf(),
            permissions: metadata.permissions(),
        });
        // Do not collect an unbounded directory listing before charging entries.
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = path.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                self.tree(root, &path, budget);
            } else {
                self.file(root, &path, budget);
            }
        }
    }

    fn file(&mut self, root: &Path, path: &Path, budget: &mut Budget) {
        relative(path);
        let file = root.join(path);
        let metadata = fs::symlink_metadata(&file).unwrap();
        assert!(metadata.is_file() && !metadata.file_type().is_symlink());
        let length = usize::try_from(metadata.len()).unwrap();
        // This budget includes all previously published seeds. Reserve before
        // opening or allocating payload; a growing file cannot grow this buffer.
        budget.file(length);
        let mut bytes = vec![0_u8; length];
        let mut input = File::open(&file).unwrap();
        input.read_exact(&mut bytes).unwrap();
        assert_eq!(
            input.read(&mut [0_u8; 1]).unwrap(),
            0,
            "indexed seed file grew during capture"
        );
        let after = fs::symlink_metadata(&file).unwrap();
        assert!(after.is_file() && !after.file_type().is_symlink());
        assert_eq!(after.len(), metadata.len());
        self.files.push(SnapshotFile {
            relative: path.to_path_buf(),
            bytes,
            permissions: metadata.permissions(),
        });
    }

    fn restore(&self, root: &Path) {
        for directory in &self.directories {
            relative(&directory.relative);
            fs::create_dir_all(root.join(&directory.relative)).unwrap();
        }
        for file in &self.files {
            relative(&file.relative);
            let destination = root.join(&file.relative);
            let mut output = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&destination)
                .unwrap();
            output.write_all(&file.bytes).unwrap();
            output.set_permissions(file.permissions.clone()).unwrap();
            drop(output);
            assert_eq!(fs::read(destination).unwrap(), file.bytes);
        }
        for directory in self.directories.iter().rev() {
            fs::set_permissions(
                root.join(&directory.relative),
                directory.permissions.clone(),
            )
            .unwrap();
        }
    }
}

fn admit(native: &Native) {
    let records = native.cli(&["check"], true);
    let tests = compact_record(&records, "tests");
    assert_eq!(compact_field(tests, "failed"), "0");
    assert_eq!(compact_field(tests, "differential"), "equal");
}

impl Seed {
    fn capture(key: blake3::Hash, authored: &Packages, budget: &mut Budget) -> Self {
        assert_eq!(authored.authors.len(), 4);
        assert_eq!(authored.exports.len(), 4);
        assert_eq!(authored.standards.len(), 4);
        assert_eq!(authored.identities.len(), 21);
        assert_eq!(authored.serial.get(), 0);
        assert!(authored.progress.is_none() && authored.matrix_batch.borrow().is_none());
        let previous = *budget;
        let snapshots = std::array::from_fn(|index| {
            if index == 4 {
                Snapshot::capture(authored.consumer.root.path(), &CONSUMER_INPUTS, budget)
            } else {
                Snapshot::capture(
                    authored.authors[index].root.path(),
                    &AUTHOR_INPUTS[index],
                    budget,
                )
            }
        });
        let exports = std::array::from_fn(|index| {
            let package = &authored.exports[index];
            assert_eq!(
                package.path,
                authored.authors[index].root.path().join("package.lkjp")
            );
            assert_eq!(
                authored.standards[index],
                authored.authors[index].root.path().join("standard.lkjp")
            );
            for field in [
                &package.package,
                &package.semantic,
                &package.revision,
                &package.transport,
            ] {
                budget.payload(field.len());
            }
            Exported {
                package: package.package.clone(),
                semantic: package.semantic.clone(),
                revision: package.revision.clone(),
                transport: package.transport.clone(),
            }
        });
        Self {
            key,
            snapshots,
            exports,
            budget: Budget {
                bytes: budget.bytes - previous.bytes,
                files: budget.files - previous.files,
                directories: budget.directories - previous.directories,
            },
        }
    }

    fn restore(&self, product: blake3::Hash, mode: semantic_cli::Mode) -> Packages {
        let mut natives = Vec::new();
        for snapshot in &self.snapshots {
            let root = tempfile::tempdir().unwrap();
            let project = root.path().join("project");
            let native = Native::at(root, project, mode);
            assert_eq!(
                fingerprint(&native.executable),
                product,
                "restored fixture selected another product"
            );
            snapshot.restore(native.root.path());
            // Copied bytes confer no admission result, even for unused suppliers.
            admit(&native);
            natives.push(native);
        }
        let consumer = natives.pop().unwrap();
        let authors = natives;
        let exports = self
            .exports
            .iter()
            .enumerate()
            .map(|(index, package)| Export {
                path: authors[index].root.path().join("package.lkjp"),
                package: package.package.clone(),
                semantic: package.semantic.clone(),
                revision: package.revision.clone(),
                transport: package.transport.clone(),
            })
            .collect();
        let standards = authors
            .iter()
            .map(|native| native.root.path().join("standard.lkjp"))
            .collect();
        // These are fresh literal descriptors with a relative artifact name, not
        // copied CLI observations or absolute paths into an earlier author root.
        let artifact = consumer.root.path().join("indexed-data.lkja");
        assert_eq!(
            fs::read(&artifact).unwrap(),
            fs::read(consumer.root.path().join("indexed-data-second.lkja")).unwrap()
        );
        let mut paths = vec![consumer.executable.clone(), artifact];
        paths.extend(authors.iter().map(|native| native.executable.clone()));
        paths.extend(
            TARGETS
                .into_iter()
                .map(|target| deployment(&consumer, "indexed-data.lkja", target)),
        );
        let identities = paths
            .into_iter()
            .map(|file| {
                let digest = blake3::hash(&fs::read(&file).unwrap());
                (file, digest)
            })
            .collect::<Vec<_>>();
        assert_eq!(identities.len(), 21);
        Packages {
            authors,
            consumer,
            exports,
            standards,
            identities,
            serial: Cell::new(0),
            progress: None,
            matrix_batch: RefCell::new(None),
        }
    }

    fn observe(&self, packages: &Packages, authored: bool) {
        packages.consumer.input(
            "authored-seed.json",
            &json!({
                "version":1, "key":self.key.to_hex().to_string(),
                "constructor_executed_here":authored,
                "bytes":self.budget.bytes, "files":self.budget.files,
                "directories":self.budget.directories,
                "copied_process_observations":false,
            })
            .to_string(),
        );
    }
}

pub(super) fn stage() -> Packages {
    static POOL: OnceLock<Mutex<Pool>> = OnceLock::new();
    static HARNESS: OnceLock<blake3::Hash> = OnceLock::new();
    let selected = binary();
    let product = fingerprint(&selected);
    let harness = *HARNESS.get_or_init(|| fingerprint(&std::env::current_exe().unwrap()));
    let mode = semantic_cli::Mode::selected();
    let key = identity(
        product,
        harness,
        mode.label(),
        RECIPE,
        &[
            LIBRARY,
            FLAT,
            CHUNKED,
            FORWARD,
            APPLICATION,
            COSTS,
            BLOCKREAD,
        ],
        &TARGETS,
    );
    let mut pool = POOL
        .get_or_init(|| Mutex::new(Pool::default()))
        .lock()
        .unwrap();
    if let Some(seed) = pool.seeds.get(key.as_bytes()).cloned() {
        drop(pool);
        let packages = seed.restore(product, mode);
        seed.observe(&packages, false);
        return packages;
    }
    // Keep the first real constructor and capture under the lock. An author or
    // capture failure poisons it; no partial seed or error can become a cache hit.
    let packages = Packages::stage_with_progress(None);
    assert_eq!(
        fingerprint(&selected),
        product,
        "selected product changed during authoring"
    );
    for native in packages
        .authors
        .iter()
        .chain(std::iter::once(&packages.consumer))
    {
        assert_eq!(native.mode, mode);
        assert_eq!(fingerprint(&native.executable), product);
    }
    let seed = Arc::new(Seed::capture(key, &packages, &mut pool.budget));
    assert!(
        pool.seeds
            .insert(*key.as_bytes(), Arc::clone(&seed))
            .is_none()
    );
    drop(pool);
    seed.observe(&packages, true);
    packages
}

#[test]
fn authored_seed_identity_binds_product_harness_mode_all_literals_targets_and_recipe() {
    let product = blake3::hash(b"product");
    let harness = blake3::hash(b"harness");
    let literals = [
        LIBRARY,
        FLAT,
        CHUNKED,
        FORWARD,
        APPLICATION,
        COSTS,
        BLOCKREAD,
    ];
    let key = identity(
        product,
        harness,
        "host-semantic",
        RECIPE,
        &literals,
        &TARGETS,
    );
    for (product, harness, mode, recipe) in [
        (blake3::hash(b"other"), harness, "host-semantic", RECIPE),
        (product, blake3::hash(b"other"), "host-semantic", RECIPE),
        (product, harness, "candidate-process", RECIPE),
        (product, harness, "host-semantic", "other recipe"),
    ] {
        assert_ne!(
            key,
            identity(product, harness, mode, recipe, &literals, &TARGETS)
        );
    }
    for index in 0..literals.len() {
        let mut changed = literals;
        changed[index] = "another literal";
        assert_ne!(
            key,
            identity(
                product,
                harness,
                "host-semantic",
                RECIPE,
                &changed,
                &TARGETS
            )
        );
    }
    for index in 0..TARGETS.len() {
        let mut changed = TARGETS;
        changed[index] = "another target";
        assert_ne!(
            key,
            identity(
                product,
                harness,
                "host-semantic",
                RECIPE,
                &literals,
                &changed
            )
        );
    }
}

#[test]
fn authored_seed_restores_independent_heads_locks_inputs_and_no_observations() {
    let original = tempfile::tempdir().unwrap();
    fs::create_dir(original.path().join("project")).unwrap();
    fs::write(original.path().join("project/HEAD"), b"original").unwrap();
    fs::write(original.path().join("project/LOCK"), b"").unwrap();
    fs::write(original.path().join("request.lkjc"), b"literal request").unwrap();
    fs::write(
        original.path().join("request-literal.json"),
        b"old observation",
    )
    .unwrap();
    fs::write(
        original.path().join("request-operations.log"),
        b"old observation",
    )
    .unwrap();
    fs::create_dir(original.path().join("cli-evidence")).unwrap();
    fs::write(
        original.path().join("cli-evidence/command.json"),
        b"old command",
    )
    .unwrap();
    let snapshot = Snapshot::capture(original.path(), &["request.lkjc"], &mut Budget::default());
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    snapshot.restore(first.path());
    snapshot.restore(second.path());
    for root in [first.path(), second.path()] {
        assert!(!root.join("request-literal.json").exists());
        assert!(!root.join("request-operations.log").exists());
        assert!(!root.join("cli-evidence").exists());
        assert_eq!(
            fs::read(root.join("request.lkjc")).unwrap(),
            b"literal request"
        );
    }
    fs::write(first.path().join("project/HEAD"), b"changed").unwrap();
    assert_eq!(
        fs::read(original.path().join("project/HEAD")).unwrap(),
        b"original"
    );
    assert_eq!(
        fs::read(second.path().join("project/HEAD")).unwrap(),
        b"original"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let inodes = [original.path(), first.path(), second.path()]
            .map(|root| fs::metadata(root.join("project/LOCK")).unwrap().ino());
        assert_ne!(inodes[0], inodes[1]);
        assert_ne!(inodes[0], inodes[2]);
        assert_ne!(inodes[1], inodes[2]);
    }
    fs::remove_dir_all(first.path().join("project")).unwrap();
    assert!(original.path().join("project/HEAD").exists());
    assert!(second.path().join("project/HEAD").exists());
}

#[test]
fn authored_seed_enforces_aggregate_byte_file_and_directory_bounds() {
    let original = tempfile::tempdir().unwrap();
    fs::create_dir(original.path().join("project")).unwrap();
    fs::write(original.path().join("project/HEAD"), b"four").unwrap();
    for (mut budget, expected) in [
        (
            Budget {
                bytes: MAXIMUM_BYTES - 1,
                ..Budget::default()
            },
            "byte bound",
        ),
        (
            Budget {
                files: MAXIMUM_FILES,
                ..Budget::default()
            },
            "file bound",
        ),
        (
            Budget {
                directories: MAXIMUM_DIRECTORIES,
                ..Budget::default()
            },
            "directory bound",
        ),
    ] {
        let failure = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Snapshot::capture(original.path(), &[], &mut budget)
        }))
        .err()
        .unwrap();
        let message = failure
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| failure.downcast_ref::<&str>().copied())
            .unwrap();
        assert!(message.contains(expected), "{message}");
    }
}

#[test]
fn authored_seed_rejects_symlinks_and_escaping_paths() {
    for path in ["../escape", "/escape", "project/../escape", "."] {
        assert!(std::panic::catch_unwind(|| relative(Path::new(path))).is_err());
    }
    #[cfg(unix)]
    {
        let original = tempfile::tempdir().unwrap();
        fs::create_dir(original.path().join("project")).unwrap();
        std::os::unix::fs::symlink("/dev/null", original.path().join("project/link")).unwrap();
        assert!(
            std::panic::catch_unwind(|| Snapshot::capture(
                original.path(),
                &[],
                &mut Budget::default()
            ))
            .is_err()
        );
    }
}
