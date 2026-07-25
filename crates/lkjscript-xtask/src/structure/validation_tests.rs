use std::collections::{BTreeMap, BTreeSet};

#[test]
fn cycles_are_detected() {
    let a = vec!["b".to_owned()];
    let b = vec!["a".to_owned()];
    let graph = BTreeMap::from([("a", a.as_slice()), ("b", b.as_slice())]);
    assert!(crate::structure_validation::cycle(
        "a",
        "a",
        &graph,
        &mut BTreeSet::new()
    ));
}

#[test]
fn byte_and_width_boundaries() {
    for (observed, count) in [(32_767, 0), (32_768, 0), (32_769, 1)] {
        let mut findings = Vec::new();
        crate::structure_validation::metric(
            &mut findings,
            "structure.file.bytes",
            "a",
            observed,
            32_768,
        );
        assert_eq!(findings.len(), count);
    }
    for (observed, count) in [(119, 0), (120, 0), (121, 1)] {
        let mut findings = Vec::new();
        crate::structure_validation::metric(
            &mut findings,
            "structure.line.scalars",
            "a",
            observed,
            120,
        );
        assert_eq!(findings.len(), count);
    }
}

#[test]
fn stale_provenance_is_rejected() {
    let root = std::env::temp_dir().join(format!("lkjscript-provenance-{}", std::process::id()));
    assert!(std::fs::create_dir_all(&root).is_ok());
    assert!(std::fs::write(root.join("a"), "actual").is_ok());
    let files = vec![crate::model::FileRecord {
        path: "a".into(),
        bytes: 6,
        lines: 1,
        max_line_scalars: 6,
        class: "immutable-fixture".into(),
        capsule: None,
    }];
    let entries = vec![crate::model::Provenance {
        path: "a".into(),
        class: "immutable-fixture".into(),
        sha256: "0".repeat(64),
        generator: None,
    }];
    let mut findings = Vec::new();
    crate::structure_validation::provenance(&root, &files, &entries, &mut findings);
    assert!(findings
        .iter()
        .any(|finding| finding.rule == "structure.provenance.stale"));
    assert!(std::fs::remove_dir_all(root).is_ok());
}
