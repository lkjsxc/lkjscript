#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use lkjscript_core::Limits;

use super::{
    finish_tree, load, parser, validate, validate_source_directory_tree, DeclarationKind, NodeKind,
    SourceFoundationBudget, SourceOrigin, FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES,
    FOUNDATION_MAX_SOURCE_FILE_BYTES, FOUNDATION_MAX_SOURCE_UNITS,
};

static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

struct TempDir(PathBuf);

impl TempDir {
    fn new(label: &str) -> std::io::Result<Self> {
        let id = NEXT_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "lkjscript-source-{label}-{}-{id}",
            std::process::id()
        ));
        fs::create_dir(&path)?;
        Ok(Self(path))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn unit_main(body: &str) -> String {
    format!("main/\nsig/\n->\nUnit\n/sig\n{body}\n/main\n")
}

fn named_def(name: &str) -> String {
    format!(
        "def/\nname/\n{name}\n/name\nfn/\nsig/\n->\nUnit\n/sig\nparams/\n/params\nunit\n/fn\n/def\n"
    )
}

#[test]
fn exact_utf8_byte_line_column_spans_are_retained() {
    let source = unit_main("str/\néλ\n/str");
    let tree = validate(&source, "src/utf8.lkjscript", &Limits::default()).expect("validate");
    let string = tree
        .nodes()
        .iter()
        .find(|node| node.kind() == NodeKind::StringLiteral)
        .expect("string node");
    let span = string.span();
    let start = source.find("str/\n").expect("string open");
    let end = source.find("\n/str").expect("string close") + "\n/str".len();
    assert_eq!(span.start().byte() as usize, start);
    assert_eq!(span.end().byte() as usize, end);
    assert_eq!(span.start().line(), 6);
    assert_eq!(span.start().column(), 1);
    assert_eq!(span.end().line(), 8);
    assert_eq!(span.end().column(), 5);
}

#[test]
fn marker_diagnostics_have_stable_schema_spans_and_renderings() {
    let mismatched =
        validate("main/\n/wrong\n", "bad.lkjscript", &Limits::default()).expect_err("mismatch");
    assert_eq!(
        mismatched.schema(),
        "lkjscript.source-diagnostic-foundation"
    );
    assert_eq!(mismatched.schema_version(), 1);
    assert_eq!(mismatched.code(), "LKJ-SRC-UNMATCHED-MARKER");
    assert_eq!(mismatched.primary_span().start().line(), 2);
    assert_eq!(mismatched.related_spans().len(), 1);
    assert!(mismatched.render_human().contains("expected /main"));
    let compact = mismatched.render_compact_agent();
    assert!(compact.starts_with(
        "schema=lkjscript.source-diagnostic-foundation;version=1;code=LKJ-SRC-UNMATCHED-MARKER"
    ));
    assert!(compact.contains("related[0].label=opening marker main/"));
    assert!(compact.contains("related[0].origin=bad.lkjscript"));
    assert_eq!(compact, mismatched.render_compact_agent());

    let unexpected =
        validate("/main\n", "bad.lkjscript", &Limits::default()).expect_err("unexpected close");
    assert_eq!(unexpected.code(), "LKJ-SRC-UNMATCHED-MARKER");
    assert_eq!(unexpected.primary_span().byte_range(), 0..5);

    let unclosed =
        validate("main/\n", "bad.lkjscript", &Limits::default()).expect_err("unclosed open");
    assert_eq!(unclosed.code(), "LKJ-SRC-UNMATCHED-MARKER");
    assert_eq!(unclosed.primary_span().start().line(), 1);
}

