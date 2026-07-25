use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::model::{
    Audit, BoundaryStatus, CapabilityKind, CapabilityStatus, Capsule, CapsuleKind,
    CapsuleProvenance, CapsuleProvenanceKind, ContextCard, FileRecord, Limits, Policy,
    UnsafeStatus,
};

pub fn root() -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let nonce = NEXT.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "lkjscript-graph-test-{}-{nonce}",
        std::process::id()
    ))
}

pub fn policy(nodes: u64, edges: u64, work: u64, bytes: u64) -> Policy {
    Policy {
        schema: "lkjscript.structure.policy.v1".into(),
        version: "test".into(),
        limits: Limits {
            authored_lines: 200,
            authored_bytes: 32_768,
            ordinary_line_scalars: 120,
            directory_entries: 16,
            warning_depth: 8,
            error_depth: 12,
            graph_nodes: nodes,
            graph_edges: edges,
            graph_work: work,
            graph_bytes: bytes,
            query_work: work,
            query_bytes: bytes,
        },
        rules: vec![],
    }
}

fn capsule() -> Capsule {
    Capsule {
        schema: "lkjscript.capsule".into(),
        version: 1,
        id: "x".into(),
        root: "crates/x".into(),
        kind: CapsuleKind::Crate,
        purpose: "test capsule".into(),
        layer: "test".into(),
        concepts: vec!["x".into()],
        facade: vec!["crates/x/src/lib.rs".into()],
        allowed_dependencies: vec!["core".into()],
        forbidden_dependencies: vec![],
        tests: vec!["crates/x/tests/sample.rs".into()],
        decisions: vec!["docs/decision.md".into()],
        unsafe_boundary: BoundaryStatus {
            status: UnsafeStatus::Forbidden,
            boundaries: vec![],
        },
        capability: CapabilityStatus {
            status: CapabilityKind::None,
            names: vec![],
        },
        provenance: CapsuleProvenance {
            class: CapsuleProvenanceKind::Authored,
            source: "test".into(),
        },
        verification: vec!["cargo test -p x".into()],
        context_card: ContextCard {
            goal: "test".into(),
            interfaces: vec!["x".into()],
            invariants: vec!["exact".into()],
        },
    }
}

pub fn fixture(root: &Path, revision: &str) -> Audit {
    let content = [
        (
            "crates/lkjscript-core/Cargo.toml",
            "[package]\nname=\"lkjscript-core\"\n",
        ),
        ("crates/lkjscript-core/src/lib.rs", "pub fn core() {}\n"),
        (
            "crates/x/Cargo.toml",
            "[package]\nname=\"lkjscript-x\"\n[dependencies]\nlkjscript-core={}\n",
        ),
        ("crates/x/src/lib.rs", "pub fn api() {}\n"),
        ("crates/x/tests/sample.rs", "#[test]\nfn works() {}\n"),
        ("docs/decision.md", "[source](../src/main.lkjscript)\n"),
        (
            "src/main.lkjscript",
            "import/\n./part.lkjscript\n/import\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n",
        ),
        (
            "src/part.lkjscript",
            "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n",
        ),
    ];
    let mut files = Vec::new();
    for (path, text) in content {
        let absolute = root.join(path);
        assert!(fs::create_dir_all(absolute.parent().unwrap_or(root)).is_ok());
        assert!(fs::write(&absolute, text).is_ok());
        files.push(FileRecord {
            path: path.into(),
            bytes: text.len() as u64,
            lines: text.lines().count() as u64,
            max_line_scalars: 80,
            class: "authored".into(),
            capsule: path.starts_with("crates/x/").then(|| "x".into()),
        });
    }
    let mut core = capsule();
    core.id = "core".into();
    core.root = "crates/lkjscript-core".into();
    core.facade = vec!["crates/lkjscript-core/src/lib.rs".into()];
    core.allowed_dependencies.clear();
    Audit {
        schema: "lkjscript.repository-audit".into(),
        version: 1,
        revision: revision.into(),
        policy_version: "test".into(),
        files,
        directories: vec![],
        classifications: vec![],
        capsules: vec![core, capsule()],
        findings: vec![],
        provenance: vec![],
        unsupported: vec![],
    }
}
