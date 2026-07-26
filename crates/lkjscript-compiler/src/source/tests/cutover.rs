use super::*;
use crate::source::SourceEdition;

#[test]
fn ordinary_compile_rejects_markerless_source_while_explicit_validation_accepts_it() {
    let source = unit_main("unit");
    let validated = validate(&source, "main.lkjscript", &Limits::default())
        .expect("explicit migration/source validation accepts Edition 1");
    assert_eq!(validated.edition(), SourceEdition::Edition1);
    let failure = crate::compile_source(&source, "main.lkjscript", &Limits::default())
        .expect_err("ordinary compilation must reject Edition 1");
    assert!(failure.to_string().contains("LKJ-SRC-EDITION-CUTOVER"));
}

#[test]
fn ordinary_path_compile_has_no_edition_inference_or_fallback() -> std::io::Result<()> {
    let directory = TempDir::new("cutover-path")?;
    let root = directory.0.join("main.lkjscript");
    fs::write(&root, unit_main("unit"))?;
    let explicit = load(&root, &Limits::default()).expect("explicit source load accepts Edition 1");
    assert_eq!(explicit.edition(), SourceEdition::Edition1);
    let failure = crate::compile_path(&root, &Limits::default())
        .expect_err("ordinary path compilation must reject Edition 1");
    assert!(failure.to_string().contains("LKJ-SRC-EDITION-CUTOVER"));
    Ok(())
}
