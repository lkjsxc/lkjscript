use super::*;

#[test]
fn trusted_reader_reaches_eof_without_a_source_byte_quota() {
    let origin = SourceOrigin::in_memory("src/reader.lkjscript");
    let bytes = vec![b'x'; 4];
    let mut reader = Cursor::new(bytes);
    let loaded =
        super::load::read_source_bytes(&mut reader, 4, Path::new("src/reader.lkjscript"), &origin)
            .expect("trusted reader reaches EOF");
    assert_eq!(reader.position(), 4);
    assert_eq!(loaded, b"xxxx");
}

#[test]
fn reader_uses_metadata_only_as_a_hint_and_detects_growth_and_shrinkage() {
    let origin = SourceOrigin::in_memory("src/change.lkjscript");
    let mut grown = Cursor::new(vec![b'x'; 4]);
    let growth =
        super::load::read_source_bytes(&mut grown, 2, Path::new("src/grown.lkjscript"), &origin)
            .expect_err("read must detect growth after reaching EOF");
    assert_eq!(grown.position(), 4);
    assert!(growth.message().contains("metadata=2; read=4"));

    let mut shortened = Cursor::new(vec![b'x']);
    let shrinkage = super::load::read_source_bytes(
        &mut shortened,
        2,
        Path::new("src/shortened.lkjscript"),
        &origin,
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
    )
    .expect_err("metadata must not reject before the actual read");
    assert_eq!(changed.category().as_str(), "source-loading");
    assert!(changed
        .message()
        .contains("metadata=18446744073709551615; read=1"));
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
    let tree = validate_source_set_for_analysis(&files, &paths[0])
        .expect("source authority has no source-unit validity ceiling");
    assert_eq!(tree.files().len(), SOURCE_UNITS);
}
