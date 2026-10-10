//! Bounded copied-product calls and FIFO-aware fixture inventories.
use super::*;
use std::os::unix::fs::FileTypeExt;
use std::time::{Duration, Instant};

pub(super) fn fifo(public: &Native) -> PathBuf {
    let current = public.project.join("derived/compiler/CURRENT");
    std::fs::remove_file(&current).unwrap();
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        &current,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .unwrap();
    current
}

pub(super) fn bounded(public: &Native, name: &str, arguments: &[&str]) -> Vec<CompactRecord> {
    let stdout = public.root.path().join(format!("{name}.stdout"));
    let stderr = public.root.path().join(format!("{name}.stderr"));
    let mut child = support::spawn(
        Command::new(&public.executable)
            .arg("--project")
            .arg(&public.project)
            .args(arguments)
            .current_dir(public.root.path())
            .env_clear()
            .env("PATH", "")
            .stdin(Stdio::null())
            .stdout(File::create(&stdout).unwrap())
            .stderr(File::create(&stderr).unwrap()),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while child.try_wait().unwrap().is_none() {
        // SpawnedChild owns kill-and-join on assertion failure, not an abandoned thread.
        assert!(
            Instant::now() < deadline,
            "{name} blocked on a derived cache FIFO"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().unwrap();
    let stdout = std::fs::read(stdout).unwrap();
    let stderr = std::fs::read(stderr).unwrap();
    assert!(
        output.status.success(),
        "{name}:\n{}\n{}",
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&stderr)
    );
    parse_records("bounded cache admission", &stdout).unwrap()
}

pub(super) fn apply(
    public: &Native,
    name: &str,
    input: &Path,
    plan: &[CompactRecord],
) -> Vec<CompactRecord> {
    bounded(
        public,
        name,
        &[
            "change",
            "apply",
            "--input-file",
            path(input),
            "--plan",
            compact_field(compact_record(plan, "plan"), "token"),
        ],
    )
}

pub(super) fn inventory(root: &Path) -> BTreeMap<PathBuf, [u8; 32]> {
    fn visit(path: &Path, files: &mut BTreeMap<PathBuf, [u8; 32]>) {
        let metadata = std::fs::symlink_metadata(path).unwrap();
        if metadata.is_dir() {
            for entry in std::fs::read_dir(path).unwrap() {
                visit(&entry.unwrap().path(), files);
            }
        } else if metadata.is_file() {
            files.insert(
                path.to_owned(),
                *blake3::hash(&std::fs::read(path).unwrap()).as_bytes(),
            );
        } else {
            assert!(
                metadata.file_type().is_fifo(),
                "unexpected fixture entry: {path:?}"
            );
            files.insert(
                path.to_owned(),
                *blake3::hash(b"writerless FIFO fixture").as_bytes(),
            );
        }
    }
    let mut files = BTreeMap::new();
    visit(root, &mut files);
    files
}