#[test]
fn lexical_and_numeric_malformed_boundaries_are_rejected() {
    for source in [
        "  one\n",
        "one two\n",
        "\"hi\"\n",
        "main/\n/main\n",
        "def/\nname/\nx\n/name\nfn/\n/fn\n/def\n",
    ] {
        assert!(validate(source, "bad.lkjscript", &Limits::default()).is_err());
    }
    for spelling in [
        "+1", "1e3", "1.", ".", "-.", "+.", ".5", "-.5", "+.5", "--1", "+-1", "1_000", "0x10",
        "1.2.3", "NaN", "+inf", "inf",
    ] {
        let source = unit_main(spelling);
        assert!(
            validate(&source, "numeric.lkjscript", &Limits::default()).is_err(),
            "accepted {spelling}"
        );
    }
    assert!(validate(
        &unit_main("-9223372036854775808"),
        "min.lkjscript",
        &Limits::default()
    )
    .is_ok());
    assert!(validate(
        &unit_main("9223372036854775807"),
        "max.lkjscript",
        &Limits::default()
    )
    .is_ok());
    assert!(validate(
        &unit_main("9223372036854775808"),
        "overflow.lkjscript",
        &Limits::default()
    )
    .is_err());
}

#[test]
fn duplicate_same_unit_global_declarations_are_structured_errors() {
    let source = format!(
        "{}{}{}",
        named_def("same"),
        named_def("same"),
        unit_main("unit")
    );
    let error = validate(&source, "src/duplicate.lkjscript", &Limits::default())
        .expect_err("duplicate key");
    assert_eq!(error.code(), "LKJ-DECL-DUPLICATE");
    assert_eq!(error.related_spans().len(), 1);
    assert!(error
        .render_human()
        .contains("duplicate function declaration same"));
}

#[test]
fn duplicate_global_names_across_source_units_are_structured_errors() {
    let temp = TempDir::new("duplicate-global").expect("temp directory");
    let root = temp.0.join("main.lkjscript");
    fs::write(temp.0.join("a.lkjscript"), named_def("same")).expect("write a");
    fs::write(temp.0.join("b.lkjscript"), named_def("same")).expect("write b");
    fs::write(
        &root,
        format!(
            "import/\n./a.lkjscript\n/import\nimport/\n./b.lkjscript\n/import\n{}",
            unit_main("unit")
        ),
    )
    .expect("write root");

    let error = load(&root, &Limits::default()).expect_err("duplicate global");
    assert_eq!(error.code(), "LKJ-DECL-DUPLICATE");
    assert_eq!(error.related_spans().len(), 1);
    assert!(error
        .render_human()
        .contains("duplicate function declaration same"));
}

#[test]
fn declaration_keys_ignore_order_offsets_and_nonsemantic_trivia() {
    let first = format!("{}{}{}", named_def("a"), named_def("b"), unit_main("unit"));
    let second = format!(
        ";; leading\n\n{}\n{}{}",
        named_def("b"),
        named_def("a"),
        unit_main("unit")
    );
    let first = validate(&first, "src/keys.lkjscript", &Limits::default()).expect("first");
    let second = validate(&second, "src/keys.lkjscript", &Limits::default()).expect("second");
    for name in ["a", "b", "$main"] {
        let left = first
            .declarations()
            .iter()
            .find(|declaration| declaration.name() == name)
            .expect("first declaration");
        let right = second
            .declarations()
            .iter()
            .find(|declaration| declaration.name() == name)
            .expect("second declaration");
        assert_eq!(left.key(), right.key());
    }
    assert_ne!(first.revision(), second.revision());
}

#[test]
fn exact_source_spelling_and_line_endings_change_revisions_and_reject_stale_nodes() {
    let numeric_one =
        validate(&unit_main("1.0"), "src/alias.lkjscript", &Limits::default()).expect("1.0");
    let numeric_two = validate(
        &unit_main("1.00"),
        "src/alias.lkjscript",
        &Limits::default(),
    )
    .expect("1.00");
    assert_ne!(numeric_one.revision(), numeric_two.revision());
    assert_eq!(
        numeric_one.format_single_source(),
        numeric_two.format_single_source()
    );
    let stale_numeric = numeric_two
        .node(numeric_one.nodes()[0].id())
        .expect_err("numeric alias NodeId must be stale");
    assert_eq!(stale_numeric.actual_revision(), numeric_one.revision());
    assert_eq!(stale_numeric.expected_revision(), numeric_two.revision());

    let lf = unit_main("unit");
    let crlf = lf.replace('\n', "\r\n");
    let lf = validate(&lf, "src/endings.lkjscript", &Limits::default()).expect("LF");
    let crlf = validate(&crlf, "src/endings.lkjscript", &Limits::default()).expect("CRLF");
    assert_ne!(lf.revision(), crlf.revision());
    assert_eq!(lf.format_single_source(), crlf.format_single_source());
    assert!(crlf.node(lf.nodes()[0].id()).is_err());
}

