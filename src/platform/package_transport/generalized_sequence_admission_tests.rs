// Included by the hostile source-loader test owner, never an authoring path.

#[test]
fn ordinary_sequence_transport_rejects_rehashed_invalid_relay_and_unused_signatures() {
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
        let (invalid, additions) = sequence_callable_element_type();
        for mut replacement in package.snapshot.owners.values().filter(|record|
            matches!(record, OwnerRecord::Declaration(record) if matches!(record.name.as_str(), "relay" | "unused"))
            || matches!(record, OwnerRecord::Parameter(_))
        ).cloned() {
            // The element's complete data contract must be checked even when only an
            // uncalled result signature or an unused parameter reaches the type.
            match &mut replacement {
                OwnerRecord::Declaration(record) => {
                    let DeclarationPayload::Function(function) = &mut record.payload else {
                        unreachable!()
                    };
                    function.result.ty = invalid;
                }
                OwnerRecord::Parameter(record) => record.ty = invalid,
                _ => unreachable!(),
            }
            let mut hostile = rehash_owner(&original, replacement);
            hostile.objects.extend(additions.clone());
            assert_sequence_source_rejected(&hostile, "kernel_owned_sequence");
        }
        // Ordinary elements do not make the enclosing sequence unrestricted. Both
        // monomorphic and exact transferable generic signatures retain custody.
        for mut replacement in package
            .snapshot
            .owners
            .values()
            .filter(|record| matches!(record, OwnerRecord::Parameter(_)))
            .cloned()
        {
            let OwnerRecord::Parameter(parameter) = &mut replacement else {
                unreachable!()
            };
            parameter.use_mode = ParameterUse::Unrestricted;
            assert_sequence_source_rejected(
                &rehash_owner(&original, replacement),
                "kernel_buffer_ownership",
            );
        }
    }
}
