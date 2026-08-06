use super::*;

#[test]
fn limited_reader_uses_a_sentinel_while_unrestricted_reader_reaches_eof() {
    let origin = SourceOrigin::in_memory("src/reader.lkjscript");
    let bytes = vec![b'x'; 4];

    let mut limited = Cursor::new(bytes.clone());
    let error = super::load::read_source_bytes(
        &mut limited,
        4,
        Path::new("src/reader.lkjscript"),
        &origin,
        SourceBytePolicy::limited(2),
        0,
    )
    .expect_err("allowance plus one must prove boundary exhaustion");
    assert_eq!(limited.position(), 3);
    assert_eq!(error.category().as_str(), "resource-limit");
    assert!(error
        .message()
        .contains("category=aggregate-source-bytes; attempted=3; limit=2"));

    let mut unrestricted = Cursor::new(bytes);
    let loaded = super::load::read_source_bytes(
        &mut unrestricted,
        4,
        Path::new("src/reader.lkjscript"),
        &origin,
        SourceBytePolicy::Unrestricted,
        0,
    )
    .expect("unrestricted reader reaches EOF");
    assert_eq!(unrestricted.position(), 4);
    assert_eq!(loaded, b"xxxx");
}

#[test]
fn reader_uses_metadata_only_as_a_hint_and_detects_growth_and_shrinkage() {
    let origin = SourceOrigin::in_memory("src/change.lkjscript");
    let mut grown = Cursor::new(vec![b'x'; 4]);
    let growth = super::load::read_source_bytes(
        &mut grown,
        2,
        Path::new("src/grown.lkjscript"),
        &origin,
        SourceBytePolicy::Unrestricted,
        0,
    )
    .expect_err("unrestricted read must detect growth after reaching EOF");
    assert_eq!(grown.position(), 4);
    assert!(growth.message().contains("metadata=2; read=4"));

    let mut shortened = Cursor::new(vec![b'x']);
    let shrinkage = super::load::read_source_bytes(
        &mut shortened,
        2,
        Path::new("src/shortened.lkjscript"),
        &origin,
        SourceBytePolicy::Unrestricted,
        0,
    )
    .expect_err("metadata/read shrink must be deterministic");
    assert_eq!(shortened.position(), 1);
    assert!(shrinkage.message().contains("metadata=2; read=1"));

    let mut stale_large_metadata = Cursor::new(vec![b'x']);
    let changed = super::load::read_source_bytes(
        &mut stale_large_metadata,
        u64::MAX,
        Path::new("src/metadata-hint.lkjscript"),
        &origin,
        SourceBytePolicy::limited(2),
        0,
    )
    .expect_err("metadata must not reject before the actual read");
    assert_eq!(changed.category().as_str(), "source-loading");
    assert!(changed
        .message()
        .contains("metadata=18446744073709551615; read=1"));
}

#[test]
fn aggregate_byte_policy_uses_checked_accounting_beyond_the_former_ceiling() {
    let origin = SourceOrigin::in_memory("src/accounting.lkjscript");
    let crossed = SourceBytePolicy::Unrestricted
        .account_source_bytes(&origin, 256 * 1024 * 1024, 1)
        .expect("trusted accounting crosses the former 256 MiB ceiling");
    assert_eq!(crossed, 256 * 1024 * 1024 + 1);

    let limited = SourceBytePolicy::limited(2)
        .account_source_bytes(&origin, 0, 3)
        .expect_err("explicit low boundary policy");
    assert_eq!(limited.category().as_str(), "resource-limit");

    let overflow = SourceBytePolicy::Unrestricted
        .account_source_bytes(&origin, u64::MAX, 1)
        .expect_err("aggregate representation overflow");
    assert_eq!(overflow.code(), "LKJ-SRC-HOST");
    assert_eq!(overflow.category().as_str(), "source-loading");
}

#[test]
fn same_source_fails_under_low_loader_policy_and_loads_unrestricted() {
    let directory = TempDir::new("byte-policy").expect("temporary source directory");
    let root = directory.0.join("main.lkjscript");
    let source = unit_main("unit");
    fs::write(&root, &source).expect("write boundary source");
    let source_bytes = u64::try_from(source.len()).expect("fixture length");

    let failure = super::load::load_with_byte_policy(
        &root,
        &Limits::default(),
        SourceBytePolicy::limited(source_bytes - 1),
    )
    .expect_err("low untrusted byte policy");
    assert_eq!(failure.category().as_str(), "resource-limit");
    assert!(failure
        .message()
        .contains("category=aggregate-source-bytes"));

    let (tree, _) = super::load::load_with_byte_policy(
        &root,
        &Limits::default(),
        SourceBytePolicy::Unrestricted,
    )
    .expect("same source loads without boundary policy");
    assert_eq!(tree.files().len(), 1);
}

#[test]
fn source_authority_accepts_more_than_65_536_in_memory_units() {
    const SOURCE_UNITS: usize = 65_537;
    let paths = (0..SOURCE_UNITS)
        .map(|index| format!("unit-{index:05}.lkjscript"))
        .collect::<Vec<_>>();
    let files = paths
        .iter()
        .map(|path| (path.as_str(), ""))
        .collect::<Vec<_>>();
    let tree = validate_source_set_for_analysis(&files, &paths[0], &Limits::default())
        .expect("source authority has no source-unit validity ceiling");
    assert_eq!(tree.files().len(), SOURCE_UNITS);
}
