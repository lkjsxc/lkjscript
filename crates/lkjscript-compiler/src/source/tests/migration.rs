use super::*;
use crate::source::{check_edition2_migration, SourceEdition};
use lkjscript_core::{ResourceCategory, ResourceProfile};

const MARKER: &str = "edition/\n2\n/edition\n";

#[test]
fn migration_diff_inserts_only_marker_after_leading_trivia() {
    let old_source = format!("\n;; retained\n{}", unit_main("unit"));
    let tree = validate(&old_source, "src/main.lkjscript", &Limits::default()).expect("Edition 1");
    let plan = check_edition2_migration(
        &tree,
        tree.revision(),
        &Limits::default(),
        ResourceProfile::default(),
    )
    .expect("migration diff");
    assert!(!plan.is_idempotent());
    assert_eq!(plan.old_edition(), SourceEdition::Edition1);
    assert_eq!(plan.new_edition(), SourceEdition::Edition2);
    assert_eq!(plan.old_bytes(), old_source.len() as u64);
    assert_eq!(plan.new_bytes(), (old_source.len() + MARKER.len()) as u64);
    assert_ne!(plan.old_revision(), plan.new_revision());
    assert_ne!(plan.old_tree_identity(), plan.new_tree_identity());
    let [change] = plan.changes() else {
        panic!("one changed source")
    };
    assert_eq!(change.path(), "src/main.lkjscript");
    assert_eq!(change.insertion_byte(), 13);
    assert_eq!(change.inserted_bytes(), MARKER);
    assert_eq!(change.old_bytes(), old_source.len() as u64);
    assert_eq!(change.new_bytes(), (old_source.len() + MARKER.len()) as u64);
    assert_ne!(change.old_identity(), change.new_identity());
    let offset = change.insertion_byte() as usize;
    let mut restored = change.replacement_source().to_string();
    restored.replace_range(offset..offset + change.inserted_bytes().len(), "");
    assert_eq!(restored, old_source);
    let migrated = validate(
        change.replacement_source(),
        change.path(),
        &Limits::default(),
    )
    .expect("staged source validates");
    assert_eq!(migrated.edition(), SourceEdition::Edition2);
    assert_eq!(migrated.revision(), plan.new_revision());
}

#[test]
fn migration_is_deterministic_and_never_publishes() -> std::io::Result<()> {
    let directory = TempDir::new("migration-no-publish")?;
    let root = directory.0.join("main.lkjscript");
    let original = unit_main("unit");
    fs::write(&root, &original)?;
    let tree = load(&root, &Limits::default())
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    let first = check_edition2_migration(
        &tree,
        tree.revision(),
        &Limits::default(),
        ResourceProfile::default(),
    )
    .map_err(|error| std::io::Error::other(error.to_string()))?;
    let second = check_edition2_migration(
        &tree,
        tree.revision(),
        &Limits::default(),
        ResourceProfile::default(),
    )
    .map_err(|error| std::io::Error::other(error.to_string()))?;
    assert_eq!(first, second);
    assert_eq!(fs::read_to_string(root)?, original);
    Ok(())
}

#[test]
fn stale_migration_revision_rejects_before_staging() {
    let tree = validate(&unit_main("unit"), "src/main.lkjscript", &Limits::default()).unwrap();
    let stale = validate(&unit_main("true"), "src/main.lkjscript", &Limits::default()).unwrap();
    let error = check_edition2_migration(
        &tree,
        stale.revision(),
        &Limits::default(),
        ResourceProfile::default(),
    )
    .expect_err("stale revision");
    assert_eq!(error.code(), "LKJ-SRC-STALE-MIGRATION");
    assert!(error.message().contains(&tree.revision().to_hex()));
}

#[test]
fn edition2_migration_check_is_idempotent() {
    let source = format!("{MARKER}{}", unit_main("unit"));
    let tree = validate(&source, "src/main.lkjscript", &Limits::default()).unwrap();
    let plan = check_edition2_migration(
        &tree,
        tree.revision(),
        &Limits::default(),
        ResourceProfile::default(),
    )
    .expect("idempotent check");
    assert!(plan.is_idempotent());
    assert_eq!(plan.old_edition(), SourceEdition::Edition2);
    assert_eq!(plan.new_edition(), SourceEdition::Edition2);
    assert_eq!(plan.old_bytes(), source.len() as u64);
    assert_eq!(plan.old_bytes(), plan.new_bytes());
    assert_eq!(plan.old_revision(), plan.new_revision());
    assert_eq!(plan.old_tree_identity(), plan.new_tree_identity());
}

#[test]
fn migration_preallocation_accepts_exact_bytes_and_rejects_plus_one() {
    let source = unit_main("unit");
    let tree = validate(&source, "src/main.lkjscript", &Limits::default()).unwrap();
    let staged_bytes = (source.len() + MARKER.len()) as u64;
    let exact = ResourceProfile::default()
        .lowered(ResourceCategory::StagedPublicationBytes, staged_bytes)
        .unwrap();
    check_edition2_migration(&tree, tree.revision(), &Limits::default(), exact)
        .expect("exact staged-byte budget");
    let below = ResourceProfile::default()
        .lowered(ResourceCategory::StagedPublicationBytes, staged_bytes - 1)
        .unwrap();
    let error = check_edition2_migration(&tree, tree.revision(), &Limits::default(), below)
        .expect_err("work is limit plus one");
    assert_eq!(error.code(), "LKJ-SRC-MIGRATION-LIMIT");
    assert!(error.message().contains("staged_publication_bytes"));
}

#[test]
fn migration_preallocation_accepts_exact_nodes_and_rejects_plus_one() {
    let tree = validate(&unit_main("unit"), "src/main.lkjscript", &Limits::default()).unwrap();
    let staged_nodes = tree.nodes().len() as u64 + 2;
    let exact = ResourceProfile::default()
        .lowered(ResourceCategory::StagedPublicationNodes, staged_nodes)
        .unwrap();
    check_edition2_migration(&tree, tree.revision(), &Limits::default(), exact)
        .expect("exact staged-node budget");
    let below = ResourceProfile::default()
        .lowered(ResourceCategory::StagedPublicationNodes, staged_nodes - 1)
        .unwrap();
    let error = check_edition2_migration(&tree, tree.revision(), &Limits::default(), below)
        .expect_err("work is limit plus one");
    assert_eq!(error.code(), "LKJ-SRC-MIGRATION-LIMIT");
    assert!(error.message().contains("staged_publication_nodes"));
}

#[test]
fn migration_plans_every_unit_in_a_homogeneous_closure() {
    let files = [
        ("src/dep.lkjscript", named_def("helper")),
        ("src/main.lkjscript", unit_main("unit")),
    ];
    let borrowed: Vec<_> = files
        .iter()
        .map(|(path, source)| (*path, source.as_str()))
        .collect();
    let tree = crate::source::validate_source_set_for_analysis(
        &borrowed,
        "src/main.lkjscript",
        &Limits::default(),
    )
    .expect("homogeneous closure");
    let plan = check_edition2_migration(
        &tree,
        tree.revision(),
        &Limits::default(),
        ResourceProfile::default(),
    )
    .expect("closure migration");
    assert_eq!(plan.changes().len(), 2);
    assert_eq!(plan.changes()[0].path(), "src/dep.lkjscript");
    assert_eq!(plan.changes()[1].path(), "src/main.lkjscript");
}
