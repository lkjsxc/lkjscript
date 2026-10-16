//! Share immutable public-authored bytes, never mutation locks or observations.
use super::*;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Component;
use std::sync::{Arc, Mutex, OnceLock};

const MAXIMUM_BYTES: usize = 64 * 1024 * 1024;
const MAXIMUM_FILES: usize = 512;

struct SnapshotFile {
    relative: PathBuf,
    bytes: Vec<u8>,
    permissions: fs::Permissions,
}

struct Snapshot {
    memory: Option<lkjscript::platform::MemProjectStore>,
    directories: Vec<PathBuf>,
    files: Vec<SnapshotFile>,
}

struct Exported {
    package: String,
    semantic: String,
    revision: String,
    transport: String,
    name: PathBuf,
}

struct Seed {
    key: blake3::Hash,
    snapshots: [Snapshot; 5],
    exports: [Exported; 4],
    bytes: usize,
    files: usize,
}

#[derive(Default)]
struct Pool {
    seeds: BTreeMap<[u8; 32], Arc<Seed>>,
    bytes: usize,
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
    hasher.finalize()
}

fn identity(
    product: blake3::Hash,
    harness: blake3::Hash,
    mode: &str,
    recipe: &[&str],
) -> blake3::Hash {
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.test.read-view-authored-seed.v1");
    for bytes in std::iter::once(product.as_bytes().as_slice())
        .chain(std::iter::once(harness.as_bytes().as_slice()))
        .chain(std::iter::once(mode.as_bytes()))
        .chain(recipe.iter().map(|source| source.as_bytes()))
    {
        hasher.update(&(bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    hasher.finalize()
}

fn relative(path: &Path) {
    assert!(!path.as_os_str().is_empty());
    assert!(
        path.components()
            .all(|component| matches!(component, Component::Normal(_)))
    );
}

impl Snapshot {
    fn capture(root: &Path, bytes: &mut usize, files: &mut usize) -> Self {
        Self::capture_source(root, None, bytes, files)
    }

    fn capture_native(native: &Native, bytes: &mut usize, files: &mut usize) -> Self {
        let memory = native.memory.read().unwrap();
        Self::capture_source(native.root.path(), memory.as_ref(), bytes, files)
    }

    fn capture_source(
        root: &Path,
        memory: Option<&lkjscript::platform::MemProjectStore>,
        bytes: &mut usize,
        files: &mut usize,
    ) -> Self {
        let mut snapshot = Self {
            memory: None,
            directories: Vec::new(),
            files: Vec::new(),
        };
        if let Some(memory) = memory {
            lkjscript::platform::contributor::visit_memory_project(memory, |name, value| {
                *bytes = bytes
                    .checked_add(name.len())
                    .unwrap()
                    .checked_add(value.len())
                    .unwrap();
                assert!(
                    *bytes <= MAXIMUM_BYTES,
                    "read-view authored seed exceeds its byte bound"
                );
            })
            .unwrap();
            snapshot.memory = Some(memory.fork().unwrap());
        } else {
            snapshot.tree(root, Path::new("project"), bytes, files);
        }
        for entry in fs::read_dir(root).unwrap() {
            let entry = entry.unwrap();
            if source_file(&entry.path()) {
                snapshot.file(root, Path::new(&entry.file_name()), bytes, files);
            }
        }
        // These are literal inputs, not copied command/terminal observations.
        for entry in fs::read_dir(root.join("literal-evidence")).unwrap() {
            let entry = entry.unwrap();
            if source_file(&entry.path()) {
                snapshot.file(
                    root,
                    &Path::new("literal-evidence").join(entry.file_name()),
                    bytes,
                    files,
                );
            }
        }
        snapshot
            .files
            .sort_by(|left, right| left.relative.cmp(&right.relative));
        snapshot
    }

    fn tree(&mut self, root: &Path, path: &Path, bytes: &mut usize, files: &mut usize) {
        relative(path);
        let directory = root.join(path);
        let metadata = fs::symlink_metadata(&directory).unwrap();
        assert!(metadata.is_dir() && !metadata.file_type().is_symlink());
        assert!(
            self.directories.len() < MAXIMUM_FILES,
            "read-view seed directory inventory exceeds its bound"
        );
        self.directories.push(path.to_path_buf());
        let mut entries = fs::read_dir(directory)
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = path.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                self.tree(root, &path, bytes, files);
            } else {
                self.file(root, &path, bytes, files);
            }
        }
    }

    fn file(&mut self, root: &Path, path: &Path, bytes: &mut usize, files: &mut usize) {
        relative(path);
        let file = root.join(path);
        let metadata = fs::symlink_metadata(&file).unwrap();
        assert!(metadata.is_file() && !metadata.file_type().is_symlink());
        let length = usize::try_from(metadata.len()).unwrap();
        *bytes = bytes.checked_add(length).unwrap();
        *files = files.checked_add(1).unwrap();
        assert!(
            *bytes <= MAXIMUM_BYTES,
            "read-view authored seed exceeds its byte bound"
        );
        assert!(
            *files <= MAXIMUM_FILES,
            "read-view authored seed exceeds its file bound"
        );
        let retained = fs::read(&file).unwrap();
        assert_eq!(
            retained.len(),
            length,
            "authored seed file changed during capture"
        );
        assert_eq!(fs::symlink_metadata(&file).unwrap().len(), metadata.len());
        self.files.push(SnapshotFile {
            relative: path.to_path_buf(),
            bytes: retained,
            permissions: metadata.permissions(),
        });
    }

    fn restore(&self, root: &Path) {
        for directory in &self.directories {
            relative(directory);
            fs::create_dir_all(root.join(directory)).unwrap();
        }
        for file in &self.files {
            relative(&file.relative);
            let destination = root.join(&file.relative);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
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
    }
}

impl Seed {
    fn capture(key: blake3::Hash, authored: &Packages, previous_bytes: usize) -> Self {
        let mut bytes = previous_bytes;
        let mut files = 0;
        let snapshots = std::array::from_fn(|index| {
            let native = if index == 4 {
                &authored.application
            } else {
                &authored.authors[index]
            };
            Snapshot::capture_native(native, &mut bytes, &mut files)
        });
        let exports = std::array::from_fn(|index| {
            let package = &authored.exports[index];
            assert_eq!(
                package.path.parent().unwrap(),
                authored.authors[index].root.path()
            );
            Exported {
                package: package.package.clone(),
                semantic: package.semantic.clone(),
                revision: package.revision.clone(),
                transport: package.transport.clone(),
                name: PathBuf::from(package.path.file_name().unwrap()),
            }
        });
        Self {
            key,
            snapshots,
            exports,
            bytes: bytes - previous_bytes,
            files,
        }
    }

    fn restore(&self, product: blake3::Hash) -> Packages {
        let mut natives = Vec::new();
        for snapshot in &self.snapshots {
            let native = native_shell();
            assert_eq!(
                fingerprint(&native.executable),
                product,
                "seed clone selected another product"
            );
            snapshot.restore(native.root.path());
            *native.memory.write().unwrap() = snapshot
                .memory
                .as_ref()
                .map(|memory| memory.fork().unwrap());
            // Copied bytes supply no admission result: each restored repository
            // re-enters the selected product's ordinary source owner.
            check(&native);
            natives.push(native);
        }
        let application = natives.pop().unwrap();
        let authors: [Native; 4] = natives.try_into().ok().unwrap();
        let exports = std::array::from_fn(|index| {
            let package = &self.exports[index];
            Export {
                path: authors[index].root.path().join(&package.name),
                package: package.package.clone(),
                semantic: package.semantic.clone(),
                revision: package.revision.clone(),
                transport: package.transport.clone(),
            }
        });
        let standards = [0, 2, 3]
            .map(|index| authors[index].root.path().join("standard.lkjp"))
            .to_vec();
        let mut source_archive = tempfile::tempdir().unwrap();
        if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
            source_archive.disable_cleanup(true);
            println!(
                "retained independent read-view source archive: {}",
                source_archive.path().display()
            );
        }
        archive(&application, "cases.json", CASES.as_bytes());
        let packages = Packages {
            authors,
            application,
            exports,
            standards,
            identities: Vec::new(),
            source_archive,
        };
        self.observation(&packages, false);
        packages
    }

    fn observation(&self, packages: &Packages, authored: bool) {
        archive(&packages.application, "authored-seed.json", &serde_json::to_vec_pretty(&json!({
            "identity": self.key.to_hex().to_string(), "bytes": self.bytes, "files": self.files,
            "source_backend":if self.snapshots[0].memory.is_some() {"memory"} else {"filesystem"},
            "constructor_executed_here": authored,
            "source": if authored {"ordered public authoring/export/checks"} else {"immutable authored bytes copied into independent repositories"},
            "shared_mutable_repositories": false, "copied_process_observations": false,
        })).unwrap());
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
        &[
            "Packages::author-v1",
            CONSUMER,
            TRANSFORMER,
            FLAT,
            FLAT_JOINED,
            CHUNKED,
            CHUNKED_JOINED,
            APPLICATION,
            APPLICATION_JOINED,
            CASES,
        ],
    );
    let mut pool = POOL.get_or_init(Mutex::default).lock().unwrap();
    if let Some(seed) = pool.seeds.get(key.as_bytes()).cloned() {
        drop(pool);
        return seed.restore(product);
    }
    // Keep the lock through complete authoring and capture. Failure publishes no
    // seed and poisons this preparation instead of disguising it as a cache hit.
    let authored = Packages::author();
    assert_eq!(
        fingerprint(&selected),
        product,
        "selected product changed during authoring"
    );
    for native in authored
        .authors
        .iter()
        .chain(std::iter::once(&authored.application))
    {
        assert_eq!(fingerprint(&native.executable), product);
    }
    let seed = Arc::new(Seed::capture(key, &authored, pool.bytes));
    pool.bytes = pool.bytes.checked_add(seed.bytes).unwrap();
    assert!(
        pool.bytes <= MAXIMUM_BYTES,
        "read-view seed pool exceeds its byte bound"
    );
    seed.observation(&authored, true);
    pool.seeds.insert(*key.as_bytes(), seed);
    authored
}

#[test]
fn authored_seed_identity_binds_product_harness_mode_and_literal_recipe() {
    let first = blake3::hash(b"first");
    let second = blake3::hash(b"second");
    let key = identity(first, second, "candidate-process", &["literal"]);
    for changed in [
        identity(second, second, "candidate-process", &["literal"]),
        identity(first, first, "candidate-process", &["literal"]),
        identity(first, second, "host-semantic", &["literal"]),
        identity(first, second, "candidate-process", &["different literal"]),
    ] {
        assert_ne!(key, changed);
    }
}

#[test]
fn authored_seed_copies_keep_mutable_heads_locks_and_files_independent() {
    let source = tempfile::tempdir().unwrap();
    fs::create_dir_all(source.path().join("project/packs")).unwrap();
    fs::create_dir(source.path().join("literal-evidence")).unwrap();
    fs::write(source.path().join("project/HEAD"), b"accepted head").unwrap();
    fs::write(source.path().join("project/LOCK"), b"").unwrap();
    fs::write(
        source.path().join("project/packs/pack.lkjp"),
        b"accepted bytes",
    )
    .unwrap();
    let snapshot = Snapshot::capture(source.path(), &mut 0, &mut 0);
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    snapshot.restore(first.path());
    snapshot.restore(second.path());
    fs::write(first.path().join("project/HEAD"), b"independent mutation").unwrap();
    assert_eq!(
        fs::read(source.path().join("project/HEAD")).unwrap(),
        b"accepted head"
    );
    assert_eq!(
        fs::read(second.path().join("project/HEAD")).unwrap(),
        b"accepted head"
    );
    fs::remove_dir_all(first.path().join("project")).unwrap();
    assert_eq!(
        fs::read(second.path().join("project/packs/pack.lkjp")).unwrap(),
        b"accepted bytes"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_ne!(
            fs::metadata(source.path().join("project/LOCK"))
                .unwrap()
                .ino(),
            fs::metadata(second.path().join("project/LOCK"))
                .unwrap()
                .ino()
        );
    }
}

#[test]
fn authored_seed_refuses_payload_over_budget_before_reading() {
    let source = tempfile::tempdir().unwrap();
    fs::create_dir(source.path().join("project")).unwrap();
    fs::create_dir(source.path().join("literal-evidence")).unwrap();
    fs::write(source.path().join("project/HEAD"), b"more than one byte").unwrap();
    assert!(
        std::panic::catch_unwind(|| {
            Snapshot::capture(source.path(), &mut (MAXIMUM_BYTES - 1), &mut 0);
        })
        .is_err()
    );
}
