use super::*;
use crate::platform::kernel::{
    DeclarationPayload, ExpressionOperation, Name, OwnerKey, OwnerRecord, encode_owner,
    validate_full,
};
use crate::platform::witness::{FullWitness, rebuild_full_witness};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn rename_overlay_and_local_delta_match_the_full_witness_oracle() {
    let base = crate::platform::kernel::tests::witness_snapshot();
    let base_witness = rebuild_full_witness(&base).expect("base witness");
    let callee = declaration_named(&base, "callee");
    let caller = declaration_named(&base, "caller");
    let mut replacement = base.owners[&callee].clone();
    let OwnerRecord::Declaration(record) = &mut replacement else {
        panic!("callee must be a declaration");
    };
    record.name = Name::new("renamed_callee").expect("valid name");
    let expected = encode_owner(&base.owners[&callee]).expect("base owner").0;
    let delta = CanonicalDelta::normalize(
        &base,
        vec![PrimitiveEdit::ReplaceOwner {
            expected,
            record: replacement,
        }],
    )
    .expect("normalized rename");
    let overlay = KernelOverlay::new(&base, &delta);
    assert_eq!(overlay.owner_count(), base.owners.len());
    assert_eq!(overlay.owner(caller), base.owners.get(&caller));
    let derived =
        derive_local_delta(&base, &overlay, &delta, &base_witness).expect("derived rename");
    assert_eq!(derived.namespaces.len(), 2);
    assert!(derived.ownership.is_empty());
    assert!(derived.relations.removed.is_empty());
    assert!(derived.relations.added.is_empty());
    assert_eq!(derived.summary_candidates, BTreeSet::from([callee]));
    let prepared =
        prepare_change_analysis(&base, &base_witness, delta.clone()).expect("rename preparation");
    assert!(prepared.summaries.plan.semantically_checked.is_empty());
    assert!(prepared.summaries.plan.compiler_units.is_empty());
    assert_eq!(prepared.summaries.final_delta.edits.len(), 1);
    assert_matches_full_oracle(&base_witness, &overlay, &derived);
}

#[test]
fn move_derives_one_ownership_and_relation_rebind() {
    let base = crate::platform::kernel::tests::witness_snapshot();
    let base_witness = rebuild_full_witness(&base).expect("base witness");
    let callee = declaration_named(&base, "callee");
    let destination = module_named(&base, "second");
    let mut replacement = base.owners[&callee].clone();
    let OwnerRecord::Declaration(record) = &mut replacement else {
        panic!("callee must be a declaration");
    };
    let OwnerKey::Module(destination) = destination else {
        panic!("destination must be a module");
    };
    record.module = destination;
    let delta = replace_owner_delta(&base, callee, replacement);
    let overlay = KernelOverlay::new(&base, &delta);
    let derived = derive_local_delta(&base, &overlay, &delta, &base_witness).expect("derived move");
    assert_eq!(derived.namespaces.len(), 2);
    assert_eq!(derived.ownership.len(), 1);
    assert_eq!(derived.relations.removed.len(), 1);
    assert_eq!(derived.relations.added.len(), 1);
    assert_eq!(derived.summary_candidates, BTreeSet::from([callee]));
    let prepared =
        prepare_change_analysis(&base, &base_witness, delta.clone()).expect("move preparation");
    assert!(prepared.summaries.plan.semantically_checked.is_empty());
    assert!(prepared.summaries.plan.compiler_units.is_empty());
    assert_matches_full_oracle(&base_witness, &overlay, &derived);
}

