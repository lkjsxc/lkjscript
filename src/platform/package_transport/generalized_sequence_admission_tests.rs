// Included by the hostile source-loader test owner, never an authoring path.

#[test]
fn ordinary_sequence_transport_rejects_rehashed_graph30_relay_and_unused_signatures() {
    for parameter in ["I64", "T"] {
        let type_parameter = if parameter == "T" {
            "(type-parameter create T (constraint transferable))"
        } else {
            ""
        };
        let source = format!(
            r#"declarations.begin
(units (module create generalized-signatures
  (function create relay (visibility private) (effect pure)
    {type_parameter}
    (parameter create p (type (owned-sequence {parameter})) (use consume))
    (returns (owned-sequence {parameter})) (body (local p)))
  (function create unused (visibility private) (effect pure)
    {type_parameter}
    (parameter create p (type (owned-sequence {parameter})) (use consume))
    (returns Unit) (body (unit)))))
declarations.end"#
        );
        let snapshot =
            crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(&source)
                .unwrap();
        let temporary = tempfile::tempdir().unwrap();
        let repository = GraphRepository::create(&temporary.path().join("source"), &snapshot, None)
            .unwrap()
            .repository;
        let original = repository.export_package_container().unwrap();
        let package = &original.packages[&original.container.root.package_revision];
        let independent =
            crate::platform::package_transport::oracle::reconstruct(&original.container).unwrap();
        assert_eq!(
            independent.snapshots[&package.snapshot.root.package_id].types,
            package.snapshot.types
        );
        for mut replacement in package.snapshot.owners.values().filter(|record|
            matches!(record, OwnerRecord::Declaration(record) if matches!(record.name.as_str(), "relay" | "unused"))
            || matches!(record, OwnerRecord::Parameter(_))
        ).cloned() {
            replacement.set_encoding_for_edit(30);
            let hostile = rehash_owner(&original, replacement);
            assert_sequence_source_rejected(&hostile, "kernel_sequence_generation");
            assert!(hostile.admit().unwrap_err().message.contains("Graph 31"));
        }
    }
}
