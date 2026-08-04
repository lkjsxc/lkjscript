use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::model::{
    Audit, BoundaryStatus, CapabilityKind, CapabilityStatus, Capsule, CapsuleKind,
    CapsuleProvenance, CapsuleProvenanceKind, ContextCard, DirectoryRecord, FileRecord, Limits,
    Policy, UnsafeStatus,
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
        schema: "lkjscript.structure.policy".into(),
        contract: lkjscript_contracts::REPOSITORY_GRAPH_DIGEST.to_hex(),
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
        contract: lkjscript_contracts::CAPSULE_MANIFEST_DIGEST.to_hex(),
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

include!("support/fixture.rs");