#[test]
fn body_edit_derives_local_relation_removal_and_enclosing_summary_candidates() {
    let base = crate::platform::kernel::tests::witness_snapshot();
    let base_witness = rebuild_full_witness(&base).expect("base witness");
    let callee = declaration_named(&base, "callee");
    let body = function_body(&base, callee);
    let mut replacement = base.owners[&body].clone();
    let OwnerRecord::Expression(record) = &mut replacement else {
        panic!("body must be an expression");
    };
    record.operation = ExpressionOperation::Unit;
    let delta = replace_owner_delta(&base, body, replacement);
    let overlay = KernelOverlay::new(&base, &delta);
    let derived =
        derive_local_delta(&base, &overlay, &delta, &base_witness).expect("derived body edit");
    assert!(derived.namespaces.is_empty());
    assert!(derived.ownership.is_empty());
    assert_eq!(derived.relations.removed.len(), 1);
    assert!(derived.relations.added.is_empty());
    assert_eq!(derived.summary_candidates, BTreeSet::from([callee, body]));
    let prepared =
        prepare_change_analysis(&base, &base_witness, delta.clone()).expect("body preparation");
    assert_eq!(
        prepared.validation.structurally_checked,
        BTreeSet::from([body])
    );
    assert!(prepared.validation.semantically_checked.contains(&callee));
    assert!(prepared.validation.summaries_reused >= 41);
    assert!(prepared.witness.work.pages_read <= 12);
    assert_matches_full_oracle(&base_witness, &overlay, &derived);
}

#[test]
fn interface_change_uses_reverse_relations_for_validation_and_compiler_impact() {
    let base = crate::platform::kernel::tests::witness_snapshot();
    let base_witness = rebuild_full_witness(&base).expect("base witness");
    let callee = declaration_named(&base, "callee");
    let caller = declaration_named(&base, "caller");
    let mut replacement = base.owners[&callee].clone();
    let OwnerRecord::Declaration(record) = &mut replacement else {
        panic!("callee must be a declaration");
    };
    record.visibility = crate::platform::kernel::DeclarationVisibility::Public;
    let delta = replace_owner_delta(&base, callee, replacement);
    let overlay = KernelOverlay::new(&base, &delta);
    let derived =
        derive_local_delta(&base, &overlay, &delta, &base_witness).expect("derived interface edit");
    let planned = plan_impact_and_summaries(&overlay, &delta, &derived, &base_witness)
        .expect("interface impact");
    assert!(planned.plan.semantically_checked.contains(&callee));
    assert!(planned.plan.semantically_checked.contains(&caller));
    assert!(planned.plan.compiler_units.contains(&callee));
    assert!(planned.plan.compiler_units.contains(&caller));
    assert!(planned.plan.tests.is_empty());
    assert!(planned.plan.reasons.iter().any(|reason| {
        reason.kind == ImpactReasonKind::ValidationDependency
            && reason.target == callee
            && reason.relation == Some(crate::platform::kernel::RelationKind::FunctionCall)
    }));
    assert_eq!(overlay.owner(caller), base.owners.get(&caller));
    assert_matches_full_oracle(&base_witness, &overlay, &derived);
}

#[test]
fn private_implementation_change_walks_behavior_edges_to_dependent_tests() {
    let base = crate::platform::kernel::tests::witness_snapshot();
    let base_witness = rebuild_full_witness(&base).expect("base witness");
    let binding = binding_named(&base, "local");
    let function = declaration_named(&base, "with_binding");
    let test = declaration_named(&base, "caller_test");
    let mut replacement = base.owners[&binding].clone();
    let OwnerRecord::Binding(record) = &mut replacement else {
        panic!("binding owner expected");
    };
    record.declared_type = None;
    let delta = replace_owner_delta(&base, binding, replacement);
    let overlay = KernelOverlay::new(&base, &delta);
    let derived = derive_local_delta(&base, &overlay, &delta, &base_witness)
        .expect("derived implementation edit");
    let planned = plan_impact_and_summaries(&overlay, &delta, &derived, &base_witness)
        .expect("implementation impact");
    assert!(planned.plan.semantically_checked.contains(&function));
    assert!(planned.plan.compiler_units.contains(&function));
    assert_eq!(planned.plan.tests, BTreeSet::from([test]));
    assert!(planned.plan.reasons.iter().any(|reason| {
        reason.kind == ImpactReasonKind::TestBehavior
            && reason.source == test
            && reason.target == function
    }));
    assert_matches_full_oracle(&base_witness, &overlay, &derived);
}

