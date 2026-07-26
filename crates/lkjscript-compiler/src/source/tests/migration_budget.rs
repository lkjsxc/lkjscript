use lkjscript_core::{BudgetLedger, ResourceCategory, ResourceProfile};

use super::*;
use crate::source::{
    check_edition2_migration, load, publish_edition2_migration_with_ledger, SourceEdition,
};

struct Case {
    _directory: TempDir,
    root: PathBuf,
    source: String,
    plan: crate::source::EditionMigrationPlan,
    staged_bytes: u64,
}

fn setup(label: &str) -> Case {
    let directory = TempDir::new(label).expect("temporary repository");
    let status = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&directory.0)
        .status()
        .expect("initialize repository");
    assert!(status.success());
    let root = directory.0.join("main.lkjscript");
    let source = unit_main("unit");
    std::fs::write(&root, &source).expect("write source");
    let tree = load(&root, &Limits::default()).expect("load source");
    let mut ledger = BudgetLedger::default();
    let plan = crate::source::check_edition2_migration_with_ledger(
        &tree,
        tree.revision(),
        &Limits::default(),
        &mut ledger,
    )
    .expect("measure checked migration");
    Case {
        _directory: directory,
        root,
        source,
        plan,
        staged_bytes: ledger.used(ResourceCategory::StagedPublicationBytes),
    }
}

#[test]
fn migration_publication_staging_exact_succeeds() {
    let case = setup("migration-budget-exact");
    let profile = ResourceProfile::default()
        .lowered(ResourceCategory::StagedPublicationBytes, case.staged_bytes)
        .expect("exact profile");
    let mut ledger = BudgetLedger::new(profile);
    let published = publish_edition2_migration_with_ledger(
        &case.root,
        &case.plan,
        &Limits::default(),
        &mut ledger,
    )
    .expect("publish exact checked migration");
    assert_eq!(published.new_edition(), SourceEdition::Edition2);
    assert_ne!(
        std::fs::read_to_string(case.root).expect("published source"),
        case.source
    );
}

#[test]
fn migration_publication_staging_plus_one_does_not_publish() {
    let case = setup("migration-budget-plus-one");
    let profile = ResourceProfile::default()
        .lowered(
            ResourceCategory::StagedPublicationBytes,
            case.staged_bytes - 1,
        )
        .expect("rejecting profile");
    let mut ledger = BudgetLedger::new(profile);
    let error = publish_edition2_migration_with_ledger(
        &case.root,
        &case.plan,
        &Limits::default(),
        &mut ledger,
    )
    .expect_err("work is limit plus one");
    assert_eq!(error.code(), "LKJ-SRC-MIGRATION-LIMIT");
    assert_eq!(
        std::fs::read_to_string(case.root).expect("unchanged source"),
        case.source
    );
}

#[test]
fn migration_check_wrapper_keeps_v2_contract() {
    let case = setup("migration-budget-wrapper");
    let tree = load(&case.root, &Limits::default()).expect("load source");
    let plan = check_edition2_migration(
        &tree,
        tree.revision(),
        &Limits::default(),
        ResourceProfile::default(),
    )
    .expect("legacy wrapper");
    assert_eq!(plan, case.plan);
}