#[test]
fn declaration_key_framing_prevents_delimiter_and_path_false_collisions() {
    let left_path = "src/a.lkjscript";
    let left_name = "x.lkjscript;kind=trait;name=y";
    let right_path = "src/a.lkjscript;kind=function;name=x.lkjscript";
    let right_name = "y";
    let old_left = format!(
        "origin={left_path};kind={};name={left_name}",
        DeclarationKind::Function.as_str()
    );
    let old_right = format!(
        "origin={right_path};kind={};name={right_name}",
        DeclarationKind::Trait.as_str()
    );
    assert_eq!(old_left, old_right, "adversarial delimiter setup");
    assert_ne!(
        super::declaration_key_bytes(left_path, DeclarationKind::Function, left_name),
        super::declaration_key_bytes(right_path, DeclarationKind::Trait, right_name)
    );

    let human = super::declaration_key_human_identity(
        "src/a=b;path.lkjscript",
        DeclarationKind::Function,
        "callable=name",
    );
    assert!(human.contains("origin=src/a%3Db%3Bpath.lkjscript"));
    assert!(human.contains("name=callable%3Dname"));
}

#[test]
fn declaration_names_must_be_spellable_source_identifiers_before_keying() {
    let sources = [
        named_def("uncallable;name"),
        "product/\nname/\nuncallable;name\n/name\nfields/\n/fields\n/product\n".into(),
        "trait/\nname/\nuncallable;name\n/name\n/trait\n".into(),
    ];
    for source in sources {
        let error = validate(&source, "src/name.lkjscript", &Limits::default())
            .expect_err("uncallable declaration name");
        assert_eq!(error.code(), "LKJ-DECL-NAME");
        assert!(error
            .message()
            .contains("not a spellable source identifier"));
    }

    let callable = validate(
        &named_def("callable=name"),
        "src/name.lkjscript",
        &Limits::default(),
    )
    .expect("spellable equals name");
    assert_eq!(callable.declarations()[0].name(), "callable=name");
}

#[test]
fn distinct_source_units_cannot_share_one_logical_origin() {
    let origin = SourceOrigin::in_memory("src/duplicate-origin.lkjscript");
    let first = parser::parse_file(
        &named_def("first"),
        origin.clone(),
        PathBuf::from("host-a.lkjscript"),
        &Limits::default(),
    )
    .expect("first source");
    let second = parser::parse_file(
        &named_def("second"),
        origin.clone(),
        PathBuf::from("host-b.lkjscript"),
        &Limits::default(),
    )
    .expect("second source");
    let error = finish_tree(
        PathBuf::from("host-a.lkjscript"),
        origin,
        vec![first, second],
    )
    .expect_err("duplicate logical origin");
    assert_eq!(error.code(), "LKJ-SRC-LOAD");
    assert_eq!(error.related_spans().len(), 1);
    assert!(error.message().contains("duplicate logical origin"));
}

