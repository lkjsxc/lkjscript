use super::super::*;

#[test]
fn source_tree_counts_git_and_target_in_sixteen_entry_rule() -> std::io::Result<()> {
    let accepted = TempDir::new("sixteen")?;
    fs::create_dir(accepted.0.join(".git"))?;
    fs::create_dir(accepted.0.join("target"))?;
    for index in 0..14 {
        fs::write(accepted.0.join(format!("source-{index}.lkjscript")), "")?;
    }
    assert!(super::validate_source_directory_tree(&accepted.0, 16).is_ok());

    let rejected = TempDir::new("seventeen")?;
    fs::create_dir(rejected.0.join(".git"))?;
    fs::create_dir(rejected.0.join("target"))?;
    for index in 0..15 {
        fs::write(rejected.0.join(format!("source-{index}.lkjscript")), "")?;
    }
    let error = super::validate_source_directory_tree(&rejected.0, 16)
        .expect_err(".git and target count as source entries");
    assert!(error.message().contains("at least 17 entries (max 16)"));
    Ok(())
}

#[test]
fn import_resolution_rejects_climbs_absolute_and_legacy_extensions() {
    let origin = Path::new("/a");
    let package = Path::new("/pkg");
    assert!(super::load::resolve_for_test("../x.lkjscript", origin, package, None).is_err());
    assert!(super::load::resolve_for_test("/x.lkjscript", origin, package, None).is_err());
    assert!(super::load::resolve_for_test("std/x.lkjml", origin, package, None).is_err());
    assert_eq!(
        super::load::resolve_for_test("./x.lkjscript", origin, package, None).ok(),
        Some(PathBuf::from("/a/x.lkjscript"))
    );
    assert_eq!(
        super::load::resolve_for_test(
            "std/list/nth.lkjscript",
            origin,
            package,
            Some(Path::new("/opt/lkjscript")),
        )
        .ok(),
        Some(PathBuf::from("/opt/lkjscript/src/std/list/nth.lkjscript"))
    );
}