#[test]
fn test_relation_rebind_updates_only_the_affected_test_dependency_entries() {
    let base = crate::platform::kernel::tests::witness_snapshot();
    let base_witness = rebuild_full_witness(&base).expect("base witness");
    let test = declaration_named(&base, "caller_test");
    let test_actual = test_actual(&base, "caller_test");
    let old_function = declaration_named(&base, "with_binding");
    let old_record = &base.owners[&old_function];
    let OwnerRecord::Declaration(old_declaration) = old_record else {
        panic!("function declaration expected");
    };
    let DeclarationPayload::Function(old_function_payload) = &old_declaration.payload else {
        panic!("function payload expected");
    };
    let new_function =
        crate::platform::semantic_id::DeclarationId::migrate(b"change-test-dependency", 1);
    let new_body =
        crate::platform::semantic_id::ExpressionId::migrate(b"change-test-dependency", 1);
    let body_record = OwnerRecord::Expression(
        crate::platform::kernel::ExpressionRecord::new(new_body, ExpressionOperation::Unit)
            .expect("new body"),
    );
    let function_record = OwnerRecord::Declaration(crate::platform::kernel::DeclarationRecord {
        header: crate::platform::kernel::OwnerHeader::new(
            OwnerKey::Declaration(new_function),
            crate::platform::kernel::OwnerKind::PureFunction,
        ),
        module: old_declaration.module,
        name: Name::new("other_function").expect("valid name"),
        visibility: crate::platform::kernel::DeclarationVisibility::Private,
        payload: DeclarationPayload::Function(crate::platform::kernel::FunctionDeclaration {
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            result: old_function_payload.result,
            effect: crate::platform::kernel::FunctionEffect::Pure,
            body: new_body,
        }),
    });
    let mut actual_record = base.owners[&OwnerKey::Expression(test_actual)].clone();
    let OwnerRecord::Expression(actual) = &mut actual_record else {
        panic!("test actual expression expected");
    };
    actual.operation = ExpressionOperation::Call {
        function: crate::platform::kernel::DeclarationReference {
            package: base.root.package_id,
            declaration: new_function,
        },
        type_arguments: Vec::new(),
        arguments: Vec::new(),
    };
    let delta = CanonicalDelta::normalize(
        &base,
        vec![
            PrimitiveEdit::InsertOwner {
                record: body_record,
            },
            PrimitiveEdit::InsertOwner {
                record: function_record,
            },
            PrimitiveEdit::ReplaceOwner {
                expected: encode_owner(&base.owners[&OwnerKey::Expression(test_actual)])
                    .expect("test actual")
                    .0,
                record: actual_record,
            },
        ],
    )
    .expect("test rebind delta");
    let overlay = KernelOverlay::new(&base, &delta);
    let derived = derive_local_delta(&base, &overlay, &delta, &base_witness)
        .expect("derived test relation rebind");
    let test_delta = derive_test_dependency_delta(&overlay, &delta, &derived, &base_witness)
        .expect("test dependency delta");
    assert_eq!(test_delta.affected_tests, BTreeSet::from([test]));
    assert_eq!(test_delta.removed.len(), 1);
    assert_eq!(test_delta.added.len(), 1);
    assert!(test_delta.removed.iter().any(|dependency| {
        dependency.target
            == crate::platform::kernel::RelationEndpoint::Owner(
                crate::platform::kernel::ExactOwnerKey {
                    package: base.root.package_id,
                    owner: old_function,
                },
            )
    }));
    assert!(test_delta.added.iter().any(|dependency| {
        dependency.target
            == crate::platform::kernel::RelationEndpoint::Owner(
                crate::platform::kernel::ExactOwnerKey {
                    package: base.root.package_id,
                    owner: OwnerKey::Declaration(new_function),
                },
            )
    }));
    assert_matches_full_oracle(&base_witness, &overlay, &derived);
}

