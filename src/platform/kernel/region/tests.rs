//! Public authoring controls for region kinds, independent of slab execution.
use super::*;

const SOURCE: &str = include_str!("../../../../tests/fixtures/region-kinds.lkjc");
const SHARED: &str = include_str!("../../../../tests/fixtures/region-shared.lkjc");

fn author(source: &str) -> Result<KernelSnapshot, String> {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(source)
}

#[test]
fn region_generic_pointer_identity_preserves_brand_through_calls() {
    let snapshot = author(SOURCE).unwrap();
    assert_eq!(
        snapshot
            .types
            .values()
            .filter(|t| matches!(t.form, TypeForm::RelPtr { .. }))
            .count(),
        2
    );
    assert!(validate_full(&snapshot).is_ok());
    for owner in snapshot.owners.values() {
        if let OwnerRecord::Declaration(d) = owner
            && let (OwnerKey::Declaration(id), DeclarationPayload::Function(f)) =
                (d.header.owner, &d.payload)
        {
            let scope = DeclarationReference {
                package: snapshot.root.package_id,
                declaration: id,
            };
            assert!(!transfer::admit_in(&snapshot, f.result.ty, Some(scope)).unwrap());
        }
    }
}

#[test]
fn region_custody_modes_keep_independent_transfer_and_read_permissions() {
    for (mode, transferable, shareable) in [
        ("unique", true, false),
        ("frozen", true, true),
        ("pinned", false, false),
    ] {
        let source = format!(
            r#"declarations.begin
(units (module create custody
  (function create identity (visibility public) (effect pure)
    (type-parameter create R (constraint region))
    (parameter create cap (type (region-cap R {mode})) (use consume))
    (returns (region-cap R {mode})) (body (local cap)))))
declarations.end"#
        );
        let snapshot = author(&source).unwrap();
        let (scope, ty) = snapshot
            .owners
            .values()
            .find_map(|owner| match owner {
                OwnerRecord::Declaration(d) => match (&d.header.owner, &d.payload) {
                    (OwnerKey::Declaration(id), DeclarationPayload::Function(f)) => Some((
                        DeclarationReference {
                            package: snapshot.root.package_id,
                            declaration: *id,
                        },
                        f.result.ty,
                    )),
                    _ => None,
                },
                _ => None,
            })
            .unwrap();
        assert!(memory::direct(&snapshot, ty).unwrap());
        assert_eq!(
            transfer::admit_in(&snapshot, ty, Some(scope)).is_ok(),
            transferable,
            "{mode}"
        );
        assert_eq!(
            share::admit_in(&snapshot, ty, Some(scope)).is_ok(),
            shareable,
            "{mode}"
        );
        assert!(
            author(&source.replace(" (use consume)", "")).is_err(),
            "unrestricted {mode} custody admitted"
        );
    }
}

#[test]
fn region_kinds_reject_ordinary_identity_and_uninhabited_value_positions() {
    for ty in [
        "(region I64)",
        "(rel-ptr I64 I64)",
        "(region-slice I64 I64)",
        "(region-cap I64 unique)",
        "R",
        "(list R)",
        "(rel-ptr R R)",
    ] {
        let source = format!(
            r#"declarations.begin
(units (module create bad-region
  (function create unused (visibility public) (effect pure)
    (type-parameter create R (constraint region))
    (parameter create value (type {ty}))
    (returns Unit) (body (unit)))))
declarations.end"#
        );
        assert!(
            author(&source).is_err(),
            "invalid value type admitted: {ty}"
        );
    }
    assert!(author(&SOURCE.replace("(constraint region)", "(constraint transferable)")).is_err());
    assert!(author(&SOURCE.replace("(types R T)", "(types T R)")).is_err());
}

