use super::*;
use crate::source::{diff_edition2_migration, publish_edition2_migration, SourceEdition};
use lkjscript_core::{ResourceProfile, ResourceProfileName};

const MARKER: &str = "edition/\n2\n/edition\n";

struct Closure {
    directory: TempDir,
    root: PathBuf,
    dependency: PathBuf,
    root_source: String,
    dependency_source: String,
}

fn closure(label: &str) -> std::io::Result<Closure> {
    let directory = TempDir::new(label)?;
    fs::create_dir(directory.0.join(".git"))?;
    let root = directory.0.join("main.lkjscript");
    let dependency = directory.0.join("dependency.lkjscript");
    let root_source = concat!(
        "import/\ndependency.lkjscript\n/import\n",
        "main/\nsig/\n->\nF64\n/sig\nhelper/\n/helper\n/main\n",
    )
    .to_string();
    let dependency_source = concat!(
        "def/\nname/\nhelper\n/name\nfn/\nsig/\n->\nF64\n/sig\n",
        "params/\n/params\n+/\n2.5\n1\n/+\n/fn\n/def\n",
    )
    .to_string();
    fs::write(&root, &root_source)?;
    fs::write(&dependency, &dependency_source)?;
    Ok(Closure {
        directory,
        root,
        dependency,
        root_source,
        dependency_source,
    })
}

fn checked(case: &Closure) -> crate::source::EditionMigrationPlan {
    let tree = load(&case.root, &Limits::default()).expect("load Edition 1 closure");
    diff_edition2_migration(
        &case.root,
        tree.revision(),
        &Limits::default(),
        ResourceProfile::new(ResourceProfileName::Deterministic),
    )
    .expect("check exact closure diff")
}

#[test]
fn all_file_publish_is_atomic_exact_and_idempotent() -> std::io::Result<()> {
    let case = closure("migration-publish")?;
    let plan = checked(&case);
    assert_eq!(plan.changes().len(), 2);
    assert_eq!(
        plan.old_bytes(),
        (case.root_source.len() + case.dependency_source.len()) as u64
    );
    assert!(plan.new_bytes() > plan.old_bytes());
    assert_eq!(plan.declarations().len(), 2);
    assert!(!plan.nodes().is_empty());
    publish_edition2_migration(
        &case.root,
        &plan,
        &Limits::default(),
        ResourceProfile::new(ResourceProfileName::Deterministic),
    )
    .expect("publish complete closure");
    for path in [&case.root, &case.dependency] {
        assert!(fs::read_to_string(path)?.starts_with(MARKER));
    }
    let tree = load(&case.root, &Limits::default()).expect("load published closure");
    assert_eq!(tree.edition(), SourceEdition::Edition2);
    let second = diff_edition2_migration(
        &case.root,
        tree.revision(),
        &Limits::default(),
        ResourceProfile::default(),
    )
    .expect("idempotent diff");
    assert!(second.is_idempotent());
    publish_edition2_migration(
        &case.root,
        &second,
        &Limits::default(),
        ResourceProfile::default(),
    )
    .expect("idempotent publish");
    drop(case.directory);
    Ok(())
}

#[test]
fn stale_conflict_rejects_without_partial_write() -> std::io::Result<()> {
    let case = closure("migration-conflict")?;
    let plan = checked(&case);
    let external = format!("{};; external\n", case.dependency_source);
    fs::write(&case.dependency, &external)?;
    let failure = publish_edition2_migration(
        &case.root,
        &plan,
        &Limits::default(),
        ResourceProfile::default(),
    )
    .expect_err("stale checked closure must reject");
    assert!(matches!(
        failure.code(),
        "LKJ-SRC-STALE-MIGRATION" | "LKJ-SRC-MIGRATION-CONFLICT"
    ));
    assert_eq!(fs::read_to_string(&case.root)?, case.root_source);
    assert_eq!(fs::read_to_string(&case.dependency)?, external);
    Ok(())
}

#[test]
fn partial_install_failure_rolls_back_every_file() -> std::io::Result<()> {
    let case = closure("migration-rollback")?;
    let plan = checked(&case);
    crate::source::migration::simulate_checked_rollback(&case.root, &plan, &Limits::default())
        .expect("injected failure rolls back");
    assert_eq!(fs::read_to_string(&case.root)?, case.root_source);
    assert_eq!(
        fs::read_to_string(&case.dependency)?,
        case.dependency_source
    );
    assert_eq!(
        load(&case.root, &Limits::default()).unwrap().edition(),
        SourceEdition::Edition1
    );
    Ok(())
}

#[test]
fn prepared_crash_recovers_before_exact_publish() -> std::io::Result<()> {
    let case = closure("migration-crash")?;
    let plan = checked(&case);
    let journal =
        crate::source::migration::simulate_checked_crash(&case.root, &plan, &Limits::default())
            .expect("simulate prepared crash");
    assert!(journal.exists());
    publish_edition2_migration(
        &case.root,
        &plan,
        &Limits::default(),
        ResourceProfile::default(),
    )
    .expect("recover then publish");
    assert!(!journal.exists());
    assert_eq!(
        load(&case.root, &Limits::default()).unwrap().edition(),
        SourceEdition::Edition2
    );
    Ok(())
}

#[test]
fn resolved_conversion_is_exact_and_preserves_old_result() {
    let source = "main/\nsig/\n->\nF64\n/sig\n+/\n2.5\n1\n/+\n/main\n";
    let old = validate(source, "main.lkjscript", &Limits::default()).unwrap();
    let old_hir = crate::analyze::analyze_program(&old).unwrap();
    let old_ssa = crate::ssa::lower_program(&old_hir).unwrap();
    let old_result = lkjscript_ir::evaluate(&old_ssa, &lkjscript_ir::EvalConfig::default());
    let plan = crate::source::check_edition2_migration(
        &old,
        old.revision(),
        &Limits::default(),
        ResourceProfile::default(),
    )
    .unwrap();
    let change = &plan.changes()[0];
    assert_eq!(change.conversion_count(), 1);
    assert!(change
        .replacement_source()
        .contains("+/\n2.5\nf64-from-i64-rounded/\n1\n/f64-from-i64-rounded\n/+"));
    let compiled = crate::compile_source(
        change.replacement_source(),
        "main.lkjscript",
        &Limits::default(),
    )
    .unwrap();
    assert_eq!(
        old_result,
        lkjscript_ir::evaluate(compiled.ssa(), &lkjscript_ir::EvalConfig::default())
    );
}
