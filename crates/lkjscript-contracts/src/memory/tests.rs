use super::*;

#[test]
fn inventory_is_sorted_unique_complete_and_explicit_about_transition() {
    let records = memory_obligations();
    assert!(records
        .windows(2)
        .all(|pair| pair[0].identity < pair[1].identity));
    for required in [
        "bool",
        "buf",
        "builtin",
        "byte-slice",
        "byte-slice-mut",
        "byte-vector",
        "bytes",
        "cache-object",
        "capability",
        "closure",
        "component-storage",
        "cyclic-graph",
        "database-page",
        "diagnostics",
        "enum",
        "error",
        "evaluator-state",
        "f64",
        "file-reader",
        "file-writer",
        "game-entity",
        "gc-heap",
        "hir",
        "i64",
        "list",
        "module-graph",
        "native-frame",
        "option",
        "ordinary-region",
        "package-metadata",
        "pair",
        "path",
        "pool",
        "pool-id",
        "precise-shared-node",
        "product",
        "resource-table",
        "result",
        "returned-value",
        "sealed-shared-region",
        "semantic-source-revision",
        "source-tree",
        "ssa",
        "static-function",
        "string",
        "symbol",
        "typed-hole-candidates",
        "unit",
        "vm-frame",
        "weak-reference",
        "web-request",
    ] {
        assert!(
            records.iter().any(|record| record.identity == required),
            "missing {required}"
        );
    }
    assert_eq!(
        records
            .iter()
            .filter(|record| record.authority.contains("resource"))
            .count(),
        ResourceKind::ALL.len(),
    );
    let bytes = records.iter().find(|record| record.identity == "bytes");
    assert!(matches!(bytes, Some(record) if record.status.contains("current exact evaluator/VM")));
    let builtin = records.iter().find(|record| record.identity == "builtin");
    assert!(matches!(builtin, Some(record) if record.current_trace_fields == "none"));
    assert!(matches!(builtin, Some(record) if record.runtime_layout.contains("no HeapObj")));
    let closure = records.iter().find(|record| record.identity == "closure");
    assert!(matches!(closure, Some(record) if record.current_trace_fields == "none"));
    assert!(matches!(closure, Some(record) if record.runtime_layout.contains("Value::Function")));
    let heap = records.iter().find(|record| record.identity == "gc-heap");
    assert!(matches!(heap, Some(record) if record.current_trace_fields.contains("trace")));
    assert!(matches!(heap, Some(record) if record.status.contains("current collector")));
}

#[test]
fn every_record_populates_every_obligation_field() {
    for record in memory_obligations() {
        for value in fields(&record) {
            assert!(!value.is_empty(), "{} has an empty field", record.identity);
        }
    }
}

fn fields(record: &MemoryObligation) -> [&str; 28] {
    [
        record.identity,
        record.authority,
        record.semantic_type,
        record.runtime_layout,
        record.value_semantics,
        record.mutability,
        record.possible_aliases,
        record.copyability,
        record.current_ownership,
        record.escape_behavior,
        record.lifetime,
        record.strong_cycles,
        record.weak_links,
        record.destructor,
        record.external_resources,
        record.portability,
        record.contention,
        record.allocation_frequency,
        record.size_class,
        record.current_trace_fields,
        record.current_exact_roots,
        record.object_identity,
        record.current_placement,
        record.candidate_placements,
        record.reclamation_plan,
        record.producers,
        record.tests,
        record.status,
    ]
}
