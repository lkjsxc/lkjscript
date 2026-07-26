use super::*;
use crate::source::{check_edition2_migration, publish_edition2_migration};
use lkjscript_core::ResourceProfile;

#[test]
fn concurrent_publication_conflict_rejects_without_writing() -> std::io::Result<()> {
    let directory = TempDir::new("migration-lock-conflict")?;
    fs::create_dir(directory.0.join(".git"))?;
    let root = directory.0.join("main.lkjscript");
    let source = unit_main("unit");
    fs::write(&root, &source)?;
    let tree = load(&root, &Limits::default()).expect("load conflict source");
    let plan = check_edition2_migration(
        &tree,
        tree.revision(),
        &Limits::default(),
        ResourceProfile::default(),
    )
    .expect("check conflict plan");
    let staging = directory.0.join("target/lkjscript/semantic-staging");
    fs::create_dir_all(&staging)?;
    fs::write(
        staging.join("publication.lock"),
        format!("{}\n", std::process::id()),
    )?;
    let failure =
        publish_edition2_migration(&root, &plan, &Limits::default(), ResourceProfile::default())
            .expect_err("concurrent publication must reject");
    assert!(failure
        .message()
        .contains("concurrent semantic publication"));
    assert_eq!(fs::read_to_string(root)?, source);
    Ok(())
}
