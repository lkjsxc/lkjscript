//! Bound FIFO admission failures in owned subprocesses, with regular-file controls.

use super::super::directory::PackDirectoryStore;
use super::super::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StoreErrorClass, StoreWork,
};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const ROOT: &str = "LKJSCRIPT_TEST_STORAGE_FIFO_ROOT";
const OPERATION: &str = "LKJSCRIPT_TEST_STORAGE_FIFO_OPERATION";
const EXPECTED: &str = "LKJSCRIPT_TEST_STORAGE_FIFO_EXPECTED";
const COMPLETED: &str = "LKJSCRIPT_TEST_STORAGE_FIFO_COMPLETED";
const PAYLOAD: &[u8] = b"regular storage admission control";

struct JoinedChild(Option<Child>);

impl Drop for JoinedChild {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn run_admission_child(root: &Path, operation: &str, expected: &str, completed: &Path) {
    let mut child = JoinedChild(Some(
        Command::new(std::env::current_exe().expect("storage test executable"))
            .args([
                "--exact",
                "platform::storage::tests::fifo_tests::fifo_admission_child",
                "--nocapture",
            ])
            .env_clear()
            .env(ROOT, root)
            .env(OPERATION, operation)
            .env(EXPECTED, expected)
            .env(COMPLETED, completed)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn owned storage admission child"),
    ));
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if child
            .0
            .as_mut()
            .expect("owned child")
            .try_wait()
            .expect("observe storage admission child")
            .is_some()
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "storage {operation} did not finish within five seconds for {expected}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child
        .0
        .take()
        .expect("exited storage admission child")
        .wait_with_output()
        .expect("join and collect storage admission child");
    assert!(
        output.status.success(),
        "storage {operation} failed for {expected}:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // A successful harness exit with no matching test must not pass this regression test.
    assert_eq!(
        std::fs::read(completed).expect("child must complete admission assertions"),
        expected.as_bytes()
    );
}

#[test]
fn directory_store_rejects_writerless_fifos_with_regular_file_controls() {
    for operation in ["open", "initialize", "recover"] {
        let temporary = tempfile::TempDir::new().expect("owned FIFO fixture parent");
        let root = temporary.path().join("objects");
        let mut store = PackDirectoryStore::initialize(&root).expect("initialize owned fixture");
        let key = ObjectKey::for_bytes(ObjectDomain::Blob, PAYLOAD);
        let mut work = StoreWork::default();
        store.stage(key, PAYLOAD, &mut work).expect("stage control");
        let receipt = store
            .seal_staged(16 * 1024, &mut work)
            .expect("seal regular-file control");
        assert_eq!(receipt.packs.len(), 1);
        let target = if operation == "recover" {
            root.join("packs").join(receipt.packs[0].file_name())
        } else {
            root.join("catalog/current.lkjc")
        };
        drop(store);

        run_admission_child(
            &root,
            operation,
            "regular",
            &temporary.path().join("regular.completed"),
        );
        std::fs::remove_file(&target).expect("replace only owned regular-file fixture");
        rustix::fs::mkfifoat(
            rustix::fs::CWD,
            &target,
            rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
        )
        .expect("create owned FIFO without a writer");
        run_admission_child(
            &root,
            operation,
            "fifo",
            &temporary.path().join("fifo.completed"),
        );
    }
}

#[test]
fn fifo_admission_child() {
    let Some(root) = std::env::var_os(ROOT) else {
        return;
    };
    let operation = std::env::var(OPERATION).expect("child operation");
    let expected = std::env::var(EXPECTED).expect("child expectation");
    let (result, code) = match operation.as_str() {
        "open" => (
            PackDirectoryStore::open(Path::new(&root)),
            "catalog_manifest_open",
        ),
        "initialize" => (
            PackDirectoryStore::initialize(Path::new(&root)),
            "pack_catalog_open",
        ),
        "recover" => (
            PackDirectoryStore::recover_catalog(Path::new(&root), "owned FIFO fixture"),
            "pack_footer_open",
        ),
        _ => panic!("unknown storage admission operation"),
    };
    match expected.as_str() {
        "regular" => {
            let store = result.expect("regular files must remain admissible");
            let key = ObjectKey::for_bytes(ObjectDomain::Blob, PAYLOAD);
            assert_eq!(
                store
                    .read(key, PAYLOAD.len(), &mut StoreWork::default())
                    .expect("read regular-file control"),
                Some(PAYLOAD.to_vec())
            );
        }
        "fifo" => {
            let error = result.expect_err("writerless FIFO must reject before reading");
            assert_eq!(error.class, StoreErrorClass::Corrupt);
            assert_eq!(error.code, code);
            assert_eq!(error.message, "store entry is not a regular file");
        }
        _ => panic!("unknown storage admission expectation"),
    }
    std::fs::write(
        std::env::var_os(COMPLETED).expect("child completion path"),
        expected.as_bytes(),
    )
    .expect("record completed admission assertions");
}
