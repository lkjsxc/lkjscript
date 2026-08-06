use super::super::*;

#[test]
fn import_resolution_accepts_only_exact_package_module_paths() {
    let origin = Path::new("/a");
    let package = Path::new("/pkg");
    assert!(super::load::resolve_for_test("../x.lkjscript", origin, package, None).is_err());
    assert!(super::load::resolve_for_test("/x.lkjscript", origin, package, None).is_err());
    assert!(super::load::resolve_for_test("std/x.lkjml", origin, package, None).is_err());
    assert!(super::load::resolve_for_test("./x.lkjscript", origin, package, None).is_err());
    assert_eq!(
        super::load::resolve_for_test("src/std/list/nth.lkjscript", origin, package, None).ok(),
        Some(PathBuf::from("/pkg/src/std/list/nth.lkjscript"))
    );
}