#[test]
fn mixed_owner_edits_share_one_sorted_overlay_and_one_derived_delta() {
    let base = crate::platform::kernel::tests::witness_snapshot();
    let base_witness = rebuild_full_witness(&base).expect("base witness");
    let binding = binding_named(&base, "local");
    let documentation = base
        .owners
        .iter()
        .find_map(|(owner, record)| {
            matches!(record, OwnerRecord::Documentation(_)).then_some(*owner)
        })
        .expect("documentation owner");
    let new_module = crate::platform::semantic_id::ModuleId::migrate(b"change-overlay", 99);
    let module = OwnerRecord::Module(crate::platform::kernel::ModuleRecord {
        header: crate::platform::kernel::OwnerHeader::new(
            OwnerKey::Module(new_module),
            crate::platform::kernel::OwnerKind::Module,
        ),
        name: Name::new("third").expect("valid module name"),
    });
    let mut renamed_binding = base.owners[&binding].clone();
    let OwnerRecord::Binding(record) = &mut renamed_binding else {
        panic!("binding record");
    };
    record.name = Name::new("local_value").expect("valid binding name");
    let delta = CanonicalDelta::normalize(
        &base,
        vec![
            PrimitiveEdit::InsertOwner { record: module },
            PrimitiveEdit::ReplaceOwner {
                expected: encode_owner(&base.owners[&binding]).expect("binding").0,
                record: renamed_binding,
            },
            PrimitiveEdit::DeleteOwner {
                owner: documentation,
                expected: encode_owner(&base.owners[&documentation])
                    .expect("documentation")
                    .0,
            },
        ],
    )
    .expect("mixed canonical delta");
    assert_eq!(delta.changed_owner_count(), 3);
    let overlay = KernelOverlay::new(&base, &delta);
    assert_eq!(overlay.owner_count(), base.owners.len());
    let mut observed = Vec::new();
    overlay.for_each_owner(|owner, _| observed.push(owner));
    assert!(observed.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(observed.len(), overlay.owner_count());
    let derived =
        derive_local_delta(&base, &overlay, &delta, &base_witness).expect("mixed derived delta");
    assert_matches_full_oracle(&base_witness, &overlay, &derived);
}

#[test]
fn dependency_edit_uses_the_same_package_relation_contract_as_full_rebuild() {
    let base = crate::platform::kernel::tests::witness_snapshot();
    let base_witness = rebuild_full_witness(&base).expect("base witness");
    let package = crate::platform::kernel::PackageId::migrate(b"change-overlay", 1);
    let dependency = crate::platform::kernel::DependencyRecord {
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        package,
        semantic_revision: crate::platform::semantic_id::RevisionId::from_digest([21; 32]),
        package_object: crate::platform::kernel::PackageObjectDigest::from_bytes([22; 32]),
    };
    let delta = CanonicalDelta::normalize(
        &base,
        vec![PrimitiveEdit::InsertDependency { record: dependency }],
    )
    .expect("dependency delta");
    let overlay = KernelOverlay::new(&base, &delta);
    assert_eq!(overlay.dependency_count(), 1);
    assert_eq!(
        overlay.dependency(package).map(|record| record.package),
        Some(package)
    );
    let derived =
        derive_local_delta(&base, &overlay, &delta, &base_witness).expect("dependency relation");
    assert!(derived.relations.removed.is_empty());
    assert_eq!(derived.relations.added.len(), 1);
    assert_matches_full_oracle(&base_witness, &overlay, &derived);
}

#[test]
fn candidate_ownership_collision_rejects_before_full_validation() {
    let base = crate::platform::kernel::tests::witness_snapshot();
    let base_witness = rebuild_full_witness(&base).expect("base witness");
    let caller = declaration_named(&base, "caller");
    let caller_root = function_body(&base, caller);
    let test_actual = test_actual(&base, "caller_test");
    let mut replacement = base.owners[&caller_root].clone();
    let OwnerRecord::Expression(record) = &mut replacement else {
        panic!("caller body must be an expression");
    };
    let ExpressionOperation::Sequence { items } = &mut record.operation else {
        panic!("caller body must be a sequence");
    };
    items.push(test_actual);
    let delta = replace_owner_delta(&base, caller_root, replacement);
    let overlay = KernelOverlay::new(&base, &delta);
    assert_eq!(
        derive_local_delta(&base, &overlay, &delta, &base_witness)
            .expect_err("one expression cannot acquire two semantic parents")
            .code,
        "change_derived_collision"
    );
}

#[test]
fn exact_preconditions_duplicates_and_live_retired_overlap_reject() {
    let base = crate::platform::kernel::tests::witness_snapshot();
    let callee = declaration_named(&base, "callee");
    let record = base.owners[&callee].clone();
    assert_eq!(
        CanonicalDelta::normalize(
            &base,
            vec![PrimitiveEdit::ReplaceOwner {
                expected: crate::platform::kernel::OwnerObjectDigest::from_bytes([0_u8; 32]),
                record: record.clone(),
            }],
        )
        .expect_err("stale digest must reject")
        .code,
        "change_exact_precondition"
    );
    let expected = encode_owner(&record).expect("owner").0;
    assert_eq!(
        CanonicalDelta::normalize(
            &base,
            vec![
                PrimitiveEdit::ReplaceOwner {
                    expected,
                    record: record.clone(),
                },
                PrimitiveEdit::ReplaceOwner {
                    expected,
                    record: record.clone(),
                },
            ],
        )
        .expect_err("duplicate no-change edits must reject")
        .code,
        "change_duplicate_primitive"
    );

    let retirement = retirement_for(&base, callee);
    assert_eq!(
        CanonicalDelta::normalize(
            &base,
            vec![PrimitiveEdit::InsertRetirement { record: retirement }],
        )
        .expect_err("live-retired overlap must reject")
        .code,
        "change_live_retired_overlap"
    );
}

#[test]
fn generic_preparation_rejects_an_invalid_result_type_on_the_owner_frontier() {
    let base = crate::platform::kernel::tests::witness_snapshot();
    let base_witness = rebuild_full_witness(&base).expect("base witness");
    let callee = declaration_named(&base, "callee");
    let function_type = base
        .owners
        .values()
        .find_map(|record| match record {
            OwnerRecord::Port(record) => Some(record.function_type),
            _ => None,
        })
        .expect("fixture function type");
    let mut replacement = base.owners[&callee].clone();
    let OwnerRecord::Declaration(record) = &mut replacement else {
        panic!("callee declaration expected");
    };
    let DeclarationPayload::Function(function) = &mut record.payload else {
        panic!("function payload expected");
    };
    function.result = function_type;
    let delta = replace_owner_delta(&base, callee, replacement);
    let diagnostics = prepare_change_analysis(&base, &base_witness, delta)
        .expect_err("invalid function result must reject incrementally");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "kernel_type_root")
    );
}

