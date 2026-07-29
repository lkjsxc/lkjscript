use std::fs;
use std::path::Path;

use lkjscript_contracts::LEGACY_TRACED_FAMILIES;

use crate::util::walk;

const RULE: &str = "LKJ-RUNTIME-NO-TRACING-COLLECTOR";
const SELF: &str = "crates/lkjscript-xtask/src/no_tracing.rs";
const FORBIDDEN_DIRECTORIES: &[&str] = &[
    "crates/lkjscript-core/src/gc",
    "crates/lkjscript-jit/src/heap",
];
const FORBIDDEN_MARKERS: &[&str] = &[
    "LegacyTraced",
    "from_legacy_traced",
    "as_legacy_traced",
    "HeapObj",
    "GcConfig",
    "GcHeap",
    "GcLimit",
    "GcStats",
    "JitHeapServices",
    "CollectReference",
    "PublishSafepoint",
    "NativeRoot",
    "ExactStackMap",
    "RootMapRequirement",
    "certified_root_locations",
    "collect_references",
    "materialize_frame_roots",
    "runtime_collect_reference",
    "runtime_publish_safepoint",
    "collector_runtime_invocations",
    "collect_after_allocations",
    "collect_before_every_allocation",
    "allocs_since_gc",
    "needs_collect",
    "set_opaque_word",
    "barrier_count",
    "live_heap_bytes",
    "peak_live_heap_bytes",
];

pub fn check(root: &Path) -> usize {
    if !LEGACY_TRACED_FAMILIES.is_empty() {
        return 0;
    }
    let mut failures = 0;
    for relative in FORBIDDEN_DIRECTORIES {
        let path = root.join(relative);
        if path.exists() {
            eprintln!("{RULE} forbidden collector directory remains: {relative}");
            failures += 1;
        }
    }
    let mut files = Vec::new();
    if let Err(error) = walk(&root.join("crates"), &mut files) {
        eprintln!("{RULE} cannot inspect crate sources: {error}");
        return failures + 1;
    }
    files.sort();
    for path in files {
        if path.extension().and_then(|value| value.to_str()) != Some("rs") || path.ends_with(SELF) {
            continue;
        }
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                eprintln!("{RULE} cannot read {}: {error}", path.display());
                failures += 1;
                continue;
            }
        };
        for marker in source_markers(&source) {
            let relative = path.strip_prefix(root).unwrap_or(&path);
            eprintln!(
                "{RULE} forbidden collector marker {marker:?} remains in {}",
                relative.display()
            );
            failures += 1;
        }
    }
    failures
}

fn source_markers(source: &str) -> Vec<&'static str> {
    FORBIDDEN_MARKERS
        .iter()
        .copied()
        .filter(|marker| source.contains(marker))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::source_markers;

    #[test]
    fn collector_symbols_and_metrics_are_rejected() {
        let source = "let heap: GcHeap; collector_runtime_invocations += 1;";
        assert_eq!(
            source_markers(source),
            vec!["GcHeap", "collector_runtime_invocations"]
        );
    }

    #[test]
    fn deterministic_graph_and_release_terms_are_allowed() {
        let source = concat!(
            "StructuralRootTable RegionStore SealedRegionStore ",
            "dependency_graph release_worklist live_roots",
        );
        assert!(source_markers(source).is_empty());
    }
}