#[test]
fn region_constraint_is_a_distinct_kind_not_a_value_property_combination() {
    let region = TypeParameterConstraints::from_names(&["region"]).unwrap();
    assert!(region.is_region());
    assert!(!region.has_owned());
    assert!(!region.requires_transfer());
    assert!(!region.requires_share());
    assert!(!region.proves_capture_safe());
    for other in [
        "owned",
        "transferable",
        "capture-safe",
        "shareable",
        "region",
    ] {
        assert!(TypeParameterConstraints::from_names(&["region", other]).is_none());
    }
    let encoded = bincode::encode_to_vec(region, bincode::config::standard()).unwrap();
    let (decoded, consumed): (TypeParameterConstraints, _) =
        bincode::decode_from_slice(&encoded, bincode::config::standard()).unwrap();
    assert_eq!(decoded, region);
    assert_eq!(consumed, encoded.len());
}

#[test]
fn region_constructor_edges_are_bounded_before_type_expansion() {
    for declaration in [
        "type.region as=@cyclic region=@cyclic",
        "type.region-cap as=@cyclic region=@cyclic mode=unique",
        "type.rel-ptr as=@cyclic region=@cyclic item=i64",
        "type.rel-ptr as=@cyclic region=i64 item=@cyclic",
        "type.region-slice as=@cyclic region=i64 item=@cyclic",
    ] {
        let error = author(declaration).unwrap_err();
        assert!(error.contains("change_type_cycle"), "{error}");
    }
}

#[test]
fn region_custody_composes_with_owned_metadata_but_cannot_be_duplicated() {
    let source = r#"declarations.begin
(units (module create region-owner
  (function create package (visibility public) (effect pure)
    (type-parameter create R (constraint region))
    (parameter create cap (type (region-cap R unique)) (use consume))
    (returns (owned-product (field left (region-cap R unique)) (field right Unit)))
    (body (pack-owned
      (type (owned-product (field left (region-cap R unique)) (field right Unit)))
      (field left (local cap)) (field right (unit)))))))
declarations.end"#;
    author(source).unwrap();
    let duplicate = source
        .replace("(field right Unit)", "(field right (region-cap R unique))")
        .replace("(field right (unit))", "(field right (local cap))");
    assert!(author(&duplicate).is_err());
}

#[test]
fn region_shared_capabilities_are_scoped_read_views_not_owning_values() {
    let snapshot = author(SHARED).unwrap();
    validate_full(&snapshot).unwrap();
    let package = snapshot.root.package_id;
    let relations = extract_relations(
        package,
        &snapshot.owners,
        &snapshot.types,
        &snapshot.dependencies,
    )
    .unwrap();
    let mut scoped_parameters = 0;
    for (key, owner) in &snapshot.owners {
        if let OwnerRecord::Parameter(ParameterRecord {
            parent: ParameterParent::Function(declaration),
            ..
        }) = owner
        {
            scoped_parameters += 1;
            // Even inspect's unused input depends on its declaring scope.
            assert!(relations.contains(&RelationEdge {
                source: RelationEndpoint::Owner(ExactOwnerKey {
                    package,
                    owner: *key,
                }),
                kind: RelationKind::ReadScopeUse,
                target: RelationEndpoint::Owner(ExactOwnerKey {
                    package,
                    owner: OwnerKey::Declaration(*declaration),
                }),
            }));
        }
    }
    assert_eq!(scoped_parameters, 3);
    for replacement in ["(use consume)", "(use unrestricted)"] {
        assert!(author(&SHARED.replace("(use borrow)", replacement)).is_err());
    }
    assert!(author(&SHARED.replace(
        "(view-from cap (read-scope (declaration forward) read_scope_b9170000000000000000000000000002))",
        "(value)"
    )).is_err());
    assert!(
        author(&SHARED.replace(
            "(view-reborrow (local cap)",
            "(view-reborrow (local absent)"
        ))
        .is_err()
    );
    assert!(author(&SHARED.replace(
        "(shared (read-scope (declaration inspect) read_scope_b9170000000000000000000000000001))",
        "(shared (read-scope (declaration forward) read_scope_b9170000000000000000000000000002))"
    )).is_err());
}