#[test]
fn generic_preparation_rejects_deleting_a_still_referenced_expression() {
    let base = crate::platform::kernel::tests::witness_snapshot();
    let base_witness = rebuild_full_witness(&base).expect("base witness");
    let callee = declaration_named(&base, "callee");
    let body = function_body(&base, callee);
    let delta = CanonicalDelta::normalize(
        &base,
        vec![PrimitiveEdit::DeleteOwner {
            owner: body,
            expected: encode_owner(&base.owners[&body]).expect("base body").0,
        }],
    )
    .expect("expression deletion delta");
    let diagnostics = prepare_change_analysis(&base, &base_witness, delta)
        .expect_err("a live declaration cannot retain a deleted expression root");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.class == crate::platform::diagnostic::DiagnosticClass::Semantic
                && diagnostic.code == "change_validate_ownership_stale"
        }),
        "unexpected diagnostics: {diagnostics:#?}"
    );
}

fn assert_matches_full_oracle(
    base: &FullWitness,
    overlay: &KernelOverlay<'_>,
    derived: &DerivedDelta,
) {
    let candidate = overlay.materialize_logical_oracle();
    validate_full(&candidate).expect("candidate must pass the independent full validator");
    let full = rebuild_full_witness(&candidate).expect("candidate full witness");

    let mut namespaces = base.entries.namespaces.clone();
    apply_value_edits(&mut namespaces, &derived.namespaces);
    assert_eq!(namespaces, full.entries.namespaces);

    let mut ownership = base.entries.ownership.clone();
    apply_value_edits(&mut ownership, &derived.ownership);
    assert_eq!(ownership, full.entries.ownership);

    let mut relations = base
        .entries
        .relations
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    for edge in &derived.relations.removed {
        assert!(relations.remove(edge));
    }
    relations.extend(derived.relations.added.iter().copied());
    assert_eq!(
        relations,
        full.entries
            .relations
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
    );

    let prepared = prepare_change_analysis(overlay.base(), base, overlay.delta().clone())
        .expect("generic candidate preparation");
    assert_eq!(&prepared.derived, derived);
    let validation = &prepared.validation;
    assert_eq!(validation.profile, INCREMENTAL_VALIDATION_PROFILE);
    assert_eq!(
        validation.canonical_owners_changed,
        overlay.delta().changed_owner_count() as u64
    );
    let summary_delta = &prepared.summaries.final_delta;
    let mut summaries = base.summaries.clone();
    let mut summary_bindings = base.entries.summaries.clone();
    for edit in &summary_delta.edits {
        match (&edit.after, edit.after_digest) {
            (Some(summary), Some(digest)) => {
                summaries.insert(edit.owner, summary.clone());
                summary_bindings.insert(edit.owner, digest);
            }
            (None, None) => {
                summaries.remove(&edit.owner);
                summary_bindings.remove(&edit.owner);
            }
            _ => panic!("summary value and digest must share one domain"),
        }
    }
    for owner in summaries
        .keys()
        .chain(full.summaries.keys())
        .copied()
        .collect::<BTreeSet<_>>()
    {
        assert_eq!(
            summaries.get(&owner),
            full.summaries.get(&owner),
            "summary mismatch for {owner:?}"
        );
        assert_eq!(
            summary_bindings.get(&owner),
            full.entries.summaries.get(&owner),
            "summary binding mismatch for {owner:?}"
        );
    }

    let test_delta = &prepared.tests;
    let mut test_dependencies = base.entries.test_dependencies.clone();
    for dependency in &test_delta.removed {
        assert!(test_dependencies.remove(dependency));
    }
    test_dependencies.extend(test_delta.added.iter().copied());
    assert_eq!(test_dependencies, full.entries.test_dependencies);

    assert_eq!(prepared.witness.roots, full.manifest.roots);
}