#[test]
fn node_ids_are_dense_preorder_deterministic_and_revision_scoped() {
    let source = unit_main("do/\nunit\ntrue\n/do");
    let first = validate(&source, "src/nodes.lkjscript", &Limits::default()).expect("first");
    let again = validate(&source, "src/nodes.lkjscript", &Limits::default()).expect("again");
    assert_eq!(first.revision(), again.revision());
    for (index, node) in first.nodes().iter().enumerate() {
        assert_eq!(node.id().index() as usize, index);
        assert_eq!(node.id().revision(), first.revision());
    }
    let root = first.nodes().first().expect("root");
    assert_eq!(root.kind(), NodeKind::Call);
    assert_eq!(root.label(), Some("main"));
    assert!(root.parent().is_none());
    assert!(!root.children().is_empty());

    let changed = validate(
        &unit_main("do/\nunit\nfalse\n/do"),
        "src/nodes.lkjscript",
        &Limits::default(),
    )
    .expect("changed");
    let stale = changed.node(root.id()).expect_err("cross-revision lookup");
    assert_eq!(stale.actual_revision(), first.revision());
    assert_eq!(stale.expected_revision(), changed.revision());
}

#[test]
fn public_validate_requires_canonical_relative_lkjscript_paths() {
    let source = unit_main("unit");
    for rejected in [
        "legacy.lkjml",
        "../escape.lkjscript",
        "/absolute.lkjscript",
        "./aliased.lkjscript",
        "src//aliased.lkjscript",
        ".hidden.lkjscript",
    ] {
        let error = validate(&source, rejected, &Limits::default())
            .expect_err("noncanonical logical path must be rejected");
        assert_eq!(error.code(), "LKJ-SRC-LOAD", "{rejected}");
    }
    let accepted = validate(&source, "src/canonical.lkjscript", &Limits::default())
        .expect("canonical relative logical path");
    assert_eq!(
        accepted.root_origin().logical_path(),
        "src/canonical.lkjscript"
    );
    assert!(accepted.format_source("src/canonical.lkjscript").is_some());
    assert!(accepted.format_source("../canonical.lkjscript").is_none());
    assert!(accepted.format_source("/src/canonical.lkjscript").is_none());
}

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
fn all_113_tracked_source_corpus_files_roundtrip_exactly() -> std::io::Result<()> {
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
    assert_eq!(files.len(), 113, "tracked source corpus changed");
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

#[test]
fn all_existing_source_limits_remain_enforced_at_boundaries() {
    let accepted = unit_main("unit");
    let limits = Limits {
        max_tokens_per_file: 7,
        max_toplevel_forms: 1,
        max_nest_depth: 2,
        max_children: 2,
        ..Limits::default()
    };
    assert!(validate(&accepted, "limit.lkjscript", &limits).is_ok());
    for limits in [
        Limits {
            max_tokens_per_file: 6,
            ..limits
        },
        Limits {
            max_nest_depth: 1,
            ..limits
        },
        Limits {
            max_children: 1,
            ..limits
        },
    ] {
        let error =
            validate(&accepted, "limit.lkjscript", &limits).expect_err("source limit boundary");
        assert_eq!(error.category().as_str(), "resource-limit");
    }
    let imports = "import/\na.lkjscript\n/import\nimport/\nb.lkjscript\n/import\n";
    let error = validate(imports, "limit.lkjscript", &limits).expect_err("top-level form boundary");
    assert_eq!(error.category().as_str(), "resource-limit");
}

#[test]
fn source_file_safety_maximum_checks_metadata_and_actual_read_without_large_input() {
    let origin = SourceOrigin::in_memory("src/limit.lkjscript");
    let budget = SourceFoundationBudget::default();
    assert!(budget
        .check_metadata(&origin, FOUNDATION_MAX_SOURCE_FILE_BYTES)
        .is_ok());
    let metadata_error = budget
        .check_metadata(&origin, FOUNDATION_MAX_SOURCE_FILE_BYTES + 1)
        .expect_err("metadata file-byte maximum");
    assert_eq!(metadata_error.code(), "LKJ-SRC-LIMIT");
    assert!(metadata_error
        .message()
        .contains("category=source-file-bytes"));

    let mut read_budget = SourceFoundationBudget::default();
    let read_error = read_budget
        .record_read(&origin, FOUNDATION_MAX_SOURCE_FILE_BYTES + 1)
        .expect_err("actual read file-byte maximum");
    assert!(read_error.message().contains("category=source-file-bytes"));
}

#[test]
fn bounded_reader_stops_at_remaining_allowance_plus_one_without_large_input() {
    let origin = SourceOrigin::in_memory("src/bounded.lkjscript");
    let budget = SourceFoundationBudget::with_usage(0, FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES - 2);
    budget
        .check_metadata(&origin, 2)
        .expect("two aggregate bytes remain");
    let mut reader = Cursor::new(vec![b'x'; 4]);
    let error = super::load::read_bounded_bytes(
        &mut reader,
        2,
        Path::new("src/bounded.lkjscript"),
        &origin,
        &budget,
    )
    .expect_err("sentinel byte must detect growth");
    assert_eq!(reader.position(), 3);
    assert!(error.message().contains("metadata=2; read=3"));

    let budget = SourceFoundationBudget::default();
    let mut shortened = Cursor::new(vec![b'x']);
    let error = super::load::read_bounded_bytes(
        &mut shortened,
        2,
        Path::new("src/shortened.lkjscript"),
        &origin,
        &budget,
    )
    .expect_err("metadata/read shrink must be deterministic");
    assert_eq!(shortened.position(), 1);
    assert!(error.message().contains("metadata=2; read=1"));
}

#[test]
fn aggregate_source_safety_maximum_uses_checked_arithmetic_without_large_input() {
    let origin = SourceOrigin::in_memory("src/limit.lkjscript");
    let budget = SourceFoundationBudget::with_usage(1, FOUNDATION_MAX_AGGREGATE_SOURCE_BYTES - 1);
    assert!(budget.check_metadata(&origin, 1).is_ok());
    let error = budget
        .check_metadata(&origin, 2)
        .expect_err("aggregate byte maximum");
    assert!(error.message().contains("category=aggregate-source-bytes"));

    let overflow = SourceFoundationBudget::with_usage(1, u64::MAX)
        .check_metadata(&origin, 1)
        .expect_err("aggregate checked addition");
    assert!(overflow
        .message()
        .contains("attempted=18446744073709551615"));
}

#[test]
fn source_unit_safety_maximum_uses_checked_arithmetic_without_many_files() {
    let origin = SourceOrigin::in_memory("src/limit.lkjscript");
    let boundary = SourceFoundationBudget::with_usage(FOUNDATION_MAX_SOURCE_UNITS - 1, 0);
    assert!(boundary.check_metadata(&origin, 0).is_ok());
    let error = SourceFoundationBudget::with_usage(FOUNDATION_MAX_SOURCE_UNITS, 0)
        .check_metadata(&origin, 0)
        .expect_err("source-unit maximum");
    assert!(error.message().contains("category=source-units"));
    let overflow = SourceFoundationBudget::with_usage(u64::MAX, 0)
        .check_metadata(&origin, 0)
        .expect_err("source-unit checked addition");
    assert!(overflow
        .message()
        .contains("attempted=18446744073709551615"));
}

#[test]
fn loader_uses_explicit_dependency_first_dfs_for_deep_import_chain() -> std::io::Result<()> {
    const DEPTH: usize = 1_500;
    const FILES_PER_GROUP: usize = 15;
    const GROUPS_PER_BUCKET: usize = 10;

    let directory = TempDir::new("deep-imports")?;
    let mut paths = Vec::with_capacity(DEPTH);
    let mut logical_paths = Vec::with_capacity(DEPTH);
    for index in 0..DEPTH {
        let group = index / FILES_PER_GROUP;
        let bucket = group / GROUPS_PER_BUCKET;
        let logical = format!("units/b{bucket:02}/g{group:03}/u{index:04}.lkjscript");
        let path = directory.0.join(&logical);
        fs::create_dir_all(path.parent().expect("unit parent"))?;
        paths.push(path);
        logical_paths.push(logical);
    }
    for (index, path) in paths.iter().enumerate() {
        let source = if let Some(next) = logical_paths.get(index + 1) {
            format!("import/\n{next}\n/import\n")
        } else {
            named_def("leaf")
        };
        fs::write(path, source)?;
    }
    let root = directory.0.join("main.lkjscript");
    fs::write(
        &root,
        format!(
            "import/\n{}\n/import\n{}",
            logical_paths[0],
            unit_main("unit")
        ),
    )?;

    let tree = load(&root, &Limits::default())
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    assert_eq!(tree.files().len(), DEPTH + 1);
    assert_eq!(tree.files()[0].path, paths[DEPTH - 1].canonicalize()?);
    assert_eq!(tree.files()[DEPTH].path, root.canonicalize()?);
    Ok(())
}

#[test]
fn source_tree_validation_uses_an_explicit_stack_for_deep_directories() -> std::io::Result<()> {
    const DEPTH: usize = 1_500;
    let directory = TempDir::new("deep-source-tree")?;
    let mut deepest = directory.0.clone();
    for _ in 0..DEPTH {
        deepest.push("d");
        fs::create_dir(&deepest)?;
    }
    fs::write(deepest.join("leaf.lkjscript"), unit_main("unit"))?;
    validate_source_directory_tree(&directory.0, Limits::default().max_dir_children)
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    Ok(())
}

#[test]
fn loader_cycle_diagnostic_retains_deterministic_related_import_spans() -> std::io::Result<()> {
    let directory = TempDir::new("cycle")?;
    let root = directory.0.join("main.lkjscript");
    let first = directory.0.join("first.lkjscript");
    let second = directory.0.join("second.lkjscript");
    fs::write(
        &root,
        format!("import/\n./first.lkjscript\n/import\n{}", unit_main("unit")),
    )?;
    fs::write(&first, "import/\n./second.lkjscript\n/import\n")?;
    fs::write(&second, "import/\n./main.lkjscript\n/import\n")?;
    let error = load(&root, &Limits::default()).expect_err("import cycle");
    assert_eq!(error.code(), "LKJ-SRC-LOAD");
    assert_eq!(error.primary_span().start().line(), 1);
    assert_eq!(error.related_spans().len(), 2);
    let compact = error.render_compact_agent();
    assert!(compact.contains("related[0].label=earlier import in cycle"));
    assert!(compact.contains("related[1].label=earlier import in cycle"));
    assert_eq!(compact, error.render_compact_agent());
    Ok(())
}

#[test]
fn loader_retains_logical_origin_separate_from_canonical_host_path() -> std::io::Result<()> {
    let directory = TempDir::new("origin")?;
    let dependency = directory.0.join("dep.lkjscript");
    let root = directory.0.join("main.lkjscript");
    fs::write(&dependency, named_def("helper"))?;
    fs::write(
        &root,
        format!("import/\n./dep.lkjscript\n/import\n{}", unit_main("unit")),
    )?;
    let tree = load(&root, &Limits::default())
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    assert_eq!(tree.source_origins().len(), 2);
    for origin in tree.source_origins() {
        assert!(!origin.logical_path().starts_with('/'));
        let host = origin.host_containment_path().expect("loaded host origin");
        assert!(host.is_absolute());
        assert_eq!(host.canonicalize()?, host);
    }
    assert_eq!(tree.root_origin().logical_path(), "main.lkjscript");
    assert!(tree
        .declarations()
        .iter()
        .any(|declaration| declaration.kind() == DeclarationKind::Function));
    Ok(())
}

#[test]
fn loader_rejects_wide_entry_directories_and_accepts_imported_declarations() -> std::io::Result<()>
{
    let wide = TempDir::new("wide-entry")?;
    let entry = wide.0.join("main.lkjscript");
    fs::write(&entry, unit_main("unit"))?;
    for index in 0..16 {
        fs::write(wide.0.join(format!("asset-{index}")), "")?;
    }
    let error = load(&entry, &Limits::default())
        .expect_err("wide source directory")
        .to_string();
    assert!(error.contains("at least 17 entries (max 16)"));

    let declarations = TempDir::new("imported-declarations")?;
    let dependency = declarations.0.join("traits.lkjscript");
    fs::write(
        &dependency,
        "trait/\nname/\nMarked\n/name\n/trait\nproduct/\nname/\nItem\n/name\nfields/\n/fields\n/product\nimpl/\ntrait/\nMarked\n/trait\nfor/\nProduct\nItem\n/for\n/impl\n",
    )?;
    let root = declarations.0.join("main.lkjscript");
    fs::write(
        &root,
        format!(
            "import/\n./traits.lkjscript\n/import\n{}",
            unit_main("unit")
        ),
    )?;
    let tree = load(&root, &Limits::default())
        .map_err(|error| std::io::Error::other(error.to_string()))?;
    assert_eq!(tree.source_origins().len(), 2);
    assert_eq!(tree.declarations().len(), 4);
    Ok(())
}

#[cfg(unix)]
#[test]
fn loader_rejects_symlink_import_escape() -> std::io::Result<()> {
    use std::os::unix::fs::symlink;

    let package = TempDir::new("package")?;
    let outside = TempDir::new("outside")?;
    fs::create_dir_all(package.0.join("src/std"))?;
    fs::write(outside.0.join("escaped.lkjscript"), named_def("escaped"))?;
    symlink(
        outside.0.join("escaped.lkjscript"),
        package.0.join("escaped.lkjscript"),
    )?;
    let entry = package.0.join("main.lkjscript");
    fs::write(
        &entry,
        format!(
            "import/\n./escaped.lkjscript\n/import\n{}",
            unit_main("unit")
        ),
    )?;
    let error = load(&entry, &Limits::default())
        .expect_err("symlink escape")
        .to_string();
    assert!(error.contains("escapes package roots"));
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn loader_rejects_fifo_as_non_regular_without_blocking() -> std::io::Result<()> {
    use std::process::Command;

    let directory = TempDir::new("fifo")?;
    let fifo = directory.0.join("main.lkjscript");
    let status = Command::new("mkfifo").arg(&fifo).status()?;
    if !status.success() {
        return Err(std::io::Error::other("mkfifo failed"));
    }
    let error = load(&fifo, &Limits::default()).expect_err("FIFO source must fail");
    assert!(error.message().contains("not a regular file"));
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn opened_outside_descriptor_is_rejected_for_inside_looking_request() -> std::io::Result<()> {
    let package = TempDir::new("descriptor-package")?;
    let outside = TempDir::new("descriptor-outside")?;
    let requested = package.0.join("inside.lkjscript");
    let actual = outside.0.join("actual.lkjscript");
    fs::write(&requested, unit_main("unit"))?;
    fs::write(&actual, unit_main("unit"))?;
    let package_root = package.0.canonicalize()?;
    let file = super::load::open_source_file(&actual)?;
    let error = super::load::opened_source_path(
        &file,
        &requested,
        &package_root,
        None,
        &SourceOrigin::in_memory("inside.lkjscript"),
    )
    .expect_err("opened outside descriptor must fail containment");
    assert!(error
        .message()
        .contains("opened source escapes package roots"));
    assert!(error.message().contains("inside.lkjscript"));
    assert!(error.message().contains("actual.lkjscript"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn non_utf8_host_logical_paths_are_rejected_without_collapse() -> std::io::Result<()> {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let package = TempDir::new("non-utf8")?;
    let package_root = package.0.canonicalize()?;
    let first = package
        .0
        .join(OsString::from_vec(b"source-\x80.lkjscript".to_vec()));
    let second = package
        .0
        .join(OsString::from_vec(b"source-\x81.lkjscript".to_vec()));
    fs::write(&first, unit_main("unit"))?;
    fs::write(&second, unit_main("unit"))?;
    let first = first.canonicalize()?;
    let second = second.canonicalize()?;
    let first_error = super::load::source_origin(&first, &package_root, None)
        .expect_err("first non-UTF-8 path must fail");
    let second_error = super::load::source_origin(&second, &package_root, None)
        .expect_err("second non-UTF-8 path must fail");
    assert!(first_error.message().contains("not valid UTF-8"));
    assert!(second_error.message().contains("not valid UTF-8"));
    assert_ne!(first_error.message(), second_error.message());
    Ok(())
}

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
