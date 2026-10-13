//! Executable discovery of the complete source-tied read-view contract.
//! Runtime invocation seals and lease identities are deliberately absent.

use super::compact_record;

pub(super) fn records(records: &mut Vec<String>) -> Result<(), String> {
    records.push(compact_record(
        "change.proposal-feature",
        &[
            ("name", "associated-read-views".to_owned()),
            (
                "schema",
                "complete-source-tied-read-view-contract".to_owned(),
            ),
            (
                "semantic-admission",
                "unsupported-until-complete-read-view-integration".to_owned(),
            ),
            (
                "runtime-availability",
                "unsupported-until-complete-read-view-integration".to_owned(),
            ),
            (
                "publication",
                "requires-complete-candidate-validation".to_owned(),
            ),
        ],
    )?);
    records.push(compact_record(
        "change.read-view-contract",
        &[
            ("category", "affine-read-view".to_owned()),
            ("member-kinds", "Owned:0|ReadViewFamily:1".to_owned()),
            (
                "scope-identity",
                "exact-owner-qualified-binder-and-parameter".to_owned(),
            ),
            (
                "binding",
                "projection-free-nominal-read-view-template-with-exact-scope-formal".to_owned(),
            ),
            (
                "inventory",
                "complete-ordered-distinct-including-unused-and-phantom".to_owned(),
            ),
            (
                "authority",
                "exact-contract-member-witness-and-defining-module".to_owned(),
            ),
            (
                "substitution",
                "simultaneous-type-witness-and-read-scope-actuals".to_owned(),
            ),
        ],
    )?);
    for (kind, source, scope, result) in [
        (
            "value",
            "none",
            "none",
            "ordinary-or-independently-admitted-owned",
        ),
        (
            "read-from",
            "one-exact-borrowed-parameter",
            "none",
            "owned-read-token",
        ),
        (
            "view-from",
            "one-exact-borrowed-parameter",
            "explicit-read-scope-actual",
            "affine-read-view-handle",
        ),
    ] {
        records.push(compact_record(
            "change.callable-result",
            &[
                ("kind", kind.to_owned()),
                ("type", "one-authoritative-result-type".to_owned()),
                ("source", source.to_owned()),
                ("scope", scope.to_owned()),
                ("result", result.to_owned()),
            ],
        )?);
    }
    for (kind, identity, authority) in [
        (
            "parameter",
            "exact-binder-and-read-scope-parameter",
            "declared-source-relationships",
        ),
        (
            "lexical",
            "exact-callable-and-loan-binding",
            "lexical-loan-containment",
        ),
    ] {
        records.push(compact_record(
            "change.read-scope-actual",
            &[
                ("kind", kind.to_owned()),
                ("identity", identity.to_owned()),
                ("authority", authority.to_owned()),
                (
                    "runtime-identities",
                    "excluded-from-canonical-meaning".to_owned(),
                ),
            ],
        )?);
    }
    for (kind, ty, scope, access) in [
        (
            "ordinary",
            "immutable-transferable-first-order-data",
            "none",
            "metadata-only",
        ),
        (
            "read-reference",
            "independently-admitted-owned-or-read-view",
            "exact-instantiated-declaration-scope",
            "exact-selected-source-readable-path",
        ),
    ] {
        records.push(compact_record(
            "change.read-view-field",
            &[
                ("kind", kind.to_owned()),
                ("type", ty.to_owned()),
                ("scope", scope.to_owned()),
                ("access", access.to_owned()),
                ("authority", "exact-defining-module".to_owned()),
            ],
        )?);
    }
    for (name, meaning) in [
        (
            "source",
            "every-field-read-and-result-path-derives-from-one-exact-selected-argument;equal-types-scopes-or-allocations-do-not-merge-arguments",
        ),
        (
            "custody",
            "every-descriptor-retains-selected-source-custody-even-when-empty;custody-only-links-grant-no-readable-path",
        ),
        (
            "handles",
            "affine-local-move-discard-and-explicit-reborrow;parameters-borrow-only;no-implicit-copy-or-source-consumption",
        ),
        (
            "sharing",
            "independent-read-shareability-for-complete-accessible-fields-and-phantom-actuals;no-transfer-or-capture-grant",
        ),
        (
            "results",
            "read-from-and-view-from-require-pure-named-functions-or-methods;tasks-and-externals-return-neither-loan-form",
        ),
        (
            "escape",
            "ordinary-and-owned-containers-unconstrained-generics-first-class-callables-captures-serialization-and-component-boundaries-reject-read-views",
        ),
        (
            "admission",
            "complete-member-binding-field-scope-source-and-result-inventories-including-unused-methods-bindings-and-untaken-syntax",
        ),
        (
            "scope",
            "explicit-forwarding-or-proved-containment-only;no-widening-union-or-runtime-invocation-in-prepared-identity",
        ),
        (
            "cleanup",
            "one-finalizer-per-immutable-backing;reserve-before-lease-publication;iterative-allocation-free-final-release",
        ),
    ] {
        records.push(compact_record(
            "change.read-view-rule",
            &[("name", name.to_owned()), ("meaning", meaning.to_owned())],
        )?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{RegistrySection, section_records};
    use crate::platform::control::{CompactRecord, parse_records};

    fn field<'a>(record: &'a CompactRecord, name: &str) -> Option<&'a str> {
        record
            .fields
            .iter()
            .find(|field| field.name == name)
            .map(|field| field.value.as_str())
    }

    #[test]
    fn read_view_discovery_keeps_category_result_and_scope_authority_distinct() {
        let records = section_records(RegistrySection::Change).expect("complete change discovery");
        let records = parse_records("read-view discovery", records.concat().as_bytes())
            .expect("valid discovery records");
        let results = records
            .iter()
            .filter(|record| record.operation == "change.callable-result")
            .collect::<Vec<_>>();
        assert_eq!(results.len(), 3);
        for (record, kind, source, scope) in [
            (results[0], "value", "none", "none"),
            (
                results[1],
                "read-from",
                "one-exact-borrowed-parameter",
                "none",
            ),
            (
                results[2],
                "view-from",
                "one-exact-borrowed-parameter",
                "explicit-read-scope-actual",
            ),
        ] {
            assert_eq!(field(record, "kind"), Some(kind));
            assert_eq!(field(record, "source"), Some(source));
            assert_eq!(field(record, "scope"), Some(scope));
        }
        let feature = records
            .iter()
            .find(|record| {
                record.operation == "change.proposal-feature"
                    && field(record, "name") == Some("associated-read-views")
            })
            .expect("read-view proposal visibility");
        assert_eq!(
            field(feature, "semantic-admission"),
            Some("unsupported-until-complete-read-view-integration")
        );
        for name in [
            "source",
            "custody",
            "handles",
            "sharing",
            "results",
            "escape",
            "admission",
            "scope",
            "cleanup",
        ] {
            assert_eq!(
                records
                    .iter()
                    .filter(|record| {
                        record.operation == "change.read-view-rule"
                            && field(record, "name") == Some(name)
                    })
                    .count(),
                1,
                "complete unique rule {name}"
            );
        }
    }

    #[test]
    fn read_view_capability_has_one_canonical_version_owner() {
        use super::super::{ContractKey, contract_descriptors};
        use crate::platform::kernel::contract::{
            ASSOCIATED_READ_VIEW_CONTRACT_IDENTITY, ASSOCIATED_READ_VIEW_CONTRACT_VERSION,
        };
        let descriptors = contract_descriptors()
            .iter()
            .filter(|descriptor| descriptor.key == ContractKey::AssociatedReadViews)
            .collect::<Vec<_>>();
        assert_eq!(descriptors.len(), 1);
        assert_eq!(
            descriptors[0].identity,
            ASSOCIATED_READ_VIEW_CONTRACT_IDENTITY
        );
        assert_eq!(
            descriptors[0].version,
            ASSOCIATED_READ_VIEW_CONTRACT_VERSION
        );
        assert_eq!(descriptors[0].identity, "lkjscript-associated-read-views-1");
        assert_eq!(descriptors[0].version, 1);
    }

    #[test]
    fn read_view_definition_and_owner_discovery_preserve_the_distinct_contract() {
        use super::super::FUNCTION_DEFINITION_RESPONSE_FIELDS;
        use crate::platform::kernel::OwnerKind;
        assert_eq!(
            OwnerKind::parse("read_view").expect("nominal view owner"),
            OwnerKind::ReadView
        );
        let owners = section_records(RegistrySection::Owners).expect("owner discovery");
        let owners =
            parse_records("owner discovery", owners.concat().as_bytes()).expect("owner records");
        assert_eq!(
            owners
                .iter()
                .filter(|record| field(record, "name") == Some("read_view"))
                .count(),
            1
        );
        for required in [
            ("definition.function", "result-kind"),
            ("definition.function", "result-source"),
            ("definition.function", "result-scope"),
            ("definition.function", "read-scope-parameters"),
            ("definition.function", "read-scope-sources"),
            ("definition.read-scope-parameter", "id"),
            ("definition.read-scope-parameter", "parent"),
            ("definition.read-scope-parameter", "index"),
            ("definition.read-scope-parameter", "name"),
            ("definition.read-scope-source", "parent"),
            ("definition.read-scope-source", "index"),
            ("definition.read-scope-source", "source"),
            ("definition.read-scope-source", "scope"),
            ("definition.read-scope-argument", "parent"),
            ("definition.read-scope-argument", "slot"),
            ("definition.read-scope-argument", "index"),
            ("definition.read-scope-argument", "scope"),
        ] {
            assert_eq!(
                FUNCTION_DEFINITION_RESPONSE_FIELDS
                    .iter()
                    .filter(|field| **field == required)
                    .count(),
                1,
                "complete unique field {required:?}"
            );
        }
        for forbidden in ["result-mode", "borrow-from", "invocation-id", "lease-id"] {
            assert!(
                !FUNCTION_DEFINITION_RESPONSE_FIELDS
                    .iter()
                    .any(|(_, field)| *field == forbidden),
                "competing/runtime authority {forbidden}"
            );
        }
    }

    #[test]
    fn read_view_discovery_covers_actual_fragment_operations_and_expression_fields() {
        let type_records = section_records(RegistrySection::Type).expect("type/scope discovery");
        let type_records = parse_records("type/scope discovery", type_records.concat().as_bytes())
            .expect("type/scope records");
        for (form, fields) in [
            (
                "parameter-actual",
                &["as", "binder-kind", "binder", "parameter"][..],
            ),
            ("lexical-actual", &["as", "callable", "binding"][..]),
        ] {
            assert_eq!(
                type_records
                    .iter()
                    .filter(|record| record.operation == "read-scope.form"
                        && field(record, "name") == Some(form))
                    .count(),
                1
            );
            for name in fields {
                assert_eq!(
                    type_records
                        .iter()
                        .filter(|record| record.operation == "read-scope.field"
                            && field(record, "form") == Some(form)
                            && field(record, "name") == Some(*name)
                            && field(record, "required") == Some("true"))
                        .count(),
                    1,
                    "required {form}.{name}"
                );
            }
        }
        let change_records = section_records(RegistrySection::Change).expect("change discovery");
        let change_records = parse_records("change discovery", change_records.concat().as_bytes())
            .expect("change records");
        for name in ["create.read-view", "set.read-view"] {
            assert_eq!(
                change_records
                    .iter()
                    .filter(|record| record.operation == "change.operation"
                        && field(record, "name") == Some(name))
                    .count(),
                1,
                "complete operation {name}"
            );
        }
        for name in [
            "read-scope.parameter",
            "read-scope.source",
            "scope.argument",
            "read-view.field",
            "expression.read-view-field",
            "expression.read-view-binding",
        ] {
            assert_eq!(
                change_records
                    .iter()
                    .filter(|record| record.operation == "change.edge"
                        && field(record, "name") == Some(name))
                    .count(),
                1,
                "complete inventory edge {name}"
            );
        }
        let expected = [
            "read-view-create",
            "read-view-borrow",
            "read-view-reborrow",
            "read-view-move",
            "read-view-drop",
            "read-view-data-field",
            "read-view-borrow-field",
        ];
        let expressions = change_records
            .iter()
            .filter(|record| {
                record.operation == "change.expression-syntax"
                    && field(record, "name").is_some_and(|name| expected.contains(&name))
            })
            .map(|record| field(record, "name").expect("form name"))
            .collect::<Vec<_>>();
        assert_eq!(expressions, expected);
        for name in [
            "declaration",
            "read-scope",
            "source",
            "read-scope-arguments",
        ] {
            assert_eq!(
                super::super::FUNCTION_DEFINITION_RESPONSE_FIELDS
                    .iter()
                    .filter(|field| **field == ("definition.expression", name))
                    .count(),
                1,
                "complete expression readback {name}"
            );
        }
    }

    #[test]
    fn read_view_discovery_contains_no_runtime_identity_or_implicit_permission() {
        let mut records = Vec::new();
        super::records(&mut records).expect("read-view schema");
        let text = records.concat();
        for forbidden in [
            "invocation-id",
            "lease-id",
            "cell-address",
            "timestamp",
            "implicit-scope",
            "view-transferable",
        ] {
            assert!(
                !text.contains(forbidden),
                "runtime/implicit authority {forbidden}"
            );
        }
        for required in [
            "Owned:0|ReadViewFamily:1",
            "including-unused-and-phantom",
            "custody-only-links-grant-no-readable-path",
            "no-transfer-or-capture-grant",
            "parameters-borrow-only",
            "one-authoritative-result-type",
        ] {
            assert!(text.contains(required), "missing distinction {required}");
        }
    }
}