fn apply_value_edits<K: Clone + Ord, V: Clone>(
    values: &mut BTreeMap<K, V>,
    edits: &[DerivedValueEdit<K, V>],
) {
    for edit in edits {
        match &edit.after {
            Some(value) => {
                values.insert(edit.key.clone(), value.clone());
            }
            None => {
                values.remove(&edit.key);
            }
        }
    }
}

fn replace_owner_delta(
    base: &crate::platform::kernel::KernelSnapshot,
    owner: OwnerKey,
    record: OwnerRecord,
) -> CanonicalDelta {
    CanonicalDelta::normalize(
        base,
        vec![PrimitiveEdit::ReplaceOwner {
            expected: encode_owner(&base.owners[&owner]).expect("base owner").0,
            record,
        }],
    )
    .expect("replace delta")
}

fn declaration_named(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Declaration(record) if record.name.as_str() == name => Some(*owner),
            _ => None,
        })
        .expect("named declaration")
}

fn module_named(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Module(record) if record.name.as_str() == name => Some(*owner),
            _ => None,
        })
        .expect("named module")
}

fn binding_named(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Binding(record) if record.name.as_str() == name => Some(*owner),
            _ => None,
        })
        .expect("named binding")
}

fn function_body(
    snapshot: &crate::platform::kernel::KernelSnapshot,
    declaration: OwnerKey,
) -> OwnerKey {
    match &snapshot.owners[&declaration] {
        OwnerRecord::Declaration(record) => match &record.payload {
            DeclarationPayload::Function(function) => OwnerKey::Expression(function.body),
            _ => panic!("declaration must be a function"),
        },
        _ => panic!("owner must be a declaration"),
    }
}

fn test_actual(
    snapshot: &crate::platform::kernel::KernelSnapshot,
    test_name: &str,
) -> crate::platform::semantic_id::ExpressionId {
    let test = declaration_named(snapshot, test_name);
    match &snapshot.owners[&test] {
        OwnerRecord::Declaration(record) => match record.payload {
            DeclarationPayload::Test { actual, .. } => actual,
            _ => panic!("declaration must be a test"),
        },
        _ => panic!("owner must be a declaration"),
    }
}

fn retirement_for(
    snapshot: &crate::platform::kernel::KernelSnapshot,
    owner: OwnerKey,
) -> crate::platform::kernel::RetirementRecord {
    crate::platform::kernel::RetirementRecord {
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        owner,
        last_kind: snapshot.owners[&owner].kind(),
        last_name: None,
        last_parent: None,
        last_live_revision: crate::platform::semantic_id::RevisionId::from_digest([17; 32]),
        deletion_change: crate::platform::kernel::ChangeDigest::of(b"delete"),
    }
}
