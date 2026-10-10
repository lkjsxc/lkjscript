//! Compiler impact must contain complete units, never their independently checked children.

use super::*;

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    pub(super) repository: GraphRepository,
    base: CompilationBuildReceipt,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let repository = GraphRepository::create(
            &root.path().join("repository"),
            &structurally_empty_snapshot(b"incremental-unit-ownership"),
            None,
        )
        .unwrap()
        .repository;
        let initial = format!(
            "request base={}\ndeclarations.begin\n(units (module create stable \
             (function create keep (visibility private) (returns I64) \
             (effect pure) (body (i64 7)))))\ndeclarations.end\n",
            repository.current().unwrap().head.revision,
        );
        let request =
            crate::platform::control::decode_compact_change("initial.lkjc", initial.as_bytes())
                .unwrap();
        let prepared = repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap();
        repository.publish(&prepared.publication).unwrap();
        let base = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
        assert_eq!(base.units_compiled, 1);
        Self {
            _root: root,
            repository,
            base,
        }
    }
}

fn new_component(port_count: usize, target: bool) {
    let fixture = Fixture::new();
    let ports = (0..port_count)
        .map(|index| {
            format!("(port create p{index} (as $p{index}) (type (function () I64)) (function one))")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let target_source = if target {
        "(target create extra-main (as $target) (component extra::console) \
         (runner command) (port extra::console::p0))"
    } else {
        ""
    };
    let source = format!(
        "request base={}\ndeclarations.begin\n(units \
         (module create extra (as $extra) \
         (function create one (as $one) (visibility public) (returns I64) \
         (effect pure) (body (i64 1))) \
         (component create console (as $console) (visibility private) {ports})) \
         {target_source})\ndeclarations.end\n",
        fixture.repository.current().unwrap().head.revision,
    );
    let request =
        crate::platform::control::decode_compact_change("new-component.lkjc", source.as_bytes())
            .unwrap();
    let prepared = fixture
        .repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap();
    let mut expected = BTreeSet::from([prepared.allocated["$one"], prepared.allocated["$console"]]);
    if target {
        expected.insert(prepared.allocated["$target"]);
    }
    assert_eq!(
        prepared.publication.compiler_units, expected,
        "{port_count} ports, target={target}; allocations={:?}",
        prepared.allocated
    );
    assert_eq!(
        prepared
            .publication
            .receipt
            .validation
            .compiler_units_planned,
        expected.len() as u64
    );
    fixture.repository.publish(&prepared.publication).unwrap();

    // Cache admission stays strict. Neither real modules nor real ports may be silently dropped.
    for symbol in [Some("$extra"), (port_count != 0).then_some("$p0")]
        .into_iter()
        .flatten()
    {
        let mut invalid = prepared.publication.clone();
        invalid.compiler_units.insert(prepared.allocated[symbol]);
        let before = fixture.repository.current().unwrap().head;
        let error = build_incremental(&fixture.repository, fixture.base.manifest_digest, &invalid)
            .unwrap_err();
        assert_eq!(
            error.class,
            crate::platform::diagnostic::DiagnosticClass::Corrupt
        );
        assert_eq!(error.code, "compilation_incremental_owner_domain");
        assert_eq!(fixture.repository.current().unwrap().head, before);
        assert!(
            load_current_compilation(&fixture.repository)
                .unwrap()
                .is_none()
        );
    }
    let incremental = build_incremental(
        &fixture.repository,
        fixture.base.manifest_digest,
        &prepared.publication,
    )
    .unwrap();
    assert_eq!(incremental.profile, CompilationBuildProfile::Incremental);
    assert_eq!(incremental.units_compiled, expected.len() as u64);
    assert_eq!(incremental.units_reused, 1);
    assert_eq!(incremental.units_removed, 0);
    assert_eq!(incremental.work.inventory_bindings, 0);
    let artifact = link_artifact(&fixture.repository, incremental.manifest_digest, &[]).unwrap();
    let clean = build_clean(
        &fixture.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .unwrap();
    assert_eq!(incremental.manifest, clean.manifest);
    assert_eq!(incremental.manifest_bytes, clean.manifest_bytes);
    assert_eq!(incremental.manifest_digest, clean.manifest_digest);
    let oracle = link_artifact(&fixture.repository, clean.manifest_digest, &[]).unwrap();
    assert_eq!(artifact.artifact.bytes, oracle.artifact.bytes);
    assert_eq!(
        artifact.artifact.bundle_digest,
        oracle.artifact.bundle_digest
    );
    assert_eq!(
        validate_current_compilation(&fixture.repository, clean.manifest_digest)
            .unwrap()
            .units,
        expected.len() as u64 + 1
    );
}

#[test]
fn empty_component_rejects_without_replacing_authority_or_cache() {
    let fixture = Fixture::new();
    let before = fixture.repository.current().unwrap().head;
    let source = format!(
        "request base={}\ndeclarations.begin\n(units (module create empty (component create console (visibility private))))\ndeclarations.end\n",
        before.revision
    );
    let request =
        crate::platform::control::decode_compact_change("empty.lkjc", source.as_bytes()).unwrap();
    let errors = fixture
        .repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "kernel_owner_child_order")
    );
    assert_eq!(fixture.repository.current().unwrap().head, before);
    assert_eq!(
        load_current_compilation(&fixture.repository)
            .unwrap()
            .unwrap()
            .digest,
        fixture.base.manifest_digest
    );
}

#[test]
fn one_port_component_has_two_exact_units() {
    new_component(1, false);
}

#[test]
fn two_port_component_has_two_exact_units() {
    new_component(2, false);
}

#[test]
fn one_port_command_has_three_exact_units() {
    new_component(1, true);
}

#[test]
fn two_port_command_has_three_exact_units() {
    new_component(2, true);
}
