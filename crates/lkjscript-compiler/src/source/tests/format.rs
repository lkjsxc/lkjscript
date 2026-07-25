use super::*;

#[test]
fn formatter_is_structural_idempotent_and_handles_escape_zero_utf8_and_lf() {
    let source = "main/\nsig/\n->\nF64\n/sig\ndo/\n;; attached to zero\n-0.0\nstr/\nλ\n\\/str\n/str\n\n/do\n/main\n;; trailing";
    let first = validate(source, "src/format.lkjscript", &Limits::default()).expect("parse");
    let formatted = first.format_single_source().expect("one source");
    assert!(formatted.ends_with('\n'));
    assert!(formatted.contains("-0.0\n"));
    assert!(formatted.contains("λ\n\\/str\n"));
    assert!(formatted.contains(";; attached to zero\n-0.0\n"));
    assert!(formatted.ends_with(";; trailing\n"));
    let reparsed =
        validate(&formatted, "src/format.lkjscript", &Limits::default()).expect("parse formatted");
    let reformatted = reparsed.format_single_source().expect("one source");
    assert_eq!(formatted, reformatted);
    assert_eq!(
        first
            .nodes()
            .iter()
            .map(|node| (node.kind(), node.label().map(str::to_owned)))
            .collect::<Vec<_>>(),
        reparsed
            .nodes()
            .iter()
            .map(|node| (node.kind(), node.label().map(str::to_owned)))
            .collect::<Vec<_>>()
    );
}

#[test]
fn all_125_tracked_source_corpus_files_roundtrip_exactly() -> std::io::Result<()> {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut files = Vec::new();
    collect_sources(&workspace.join("src"), &mut files)?;
    collect_sources(
        &workspace.join("crates/lkjscript-app/tests/fixtures"),
        &mut files,
    )?;
    collect_sources(
        &workspace.join("meta/benchmarks/jit/pre-jit-workload"),
        &mut files,
    )?;
    files.sort();
    assert_eq!(files.len(), 125, "tracked source corpus changed");
    for path in files {
        let source = fs::read_to_string(&path)?;
        let logical = path
            .strip_prefix(&workspace)
            .expect("workspace source")
            .to_str()
            .expect("workspace source path must be UTF-8");
        let tree = validate(&source, logical, &Limits::default())
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let formatted = tree.format_single_source().expect("single file");
        assert_eq!(
            formatted.as_bytes(),
            source.as_bytes(),
            "{}",
            path.display()
        );
        let reparsed = validate(&formatted, logical, &Limits::default())
            .expect("parse formatted corpus source");
        assert_eq!(
            reparsed.format_single_source().as_deref(),
            Some(formatted.as_str()),
            "{}",
            path.display()
        );
    }
    Ok(())
}

fn collect_sources(directory: &Path, output: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            collect_sources(&entry.path(), output)?;
        } else if entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("lkjscript")
        {
            output.push(entry.path());
        }
    }
    Ok(())
}
