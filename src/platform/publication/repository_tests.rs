use super::*;
use crate::platform::change::{
    AuthoredAnnotationValue, AuthoredBindingDefinition, AuthoredCase, AuthoredCaseReference,
    AuthoredChange, AuthoredChangeSet, AuthoredDeclarationReference, AuthoredDeletePolicy,
    AuthoredExpression, AuthoredExpressionOperation, AuthoredField, AuthoredFieldReference,
    AuthoredFieldSelector, AuthoredFunctionEffect, AuthoredLetBinding, AuthoredLocalReference,
    AuthoredMapExpressionEntry, AuthoredMatchExpressionArm, AuthoredOperation,
    AuthoredOperationReference, AuthoredOwnerParent, AuthoredParameter, AuthoredPort,
    AuthoredPortImplementation, AuthoredPortReference, AuthoredPrecondition,
    AuthoredRecordExpressionField, AuthoredRequirement, AuthoredRequirementReference,
    AuthoredResourceLimit, AuthoredStructuralTypeField, AuthoredType, AuthoredTypeParameter,
    AuthoredTypeParameterReference, BudgetedCanonicalBase, CanonicalBaseRead, CanonicalDelta,
    CanonicalReadAdmission, CanonicalReadWork, ChangeBudget, DeclarationSelector, KernelOverlay,
    ModuleSelector, OwnerSelector, ParameterParentSelector, PrimitiveEdit, derive_local_delta,
    derive_summary_delta, derive_test_dependency_delta, lower_authored_changes,
    plan_impact_and_summaries, prepare_change_analysis, validate_incremental_frontier,
    validate_structural_frontier,
};
use crate::platform::diagnostic::DiagnosticClass;
use crate::platform::kernel::{
    AnnotationClass, DeclarationPayload, DeclarationRecord, DeclarationVisibility,
    DependencyRecord, DocumentContent, DocumentationClass, DocumentationRecord, ExactOwnerKey,
    ExpressionOperation, ExternalVisibility, FunctionEffect, Idempotency, LocalValueReference,
    ModuleRecord, Name, NamespaceClass, OwnerHeader, OwnerKey, OwnerKind, OwnerRecord, PackageId,
    PackageRevisionDigest, RelationEdge, RelationEndpoint, RelationKind, RequirementReference,
    ResourceUnit, SemanticRoot, TypeForm, TypeObject, TypeObjectDigest, encode_owner,
    encode_type_object, extract_relations,
};
use crate::platform::package::RunnerKind;
use crate::platform::persistent_map::{MapRoot, PageDigest};
use crate::platform::semantic_id::{DeclarationId, DocumentationId, RepositoryId, RevisionId};
use crate::platform::storage::directory::SealCheckpoint;
use crate::platform::storage::object::{ObjectDomain, ObjectKey, StageOutcome};
use crate::platform::storage::pack::{PackBuilder, PackMetadata};
use crate::platform::witness::{NamespaceKey, OwnershipParent};
use std::collections::BTreeSet;

#[test]
fn repository_create_reopen_and_exact_current_reads_bind_every_object() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(
        &destination,
        &logical,
        Some("repository fixture".to_owned()),
    )
    .expect("Graph 5 repository creation");

    assert_eq!(created.repository.root(), destination.as_path());
    assert_eq!(created.current.head, created.initial.publication.head);
    assert_eq!(
        created.current.semantic_root,
        created.initial.publication.authority.semantic.root
    );
    assert_eq!(
        created.current.witness,
        created.initial.publication.authority.witness.manifest
    );
    assert_eq!(
        created.current.receipt.status,
        PublicationStatus::ProjectCreated
    );
    assert!(created.current.store_work.objects_read >= 6);
    assert!(
        created
            .repository
            .head_staging_leftovers()
            .unwrap()
            .is_empty()
    );

    let reopened = GraphRepository::open(&destination).expect("repository reopen");
    let current = reopened.current().expect("exact current read");
    assert_eq!(current.head, created.current.head);
    assert_eq!(current.revision, created.current.revision);
    assert_eq!(current.receipt, created.current.receipt);
    assert_eq!(current.transaction, created.current.transaction);
    assert_eq!(current.semantic_diff, created.current.semantic_diff);
    assert_eq!(current.accepted, created.current.accepted);
}

#[test]
fn staged_package_transports_bind_authored_dependency_lifecycle_without_advancing_head() {
    let temporary = tempfile::tempdir().expect("temporary package staging repositories");
    let source_path = temporary.path().join("source");
    let target_path = temporary.path().join("target");
    let source = GraphRepository::create(&source_path, &empty_snapshot(b"dependency-source"), None)
        .expect("source repository");
    let target = GraphRepository::create(
        &target_path,
        &crate::platform::kernel::tests::witness_snapshot(),
        None,
    )
    .expect("target repository");

    let exported = source
        .repository
        .export_package_transport()
        .expect("exact source package transport");
    assert!(exported.revision.dependencies.is_empty());
    assert_eq!(
        exported.revision.revision.revision_id().unwrap(),
        source.current.head.revision
    );
    assert_eq!(exported.interface_owner_count, 0);
    assert_eq!(exported.interface_type_count, 0);
    assert!(!exported.packs.is_empty());
    let target_head = target.current.head;

    let unrelated_type = TypeObject::new(TypeForm::Option {
        item: TypeObjectDigest::from_bytes([99; 32]),
    })
    .expect("locally valid unrelated type");
    let (unrelated_digest, unrelated_bytes) =
        encode_type_object(&unrelated_type).expect("unrelated type encoding");
    let mut unrelated_pack = PackBuilder::default();
    unrelated_pack
        .insert(
            ObjectKey::from_digest(ObjectDomain::Type, unrelated_digest.bytes()),
            &unrelated_bytes,
        )
        .unwrap();
    let mut overcomplete = exported.packs.clone();
    overcomplete.push(unrelated_pack.seal().unwrap().bytes);
    assert_eq!(
        target
            .repository
            .stage_package_transport(exported.transport_digest, &overcomplete)
            .expect_err("unreachable transport object must reject")
            .code,
        "publication_package_transport_reachability"
    );
    assert_eq!(target.repository.current().unwrap().head, target_head);

    let staged = target
        .repository
        .stage_package_transport(exported.transport_digest, &exported.packs)
        .expect("stage exact package transport");
    assert_eq!(staged.outcome, StageOutcome::Inserted);
    assert_eq!(staged.package_revision, exported.revision_digest);
    assert_eq!(staged.package_transport, exported.transport_digest);
    assert_eq!(staged.current_revision, target_head.revision);
    assert_eq!(target.repository.current().unwrap().head, target_head);
    let repeated = target
        .repository
        .stage_package_transport(exported.transport_digest, &exported.packs)
        .expect("idempotent package staging");
    assert_eq!(repeated.outcome, StageOutcome::Reused);
    assert!(repeated.seal.packs.is_empty());
    assert_eq!(target.repository.current().unwrap().head, target_head);

    let missing_revision = crate::platform::kernel::PackageRevisionDigest::from_bytes([77; 32]);
    let missing = AuthoredChangeSet {
        base: target_head.revision,
        preconditions: Vec::new(),
        changes: vec![AuthoredChange::AddDependency {
            package: PackageId::migrate(b"missing-package-revision", 0),
            semantic_revision: crate::platform::semantic_id::RevisionId::from_digest([78; 32]),
            package_revision: missing_revision,
        }],
        budget: ChangeBudget::default(),
    };
    assert_eq!(
        target
            .repository
            .prepare_authored_change(&missing, PublicationOptions::default())
            .expect_err("missing logical package revision must reject")[0]
            .code,
        "package_revision_missing"
    );
    assert_eq!(target.repository.current().unwrap().head, target_head);

    let wrong_revision = AuthoredChangeSet {
        base: target_head.revision,
        preconditions: Vec::new(),
        changes: vec![AuthoredChange::AddDependency {
            package: exported.revision.package,
            semantic_revision: crate::platform::semantic_id::RevisionId::from_digest([79; 32]),
            package_revision: exported.revision_digest,
        }],
        budget: ChangeBudget::default(),
    };
    assert_eq!(
        target
            .repository
            .prepare_authored_change(&wrong_revision, PublicationOptions::default())
            .expect_err("wrong package revision must reject")[0]
            .code,
        "package_revision_dependency_binding"
    );

    let self_dependency = AuthoredChangeSet {
        base: target_head.revision,
        preconditions: Vec::new(),
        changes: vec![AuthoredChange::AddDependency {
            package: target.current.semantic_root.package_id,
            semantic_revision: target_head.revision,
            package_revision: exported.revision_digest,
        }],
        budget: ChangeBudget::default(),
    };
    assert_eq!(
        target
            .repository
            .prepare_authored_change(&self_dependency, PublicationOptions::default())
            .expect_err("self dependency must reject")[0]
            .code,
        "change_authored_dependency_self"
    );

    let absent_package = PackageId::migrate(b"absent-dependency", 0);
    for change in [
        AuthoredChange::ReplaceDependency {
            package: absent_package,
            semantic_revision: exported.revision.revision.revision_id().unwrap(),
            package_revision: exported.revision_digest,
        },
        AuthoredChange::DeleteDependency {
            package: absent_package,
        },
    ] {
        let request = AuthoredChangeSet {
            base: target_head.revision,
            preconditions: Vec::new(),
            changes: vec![change],
            budget: ChangeBudget::default(),
        };
        assert_eq!(
            target
                .repository
                .prepare_authored_change(&request, PublicationOptions::default())
                .expect_err("missing dependency lifecycle transition must reject")[0]
                .code,
            "change_authored_dependency_missing"
        );
    }

    let add = AuthoredChangeSet {
        base: target_head.revision,
        preconditions: Vec::new(),
        changes: vec![AuthoredChange::AddDependency {
            package: exported.revision.package,
            semantic_revision: exported.revision.revision.revision_id().unwrap(),
            package_revision: exported.revision_digest,
        }],
        budget: ChangeBudget::default(),
    };
    let prepared_add = target
        .repository
        .prepare_authored_change(&add, PublicationOptions::default())
        .expect("prepare dependency insertion");
    assert_eq!(
        prepared_add.publication.receipt.counts.dependencies_changed,
        1
    );
    let PublicationOutcome::Accepted { current: added, .. } = target
        .repository
        .publish(&prepared_add.publication)
        .expect("publish dependency insertion")
    else {
        panic!("dependency insertion must advance HEAD")
    };
    let expected_semantic_revision = exported.revision.revision.revision_id().unwrap();
    let wrong_guard = AuthoredChangeSet {
        base: added.head.revision,
        preconditions: vec![AuthoredPrecondition::DependencyBinding {
            package: exported.revision.package,
            semantic_revision: RevisionId::from_digest([0xaa; 32]),
            package_revision: exported.revision_digest,
        }],
        changes: vec![AuthoredChange::CreateModule {
            symbol: "$never_guarded_module".to_owned(),
            name: Name::new("never_guarded_module").unwrap(),
        }],
        budget: ChangeBudget::default(),
    };
    assert_eq!(
        target
            .repository
            .prepare_authored_change(&wrong_guard, PublicationOptions::default())
            .expect_err("wrong semantic dependency revision must reject")[0]
            .code,
        "change_precondition_dependency_binding"
    );
    assert_eq!(target.repository.current().unwrap().head, added.head);

    let guarded_change = AuthoredChangeSet {
        base: added.head.revision,
        preconditions: vec![AuthoredPrecondition::DependencyBinding {
            package: exported.revision.package,
            semantic_revision: expected_semantic_revision,
            package_revision: exported.revision_digest,
        }],
        changes: vec![AuthoredChange::CreateModule {
            symbol: "$guarded_module".to_owned(),
            name: Name::new("guarded_module").unwrap(),
        }],
        budget: ChangeBudget::default(),
    };
    let guarded_change = target
        .repository
        .prepare_authored_change(&guarded_change, PublicationOptions::default())
        .expect("exact semantic dependency binding must prepare");
    let PublicationOutcome::Accepted {
        current: guarded, ..
    } = target
        .repository
        .publish(&guarded_change.publication)
        .expect("publish dependency-guarded change")
    else {
        panic!("dependency-guarded change must advance HEAD")
    };
    let duplicate_add = AuthoredChangeSet {
        base: guarded.head.revision,
        preconditions: Vec::new(),
        changes: vec![AuthoredChange::AddDependency {
            package: exported.revision.package,
            semantic_revision: exported.revision.revision.revision_id().unwrap(),
            package_revision: exported.revision_digest,
        }],
        budget: ChangeBudget::default(),
    };
    assert_eq!(
        target
            .repository
            .prepare_authored_change(&duplicate_add, PublicationOptions::default())
            .expect_err("duplicate dependency insertion must reject")[0]
            .code,
        "change_authored_dependency_present"
    );
    assert_eq!(
        target
            .repository
            .view_current()
            .unwrap()
            .dependency(exported.revision.package)
            .unwrap()
            .value
            .unwrap()
            .package_revision,
        exported.revision_digest
    );

    let transport_selection = target
        .repository
        .root()
        .join("PACKAGE-TRANSPORTS")
        .join(crate::platform::semantic_id::encode_hex(
            &exported.revision_digest.bytes(),
        ))
        .join("CURRENT");
    std::fs::remove_file(&transport_selection).expect("remove derived transport selection");
    let without_index = target
        .repository
        .export_package_transport()
        .expect("bounded independent transport fallback without selection");
    assert_eq!(without_index.revision.dependencies.len(), 1);
    assert_eq!(
        without_index.revision.dependencies[0].package_revision,
        exported.revision_digest
    );
    std::fs::write(&transport_selection, b"corrupt derived transport selection")
        .expect("write corrupt derived transport selection");
    let with_corrupt_index = target
        .repository
        .export_package_transport()
        .expect("bounded independent transport fallback with corrupt selection");
    assert_eq!(with_corrupt_index.revision, without_index.revision);
    let missing_binding_selection =
        crate::platform::package_transport::PackageTransportSelection::new(
            crate::platform::package_transport::PackageTransportBinding {
                package_revision: exported.revision_digest,
                transport: crate::platform::kernel::PackageTransportDigest::from_bytes([0xdd; 32]),
            },
        );
    std::fs::write(
        &transport_selection,
        missing_binding_selection
            .encode()
            .expect("valid derived selection"),
    )
    .expect("write syntactically valid missing transport binding");
    target
        .repository
        .export_package_transport()
        .expect("valid selection with missing transport uses bounded independent fallback");
    std::fs::write(
        &transport_selection,
        vec![
            0_u8;
            crate::platform::package_transport::MAXIMUM_PACKAGE_TRANSPORT_SELECTION_BYTES + 1
        ],
    )
    .expect("write oversized derived selection");
    target
        .repository
        .export_package_transport()
        .expect("oversized derived selection uses bounded independent fallback");
    std::fs::remove_file(&transport_selection).expect("remove derived transport selection");
    let outside = target.repository.root().join("outside-transport-selection");
    std::fs::write(&outside, b"outside").expect("write outside selection target");
    {
        use std::os::unix::fs::symlink;
        symlink(&outside, &transport_selection).expect("symlink derived transport selection");
    }
    let index_read_error = target
        .repository
        .export_package_transport()
        .expect_err("transport selection infrastructure errors must not become cache misses");
    assert_eq!(index_read_error.class, DiagnosticClass::Infrastructure);
    assert_eq!(
        index_read_error.code,
        "publication_package_transport_selection_read"
    );
    std::fs::remove_file(&transport_selection).expect("remove transport selection symlink");
    let transport_candidate = transport_selection.parent().unwrap().join(format!(
        "candidate-{}",
        crate::platform::semantic_id::encode_hex(&exported.transport_digest.bytes())
    ));
    std::fs::remove_file(&transport_candidate).expect("remove package transport candidate marker");
    {
        use std::os::unix::fs::symlink;
        symlink(&outside, &transport_candidate)
            .expect("symlink package transport candidate marker");
    }
    let candidate_read_error = target
        .repository
        .export_package_transport()
        .expect_err("candidate infrastructure errors must not become invalid-candidate skips");
    assert_eq!(candidate_read_error.class, DiagnosticClass::Infrastructure);
    assert_eq!(
        candidate_read_error.code,
        "publication_package_transport_candidate_marker_read"
    );
    std::fs::remove_file(&transport_candidate).expect("remove transport candidate symlink");
    std::fs::File::create(&transport_candidate).expect("restore empty transport candidate marker");
    let unrelated_revision_directory = target
        .repository
        .root()
        .join("PACKAGE-TRANSPORTS")
        .join(crate::platform::semantic_id::encode_hex(&[0xab; 32]));
    std::fs::create_dir(&unrelated_revision_directory)
        .expect("create unrelated retained revision bucket");
    {
        use std::os::unix::fs::symlink;
        symlink(&outside, unrelated_revision_directory.join("CURRENT"))
            .expect("symlink unrelated retained revision selection");
    }
    target
        .repository
        .export_package_transport()
        .expect("unrelated retained revision bucket cannot disable exact dependency resolution");

    let source_change = AuthoredChangeSet {
        base: source.current.head.revision,
        preconditions: Vec::new(),
        changes: vec![AuthoredChange::CreateModule {
            symbol: "$module".to_owned(),
            name: Name::new("later").unwrap(),
        }],
        budget: ChangeBudget::default(),
    };
    let source_prepared = source
        .repository
        .prepare_authored_change(&source_change, PublicationOptions::default())
        .expect("prepare source revision advancement");
    assert!(matches!(
        source
            .repository
            .publish(&source_prepared.publication)
            .expect("advance source revision"),
        PublicationOutcome::Accepted { .. }
    ));
    let replacement = source
        .repository
        .export_package_transport()
        .expect("replacement source package transport");
    assert_ne!(replacement.revision_digest, exported.revision_digest);
    target
        .repository
        .stage_package_transport(replacement.transport_digest, &replacement.packs)
        .expect("stage replacement package transport");
    let invalid_binding_selection =
        crate::platform::package_transport::PackageTransportSelection::new(
            crate::platform::package_transport::PackageTransportBinding {
                package_revision: exported.revision_digest,
                transport: replacement.transport_digest,
            },
        );
    std::fs::write(
        &transport_selection,
        invalid_binding_selection
            .encode()
            .expect("valid selection with foreign transport binding"),
    )
    .expect("write foreign transport binding");
    target
        .repository
        .export_package_transport()
        .expect("foreign indexed transport uses bounded independently validated fallback");

    let replace = AuthoredChangeSet {
        base: guarded.head.revision,
        preconditions: Vec::new(),
        changes: vec![AuthoredChange::ReplaceDependency {
            package: replacement.revision.package,
            semantic_revision: replacement.revision.revision.revision_id().unwrap(),
            package_revision: replacement.revision_digest,
        }],
        budget: ChangeBudget::default(),
    };
    let prepared_replace = target
        .repository
        .prepare_authored_change(&replace, PublicationOptions::default())
        .expect("prepare exact dependency replacement");
    let PublicationOutcome::Accepted {
        current: replaced, ..
    } = target
        .repository
        .publish(&prepared_replace.publication)
        .expect("publish dependency replacement")
    else {
        panic!("dependency replacement must advance HEAD")
    };
    assert_eq!(
        prepared_replace
            .publication
            .receipt
            .counts
            .dependencies_changed,
        1
    );

    let delete = AuthoredChangeSet {
        base: replaced.head.revision,
        preconditions: Vec::new(),
        changes: vec![AuthoredChange::DeleteDependency {
            package: replacement.revision.package,
        }],
        budget: ChangeBudget::default(),
    };
    let prepared_delete = target
        .repository
        .prepare_authored_change(&delete, PublicationOptions::default())
        .expect("prepare exact dependency deletion");
    let PublicationOutcome::Accepted {
        current: deleted, ..
    } = target
        .repository
        .publish(&prepared_delete.publication)
        .expect("publish dependency deletion")
    else {
        panic!("dependency deletion must advance HEAD")
    };
    assert_eq!(deleted.semantic_root.dependencies.entries(), 0);
    assert_eq!(
        prepared_delete
            .publication
            .receipt
            .counts
            .dependencies_changed,
        1
    );
}

#[test]
fn staged_package_interface_validates_an_exact_cross_package_pure_call() {
    let temporary = tempfile::tempdir().expect("temporary cross-package repositories");
    let source = GraphRepository::create(
        &temporary.path().join("source"),
        &empty_snapshot(b"cross-package-source"),
        None,
    )
    .expect("source repository");
    let source_change = AuthoredChangeSet {
        base: source.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::CreateModule {
                symbol: "$source_module".to_owned(),
                name: Name::new("library").unwrap(),
            },
            AuthoredChange::CreateFunction {
                symbol: "$source_function".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$source_module".to_owned(),
                },
                name: Name::new("produce").unwrap(),
                visibility: DeclarationVisibility::Public,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: AuthoredType::Unit {},
                effect: AuthoredFunctionEffect::Pure {},
                body: AuthoredExpression {
                    symbol: Some("$source_body".to_owned()),
                    operation: AuthoredExpressionOperation::Unit {},
                },
            },
        ],
    };
    let prepared_source = source
        .repository
        .prepare_authored_change(&source_change, PublicationOptions::default())
        .expect("prepare source package");
    let OwnerKey::Declaration(source_function) = prepared_source.allocated["$source_function"]
    else {
        panic!("source function allocation domain")
    };
    source
        .repository
        .publish(&prepared_source.publication)
        .expect("publish source package");
    let exported = source
        .repository
        .export_package_transport()
        .expect("export exact source interface");
    assert_eq!(exported.interface_owner_count, 1);
    assert_eq!(exported.interface_type_count, 1);
    assert_eq!(exported.packs.len(), 1);
    let transport = PackMetadata::decode(&exported.packs[0], true).unwrap();
    assert_eq!(transport.entries.len(), 6);
    assert_eq!(
        transport
            .entries
            .iter()
            .map(|entry| entry.key.domain)
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([
            ObjectDomain::MapPage,
            ObjectDomain::PackageInterface,
            ObjectDomain::PackageRevision,
            ObjectDomain::PackageTransport,
            ObjectDomain::SemanticRoot,
            ObjectDomain::Type,
        ])
    );

    let target_path = temporary.path().join("target");
    let target =
        GraphRepository::create(&target_path, &empty_snapshot(b"cross-package-target"), None)
            .expect("target repository");
    target
        .repository
        .stage_package_transport(exported.transport_digest, &exported.packs)
        .expect("stage source interface closure");
    let package_view = target
        .repository
        .view_current()
        .expect("pinned target view for package admission");
    let dependency = DependencyRecord {
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        package: exported.revision.package,
        semantic_revision: exported.revision.revision.revision_id().unwrap(),
        package_revision: exported.revision_digest,
    };
    let package_read = CanonicalBaseRead::read_package_interface_owner(
        &package_view,
        &dependency,
        OwnerKey::Declaration(source_function),
    )
    .expect("unbounded exact package-interface read");
    assert!(package_read.value.is_some());
    type SetPackageReadLimit = fn(&mut CanonicalReadAdmission, u64);
    let package_read_dimensions: &[(SetPackageReadLimit, u64, &str)] = &[
        (
            |admission, value| admission.maximum_point_reads = value,
            package_read.work.point_reads,
            "change_budget_canonical_point_reads",
        ),
        (
            |admission, value| admission.maximum_map_pages = value,
            package_read.work.map_pages_read,
            "change_budget_canonical_map_pages",
        ),
        (
            |admission, value| admission.maximum_map_entries = value,
            package_read.work.map_entries_visited,
            "change_budget_canonical_map_entries",
        ),
        (
            |admission, value| admission.maximum_catalog_lookups = value,
            package_read.work.catalog_lookups,
            "change_budget_canonical_catalog_lookups",
        ),
        (
            |admission, value| admission.maximum_objects = value,
            package_read.work.objects_read,
            "change_budget_canonical_objects",
        ),
        (
            |admission, value| admission.maximum_bytes = value,
            package_read.work.bytes_read,
            "change_budget_canonical_bytes",
        ),
        (
            |admission, value| admission.maximum_decoded_records = value,
            package_read.work.canonical_records_decoded,
            "change_budget_canonical_decoded_records",
        ),
    ];
    for (set_limit, observed, expected) in package_read_dimensions {
        assert!(*observed > 0, "package read must exercise {expected}");
        let mut admission = CanonicalReadAdmission::default();
        set_limit(&mut admission, observed - 1);
        let admitted =
            BudgetedCanonicalBase::new(&package_view, admission, CanonicalReadWork::default())
                .expect("valid package read admission");
        let diagnostic = admitted
            .read_package_interface_owner(&dependency, OwnerKey::Declaration(source_function))
            .expect_err("each package read dimension must reject independently");
        assert_eq!(diagnostic.code, *expected);
        assert_eq!(
            target.repository.current().unwrap().head,
            target.current.head,
            "an exhausted package read must not publish"
        );
    }
    let target_change = |function| AuthoredChangeSet {
        base: target.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::AddDependency {
                package: exported.revision.package,
                semantic_revision: exported.revision.revision.revision_id().unwrap(),
                package_revision: exported.revision_digest,
            },
            AuthoredChange::CreateModule {
                symbol: "$target_module".to_owned(),
                name: Name::new("application").unwrap(),
            },
            AuthoredChange::CreateFunction {
                symbol: "$caller".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$target_module".to_owned(),
                },
                name: Name::new("call_library").unwrap(),
                visibility: DeclarationVisibility::Private,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: AuthoredType::Unit {},
                effect: AuthoredFunctionEffect::Pure {},
                body: AuthoredExpression {
                    symbol: Some("$call".to_owned()),
                    operation: AuthoredExpressionOperation::Call {
                        function: AuthoredDeclarationReference::Exact {
                            package: exported.revision.package,
                            declaration: function,
                        },
                        type_arguments: Vec::new(),
                        arguments: Vec::new(),
                    },
                },
            },
        ],
    };
    let absent = DeclarationId::migrate(b"absent-package-interface-owner", 0);
    let rejected = target
        .repository
        .prepare_authored_change(&target_change(absent), PublicationOptions::default())
        .expect_err("a bound package cannot authorize an owner absent from its exact interface");
    assert!(
        rejected
            .iter()
            .any(|diagnostic| diagnostic.code == "kernel_type_dependency_owner_missing"),
        "unexpected diagnostics: {rejected:#?}"
    );
    assert_eq!(
        target.repository.current().unwrap().head,
        target.current.head,
        "failed candidate validation must not publish the dependency"
    );
    let target_change = target_change(source_function);
    let prepared_target = target
        .repository
        .prepare_authored_change(&target_change, PublicationOptions::default())
        .expect("exact dependency interface must validate the foreign call");
    assert_eq!(
        prepared_target.publication.receipt.validation.profile,
        ValidationProfile::IncrementalOwnerFrontier
    );
    assert_eq!(
        prepared_target
            .publication
            .receipt
            .validation
            .semantically_checked,
        2
    );
    let PublicationOutcome::Accepted { current, .. } = target
        .repository
        .publish(&prepared_target.publication)
        .expect("publish exact cross-package caller")
    else {
        panic!("cross-package caller must advance HEAD")
    };
    assert_eq!(current.semantic_root.dependencies.entries(), 1);
    let OwnerKey::Expression(call) = prepared_target.allocated["$call"] else {
        panic!("call allocation domain")
    };
    assert!(matches!(
        target.repository.view_current().unwrap().owner(OwnerKey::Expression(call)).unwrap().value,
        Some(OwnerRecord::Expression(record))
            if matches!(record.operation, ExpressionOperation::Call { function, .. }
                if function.package == exported.revision.package
                    && function.declaration == source_function)
    ));

    let reopened = GraphRepository::open(&target_path).expect("reopen cross-package target");
    let accepted = reopened
        .current()
        .expect("accepted cross-package publication");
    let oracle = reopened
        .view_current()
        .expect("revision-pinned cross-package view")
        .reconstruct_full_oracle()
        .expect("reconstruct exact full-oracle dependency interfaces");
    assert_eq!(oracle.revision, current.head.revision);
    assert_eq!(oracle.value.dependencies.len(), 1);
    assert_eq!(oracle.value.dependency_interfaces.len(), 1);
    assert_eq!(oracle.value.dependency_types.len(), 1);
    assert!(matches!(
        oracle.value.dependency_interfaces[&exported.revision_digest]
            .get(&OwnerKey::Declaration(source_function)),
        Some(crate::platform::kernel::PackageInterfaceRecord::Declaration(declaration))
            if matches!(
                declaration.payload,
                crate::platform::kernel::PackageInterfaceDeclarationPayload::Function(_)
            )
    ));
    crate::platform::kernel::validate_full(&oracle.value)
        .expect("reconstructed dependency interfaces must pass the independent full validator");
    let rebuilt = crate::platform::witness::rebuild_full_witness(&oracle.value)
        .expect("reconstruct accepted witness from exact package interfaces");
    assert_eq!(
        rebuilt.manifest_digest,
        accepted.accepted.validation_witness
    );
    assert_eq!(rebuilt.manifest, accepted.witness);
    assert!(oracle.work.canonical_records_decoded >= 6);
}

#[test]
fn staged_package_interface_validates_exact_cross_package_task_requirements() {
    let temporary = tempfile::tempdir().expect("temporary cross-package task repositories");
    let source_snapshot = crate::platform::kernel::tests::witness_snapshot();
    let source_package = source_snapshot.root.package_id;
    let OwnerKey::Declaration(source_function) = owner_named(&source_snapshot, "caller") else {
        panic!("source task function identity domain")
    };
    let OwnerKey::Requirement(source_requirement) = requirement_named(&source_snapshot, "store")
    else {
        panic!("source requirement identity domain")
    };
    let OwnerKey::Operation(source_operation) = operation_named(&source_snapshot, "read") else {
        panic!("source operation identity domain")
    };
    let source = GraphRepository::create(
        &temporary.path().join("source-task"),
        &source_snapshot,
        None,
    )
    .expect("source task repository");
    let exported = source
        .repository
        .export_package_transport()
        .expect("export exact source task interface");

    let target_path = temporary.path().join("target-task");
    let target = GraphRepository::create(
        &target_path,
        &empty_snapshot(b"cross-package-task-target"),
        None,
    )
    .expect("target task repository");
    target
        .repository
        .stage_package_transport(exported.transport_digest, &exported.packs)
        .expect("stage source task interface closure");

    let exact_requirement = AuthoredRequirementReference::Exact {
        package: source_package,
        requirement: source_requirement,
    };
    let request = |requirements| AuthoredChangeSet {
        base: target.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::AddDependency {
                package: exported.revision.package,
                semantic_revision: exported.revision.revision.revision_id().unwrap(),
                package_revision: exported.revision_digest,
            },
            AuthoredChange::CreateModule {
                symbol: "$task_module".to_owned(),
                name: Name::new("application").unwrap(),
            },
            AuthoredChange::CreateFunction {
                symbol: "$foreign_task".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$task_module".to_owned(),
                },
                name: Name::new("use_library_task").unwrap(),
                visibility: DeclarationVisibility::Public,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: AuthoredType::Unit {},
                effect: AuthoredFunctionEffect::Task { requirements },
                body: AuthoredExpression {
                    symbol: Some("$foreign_body".to_owned()),
                    operation: AuthoredExpressionOperation::Sequence {
                        items: vec![
                            AuthoredExpression {
                                symbol: Some("$foreign_call".to_owned()),
                                operation: AuthoredExpressionOperation::Call {
                                    function: AuthoredDeclarationReference::Exact {
                                        package: source_package,
                                        declaration: source_function,
                                    },
                                    type_arguments: Vec::new(),
                                    arguments: Vec::new(),
                                },
                            },
                            AuthoredExpression {
                                symbol: Some("$foreign_capability".to_owned()),
                                operation: AuthoredExpressionOperation::CapabilityCall {
                                    requirement: exact_requirement.clone(),
                                    operation: AuthoredOperationReference::Exact {
                                        package: source_package,
                                        operation: source_operation,
                                    },
                                    arguments: Vec::new(),
                                },
                            },
                        ],
                    },
                },
            },
        ],
    };

    let rejected = target
        .repository
        .prepare_authored_change(&request(Vec::new()), PublicationOptions::default())
        .expect_err("a foreign task call must require its exact capability grant");
    assert!(
        rejected
            .iter()
            .any(|diagnostic| diagnostic.code == "kernel_type_task_requirement")
    );
    assert_eq!(
        target.repository.current().unwrap().head,
        target.current.head,
        "rejected foreign task preparation must publish nothing"
    );

    let prepared = target
        .repository
        .prepare_authored_change(
            &request(vec![exact_requirement.clone()]),
            PublicationOptions::default(),
        )
        .expect("exact foreign task requirement must validate");
    let OwnerKey::Declaration(target_function) = prepared.allocated["$foreign_task"] else {
        panic!("target task function allocation domain")
    };
    let OwnerKey::Expression(foreign_call) = prepared.allocated["$foreign_call"] else {
        panic!("foreign call allocation domain")
    };
    let OwnerKey::Expression(foreign_capability) = prepared.allocated["$foreign_capability"] else {
        panic!("foreign capability allocation domain")
    };
    let PublicationOutcome::Accepted { current, .. } = target
        .repository
        .publish(&prepared.publication)
        .expect("publish exact foreign task caller")
    else {
        panic!("cross-package task publication must advance HEAD")
    };

    let view = target.repository.view_current().unwrap();
    assert!(matches!(
        view.owner(OwnerKey::Declaration(target_function)).unwrap().value,
        Some(OwnerRecord::Declaration(record))
            if matches!(record.payload, DeclarationPayload::Function(ref function)
                if function.effect == FunctionEffect::Task {
                    requirements: vec![RequirementReference {
                        package: source_package,
                        requirement: source_requirement,
                    }],
                })
    ));
    assert!(matches!(
        view.owner(OwnerKey::Expression(foreign_call)).unwrap().value,
        Some(OwnerRecord::Expression(record))
            if matches!(record.operation, ExpressionOperation::Call { function, .. }
                if function.package == source_package
                    && function.declaration == source_function)
    ));
    assert!(matches!(
        view.owner(OwnerKey::Expression(foreign_capability)).unwrap().value,
        Some(OwnerRecord::Expression(record))
            if matches!(record.operation, ExpressionOperation::CapabilityCall {
                requirement,
                operation,
                ..
            } if requirement.package == source_package
                && requirement.requirement == source_requirement
                && operation.package == source_package
                && operation.operation == source_operation)
    ));
    let requirement_edge = RelationEdge {
        source: RelationEndpoint::Owner(ExactOwnerKey {
            package: target
                .repository
                .current()
                .unwrap()
                .semantic_root
                .package_id,
            owner: OwnerKey::Declaration(target_function),
        }),
        kind: RelationKind::FunctionRequirement,
        target: RelationEndpoint::Owner(ExactOwnerKey {
            package: source_package,
            owner: OwnerKey::Requirement(source_requirement),
        }),
    };
    assert!(
        view.contains_forward_relation(requirement_edge)
            .unwrap()
            .value
    );
    let reexported = target
        .repository
        .export_package_transport()
        .expect("public foreign-task signature must export with its exact dependency closure");
    assert_eq!(reexported.interface_owner_count, 1);
    assert_eq!(reexported.revision.dependencies.len(), 1);
    assert_eq!(reexported.revision.dependencies[0].package, source_package);

    let reopened = GraphRepository::open(&target_path).expect("reopen cross-package task target");
    let accepted = reopened
        .current()
        .expect("accepted cross-package task publication");
    let oracle = reopened
        .view_current()
        .expect("revision-pinned cross-package task view")
        .reconstruct_full_oracle()
        .expect("reconstruct exact cross-package task oracle");
    assert_eq!(oracle.revision, current.head.revision);
    crate::platform::kernel::validate_full(&oracle.value)
        .expect("foreign task and capability references must pass the full validator");
    let rebuilt = crate::platform::witness::rebuild_full_witness(&oracle.value)
        .expect("foreign task witness must rebuild from canonical authority");
    assert_eq!(
        rebuilt.manifest_digest,
        accepted.accepted.validation_witness
    );
    assert_eq!(rebuilt.manifest, accepted.witness);
}

#[test]
fn revision_view_reads_exact_canonical_and_witness_records_with_local_work() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let view = created
        .repository
        .view_current()
        .expect("revision-pinned view");
    let callee = owner_named(&created.initial.snapshot, "callee");

    let owner = view.owner(callee).expect("exact owner read");
    assert_eq!(owner.revision, created.current.head.revision);
    assert_eq!(
        owner.value.as_ref(),
        created.initial.snapshot.owners.get(&callee)
    );
    assert_eq!(owner.work.canonical_records_decoded, 1);
    assert_eq!(owner.work.items_returned, 1);
    assert!(owner.work.map.pages_read > 0);
    assert!(owner.work.map.pages_read < 16);
    assert_eq!(
        owner.work.store.objects_read,
        owner.work.map.pages_read + 1,
        "point read must touch only its map path and selected owner object"
    );

    let absent = OwnerKey::Declaration(DeclarationId::migrate(b"absent-read", 0));
    let missing = view.owner(absent).expect("bounded absent owner lookup");
    assert_eq!(missing.revision, owner.revision);
    assert!(missing.value.is_none());
    assert_eq!(missing.work.canonical_records_decoded, 0);
    assert_eq!(missing.work.items_returned, 0);
    assert!(missing.work.map.pages_read < 16);

    let OwnerRecord::Declaration(declaration) = &created.initial.snapshot.owners[&callee] else {
        panic!("callee must be a declaration")
    };
    let namespace_key = NamespaceKey {
        parent: Some(OwnerKey::Module(declaration.module)),
        class: NamespaceClass::Declaration,
        name: declaration.name.clone(),
    };
    let namespace = view
        .namespace(&namespace_key)
        .expect("exact namespace lookup");
    assert_eq!(namespace.value, Some(callee));
    assert_eq!(namespace.work.witness_records_decoded, 1);

    let ownership = view.ownership(callee).expect("exact ownership lookup");
    assert_eq!(
        ownership.value,
        created
            .initial
            .witness
            .entries
            .ownership
            .get(&callee)
            .copied()
    );
    assert_eq!(ownership.work.witness_records_decoded, 1);

    let summary = view
        .bound_owner_summary(callee)
        .expect("exact bound-summary lookup");
    assert_eq!(
        summary.value.as_ref().map(|bound| &bound.summary),
        created.initial.witness.summaries.get(&callee),
    );
    assert_eq!(
        summary.value.as_ref().map(|bound| bound.digest),
        created
            .initial
            .witness
            .entries
            .summaries
            .get(&callee)
            .copied()
    );
    assert_eq!(summary.work.witness_records_decoded, 1);
    assert_eq!(
        summary.work.store.objects_read,
        summary.work.map.pages_read + 1,
        "summary lookup must touch one witness path and one summary object"
    );

    let (type_digest, expected_type) = created
        .initial
        .snapshot
        .types
        .iter()
        .next()
        .expect("fixture type object");
    let type_read = view.type_object(*type_digest).expect("exact type read");
    assert_eq!(type_read.value.as_ref(), Some(expected_type));
    assert_eq!(type_read.work.map.pages_read, 0);
    assert_eq!(type_read.work.store.objects_read, 1);

    let foreign_package = PackageId::migrate(b"absent-dependency", 0);
    assert!(view.dependency(foreign_package).unwrap().value.is_none());
    assert!(view.retirement(absent).unwrap().value.is_none());

    let package = view.package();
    let source = created
        .initial
        .witness
        .entries
        .relations
        .iter()
        .map(|edge| edge.source)
        .find(|candidate| {
            created
                .initial
                .witness
                .entries
                .relations
                .iter()
                .filter(|edge| edge.source == *candidate)
                .count()
                > 1
        })
        .expect("fixture source with relation fanout");
    let expected = created
        .initial
        .witness
        .entries
        .relations
        .iter()
        .filter(|edge| edge.source == source)
        .copied()
        .collect::<Vec<_>>();
    let relations = view
        .outgoing_relations(source, None, MAXIMUM_RELATION_READ_ITEMS)
        .expect("bounded outgoing relations");
    assert_eq!(relations.value.edges, expected);
    assert!(!relations.value.truncated);
    assert_eq!(relations.work.items_returned, expected.len() as u64);
    assert_eq!(
        relations.work.witness_records_decoded,
        expected.len() as u64
    );
    assert!(relations.work.map.pages_read > 0);

    let incoming_target = expected[0].target;
    let incoming_kind = expected[0].kind;
    let expected_incoming = created
        .initial
        .witness
        .entries
        .relations
        .iter()
        .filter(|edge| edge.target == incoming_target && edge.kind == incoming_kind)
        .copied()
        .collect::<Vec<_>>();
    let incoming = view
        .incoming_relations(
            incoming_target,
            Some(incoming_kind),
            MAXIMUM_RELATION_READ_ITEMS,
        )
        .expect("bounded incoming relations");
    assert_eq!(incoming.value.edges, expected_incoming);
    assert!(!incoming.value.truncated);

    let bounded = view
        .outgoing_relations(source, None, 1)
        .expect("truncated relation read");
    assert_eq!(bounded.value.edges, expected[..1]);
    assert!(bounded.value.truncated);
    assert_eq!(bounded.work.items_returned, 1);
    assert_eq!(
        view.outgoing_relations(
            RelationEndpoint::Owner(ExactOwnerKey {
                package,
                owner: callee,
            }),
            None,
            0,
        )
        .expect_err("zero item budget must reject")
        .code,
        "publication_relation_item_budget"
    );

    let test = owner_named(&created.initial.snapshot, "caller_test");
    let expected_dependencies = created.initial.witness.entries.test_dependencies_by_test[&test]
        .iter()
        .copied()
        .collect::<Vec<_>>();
    let dependencies = view
        .test_dependencies(test, MAXIMUM_TEST_DEPENDENCY_READ_ITEMS)
        .expect("bounded test dependencies");
    assert_eq!(dependencies.value.dependencies, expected_dependencies);
    assert!(!dependencies.value.truncated);
    assert_eq!(
        dependencies.work.witness_records_decoded,
        expected_dependencies.len() as u64
    );
    assert!(dependencies.work.map.pages_read > 0);
    assert_eq!(
        view.test_dependencies(callee, 0)
            .expect_err("zero test-dependency budget must reject")
            .code,
        "publication_test_dependency_item_budget"
    );
}

#[test]
fn canonical_normalization_reads_only_touched_repository_keys() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let view = created.repository.view_current().expect("pinned view");
    let callee = owner_named(&created.initial.snapshot, "callee");
    let mut replacement = created.initial.snapshot.owners[&callee].clone();
    let OwnerRecord::Declaration(declaration) = &mut replacement else {
        panic!("callee must be a declaration")
    };
    declaration.name = Name::new("repository_normalized").unwrap();
    let expected = encode_owner(&created.initial.snapshot.owners[&callee])
        .expect("base owner encoding")
        .0;
    let edits = vec![PrimitiveEdit::ReplaceOwner {
        expected,
        record: replacement,
    }];

    let repository = CanonicalDelta::normalize_from(&view, edits.clone())
        .expect("repository-backed normalization");
    let memory = CanonicalDelta::normalize(&created.initial.snapshot, edits)
        .expect("in-memory normalization oracle");
    assert_eq!(
        repository.base_revision,
        Some(created.current.head.revision)
    );
    assert_eq!(repository.canonical.owners, memory.owners);
    assert_eq!(repository.canonical.type_additions, memory.type_additions);
    assert_eq!(repository.canonical.dependencies, memory.dependencies);
    assert_eq!(repository.canonical.retirements, memory.retirements);
    assert_eq!(repository.work.point_reads, 2);
    assert_eq!(repository.work.canonical_records_decoded, 1);
    assert!(repository.work.map_pages_read > 0);
    assert!(repository.work.map_pages_read < 32);
    assert_eq!(
        repository.work.objects_read,
        repository.work.map_pages_read + 1,
        "normalization must read owner and retirement map paths plus only the selected owner"
    );
}

#[test]
fn local_derived_delta_reads_only_affected_witness_keys() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let view = created.repository.view_current().expect("pinned view");
    let callee = owner_named(&created.initial.snapshot, "callee");
    let mut replacement = created.initial.snapshot.owners[&callee].clone();
    let OwnerRecord::Declaration(declaration) = &mut replacement else {
        panic!("callee must be a declaration")
    };
    declaration.name = Name::new("repository_derived").unwrap();
    let expected = encode_owner(&created.initial.snapshot.owners[&callee])
        .expect("base owner encoding")
        .0;
    let canonical = CanonicalDelta::normalize_from(
        &view,
        vec![PrimitiveEdit::ReplaceOwner {
            expected,
            record: replacement,
        }],
    )
    .expect("repository-backed normalization")
    .canonical;
    let overlay = KernelOverlay::new(&created.initial.snapshot, &canonical);

    let repository =
        derive_local_delta(&overlay, &canonical, &view).expect("repository-backed derived delta");
    let oracle = derive_local_delta(&overlay, &canonical, &created.initial.witness)
        .expect("in-memory derived oracle");
    assert_eq!(repository.namespaces, oracle.namespaces);
    assert_eq!(repository.ownership, oracle.ownership);
    assert_eq!(repository.relations, oracle.relations);
    assert_eq!(repository.summary_candidates, oracle.summary_candidates);
    assert_eq!(repository.read_work.point_reads, 4);
    assert_eq!(repository.read_work.witness_records_decoded, 3);
    assert!(repository.read_work.map_pages_read > 0);
    assert!(repository.read_work.map_pages_read < 48);
    assert_eq!(
        repository.read_work.objects_read, repository.read_work.map_pages_read,
        "derived rename analysis must read only witness map paths, not unrelated objects"
    );
}

#[test]
fn repository_summary_rebuild_reads_only_selected_witness_closure() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let view = created.repository.view_current().expect("pinned view");
    let callee = owner_named(&created.initial.snapshot, "callee");
    let mut replacement = created.initial.snapshot.owners[&callee].clone();
    let OwnerRecord::Declaration(declaration) = &mut replacement else {
        panic!("callee must be a declaration")
    };
    declaration.name = Name::new("repository_summary").unwrap();
    let canonical = CanonicalDelta::normalize(
        &created.initial.snapshot,
        vec![PrimitiveEdit::ReplaceOwner {
            expected: encode_owner(&created.initial.snapshot.owners[&callee])
                .expect("base owner encoding")
                .0,
            record: replacement,
        }],
    )
    .expect("canonical rename");
    let overlay = KernelOverlay::new(&created.initial.snapshot, &canonical);
    let derived =
        derive_local_delta(&overlay, &canonical, &view).expect("repository-backed derived delta");

    let repository =
        derive_summary_delta(&overlay, &derived, &view).expect("repository-backed summary delta");
    let oracle = derive_summary_delta(&overlay, &derived, &created.initial.witness)
        .expect("in-memory summary oracle");
    assert_eq!(repository.selected, oracle.selected);
    assert_eq!(repository.edits, oracle.edits);
    assert_eq!(repository.new_objects, oracle.new_objects);
    assert!(repository.read_work.point_reads > 0);
    assert!(repository.read_work.point_reads < 64);
    assert!(repository.read_work.map_pages_read > 0);
    assert!(repository.read_work.map_pages_read < 256);
    assert!(repository.read_work.objects_read >= repository.read_work.map_pages_read);
    assert!(repository.read_work.objects_read < 256);
}

#[test]
fn repository_impact_plan_matches_full_witness_oracle() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let view = created.repository.view_current().expect("pinned view");
    let binding = binding_named(&created.initial.snapshot, "local");
    let mut replacement = created.initial.snapshot.owners[&binding].clone();
    let OwnerRecord::Binding(record) = &mut replacement else {
        panic!("local must be a binding")
    };
    record.declared_type = None;
    let canonical = CanonicalDelta::normalize(
        &created.initial.snapshot,
        vec![PrimitiveEdit::ReplaceOwner {
            expected: encode_owner(&created.initial.snapshot.owners[&binding])
                .expect("base owner encoding")
                .0,
            record: replacement,
        }],
    )
    .expect("canonical binding edit");
    let overlay = KernelOverlay::new(&created.initial.snapshot, &canonical);
    let derived =
        derive_local_delta(&overlay, &canonical, &view).expect("repository-backed derived delta");

    let repository = plan_impact_and_summaries(&overlay, &canonical, &derived, &view)
        .expect("repository-backed impact plan");
    let oracle =
        plan_impact_and_summaries(&overlay, &canonical, &derived, &created.initial.witness)
            .expect("full-witness impact oracle");
    assert_eq!(repository.initial.selected, oracle.initial.selected);
    assert_eq!(repository.initial.edits, oracle.initial.edits);
    assert_eq!(repository.initial.new_objects, oracle.initial.new_objects);
    assert_eq!(repository.final_delta.selected, oracle.final_delta.selected);
    assert_eq!(repository.final_delta.edits, oracle.final_delta.edits);
    assert_eq!(
        repository.final_delta.new_objects,
        oracle.final_delta.new_objects
    );
    assert_eq!(
        repository.plan.structurally_checked,
        oracle.plan.structurally_checked
    );
    assert_eq!(
        repository.plan.semantically_checked,
        oracle.plan.semantically_checked
    );
    assert_eq!(repository.plan.summary_owners, oracle.plan.summary_owners);
    assert_eq!(repository.plan.compiler_units, oracle.plan.compiler_units);
    assert_eq!(repository.plan.tests, oracle.plan.tests);
    assert_eq!(repository.plan.reasons, oracle.plan.reasons);
    assert_eq!(
        repository.plan.tests,
        std::collections::BTreeSet::from([owner_named(&created.initial.snapshot, "caller_test")])
    );
    assert_eq!(
        repository.plan.work.summary_edits_examined,
        oracle.plan.work.summary_edits_examined
    );
    assert_eq!(
        repository.plan.work.reverse_edges_visited,
        oracle.plan.work.reverse_edges_visited
    );
    assert_eq!(
        repository.plan.work.ownership_steps,
        oracle.plan.work.ownership_steps
    );
    assert_eq!(
        repository.plan.work.behavior_owners_visited,
        oracle.plan.work.behavior_owners_visited
    );
    assert!(repository.plan.work.witness_reads.point_reads > 0);
    assert!(repository.plan.work.witness_reads.point_reads < 64);
    assert!(repository.plan.work.witness_reads.map_pages_read < 256);
    assert!(repository.plan.work.witness_reads.objects_read < 256);
}

#[test]
fn repository_incremental_validation_reads_only_frontier_ownership() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let view = created.repository.view_current().expect("pinned view");
    let binding = binding_named(&created.initial.snapshot, "local");
    let mut replacement = created.initial.snapshot.owners[&binding].clone();
    let OwnerRecord::Binding(record) = &mut replacement else {
        panic!("local must be a binding")
    };
    record.declared_type = None;
    let canonical = CanonicalDelta::normalize(
        &created.initial.snapshot,
        vec![PrimitiveEdit::ReplaceOwner {
            expected: encode_owner(&created.initial.snapshot.owners[&binding])
                .expect("base binding encoding")
                .0,
            record: replacement,
        }],
    )
    .expect("canonical binding edit");
    let overlay = KernelOverlay::new(&created.initial.snapshot, &canonical);
    let derived =
        derive_local_delta(&overlay, &canonical, &view).expect("repository-backed derived delta");
    let repository_plan = plan_impact_and_summaries(&overlay, &canonical, &derived, &view)
        .expect("repository-backed impact plan");
    let oracle_plan =
        plan_impact_and_summaries(&overlay, &canonical, &derived, &created.initial.witness)
            .expect("full-witness impact oracle");
    let repository_structural = validate_structural_frontier(&overlay, &canonical, &derived, &view)
        .expect("repository-backed structural validation");
    let oracle_structural =
        validate_structural_frontier(&overlay, &canonical, &derived, &created.initial.witness)
            .expect("full-witness structural validation oracle");
    assert_eq!(
        repository_structural.structurally_checked,
        oracle_structural.structurally_checked
    );
    assert_eq!(
        repository_structural.work.owner_records_checked,
        oracle_structural.work.owner_records_checked
    );
    assert_eq!(
        repository_structural.work.ownership_entries_checked,
        oracle_structural.work.ownership_entries_checked
    );
    assert_eq!(
        repository_structural.work.type_objects_checked,
        oracle_structural.work.type_objects_checked
    );
    assert_eq!(repository_structural.work.witness_reads.point_reads, 1);
    assert!(repository_structural.work.witness_reads.map_pages_read > 0);
    assert!(repository_structural.work.witness_reads.map_pages_read < 32);
    assert_eq!(
        repository_structural.work.witness_reads.objects_read,
        repository_structural.work.witness_reads.map_pages_read
    );

    let repository = validate_incremental_frontier(
        &overlay,
        &canonical,
        &repository_plan.plan,
        &repository_plan.final_delta,
        &view,
        repository_structural,
    )
    .expect("repository-backed incremental validation");
    let oracle = validate_incremental_frontier(
        &overlay,
        &canonical,
        &oracle_plan.plan,
        &oracle_plan.final_delta,
        &created.initial.witness,
        oracle_structural,
    )
    .expect("full-witness incremental validation oracle");
    assert_eq!(repository.profile, oracle.profile);
    assert_eq!(
        repository.canonical_owners_changed,
        oracle.canonical_owners_changed
    );
    assert_eq!(repository.structurally_checked, oracle.structurally_checked);
    assert_eq!(repository.semantically_checked, oracle.semantically_checked);
    assert_eq!(repository.summaries_reused, oracle.summaries_reused);
    assert_eq!(repository.tests_selected, oracle.tests_selected);
    assert_eq!(
        repository.work.owner_records_checked,
        oracle.work.owner_records_checked
    );
    assert_eq!(
        repository.work.ownership_entries_checked,
        oracle.work.ownership_entries_checked
    );
    assert_eq!(
        repository.work.type_objects_checked,
        oracle.work.type_objects_checked
    );
    assert_eq!(repository.work.expression_work, oracle.work.expression_work);
}

#[test]
fn repository_test_delta_reads_only_the_affected_test_witness_closure() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let view = created.repository.view_current().expect("pinned view");
    let test = owner_named(&created.initial.snapshot, "caller_test");
    let actual = test_actual(&created.initial.snapshot, "caller_test");
    let mut replacement = created.initial.snapshot.owners[&actual].clone();
    let OwnerRecord::Expression(record) = &mut replacement else {
        panic!("test actual must be an expression")
    };
    record.operation = ExpressionOperation::Unit {};
    let canonical = CanonicalDelta::normalize(
        &created.initial.snapshot,
        vec![PrimitiveEdit::ReplaceOwner {
            expected: encode_owner(&created.initial.snapshot.owners[&actual])
                .expect("base test expression encoding")
                .0,
            record: replacement,
        }],
    )
    .expect("canonical test edit");
    let overlay = KernelOverlay::new(&created.initial.snapshot, &canonical);
    let derived =
        derive_local_delta(&overlay, &canonical, &view).expect("repository-backed derived delta");

    let repository = derive_test_dependency_delta(&overlay, &canonical, &derived, &view)
        .expect("repository-backed test delta");
    let oracle =
        derive_test_dependency_delta(&overlay, &canonical, &derived, &created.initial.witness)
            .expect("full-witness test delta oracle");
    assert_eq!(
        repository.affected_tests,
        std::collections::BTreeSet::from([test])
    );
    assert_eq!(repository.affected_tests, oracle.affected_tests);
    assert_eq!(repository.removed, oracle.removed);
    assert_eq!(repository.added, oracle.added);
    assert!(!repository.removed.is_empty());
    assert!(repository.added.is_empty());
    assert_eq!(repository.work.ownership_steps, oracle.work.ownership_steps);
    assert_eq!(repository.work.owners_visited, oracle.work.owners_visited);
    assert_eq!(
        repository.work.relation_edges_visited,
        oracle.work.relation_edges_visited
    );
    assert!(repository.work.witness_reads.point_reads > 0);
    assert!(repository.work.witness_reads.point_reads < 64);
    assert!(repository.work.witness_reads.map_pages_read < 256);
    assert!(repository.work.witness_reads.objects_read < 256);
}

#[test]
fn repository_path_copies_witness_maps_from_packed_base_pages() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let view = created.repository.view_current().expect("pinned view");
    let callee = owner_named(&created.initial.snapshot, "callee");
    let mut replacement = created.initial.snapshot.owners[&callee].clone();
    let OwnerRecord::Declaration(declaration) = &mut replacement else {
        panic!("callee must be a declaration")
    };
    declaration.name = Name::new("repository_path_copy").unwrap();
    let canonical = CanonicalDelta::normalize(
        &created.initial.snapshot,
        vec![PrimitiveEdit::ReplaceOwner {
            expected: encode_owner(&created.initial.snapshot.owners[&callee])
                .expect("base owner encoding")
                .0,
            record: replacement,
        }],
    )
    .expect("canonical rename");
    let oracle = prepare_change_analysis(
        &created.initial.snapshot,
        &created.initial.witness,
        canonical.clone(),
    )
    .expect("in-memory preparation oracle");
    let analysis = prepare_change_analysis(&view, &view, canonical)
        .expect("repository-backed generic preparation");

    assert_eq!(analysis.derived.namespaces, oracle.derived.namespaces);
    assert_eq!(analysis.derived.ownership, oracle.derived.ownership);
    assert_eq!(analysis.derived.relations, oracle.derived.relations);
    assert_eq!(
        analysis.derived.summary_candidates,
        oracle.derived.summary_candidates
    );
    assert_eq!(
        analysis.summaries.initial.edits,
        oracle.summaries.initial.edits
    );
    assert_eq!(
        analysis.summaries.final_delta.edits,
        oracle.summaries.final_delta.edits
    );
    assert_eq!(
        analysis.summaries.plan.reasons,
        oracle.summaries.plan.reasons
    );
    assert_eq!(analysis.tests.removed, oracle.tests.removed);
    assert_eq!(analysis.tests.added, oracle.tests.added);
    assert_eq!(analysis.witness.roots, oracle.witness.roots);
    assert_eq!(analysis.witness.edits, oracle.witness.edits);
    assert_eq!(
        analysis.validation.semantically_checked,
        oracle.validation.semantically_checked
    );
    assert!(analysis.canonical_read_work.point_reads > 0);
    assert!(analysis.canonical_read_work.point_reads < 64);
    assert!(analysis.canonical_read_work.map_pages_read > 0);
    assert!(analysis.canonical_read_work.map_pages_read < 256);
    assert!(analysis.canonical_read_work.canonical_records_decoded > 0);
    assert!(analysis.canonical_read_work.canonical_records_decoded < 64);

    let repository = view
        .update_witness_maps(
            &analysis.derived,
            &analysis.summaries.final_delta,
            &analysis.tests,
            crate::platform::change::WitnessMapAdmission::unbounded(),
        )
        .expect("repository-backed witness update");
    assert_eq!(repository.revision, created.current.head.revision);
    assert_eq!(repository.update.roots, analysis.witness.roots);
    assert_eq!(repository.update.edits, analysis.witness.edits);
    let repository_pages = repository
        .update
        .new_pages
        .objects()
        .map(|(digest, bytes)| (digest, bytes.to_vec()))
        .collect::<Vec<_>>();
    let oracle_pages = analysis
        .witness
        .new_pages
        .objects()
        .map(|(digest, bytes)| (digest, bytes.to_vec()))
        .collect::<Vec<_>>();
    assert_eq!(repository_pages, oracle_pages);
    assert!(repository.store_work.objects_read > 0);
    assert!(repository.store_work.objects_read < 128);
    assert!(repository.store_work.catalog_lookups >= repository.store_work.objects_read);
    assert!(repository.store_work.catalog_lookups < 128);
    assert_eq!(repository.store_work.objects_staged, 0);
    assert!(repository.store_work.objects_read <= repository.update.work.pages_read);
}

#[test]
fn pinned_view_keeps_old_revision_and_namespace_after_head_advances() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let old_view = created.repository.view_current().expect("old pinned view");
    let callee = owner_named(&created.initial.snapshot, "callee");
    let OwnerRecord::Declaration(old_declaration) = &created.initial.snapshot.owners[&callee]
    else {
        panic!("callee must be a declaration")
    };
    let old_key = NamespaceKey {
        parent: Some(OwnerKey::Module(old_declaration.module)),
        class: NamespaceClass::Declaration,
        name: old_declaration.name.clone(),
    };
    let prepared = prepare_rename_publication(&created, "renamed_callee", None);
    assert!(matches!(
        created.repository.publish(&prepared).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    let new_view = created.repository.view_current().expect("new pinned view");

    assert_eq!(old_view.revision(), created.current.head.revision);
    assert_eq!(new_view.revision(), prepared.head.revision);
    let Some(OwnerRecord::Declaration(old_record)) = old_view.owner(callee).unwrap().value else {
        panic!("old view must retain declaration")
    };
    let Some(OwnerRecord::Declaration(new_record)) = new_view.owner(callee).unwrap().value else {
        panic!("new view must retain declaration")
    };
    assert_eq!(old_record.name.as_str(), "callee");
    assert_eq!(new_record.name.as_str(), "renamed_callee");
    assert_eq!(old_view.namespace(&old_key).unwrap().value, Some(callee));
    assert_eq!(new_view.namespace(&old_key).unwrap().value, None);
    let new_key = NamespaceKey {
        parent: old_key.parent,
        class: old_key.class,
        name: Name::new("renamed_callee").unwrap(),
    };
    assert_eq!(new_view.namespace(&new_key).unwrap().value, Some(callee));
}

#[test]
fn revision_view_detects_corruption_in_the_selected_owner_without_full_scan() {
    use std::io::{Read, Seek, SeekFrom, Write};

    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let view = created.repository.view_current().expect("pinned view");
    let callee = owner_named(&created.initial.snapshot, "callee");
    let digest = encode_owner(&created.initial.snapshot.owners[&callee])
        .expect("owner encoding")
        .0;
    let key = ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes());
    let store = created.repository.object_store().expect("object store");
    let location = store.catalog().get(key).expect("owner catalog location");
    let pack = destination.join("packs").join(location.pack.file_name());
    drop(store);

    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(pack)
        .expect("open test pack");
    file.seek(SeekFrom::Start(location.offset))
        .expect("seek owner payload");
    let mut byte = [0_u8; 1];
    file.read_exact(&mut byte).expect("read owner payload byte");
    byte[0] ^= 0x80;
    file.seek(SeekFrom::Start(location.offset))
        .expect("seek owner payload again");
    file.write_all(&byte).expect("corrupt owner payload byte");
    file.sync_all().expect("sync test corruption");

    let error = view
        .owner(callee)
        .expect_err("selected corrupt owner must reject");
    assert_eq!(
        error.class,
        crate::platform::diagnostic::DiagnosticClass::Corrupt
    );
}

#[test]
fn authored_allocation_ignores_local_labels_budgets_and_operational_options() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let created = GraphRepository::create(
        &destination,
        &empty_snapshot(b"authored-allocation-normalization"),
        None,
    )
    .expect("create repository");
    let base = created.current.head.revision;
    let request = |first: &str, second: &str, budget| AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        changes: vec![
            AuthoredChange::CreateModule {
                symbol: first.to_owned(),
                name: Name::new("first_module").unwrap(),
            },
            AuthoredChange::CreateModule {
                symbol: second.to_owned(),
                name: Name::new("second_module").unwrap(),
            },
        ],
        budget,
    };

    let first = created
        .repository
        .prepare_authored_change(
            &request("$z_first", "$a_second", ChangeBudget::default()),
            PublicationOptions {
                idempotency_key: Some("allocation-one".to_owned()),
                intent: Some("first operational observation".to_owned()),
            },
        )
        .expect("prepare first spelling");
    let mut changed_budget = ChangeBudget::default();
    changed_budget.canonical_reads.maximum_bytes -= 1;
    let second = created
        .repository
        .prepare_authored_change(
            &request("$a_first", "$z_second", changed_budget),
            PublicationOptions {
                idempotency_key: Some("allocation-two".to_owned()),
                intent: Some("different operational observation".to_owned()),
            },
        )
        .expect("prepare renamed labels and operational controls");

    assert_eq!(first.allocated["$z_first"], second.allocated["$a_first"]);
    assert_eq!(first.allocated["$a_second"], second.allocated["$z_second"]);
    assert_eq!(
        first.publication.authority.semantic.root,
        second.publication.authority.semantic.root
    );
    assert_eq!(created.repository.current().unwrap().head.revision, base);
}

#[test]
fn authored_request_allocates_forward_symbols_and_preserves_exact_uses() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let callee = owner_named(&created.initial.snapshot, "callee");
    let OwnerKey::Declaration(callee_id) = callee else {
        panic!("callee must be a declaration")
    };
    let caller = owner_named(&created.initial.snapshot, "caller");
    let binding = binding_named(&created.initial.snapshot, "local");
    let body = function_body(&created.initial.snapshot, "callee");
    let OwnerKey::Expression(body) = body else {
        panic!("callee body must be an expression")
    };
    let call = expression_calling(&created.initial.snapshot, callee);
    let local_use = expression_using_binding(&created.initial.snapshot, binding);
    let caller_before = encode_owner(&created.initial.snapshot.owners[&caller])
        .expect("caller encoding")
        .0;
    let call_before = encode_owner(&created.initial.snapshot.owners[&call])
        .expect("call encoding")
        .0;
    let local_use_before = encode_owner(&created.initial.snapshot.owners[&local_use])
        .expect("local-use encoding")
        .0;
    let request = AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::MoveDeclaration {
                declaration: DeclarationSelector::Qualified {
                    module: ModuleSelector::Name {
                        name: Name::new("first").unwrap(),
                    },
                    name: Name::new("callee").unwrap(),
                },
                module: ModuleSelector::Symbol {
                    symbol: "$destination".to_owned(),
                },
            },
            AuthoredChange::RenameOwner {
                owner: OwnerSelector::Exact { owner: callee },
                name: Name::new("renamed_callee").unwrap(),
            },
            AuthoredChange::RenameOwner {
                owner: OwnerSelector::Exact { owner: binding },
                name: Name::new("renamed_local").unwrap(),
            },
            AuthoredChange::ReplaceFunctionBody {
                function: DeclarationSelector::Id {
                    declaration: callee_id,
                },
                body: AuthoredExpression {
                    symbol: Some("$replacement_body".to_owned()),
                    operation: AuthoredExpressionOperation::Unit {},
                },
            },
            AuthoredChange::CreateModule {
                symbol: "$destination".to_owned(),
                name: Name::new("destination").unwrap(),
            },
        ],
    };
    let options = PublicationOptions {
        idempotency_key: Some("authored-request-1".to_owned()),
        intent: Some("stable move, rename, and body replacement".to_owned()),
    };
    let prepared = created
        .repository
        .prepare_authored_change(&request, options.clone())
        .expect("prepare authored request");
    let repeated = created
        .repository
        .prepare_authored_change(&request, options)
        .expect("repeat deterministic authored preparation");
    assert_eq!(prepared.allocated, repeated.allocated);
    assert_eq!(prepared.publication.head, repeated.publication.head);
    assert_eq!(prepared.publication.receipt.counts.owners_created, 2);
    assert_eq!(prepared.publication.receipt.counts.owners_updated, 2);
    assert_eq!(prepared.publication.receipt.counts.owners_deleted, 1);
    assert!(prepared.lowering_work.canonical.point_reads <= 8);
    assert!(prepared.lowering_work.canonical.canonical_records_decoded <= 8);
    assert!(prepared.lowering_work.witness.point_reads <= 8);
    assert!(prepared.publication.receipt.work.map_pages_read > 0);
    let new_module = prepared.allocated["$destination"];
    let OwnerKey::Module(new_module_id) = new_module else {
        panic!("module symbol must allocate one module identity")
    };
    let replacement_body = prepared.allocated["$replacement_body"];

    assert!(matches!(
        created
            .repository
            .publish(&prepared.publication)
            .expect("publish authored request"),
        PublicationOutcome::Accepted { .. }
    ));
    let view = created.repository.view_current().expect("advanced view");
    let Some(OwnerRecord::Declaration(declaration)) = view.owner(callee).unwrap().value else {
        panic!("moved declaration must remain live")
    };
    assert_eq!(declaration.module, new_module_id);
    assert_eq!(declaration.name.as_str(), "renamed_callee");
    let crate::platform::kernel::DeclarationPayload::Function(function) = declaration.payload
    else {
        panic!("moved declaration must remain a function")
    };
    assert_eq!(OwnerKey::Expression(function.body), replacement_body);
    assert!(
        view.owner(OwnerKey::Expression(body))
            .unwrap()
            .value
            .is_none()
    );
    let Some(OwnerRecord::Expression(expression)) = view.owner(replacement_body).unwrap().value
    else {
        panic!("replacement body must be live")
    };
    assert_eq!(expression.operation, ExpressionOperation::Unit {});
    let Some(OwnerRecord::Binding(renamed_binding)) = view.owner(binding).unwrap().value else {
        panic!("renamed binding must remain live")
    };
    assert_eq!(renamed_binding.name.as_str(), "renamed_local");

    let caller_after = view
        .owner(caller)
        .unwrap()
        .value
        .expect("caller remains live");
    let call_after = view.owner(call).unwrap().value.expect("call remains live");
    let local_use_after = view
        .owner(local_use)
        .unwrap()
        .value
        .expect("local use remains live");
    assert_eq!(encode_owner(&caller_after).unwrap().0, caller_before);
    assert_eq!(encode_owner(&call_after).unwrap().0, call_before);
    assert_eq!(encode_owner(&local_use_after).unwrap().0, local_use_before);
    let OwnerRecord::Expression(call_record) = call_after else {
        panic!("exact caller use must remain an expression")
    };
    let ExpressionOperation::Call { function, .. } = call_record.operation else {
        panic!("exact caller use must remain a call")
    };
    assert_eq!(OwnerKey::Declaration(function.declaration), callee);
    let OwnerRecord::Expression(local_record) = local_use_after else {
        panic!("exact local use must remain an expression")
    };
    assert_eq!(
        local_record.operation,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(match binding {
                OwnerKey::Binding(binding) => binding,
                _ => panic!("binding helper returned a foreign owner"),
            }),
        }
    );
    assert_eq!(
        view.namespace(&NamespaceKey {
            parent: Some(new_module),
            class: NamespaceClass::Declaration,
            name: Name::new("renamed_callee").unwrap(),
        })
        .unwrap()
        .value,
        Some(callee)
    );
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&request, PublicationOptions::default())
            .expect_err("current view must reject stale authored base")[0]
            .code,
        "change_authored_stale_base"
    );
}

#[test]
fn authored_request_rejects_invalid_symbols_kinds_and_empty_work() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let base = created.current.head.revision;
    let callee = owner_named(&created.initial.snapshot, "callee");
    let body = function_body(&created.initial.snapshot, "callee");

    let duplicate = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::CreateModule {
                symbol: "$module".to_owned(),
                name: Name::new("one").unwrap(),
            },
            AuthoredChange::CreateModule {
                symbol: "$module".to_owned(),
                name: Name::new("two").unwrap(),
            },
        ],
    };
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&duplicate, PublicationOptions::default())
            .expect_err("duplicate request-local symbols must reject")[0]
            .code,
        "change_authored_symbol_duplicate"
    );

    let missing = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::MoveDeclaration {
            declaration: match callee {
                OwnerKey::Declaration(declaration) => DeclarationSelector::Id { declaration },
                _ => panic!("named function must be a declaration"),
            },
            module: ModuleSelector::Symbol {
                symbol: "$missing".to_owned(),
            },
        }],
    };
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&missing, PublicationOptions::default())
            .expect_err("undefined request-local symbol must reject")[0]
            .code,
        "change_authored_symbol_missing"
    );

    let wrong_kind = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::RenameOwner {
            owner: OwnerSelector::Exact { owner: body },
            name: Name::new("not_an_expression_name").unwrap(),
        }],
    };
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&wrong_kind, PublicationOptions::default())
            .expect_err("expression rename must reject")[0]
            .code,
        "change_authored_rename_kind"
    );

    let empty = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: Vec::new(),
    };
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&empty, PublicationOptions::default())
            .expect_err("empty authored request must reject")[0]
            .code,
        "change_authored_count"
    );

    let duplicate_member_name = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::CreateRecord {
            symbol: "$record".to_owned(),
            module: ModuleSelector::Name {
                name: Name::new("second").unwrap(),
            },
            name: Name::new("DuplicateFields").unwrap(),
            visibility: DeclarationVisibility::Private,
            fields: vec![
                AuthoredField {
                    symbol: "$first_field".to_owned(),
                    name: Name::new("same").unwrap(),
                    ty: AuthoredType::Unit {},
                },
                AuthoredField {
                    symbol: "$second_field".to_owned(),
                    name: Name::new("same").unwrap(),
                    ty: AuthoredType::Unit {},
                },
            ],
        }],
    };
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&duplicate_member_name, PublicationOptions::default())
            .expect_err("duplicate member namespace must reject")[0]
            .code,
        "change_namespace_candidate_duplicate"
    );
    assert_eq!(
        created.repository.current().unwrap().head,
        created.current.head
    );
}

#[test]
fn authored_reject_deletion_never_infers_an_owned_closure() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let base = created.current.head.revision;
    let test = owner_named(&created.initial.snapshot, "caller_test");
    let request = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::DeleteOwner {
            owner: OwnerSelector::Exact { owner: test },
            policy: AuthoredDeletePolicy::Reject,
        }],
    };
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&request, PublicationOptions::default())
            .expect_err("reject policy must not infer an owned closure")[0]
            .code,
        "change_delete_owned_children"
    );
    assert_eq!(created.repository.current().unwrap().head.revision, base);
}

#[test]
fn authored_owned_closure_covers_every_owner_kind_with_complete_oracle() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let expected = logical.owners.keys().copied().collect::<BTreeSet<_>>();
    assert_eq!(
        logical
            .owners
            .values()
            .map(OwnerRecord::kind)
            .collect::<BTreeSet<_>>(),
        OwnerKind::ALL.into_iter().collect::<BTreeSet<_>>()
    );
    let roots = logical
        .owners
        .iter()
        .filter_map(|(owner, record)| {
            matches!(record, OwnerRecord::Module(_) | OwnerRecord::Target(_)).then_some(*owner)
        })
        .collect::<Vec<_>>();
    assert_eq!(roots.len(), 3);
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let base = created.current.head.revision;
    let request = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: roots
            .iter()
            .map(|owner| AuthoredChange::DeleteOwner {
                owner: OwnerSelector::Exact { owner: *owner },
                policy: AuthoredDeletePolicy::OwnedClosure,
            })
            .collect(),
    };

    // Complete materialization is deliberately oracle-only. Production lowering selects the
    // closure through point reads and exact reverse-relation prefixes.
    let pinned = created.repository.view_current().expect("open exact base");
    let lowering =
        lower_authored_changes(&pinned, &pinned, &request).expect("lower complete owner matrix");
    assert_eq!(
        lowering.work.ownership_steps,
        u64::try_from(expected.len() - roots.len()).unwrap()
    );
    let canonical = CanonicalDelta::normalize(&logical, lowering.edits)
        .expect("normalize complete owner matrix deletion");
    let candidate = KernelOverlay::new(&logical, &canonical).materialize_logical_oracle();
    assert!(candidate.owners.is_empty());
    assert_eq!(
        candidate
            .retirements
            .keys()
            .copied()
            .collect::<BTreeSet<_>>(),
        expected
    );

    let prepared = created
        .repository
        .prepare_authored_change(&request, PublicationOptions::default())
        .expect("prepare complete owner matrix deletion");
    assert_eq!(
        prepared
            .logical_plan
            .retirements
            .keys()
            .copied()
            .collect::<BTreeSet<_>>(),
        expected
    );
    assert_eq!(
        prepared.logical_plan.relations.removed,
        created
            .initial
            .witness
            .entries
            .relations
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
    );
    assert!(prepared.logical_plan.relations.added.is_empty());
    let SemanticDiffBody::Change { owners, .. } = &prepared.publication.semantic_diff.body else {
        panic!("owned closure must produce a semantic change")
    };
    assert_eq!(
        owners
            .iter()
            .filter(|edit| edit.objects.after.is_none())
            .map(|edit| edit.owner)
            .collect::<BTreeSet<_>>(),
        expected
    );
    created
        .repository
        .publish(&prepared.publication)
        .expect("publish complete owner matrix deletion");
    let reopened = GraphRepository::open(&destination)
        .expect("reopen deleted authority")
        .view_current()
        .expect("open deleted revision");
    for owner in expected {
        assert!(reopened.owner(owner).unwrap().value.is_none());
        let retirement = reopened
            .retirement(owner)
            .unwrap()
            .value
            .expect("every deleted stable owner has one retirement");
        assert_eq!(retirement.last_live_revision, base);
        assert_eq!(retirement.owner, owner);
    }
}

#[test]
fn authored_owned_closure_removes_operation_parameters_and_updates_one_surviving_parent() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let created = GraphRepository::create(
        &destination,
        &empty_snapshot(b"owned-closure-operation-parent"),
        None,
    )
    .expect("create repository");
    let create = AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::CreateModule {
                symbol: "$module".to_owned(),
                name: Name::new("module").unwrap(),
            },
            AuthoredChange::CreateInterface {
                symbol: "$interface".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$module".to_owned(),
                },
                name: Name::new("Interface").unwrap(),
                visibility: DeclarationVisibility::Public,
                operations: vec![
                    AuthoredOperation {
                        symbol: "$removed_operation".to_owned(),
                        name: Name::new("removed").unwrap(),
                        parameters: vec![AuthoredParameter {
                            symbol: "$removed_parameter".to_owned(),
                            name: Name::new("value").unwrap(),
                            ty: AuthoredType::Unit {},
                        }],
                        result: AuthoredType::Unit {},
                        idempotency: Idempotency::Idempotent,
                        external_visibility: ExternalVisibility::None,
                    },
                    AuthoredOperation {
                        symbol: "$surviving_operation".to_owned(),
                        name: Name::new("surviving").unwrap(),
                        parameters: Vec::new(),
                        result: AuthoredType::Unit {},
                        idempotency: Idempotency::Idempotent,
                        external_visibility: ExternalVisibility::None,
                    },
                ],
            },
        ],
    };
    let prepared = created
        .repository
        .prepare_authored_change(&create, PublicationOptions::default())
        .expect("prepare operation-parent fixture");
    let interface = prepared.allocated["$interface"];
    let removed_operation = prepared.allocated["$removed_operation"];
    let removed_parameter = prepared.allocated["$removed_parameter"];
    let surviving_operation = prepared.allocated["$surviving_operation"];
    created
        .repository
        .publish(&prepared.publication)
        .expect("publish operation-parent fixture");

    let request = AuthoredChangeSet {
        base: created.repository.current().unwrap().head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::DeleteOwner {
            owner: OwnerSelector::Exact {
                owner: removed_operation,
            },
            policy: AuthoredDeletePolicy::OwnedClosure,
        }],
    };
    let prepared = created
        .repository
        .prepare_authored_change(&request, PublicationOptions::default())
        .expect("prepare operation ownership closure");
    let SemanticDiffBody::Change { owners, .. } = &prepared.publication.semantic_diff.body else {
        panic!("operation closure must produce a semantic change")
    };
    let deleted = owners
        .iter()
        .filter(|edit| edit.objects.after.is_none())
        .map(|edit| edit.owner)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        deleted,
        BTreeSet::from([removed_operation, removed_parameter])
    );
    assert_eq!(
        owners
            .iter()
            .filter(|edit| edit.objects.before.is_some() && edit.objects.after.is_some())
            .map(|edit| edit.owner)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from([interface])
    );
    assert_eq!(
        prepared
            .logical_plan
            .retirements
            .keys()
            .copied()
            .collect::<BTreeSet<_>>(),
        deleted
    );
    created
        .repository
        .publish(&prepared.publication)
        .expect("publish operation ownership closure");
    let view = created.repository.view_current().expect("open result");
    assert!(view.owner(removed_operation).unwrap().value.is_none());
    assert!(view.owner(removed_parameter).unwrap().value.is_none());
    let interface = view
        .owner(interface)
        .unwrap()
        .value
        .expect("surviving interface");
    let OwnerRecord::Declaration(interface) = interface else {
        panic!("interface identity must retain a declaration")
    };
    let DeclarationPayload::Interface { operations } = interface.payload else {
        panic!("surviving declaration must remain an interface")
    };
    let OwnerKey::Operation(surviving_operation) = surviving_operation else {
        panic!("surviving operation identity must be operation-shaped")
    };
    assert_eq!(operations, vec![surviving_operation]);
}

#[test]
fn authored_owned_closure_unions_overlap_and_rejects_invalid_roots_and_created_descendants() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let base = created.current.head.revision;
    let second = logical
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Module(module) if module.name.as_str() == "second" => Some(*owner),
            _ => None,
        })
        .expect("second module");
    let descendant = owner_named(&logical, "with_binding");
    let expected = complete_ownership_descendants(&created.initial.witness, second);
    let overlap = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::DeleteOwner {
                owner: OwnerSelector::Exact { owner: second },
                policy: AuthoredDeletePolicy::OwnedClosure,
            },
            AuthoredChange::DeleteOwner {
                owner: OwnerSelector::Exact { owner: descendant },
                policy: AuthoredDeletePolicy::OwnedClosure,
            },
        ],
    };
    let prepared = created
        .repository
        .prepare_authored_change(&overlap, PublicationOptions::default())
        .expect("overlapping roots form one deterministic union");
    let SemanticDiffBody::Change { owners, .. } = &prepared.publication.semantic_diff.body else {
        panic!("overlap must produce a semantic change")
    };
    assert_eq!(
        owners
            .iter()
            .filter(|edit| edit.objects.after.is_none())
            .map(|edit| edit.owner)
            .collect::<BTreeSet<_>>(),
        expected
    );

    let failures = [
        (
            "change_delete_owned_children",
            vec![
                AuthoredChange::DeleteOwner {
                    owner: OwnerSelector::Exact { owner: second },
                    policy: AuthoredDeletePolicy::OwnedClosure,
                },
                AuthoredChange::DeleteOwner {
                    owner: OwnerSelector::Exact { owner: descendant },
                    policy: AuthoredDeletePolicy::Reject,
                },
            ],
        ),
        (
            "change_delete_duplicate",
            vec![
                AuthoredChange::DeleteOwner {
                    owner: OwnerSelector::Exact { owner: second },
                    policy: AuthoredDeletePolicy::OwnedClosure,
                },
                AuthoredChange::DeleteOwner {
                    owner: OwnerSelector::Exact { owner: second },
                    policy: AuthoredDeletePolicy::Reject,
                },
            ],
        ),
    ];
    for (code, changes) in failures {
        let request = AuthoredChangeSet {
            base,
            preconditions: Vec::new(),
            budget: ChangeBudget::default(),
            changes,
        };
        assert_eq!(
            created
                .repository
                .prepare_authored_change(&request, PublicationOptions::default())
                .expect_err("invalid overlapping root selection must reject")[0]
                .code,
            code
        );
    }

    for owner in logical
        .owners
        .keys()
        .copied()
        .filter(|owner| matches!(owner, OwnerKey::Binding(_) | OwnerKey::Expression(_)))
        .take(2)
    {
        let request = AuthoredChangeSet {
            base,
            preconditions: Vec::new(),
            budget: ChangeBudget::default(),
            changes: vec![AuthoredChange::DeleteOwner {
                owner: OwnerSelector::Exact { owner },
                policy: AuthoredDeletePolicy::OwnedClosure,
            }],
        };
        assert_eq!(
            created
                .repository
                .prepare_authored_change(&request, PublicationOptions::default())
                .expect_err("expression-shaped public root must reject")[0]
                .code,
            "change_delete_expression_parent"
        );
    }

    let OwnerKey::Declaration(record) = owner_named(&logical, "Payload") else {
        panic!("Payload must be a declaration")
    };
    let created_descendant = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::AddField {
                record: DeclarationSelector::Id {
                    declaration: record,
                },
                field: AuthoredField {
                    symbol: "$created_then_closed".to_owned(),
                    name: Name::new("created_then_closed").unwrap(),
                    ty: AuthoredType::Unit {},
                },
            },
            AuthoredChange::DeleteOwner {
                owner: OwnerSelector::Exact {
                    owner: OwnerKey::Declaration(record),
                },
                policy: AuthoredDeletePolicy::OwnedClosure,
            },
        ],
    };
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&created_descendant, PublicationOptions::default())
            .expect_err("request-local descendants cannot disappear in a closure")[0]
            .code,
        "change_delete_created_owner"
    );
    assert_eq!(created.repository.current().unwrap().head.revision, base);
}

#[test]
fn authored_owned_closure_follows_ownership_without_deleting_referenced_targets() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let created = GraphRepository::create(
        &destination,
        &empty_snapshot(b"owned-closure-reference-separation"),
        None,
    )
    .expect("create repository");
    let create = AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::CreateModule {
                symbol: "$module".to_owned(),
                name: Name::new("module").unwrap(),
            },
            AuthoredChange::CreateFunction {
                symbol: "$callee".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$module".to_owned(),
                },
                name: Name::new("callee").unwrap(),
                visibility: DeclarationVisibility::Private,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: AuthoredType::Unit {},
                effect: AuthoredFunctionEffect::Pure {},
                body: authored_expression(AuthoredExpressionOperation::Unit {}),
            },
            AuthoredChange::CreateFunction {
                symbol: "$caller".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$module".to_owned(),
                },
                name: Name::new("caller").unwrap(),
                visibility: DeclarationVisibility::Private,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: AuthoredType::Unit {},
                effect: AuthoredFunctionEffect::Pure {},
                body: authored_expression(AuthoredExpressionOperation::Call {
                    function: AuthoredDeclarationReference::Local {
                        declaration: DeclarationSelector::Symbol {
                            symbol: "$callee".to_owned(),
                        },
                    },
                    type_arguments: Vec::new(),
                    arguments: Vec::new(),
                }),
            },
        ],
    };
    let prepared = created
        .repository
        .prepare_authored_change(&create, PublicationOptions::default())
        .expect("prepare reference-separation fixture");
    let module = prepared.allocated["$module"];
    let callee = prepared.allocated["$callee"];
    let caller = prepared.allocated["$caller"];
    created
        .repository
        .publish(&prepared.publication)
        .expect("publish reference-separation fixture");

    let fixture_base = created.repository.current().unwrap().head.revision;
    let callee_only = AuthoredChangeSet {
        base: fixture_base,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::DeleteOwner {
            owner: OwnerSelector::Exact { owner: callee },
            policy: AuthoredDeletePolicy::OwnedClosure,
        }],
    };
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&callee_only, PublicationOptions::default())
            .expect_err("surviving caller must block callee closure deletion")[0]
            .code,
        "change_delete_live_reference"
    );
    let delete_both = AuthoredChangeSet {
        base: fixture_base,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::DeleteOwner {
                owner: OwnerSelector::Exact { owner: callee },
                policy: AuthoredDeletePolicy::OwnedClosure,
            },
            AuthoredChange::DeleteOwner {
                owner: OwnerSelector::Exact { owner: caller },
                policy: AuthoredDeletePolicy::OwnedClosure,
            },
        ],
    };
    created
        .repository
        .prepare_authored_change(&delete_both, PublicationOptions::default())
        .expect("explicitly deleting the referrer must close the final candidate");

    let delete = AuthoredChangeSet {
        base: fixture_base,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::DeleteOwner {
            owner: OwnerSelector::Exact { owner: caller },
            policy: AuthoredDeletePolicy::OwnedClosure,
        }],
    };
    let prepared = created
        .repository
        .prepare_authored_change(&delete, PublicationOptions::default())
        .expect("outgoing reference must not expand semantic ownership");
    created
        .repository
        .publish(&prepared.publication)
        .expect("publish caller ownership closure");
    let view = created
        .repository
        .view_current()
        .expect("open deleted caller revision");
    assert!(view.owner(caller).unwrap().value.is_none());
    assert!(view.owner(callee).unwrap().value.is_some());
    assert!(view.owner(module).unwrap().value.is_some());
}

#[test]
fn authored_owned_closure_accepts_explicit_same_request_reference_repair() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let callee = owner_named(&logical, "callee");
    let caller = owner_named(&logical, "caller");
    let expected_callee_closure = complete_ownership_descendants(&created.initial.witness, callee);
    let OwnerKey::Declaration(caller_id) = caller else {
        panic!("caller must be a declaration")
    };
    let request = AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::ReplaceFunctionBody {
                function: DeclarationSelector::Id {
                    declaration: caller_id,
                },
                body: AuthoredExpression {
                    symbol: Some("$repaired_body".to_owned()),
                    operation: AuthoredExpressionOperation::Unit {},
                },
            },
            AuthoredChange::DeleteOwner {
                owner: OwnerSelector::Exact { owner: callee },
                policy: AuthoredDeletePolicy::OwnedClosure,
            },
        ],
    };
    let prepared = created
        .repository
        .prepare_authored_change(&request, PublicationOptions::default())
        .expect("explicit candidate body repair must remove the surviving reference");
    let repaired_body = prepared.allocated["$repaired_body"];
    let SemanticDiffBody::Change { owners, .. } = &prepared.publication.semantic_diff.body else {
        panic!("repair and closure deletion must produce a semantic change")
    };
    let removed = owners
        .iter()
        .filter(|edit| edit.objects.after.is_none())
        .map(|edit| edit.owner)
        .collect::<BTreeSet<_>>();
    assert!(expected_callee_closure.is_subset(&removed));
    created
        .repository
        .publish(&prepared.publication)
        .expect("publish explicit reference repair and closure deletion");
    let view = created
        .repository
        .view_current()
        .expect("open repaired revision");
    for owner in expected_callee_closure {
        assert!(view.owner(owner).unwrap().value.is_none());
    }
    assert!(view.owner(caller).unwrap().value.is_some());
    assert!(view.owner(repaired_body).unwrap().value.is_some());
}

#[test]
fn authored_owned_closure_charges_independent_selection_admissions() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let base = created.current.head.revision;
    let root = logical
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Module(module) if module.name.as_str() == "second" => Some(*owner),
            _ => None,
        })
        .expect("second module");
    let request = |budget| AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget,
        changes: vec![AuthoredChange::DeleteOwner {
            owner: OwnerSelector::Exact { owner: root },
            policy: AuthoredDeletePolicy::OwnedClosure,
        }],
    };
    let pinned = created.repository.view_current().expect("open exact base");
    let baseline = lower_authored_changes(&pinned, &pinned, &request(ChangeBudget::default()))
        .expect("baseline closure lowering");
    assert!(baseline.work.ownership_steps > 0);
    assert!(baseline.work.relation_edges_read > 0);
    assert!(baseline.work.canonical.point_reads > 0);
    assert!(baseline.work.witness.point_reads > 0);

    type Exhaust = fn(&mut ChangeBudget);
    let exhaustions: &[(Exhaust, &str)] = &[
        (
            |budget| budget.canonical_edits.maximum_owner_edits = 0,
            "change_budget_canonical_owner_edits",
        ),
        (
            |budget| budget.canonical_edits.maximum_retirement_edits = 0,
            "change_budget_canonical_retirement_edits",
        ),
        (
            |budget| budget.canonical_reads.maximum_point_reads = 0,
            "change_budget_canonical_point_reads",
        ),
        (
            |budget| budget.witness_reads.maximum_point_reads = 0,
            "change_budget_witness_point_reads",
        ),
        (
            |budget| budget.impact.maximum_ownership_steps = 0,
            "change_budget_impact_ownership_steps",
        ),
        (
            |budget| budget.impact.maximum_relation_edges = 0,
            "change_budget_relation_edges",
        ),
        (
            |budget| budget.impact.maximum_relation_fanout = 0,
            "change_budget_relation_fanout",
        ),
    ];
    for (exhaust, expected) in exhaustions {
        let mut budget = ChangeBudget::default();
        exhaust(&mut budget);
        let diagnostic = lower_authored_changes(&pinned, &pinned, &request(budget))
            .expect_err("closure selection admission must reject deterministically");
        assert_eq!(diagnostic.class, DiagnosticClass::Resource);
        assert_eq!(diagnostic.code, *expected);
        assert_eq!(created.repository.current().unwrap().head.revision, base);
    }
}

#[test]
fn authored_owned_closure_plan_output_admissions_are_independent_and_nonpublishing() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let base = created.current.head.revision;
    let root = logical
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Module(module) if module.name.as_str() == "second" => Some(*owner),
            _ => None,
        })
        .expect("second module");
    let request = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::DeleteOwner {
            owner: OwnerSelector::Exact { owner: root },
            policy: AuthoredDeletePolicy::OwnedClosure,
        }],
    };
    let normalized =
        crate::platform::control::normalize_change_request(request, PublicationOptions::default())
            .expect("normalize closure request");
    let prepared = created
        .repository
        .prepare_authored_change(&normalized.semantic, normalized.options)
        .expect("prepare ownership closure");
    let plan =
        crate::platform::control::LogicalChangePlan::new(normalized.request_commitment, &prepared)
            .expect("construct ownership-closure plan");
    let admitted = crate::platform::control::encode_logical_change_plan(&plan, |_| Ok(()))
        .expect("encode admitted ownership-closure plan");
    assert!(admitted.records > 1);
    assert!(admitted.bytes > 1);

    let record_error = crate::platform::control::encode_logical_change_plan_with_limits(
        &plan,
        |_| Ok(()),
        u64::MAX,
        admitted.records - 1,
    )
    .expect_err("record admission must reject a complete oversized plan");
    assert_eq!(record_error.class, DiagnosticClass::Resource);
    assert_eq!(record_error.code, "change_plan_output_record_budget");

    let byte_error = crate::platform::control::encode_logical_change_plan_with_limits(
        &plan,
        |_| Ok(()),
        admitted.bytes - 1,
        u64::MAX,
    )
    .expect_err("byte admission must reject a complete oversized plan");
    assert_eq!(byte_error.class, DiagnosticClass::Resource);
    assert_eq!(byte_error.code, "change_plan_output_byte_budget");
    assert_eq!(created.repository.current().unwrap().head.revision, base);
}

#[test]
fn authored_owned_closure_rejects_inconsistent_ownership_and_relation_witnesses() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let base = created.repository.view_current().expect("open exact base");
    let root = logical
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Module(module) if module.name.as_str() == "second" => Some(*owner),
            _ => None,
        })
        .expect("second module");
    let request = AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::DeleteOwner {
            owner: OwnerSelector::Exact { owner: root },
            policy: AuthoredDeletePolicy::OwnedClosure,
        }],
    };
    let reject = |witness: &crate::platform::witness::FullWitness, expected: &str| {
        let diagnostic = lower_authored_changes(&base, witness, &request)
            .expect_err("inconsistent accepted witness must reject closure deletion");
        assert_eq!(diagnostic.class, DiagnosticClass::Corrupt);
        assert_eq!(diagnostic.code, expected);
        assert_eq!(
            created.repository.current().unwrap().head.revision,
            request.base
        );
    };

    let mut missing = created.initial.witness.clone();
    assert!(missing.entries.ownership.remove(&root).is_some());
    reject(&missing, "change_delete_ownership_missing");

    let mut wrong_parent = created.initial.witness.clone();
    wrong_parent
        .entries
        .ownership
        .get_mut(&root)
        .expect("root ownership")
        .parent = OwnershipParent::Owner(root);
    reject(&wrong_parent, "change_delete_ownership_disagreement");

    let mut foreign = created.initial.witness.clone();
    let external_child = foreign
        .entries
        .reverse_relations
        .iter_mut()
        .find(|edge| {
            edge.kind == RelationKind::DeclarationModule
                && edge.target
                    == RelationEndpoint::Owner(ExactOwnerKey {
                        package: logical.root.package_id,
                        owner: root,
                    })
        })
        .expect("module-owned declaration relation");
    let RelationEndpoint::Owner(mut source) = external_child.source else {
        panic!("declaration ownership source must be an owner")
    };
    source.package = PackageId::migrate(b"foreign-ownership-package", 1);
    external_child.source = RelationEndpoint::Owner(source);
    foreign
        .entries
        .reverse_relations
        .sort_unstable_by_key(|edge| (edge.target, edge.kind, edge.source));
    reject(&foreign, "change_delete_ownership_package");

    let mut relation_disagreement = created.initial.witness.clone();
    relation_disagreement
        .entries
        .reverse_relations
        .push(RelationEdge {
            source: RelationEndpoint::Owner(ExactOwnerKey {
                package: logical.root.package_id,
                owner: root,
            }),
            kind: RelationKind::MemberDeclaration,
            target: RelationEndpoint::Owner(ExactOwnerKey {
                package: logical.root.package_id,
                owner: root,
            }),
        });
    relation_disagreement
        .entries
        .reverse_relations
        .sort_unstable_by_key(|edge| (edge.target, edge.kind, edge.source));
    reject(
        &relation_disagreement,
        "change_delete_relation_disagreement",
    );
}

#[test]
fn authored_owned_closure_selection_work_is_independent_of_unrelated_owner_count() {
    const UNRELATED_MODULES: u64 = 2_000;
    let temporary = tempfile::tempdir().expect("temporary locality repositories");
    let small_snapshot = crate::platform::kernel::tests::witness_snapshot();
    let mut large_snapshot = small_snapshot.clone();
    for ordinal in 0..UNRELATED_MODULES {
        let module = crate::platform::semantic_id::ModuleId::migrate(
            b"owned-closure-unrelated-module",
            ordinal,
        );
        let owner = OwnerKey::Module(module);
        assert!(
            large_snapshot
                .owners
                .insert(
                    owner,
                    OwnerRecord::Module(ModuleRecord {
                        header: OwnerHeader::new(owner, OwnerKind::Module),
                        name: Name::new(format!("unrelated_{ordinal:04}")).unwrap(),
                    }),
                )
                .is_none()
        );
    }
    large_snapshot.root.owners = MapRoot::from_parts(
        PageDigest::from_bytes([201; 32]),
        u64::try_from(large_snapshot.owners.len()).unwrap(),
        crate::platform::persistent_map::MapContentDigest::from_bytes([202; 32]),
    );
    let root = small_snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Module(module) if module.name.as_str() == "second" => Some(*owner),
            _ => None,
        })
        .expect("selected locality root");
    let small = GraphRepository::create(&temporary.path().join("small"), &small_snapshot, None)
        .expect("create small locality repository");
    let large = GraphRepository::create(&temporary.path().join("large"), &large_snapshot, None)
        .expect("create large locality repository");
    let run = |repository: &GraphRepository| {
        let view = repository.view_current().expect("open locality base");
        let request = AuthoredChangeSet {
            base: view.revision(),
            preconditions: Vec::new(),
            budget: ChangeBudget::default(),
            changes: vec![AuthoredChange::DeleteOwner {
                owner: OwnerSelector::Exact { owner: root },
                policy: AuthoredDeletePolicy::OwnedClosure,
            }],
        };
        let lowering =
            lower_authored_changes(&view, &view, &request).expect("lower local ownership closure");
        let prepared = view
            .prepare_authored_change(&request, PublicationOptions::default())
            .expect("prepare local ownership closure");
        (lowering, prepared)
    };
    let (small_lowering, small_prepared) = run(&small.repository);
    let (large_lowering, large_prepared) = run(&large.repository);
    let closure = complete_ownership_descendants(&small.initial.witness, root);
    assert_eq!(
        closure,
        complete_ownership_descendants(&large.initial.witness, root)
    );
    assert_eq!(
        small_lowering.work.ownership_steps,
        large_lowering.work.ownership_steps
    );
    assert_eq!(
        small_lowering.work.relation_edges_read,
        large_lowering.work.relation_edges_read
    );
    assert_eq!(
        small_lowering.work.canonical.point_reads,
        large_lowering.work.canonical.point_reads
    );
    assert_eq!(
        small_lowering.work.canonical.canonical_records_decoded,
        large_lowering.work.canonical.canonical_records_decoded
    );
    assert_eq!(
        small_lowering.work.witness.point_reads,
        large_lowering.work.witness.point_reads
    );
    assert_eq!(
        small_lowering.work.witness.witness_records_decoded,
        large_lowering.work.witness.witness_records_decoded
    );
    let edit_counts = |edits: &[PrimitiveEdit]| {
        edits
            .iter()
            .fold((0_u64, 0_u64), |counts, edit| match edit {
                PrimitiveEdit::DeleteOwner { .. } => (counts.0 + 1, counts.1),
                PrimitiveEdit::InsertRetirement { .. } => (counts.0, counts.1 + 1),
                _ => counts,
            })
    };
    assert_eq!(
        edit_counts(&small_lowering.edits),
        edit_counts(&large_lowering.edits)
    );
    assert_eq!(
        small_prepared.publication.receipt.counts.owners_deleted,
        large_prepared.publication.receipt.counts.owners_deleted
    );
    assert_eq!(
        small_prepared
            .publication
            .receipt
            .counts
            .retirements_changed,
        large_prepared
            .publication
            .receipt
            .counts
            .retirements_changed
    );
    assert_eq!(
        small_prepared.logical_plan.relations.removed.len(),
        large_prepared.logical_plan.relations.removed.len()
    );
    assert_eq!(
        small_prepared.logical_plan.relations.added.len(),
        large_prepared.logical_plan.relations.added.len()
    );
    let (owner_edits, retirement_edits) = edit_counts(&small_lowering.edits);
    let metrics = format!(
        "owned-closure-locality selected-roots=1 closure={} small-owners={} large-owners={} unrelated-owners={UNRELATED_MODULES} ownership-steps={} relation-edges={} canonical-point-reads={} canonical-records={} canonical-pages-small={} canonical-pages-large={} witness-point-reads={} witness-records={} witness-pages-small={} witness-pages-large={} owner-edits={owner_edits} retirement-edits={retirement_edits} removed-relations={} added-relations={} structural-owners={} semantic-owners={} selected-tests={} impact-reasons={}\n",
        closure.len(),
        small_snapshot.owners.len(),
        large_snapshot.owners.len(),
        small_lowering.work.ownership_steps,
        small_lowering.work.relation_edges_read,
        small_lowering.work.canonical.point_reads,
        small_lowering.work.canonical.canonical_records_decoded,
        small_lowering.work.canonical.map_pages_read,
        large_lowering.work.canonical.map_pages_read,
        small_lowering.work.witness.point_reads,
        small_lowering.work.witness.witness_records_decoded,
        small_lowering.work.witness.map_pages_read,
        large_lowering.work.witness.map_pages_read,
        small_prepared.logical_plan.relations.removed.len(),
        small_prepared.logical_plan.relations.added.len(),
        small_prepared.logical_plan.structurally_checked.len(),
        small_prepared.logical_plan.semantically_checked.len(),
        small_prepared.logical_plan.tests.len(),
        small_prepared.logical_plan.reasons.len(),
    );
    eprint!("{metrics}");
    if let Some(evidence_directory) =
        std::env::var_os("LKJSCRIPT_OWNED_CLOSURE_EVIDENCE_DIR").map(std::path::PathBuf::from)
    {
        std::fs::create_dir_all(&evidence_directory)
            .expect("create ownership-closure evidence directory");
        std::fs::write(evidence_directory.join("locality-metrics.txt"), metrics)
            .expect("retain ownership-closure locality metrics");
    }
}

#[test]
#[ignore = "release-scale ownership closure; run explicitly with --ignored --nocapture"]
fn authored_owned_closure_scale_emits_complete_plan_under_default_admission() {
    const DESCENDANTS: u64 = 1_500;

    fn directory_bytes(path: &std::path::Path) -> u64 {
        std::fs::read_dir(path)
            .expect("read scale repository directory")
            .map(|entry| {
                let entry = entry.expect("read scale repository entry");
                let metadata = entry.metadata().expect("read scale entry metadata");
                if metadata.is_dir() {
                    directory_bytes(&entry.path())
                } else if metadata.is_file() {
                    metadata.len()
                } else {
                    0
                }
            })
            .sum()
    }

    let temporary = tempfile::tempdir().expect("temporary scale repository");
    let destination = temporary.path().join("meaning");
    let mut logical = empty_snapshot(b"owned-closure-scale");
    let module = crate::platform::semantic_id::ModuleId::migrate(b"owned-closure-scale", 1);
    let root = OwnerKey::Module(module);
    logical.owners.insert(
        root,
        OwnerRecord::Module(ModuleRecord {
            header: OwnerHeader::new(root, OwnerKind::Module),
            name: Name::new("scale_root").unwrap(),
        }),
    );
    let mut hierarchy = vec![root];
    for ordinal in 0..DESCENDANTS {
        let documentation = DocumentationId::migrate(b"owned-closure-scale-documentation", ordinal);
        let owner = OwnerKey::Documentation(documentation);
        let parent = hierarchy[usize::try_from(ordinal / 8).unwrap()];
        assert!(
            logical
                .owners
                .insert(
                    owner,
                    OwnerRecord::Documentation(DocumentationRecord {
                        header: OwnerHeader::new(owner, OwnerKind::Documentation),
                        owner: parent,
                        class: DocumentationClass::Nonsemantic,
                        content: DocumentContent::Inline(format!("scale record {ordinal:04}")),
                    }),
                )
                .is_none()
        );
        hierarchy.push(owner);
    }
    logical.root.owners = MapRoot::from_parts(
        PageDigest::from_bytes([211; 32]),
        u64::try_from(logical.owners.len()).unwrap(),
        crate::platform::persistent_map::MapContentDigest::from_bytes([212; 32]),
    );
    let bootstrap_started = std::time::Instant::now();
    let created = GraphRepository::create(&destination, &logical, None)
        .expect("create scale ownership repository");
    let bootstrap_elapsed = bootstrap_started.elapsed();
    let repository_before_plan = directory_bytes(&destination);
    let request = AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::DeleteOwner {
            owner: OwnerSelector::Exact { owner: root },
            policy: AuthoredDeletePolicy::OwnedClosure,
        }],
    };
    let normalized = crate::platform::control::normalize_change_request(
        request,
        PublicationOptions {
            idempotency_key: Some("owned-closure-scale".to_owned()),
            intent: Some("release scale evidence".to_owned()),
        },
    )
    .expect("normalize scale closure request");
    let plan_started = std::time::Instant::now();
    let prepared = created
        .repository
        .prepare_authored_change(&normalized.semantic, normalized.options)
        .expect("prepare scale ownership closure");
    let plan =
        crate::platform::control::LogicalChangePlan::new(normalized.request_commitment, &prepared)
            .expect("construct scale ownership plan");
    let mut plan_bytes = Vec::new();
    let encoded = crate::platform::control::encode_logical_change_plan(&plan, |record| {
        plan_bytes.extend_from_slice(record);
        Ok(())
    })
    .expect("encode complete scale ownership plan");
    let plan_elapsed = plan_started.elapsed();
    let decoded =
        crate::platform::control::decode_logical_change_plan(std::io::Cursor::new(&plan_bytes))
            .expect("strictly decode scale ownership plan");
    assert_eq!(decoded.token, encoded.token.to_string());
    assert_eq!(decoded.counts.owners, DESCENDANTS + 1);
    assert_eq!(decoded.counts.retirements, DESCENDANTS + 1);
    assert_eq!(prepared.lowering_work.ownership_steps, DESCENDANTS);
    assert_eq!(
        prepared.publication.budget_work.canonical_edits.owner_edits,
        DESCENDANTS + 1
    );
    assert_eq!(
        prepared
            .publication
            .budget_work
            .canonical_edits
            .retirement_edits,
        DESCENDANTS + 1
    );
    let repository_after_plan = directory_bytes(&destination);
    assert_eq!(repository_after_plan, repository_before_plan);
    let work = prepared.publication.budget_work;
    let revision = prepared.publication.head.revision;
    let plan_digest =
        crate::platform::semantic_id::encode_hex(blake3::hash(&plan_bytes).as_bytes());
    let apply_started = std::time::Instant::now();
    let PublicationOutcome::Accepted { .. } = created
        .repository
        .publish(&prepared.publication)
        .expect("publish scale ownership closure")
    else {
        panic!("scale ownership closure must advance HEAD exactly once")
    };
    let apply_elapsed = apply_started.elapsed();
    let repository_after_apply = directory_bytes(&destination);
    let reopened = GraphRepository::open(&destination)
        .expect("reopen scale ownership repository")
        .view_current()
        .expect("open scale ownership result");
    assert_eq!(reopened.revision(), revision);
    assert_eq!(reopened.current().semantic_root.owners.entries(), 0);
    assert_eq!(
        reopened.current().semantic_root.retirements.entries(),
        DESCENDANTS + 1
    );
    let peak_rss_kib = std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix("VmHWM:")
                    .and_then(|value| value.split_whitespace().next())
                    .and_then(|value| value.parse::<u64>().ok())
            })
        });
    let metrics = format!(
        "owned-closure-scale cache=cold topology=module-plus-8ary-inline-documentation-tree descendants={DESCENDANTS} closure={} revision={revision} bootstrap-wall-us={} plan-wall-us={} apply-wall-us={} cpu-time=external peak-rss-kib={} plan-records={} plan-bytes={} plan-blake3={} canonical-point-reads={} canonical-pages-read={} canonical-records={} witness-point-reads={} witness-pages-read={} witness-records={} ownership-steps={} relation-edges={} owner-edits={} retirement-edits={} validation-owners={} selected-tests={} staged-objects={} staged-pages={} staged-bytes={} repository-before-plan-bytes={} repository-after-plan-bytes={} repository-after-apply-bytes={}\n",
        DESCENDANTS + 1,
        bootstrap_elapsed.as_micros(),
        plan_elapsed.as_micros(),
        apply_elapsed.as_micros(),
        peak_rss_kib.map_or_else(|| "unavailable".to_owned(), |value| value.to_string()),
        decoded.records,
        decoded.bytes,
        plan_digest,
        work.canonical_reads.point_reads,
        work.canonical_reads.map_pages_read,
        work.canonical_reads.canonical_records_decoded,
        work.witness_reads.point_reads,
        work.witness_reads.map_pages_read,
        work.witness_reads.witness_records_decoded,
        prepared.lowering_work.ownership_steps,
        work.relation_edges,
        work.canonical_edits.owner_edits,
        work.canonical_edits.retirement_edits,
        work.validation.owner_records,
        work.tests.selected,
        work.staging.objects,
        work.staging.pages,
        work.staging.bytes,
        repository_before_plan,
        repository_after_plan,
        repository_after_apply,
    );
    eprint!("{metrics}");
    if let Some(evidence_directory) =
        std::env::var_os("LKJSCRIPT_OWNED_CLOSURE_EVIDENCE_DIR").map(std::path::PathBuf::from)
    {
        std::fs::create_dir_all(&evidence_directory).expect("create scale evidence directory");
        std::fs::write(evidence_directory.join("scale.logical-plan"), &plan_bytes)
            .expect("retain scale logical plan");
        std::fs::write(evidence_directory.join("scale-metrics.txt"), metrics)
            .expect("retain scale metrics");
    }
}

#[test]
fn authored_deletion_rejects_untouched_live_references_and_created_owner_erasure() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let base = created.current.head.revision;
    let field = field_named(&created.initial.snapshot, "value");
    let live_reference = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::DeleteOwner {
            owner: OwnerSelector::Exact { owner: field },
            policy: AuthoredDeletePolicy::Reject,
        }],
    };
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&live_reference, PublicationOptions::default())
            .expect_err("untouched nominal field uses must block deletion")[0]
            .code,
        "change_delete_live_reference"
    );

    let created_then_deleted = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::CreateModule {
                symbol: "$temporary_module".to_owned(),
                name: Name::new("temporary_module").unwrap(),
            },
            AuthoredChange::DeleteOwner {
                owner: OwnerSelector::Symbol {
                    symbol: "$temporary_module".to_owned(),
                },
                policy: AuthoredDeletePolicy::Reject,
            },
        ],
    };
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&created_then_deleted, PublicationOptions::default())
            .expect_err("creation and deletion must not hide an authored no-change")[0]
            .code,
        "change_delete_created_owner"
    );
    assert_eq!(created.repository.current().unwrap().head.revision, base);
}

#[test]
fn authored_leaf_deletion_accepts_same_request_reference_repair() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let record = owner_named(&created.initial.snapshot, "Payload");
    let OwnerKey::Declaration(record_id) = record else {
        panic!("Payload must have declaration identity")
    };
    let caller = owner_named(&created.initial.snapshot, "caller");
    let OwnerKey::Declaration(caller_id) = caller else {
        panic!("caller must have declaration identity")
    };
    let removed = field_named(&created.initial.snapshot, "value");
    let previous_body = function_body(&created.initial.snapshot, "caller");
    let request = AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::AddField {
                record: DeclarationSelector::Id {
                    declaration: record_id,
                },
                field: AuthoredField {
                    symbol: "$replacement_field".to_owned(),
                    name: Name::new("replacement").unwrap(),
                    ty: AuthoredType::Unit {},
                },
            },
            AuthoredChange::ReplaceFunctionBody {
                function: DeclarationSelector::Id {
                    declaration: caller_id,
                },
                body: AuthoredExpression {
                    symbol: Some("$replacement_body".to_owned()),
                    operation: AuthoredExpressionOperation::Unit {},
                },
            },
            AuthoredChange::DeleteOwner {
                owner: OwnerSelector::Exact { owner: removed },
                policy: AuthoredDeletePolicy::Reject,
            },
        ],
    };
    let prepared = created
        .repository
        .prepare_authored_change(&request, PublicationOptions::default())
        .expect("same-request body repair must release the exact leaf reference");
    assert_eq!(prepared.publication.receipt.counts.owners_updated, 2);
    assert!(prepared.publication.receipt.counts.owners_deleted > 1);
    let replacement_field = prepared.allocated["$replacement_field"];
    let replacement_body = prepared.allocated["$replacement_body"];
    created
        .repository
        .publish(&prepared.publication)
        .expect("publish repaired leaf deletion");

    let view = created.repository.view_current().expect("advanced view");
    assert!(view.owner(removed).unwrap().value.is_none());
    assert!(view.owner(previous_body).unwrap().value.is_none());
    let Some(OwnerRecord::Declaration(record)) =
        view.owner(OwnerKey::Declaration(record_id)).unwrap().value
    else {
        panic!("repaired record must remain live")
    };
    let DeclarationPayload::Record { fields } = record.payload else {
        panic!("Payload must remain a record")
    };
    assert_eq!(
        fields,
        vec![match replacement_field {
            OwnerKey::Field(field) => field,
            _ => panic!("replacement field must have field identity"),
        }]
    );
    let Some(OwnerRecord::Declaration(caller)) =
        view.owner(OwnerKey::Declaration(caller_id)).unwrap().value
    else {
        panic!("repaired caller must remain live")
    };
    let DeclarationPayload::Function(caller) = caller.payload else {
        panic!("caller must remain a function")
    };
    assert_eq!(OwnerKey::Expression(caller.body), replacement_body);
    let Some(OwnerRecord::Expression(body)) = view.owner(replacement_body).unwrap().value else {
        panic!("replacement body must remain live")
    };
    assert!(matches!(body.operation, ExpressionOperation::Unit {}));
}

#[test]
fn reviewed_change_plan_owned_body_closure_matches_complete_before_after_oracle() {
    let temporary = tempfile::tempdir().expect("temporary reviewed-plan repository");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let function = owner_named(&created.initial.snapshot, "with_binding");
    let OwnerKey::Declaration(function) = function else {
        panic!("with_binding must be a function declaration")
    };
    let request = AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::ReplaceFunctionBody {
            function: DeclarationSelector::Id {
                declaration: function,
            },
            body: AuthoredExpression {
                symbol: Some("$replacement_body".to_owned()),
                operation: AuthoredExpressionOperation::Unit {},
            },
        }],
    };
    let normalized = crate::platform::control::normalize_change_request(
        request.clone(),
        PublicationOptions::default(),
    )
    .expect("normalize reviewed request");

    // This broad before/after reconstruction is deliberately disjoint from the logical-plan
    // projection and remains an oracle-only path.
    let pinned = created
        .repository
        .view_current()
        .expect("open exact oracle base");
    let lowering = lower_authored_changes(&pinned, &pinned, &request)
        .expect("lower reviewed request for oracle");
    let canonical = CanonicalDelta::normalize(&created.initial.snapshot, lowering.edits)
        .expect("normalize reviewed oracle delta");
    let overlay = KernelOverlay::new(&created.initial.snapshot, &canonical);
    let candidate = overlay.materialize_logical_oracle();
    let before_relations = extract_relations(
        created.initial.snapshot.root.package_id,
        &created.initial.snapshot.owners,
        &created.initial.snapshot.types,
        &created.initial.snapshot.dependencies,
    )
    .expect("extract complete before relations")
    .into_iter()
    .collect::<BTreeSet<_>>();
    let after_relations = extract_relations(
        candidate.root.package_id,
        &candidate.owners,
        &candidate.types,
        &candidate.dependencies,
    )
    .expect("extract complete after relations")
    .into_iter()
    .collect::<BTreeSet<_>>();
    let removed_relations = before_relations
        .difference(&after_relations)
        .copied()
        .collect::<BTreeSet<_>>();
    let added_relations = after_relations
        .difference(&before_relations)
        .copied()
        .collect::<BTreeSet<_>>();
    let removed_owners = created
        .initial
        .snapshot
        .owners
        .keys()
        .filter(|owner| !candidate.owners.contains_key(owner))
        .copied()
        .collect::<BTreeSet<_>>();
    assert!(
        removed_owners
            .iter()
            .any(|owner| matches!(owner, OwnerKey::Binding(_)))
    );
    assert!(
        removed_owners
            .iter()
            .any(|owner| matches!(owner, OwnerKey::Expression(_)))
    );

    let oracle_analysis = prepare_change_analysis(
        &created.initial.snapshot,
        &created.initial.witness,
        canonical,
    )
    .expect("complete in-memory impact and validation oracle");
    let prepared = created
        .repository
        .prepare_authored_change(&normalized.semantic, normalized.options)
        .expect("prepare reviewed logical plan");
    assert_eq!(prepared.logical_plan.relations.removed, removed_relations);
    assert_eq!(prepared.logical_plan.relations.added, added_relations);
    assert_eq!(
        prepared.logical_plan.structurally_checked,
        oracle_analysis.validation.structurally_checked
    );
    assert_eq!(
        prepared.logical_plan.semantically_checked,
        oracle_analysis.validation.semantically_checked
    );
    assert_eq!(
        prepared.logical_plan.tests,
        oracle_analysis.summaries.plan.tests
    );
    assert_eq!(
        prepared.logical_plan.reasons,
        oracle_analysis.summaries.plan.reasons
    );
    let SemanticDiffBody::Change { owners, .. } = &prepared.publication.semantic_diff.body else {
        panic!("authored preparation must produce a change diff")
    };
    let exported_deletions = owners
        .iter()
        .filter(|entry| entry.objects.after.is_none())
        .map(|entry| entry.owner)
        .collect::<BTreeSet<_>>();
    assert_eq!(exported_deletions, removed_owners);
    assert_eq!(
        prepared
            .logical_plan
            .retirements
            .keys()
            .copied()
            .collect::<BTreeSet<_>>(),
        removed_owners
    );

    let plan =
        crate::platform::control::LogicalChangePlan::new(normalized.request_commitment, &prepared)
            .expect("construct reviewed logical plan");
    let mut bytes = Vec::new();
    let encoded = crate::platform::control::encode_logical_change_plan(&plan, |record| {
        bytes.extend_from_slice(record);
        Ok(())
    })
    .expect("encode reviewed logical plan");
    let decoded =
        crate::platform::control::decode_logical_change_plan(std::io::Cursor::new(&bytes))
            .expect("strictly decode reviewed logical plan");
    assert_eq!(decoded.token, encoded.token.to_string());
    assert_eq!(decoded.bytes, u64::try_from(bytes.len()).unwrap());

    let canonical = String::from_utf8(bytes.clone()).expect("logical plan UTF-8");
    let mut truncated = bytes.clone();
    assert_eq!(truncated.pop(), Some(b'\n'));
    assert_eq!(
        logical_plan_decode_error(&truncated),
        "change_plan_file_truncated"
    );

    let changed_body = canonical.replacen("intent-present=false", "intent-present=true", 1);
    assert_ne!(changed_body, canonical);
    assert_eq!(
        logical_plan_decode_error(changed_body.as_bytes()),
        "change_plan_file_digest"
    );

    let first_line_end = canonical.find('\n').expect("first logical plan record") + 1;
    let duplicate_singleton = format!("{}{}", &canonical[..first_line_end], canonical);
    assert_eq!(
        logical_plan_decode_error(duplicate_singleton.as_bytes()),
        "change_plan_file_singleton_duplicate"
    );
    let unknown = canonical.replacen("logical-plan ", "logical-plan.unknown ", 1);
    assert_eq!(
        logical_plan_decode_error(unknown.as_bytes()),
        "change_plan_file_record_unknown"
    );

    let mut lines = canonical.lines().map(str::to_owned).collect::<Vec<_>>();
    let owner_indexes = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| line.starts_with("logical-plan.owner ").then_some(index))
        .collect::<Vec<_>>();
    assert!(owner_indexes.len() >= 2);
    lines.swap(owner_indexes[0], owner_indexes[1]);
    let reordered = format!("{}\n", lines.join("\n"));
    assert_eq!(
        logical_plan_decode_error(reordered.as_bytes()),
        "change_plan_file_owner_order"
    );

    let mut lines = canonical.lines().map(str::to_owned).collect::<Vec<_>>();
    let owner_index = lines
        .iter()
        .position(|line| line.starts_with("logical-plan.owner "))
        .expect("logical owner record");
    let duplicate_owner = lines[owner_index].clone();
    lines.insert(owner_index + 1, duplicate_owner);
    let duplicated = format!("{}\n", lines.join("\n"));
    assert_eq!(
        logical_plan_decode_error(duplicated.as_bytes()),
        "change_plan_file_owner_order"
    );

    let mut lines = canonical.lines().map(str::to_owned).collect::<Vec<_>>();
    let counts_index = lines
        .iter()
        .position(|line| line.starts_with("logical-plan.counts "))
        .expect("logical counts record");
    lines.insert(
        counts_index,
        "logical-plan.compiler-unit owner=decl_00000000000000000000000000000001".to_owned(),
    );
    let operational_injection = format!("{}\n", lines.join("\n"));
    assert_eq!(
        logical_plan_decode_error(operational_injection.as_bytes()),
        "change_plan_file_record_unknown"
    );

    let malformed_escape = canonical.replacen("intent=\"\"", "intent=\"", 1);
    assert_eq!(
        logical_plan_decode_error(malformed_escape.as_bytes()),
        "control_quote_unclosed"
    );
    let mut wrong_trailer = canonical.clone().into_bytes();
    let last_hex = wrong_trailer
        .iter()
        .rposition(|byte| byte.is_ascii_hexdigit())
        .expect("digest trailer hexadecimal");
    wrong_trailer[last_hex] = if wrong_trailer[last_hex] == b'0' {
        b'1'
    } else {
        b'0'
    };
    assert_eq!(
        logical_plan_decode_error(&wrong_trailer),
        "change_plan_file_digest"
    );
    let trailing = format!("{canonical}logical-plan contract=trailing version=1\n");
    assert_eq!(
        logical_plan_decode_error(trailing.as_bytes()),
        "change_plan_file_trailing"
    );
}

#[test]
fn authored_member_deletion_detaches_the_exact_parent_and_preserves_siblings() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let create = AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::CreateRecord {
            symbol: "$member_delete_record".to_owned(),
            module: ModuleSelector::Name {
                name: Name::new("second").unwrap(),
            },
            name: Name::new("MemberDeleteRecord").unwrap(),
            visibility: DeclarationVisibility::Private,
            fields: vec![
                AuthoredField {
                    symbol: "$removed_field".to_owned(),
                    name: Name::new("removed").unwrap(),
                    ty: AuthoredType::Unit {},
                },
                AuthoredField {
                    symbol: "$retained_field".to_owned(),
                    name: Name::new("retained").unwrap(),
                    ty: AuthoredType::Unit {},
                },
            ],
        }],
    };
    let prepared_create = created
        .repository
        .prepare_authored_change(&create, PublicationOptions::default())
        .expect("prepare record with independent fields");
    let declaration = prepared_create.allocated["$member_delete_record"];
    let removed = prepared_create.allocated["$removed_field"];
    let retained = prepared_create.allocated["$retained_field"];
    created
        .repository
        .publish(&prepared_create.publication)
        .expect("publish record fixture");

    let delete = AuthoredChangeSet {
        base: created.repository.current().unwrap().head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::DeleteOwner {
            owner: OwnerSelector::Exact { owner: removed },
            policy: AuthoredDeletePolicy::Reject,
        }],
    };
    let prepared_delete = created
        .repository
        .prepare_authored_change(&delete, PublicationOptions::default())
        .expect("member deletion must detach its exact canonical parent");
    assert_eq!(prepared_delete.publication.receipt.counts.owners_deleted, 1);
    assert_eq!(prepared_delete.publication.receipt.counts.owners_updated, 1);
    created
        .repository
        .publish(&prepared_delete.publication)
        .expect("publish member deletion");

    let view = created.repository.view_current().expect("advanced view");
    let Some(OwnerRecord::Declaration(record)) = view.owner(declaration).unwrap().value else {
        panic!("record declaration must remain live")
    };
    let DeclarationPayload::Record { fields } = record.payload else {
        panic!("created declaration must remain a record")
    };
    let OwnerKey::Field(retained) = retained else {
        panic!("retained field symbol must have field identity")
    };
    assert_eq!(fields, vec![retained]);
    assert!(view.owner(removed).unwrap().value.is_none());
    assert!(view.retirement(removed).unwrap().value.is_some());
    assert!(
        view.owner(OwnerKey::Field(retained))
            .unwrap()
            .value
            .is_some()
    );
}

#[test]
fn authored_preconditions_are_exact_base_point_reads_and_publish_once() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let callee = owner_named(&created.initial.snapshot, "callee");
    let callee_record = created.initial.snapshot.owners[&callee].clone();
    let OwnerRecord::Declaration(callee_declaration) = &callee_record else {
        panic!("callee must be a declaration")
    };
    let module = OwnerKey::Module(callee_declaration.module);
    let absent = OwnerKey::Declaration(DeclarationId::migrate(b"precondition-absent", 1));
    let base = created.current.head.revision;
    let request = AuthoredChangeSet {
        base,
        preconditions: vec![
            AuthoredPrecondition::OwnerExists { owner: callee },
            AuthoredPrecondition::OwnerAbsent { owner: absent },
            AuthoredPrecondition::OwnerName {
                owner: callee,
                equals: Name::new("callee").unwrap(),
            },
            AuthoredPrecondition::OwnerParent {
                owner: callee,
                equals: AuthoredOwnerParent::Owner(module),
            },
            AuthoredPrecondition::NamespacePointsTo {
                parent: Some(module),
                class: NamespaceClass::Declaration,
                name: Name::new("callee").unwrap(),
                owner: callee,
            },
            AuthoredPrecondition::NamespaceAbsent {
                parent: Some(module),
                class: NamespaceClass::Declaration,
                name: Name::new("guarded_callee").unwrap(),
            },
        ],
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::RenameOwner {
            owner: OwnerSelector::Exact { owner: callee },
            name: Name::new("guarded_callee").unwrap(),
        }],
    };

    let prepared = created
        .repository
        .prepare_authored_change(&request, PublicationOptions::default())
        .expect("prepare guarded authored change");
    assert_eq!(prepared.lowering_work.operations_lowered, 1);
    assert_eq!(prepared.lowering_work.preconditions_checked, 6);
    assert_eq!(prepared.publication.budget_work.preconditions_checked, 6);
    request
        .budget
        .check_observed(
            prepared.publication.budget_work,
            "accepted preparation test",
        )
        .expect("accepted preparation must retain every independent budget");
    assert_eq!(
        prepared.publication.expected_base,
        Some(created.current.head)
    );

    let accepted = created
        .repository
        .publish(&prepared.publication)
        .expect("publish guarded authored change");
    let PublicationOutcome::Accepted { current, .. } = accepted else {
        panic!("guarded authored change must publish once")
    };
    assert_ne!(current.head.revision, base);
    let reopened = created
        .repository
        .view_current()
        .expect("reopen new revision");
    let renamed = reopened
        .owner(callee)
        .expect("renamed owner read")
        .value
        .expect("renamed owner");
    assert!(matches!(
        renamed,
        OwnerRecord::Declaration(ref declaration)
            if declaration.name.as_str() == "guarded_callee"
    ));
}

#[test]
fn failed_authored_preconditions_publish_nothing_with_stable_codes() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let base = created.current.head.revision;
    let callee = owner_named(&created.initial.snapshot, "callee");
    let body = function_body(&created.initial.snapshot, "callee");
    let callee_record = &created.initial.snapshot.owners[&callee];
    let OwnerRecord::Declaration(declaration) = callee_record else {
        panic!("callee must be a declaration")
    };
    let module = OwnerKey::Module(declaration.module);
    let absent = OwnerKey::Declaration(DeclarationId::migrate(b"failed-precondition", 1));
    let foreign_package = PackageId::migrate(b"failed-precondition-package", 1);
    let failures = vec![
        (
            "change_precondition_owner_missing",
            AuthoredPrecondition::OwnerExists { owner: absent },
        ),
        (
            "change_precondition_owner_present",
            AuthoredPrecondition::OwnerAbsent { owner: callee },
        ),
        (
            "change_precondition_owner_name",
            AuthoredPrecondition::OwnerName {
                owner: callee,
                equals: Name::new("wrong").unwrap(),
            },
        ),
        (
            "change_precondition_owner_parent",
            AuthoredPrecondition::OwnerParent {
                owner: callee,
                equals: AuthoredOwnerParent::Package,
            },
        ),
        (
            "change_precondition_namespace_present",
            AuthoredPrecondition::NamespaceAbsent {
                parent: Some(module),
                class: NamespaceClass::Declaration,
                name: Name::new("callee").unwrap(),
            },
        ),
        (
            "change_precondition_namespace_owner",
            AuthoredPrecondition::NamespacePointsTo {
                parent: Some(module),
                class: NamespaceClass::Declaration,
                name: Name::new("callee").unwrap(),
                owner: body,
            },
        ),
        (
            "change_precondition_dependency_binding",
            AuthoredPrecondition::DependencyBinding {
                package: foreign_package,
                semantic_revision: RevisionId::from_digest([0x22; 32]),
                package_revision: PackageRevisionDigest::from_bytes([0x33; 32]),
            },
        ),
    ];

    for (expected_code, precondition) in failures {
        let request = AuthoredChangeSet {
            base,
            preconditions: vec![precondition],
            budget: ChangeBudget::default(),
            changes: vec![AuthoredChange::RenameOwner {
                owner: OwnerSelector::Exact { owner: callee },
                name: Name::new("never_published").unwrap(),
            }],
        };
        let diagnostics = created
            .repository
            .prepare_authored_change(&request, PublicationOptions::default())
            .expect_err("failed precondition must reject preparation");
        assert_eq!(diagnostics[0].code, expected_code);
        assert_eq!(created.repository.current().unwrap().head.revision, base);
    }
}

#[test]
fn semantic_preconditions_reject_derived_entries_that_disagree_with_meaning() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let base = created
        .repository
        .view_current()
        .expect("exact accepted base");
    let callee = owner_named(&created.initial.snapshot, "callee");
    let body = function_body(&created.initial.snapshot, "callee");
    let OwnerRecord::Declaration(declaration) = &created.initial.snapshot.owners[&callee] else {
        panic!("callee must be a declaration")
    };
    let key = NamespaceKey {
        parent: Some(OwnerKey::Module(declaration.module)),
        class: NamespaceClass::Declaration,
        name: Name::new("callee").unwrap(),
    };
    let mut corrupt_witness = created.initial.witness.clone();
    corrupt_witness.entries.namespaces.insert(key.clone(), body);
    let request = AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: vec![AuthoredPrecondition::NamespacePointsTo {
            parent: key.parent,
            class: key.class,
            name: key.name,
            owner: body,
        }],
        changes: vec![AuthoredChange::RenameOwner {
            owner: OwnerSelector::Exact { owner: callee },
            name: Name::new("never_published").unwrap(),
        }],
        budget: ChangeBudget::default(),
    };
    let diagnostic = lower_authored_changes(&base, &corrupt_witness, &request)
        .expect_err("derived namespace data cannot satisfy false semantic caller intent");
    assert_eq!(diagnostic.class, DiagnosticClass::Corrupt);
    assert_eq!(diagnostic.code, "change_precondition_namespace_witness");
    assert_eq!(
        created.repository.current().unwrap().head.revision,
        request.base
    );

    let mut corrupt_ownership = created.initial.witness.clone();
    let ownership = corrupt_ownership
        .entries
        .ownership
        .get_mut(&callee)
        .expect("callee ownership");
    ownership.parent = OwnershipParent::Package;
    let request = AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: vec![AuthoredPrecondition::OwnerParent {
            owner: callee,
            equals: AuthoredOwnerParent::Package,
        }],
        changes: vec![AuthoredChange::RenameOwner {
            owner: OwnerSelector::Exact { owner: callee },
            name: Name::new("also_never_published").unwrap(),
        }],
        budget: ChangeBudget::default(),
    };
    let diagnostic = lower_authored_changes(&base, &corrupt_ownership, &request)
        .expect_err("derived ownership data cannot satisfy false semantic caller intent");
    assert_eq!(diagnostic.class, DiagnosticClass::Semantic);
    assert_eq!(diagnostic.code, "change_precondition_owner_parent");
    assert_eq!(
        created.repository.current().unwrap().head.revision,
        request.base
    );
}

#[test]
fn authored_budget_rejects_invalid_and_exhausted_work_before_publication() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let base = created.current.head.revision;
    let callee = owner_named(&created.initial.snapshot, "callee");
    let rename = || AuthoredChange::RenameOwner {
        owner: OwnerSelector::Exact { owner: callee },
        name: Name::new("budgeted_callee").unwrap(),
    };

    let mut invalid_budget = ChangeBudget::default();
    invalid_budget.authored.maximum_operations = 0;
    let invalid = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: invalid_budget,
        changes: vec![rename()],
    };
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&invalid, PublicationOptions::default())
            .expect_err("zero operation budget must reject")[0]
            .code,
        "change_budget_invalid_operations"
    );

    let mut operation_budget = ChangeBudget::default();
    operation_budget.authored.maximum_operations = 1;
    let too_many_operations = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: operation_budget,
        changes: vec![rename(), rename()],
    };
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&too_many_operations, PublicationOptions::default())
            .expect_err("operation budget must reject")[0]
            .code,
        "change_budget_operations"
    );

    let too_many_preconditions = AuthoredChangeSet {
        base,
        preconditions: vec![
            AuthoredPrecondition::OwnerExists { owner: callee },
            AuthoredPrecondition::OwnerExists { owner: callee },
        ],
        budget: {
            let mut budget = operation_budget;
            budget.authored.maximum_preconditions = 1;
            budget
        },
        changes: vec![rename()],
    };
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&too_many_preconditions, PublicationOptions::default())
            .expect_err("precondition budget must reject")[0]
            .code,
        "change_budget_preconditions"
    );

    let mut work_budget = ChangeBudget::default();
    work_budget.canonical_reads.maximum_point_reads = 0;
    let exhausted_work = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: work_budget,
        changes: vec![rename()],
    };
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&exhausted_work, PublicationOptions::default())
            .expect_err("canonical point-read budget must reject")[0]
            .code,
        "change_budget_canonical_point_reads"
    );

    let mut affected_budget = ChangeBudget::default();
    affected_budget.authored.maximum_operations = 2;
    affected_budget.impact.maximum_affected_owners = 1;
    let exhausted_owners = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: affected_budget,
        changes: vec![
            AuthoredChange::CreateModule {
                symbol: "$budget_one".to_owned(),
                name: Name::new("budget_one").unwrap(),
            },
            AuthoredChange::CreateModule {
                symbol: "$budget_two".to_owned(),
                name: Name::new("budget_two").unwrap(),
            },
        ],
    };
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&exhausted_owners, PublicationOptions::default())
            .expect_err("affected-owner budget must reject")[0]
            .code,
        "change_budget_affected_frontier_owners"
    );

    let mut relation_budget = ChangeBudget::default();
    relation_budget.impact.maximum_relation_edges = 1;
    let caller = owner_named(&created.initial.snapshot, "caller");
    let OwnerKey::Declaration(caller) = caller else {
        panic!("caller must be a function declaration")
    };
    let exhausted_relations = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: relation_budget,
        changes: vec![AuthoredChange::ReplaceFunctionBody {
            function: DeclarationSelector::Id {
                declaration: caller,
            },
            body: authored_expression(AuthoredExpressionOperation::Unit {}),
        }],
    };
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&exhausted_relations, PublicationOptions::default())
            .expect_err("relation-edge budget must reject")[0]
            .code,
        "change_budget_relation_edges"
    );
    assert_eq!(created.repository.current().unwrap().head.revision, base);
}

#[test]
fn authored_budget_dimensions_exhaust_independently_without_advancing_head() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let base = created.current.head.revision;
    let callee = owner_named(&created.initial.snapshot, "callee");
    let rename = AuthoredChange::RenameOwner {
        owner: OwnerSelector::Exact { owner: callee },
        name: Name::new("dimensioned_callee").unwrap(),
    };
    let default_request = |changes: Vec<AuthoredChange>, budget: ChangeBudget| AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget,
        changes,
    };
    let baseline = created
        .repository
        .prepare_authored_change(
            &default_request(vec![rename.clone()], ChangeBudget::default()),
            PublicationOptions::default(),
        )
        .expect("baseline rename preparation");
    let work = baseline.publication.budget_work;
    let reject = |budget: ChangeBudget, changes: Vec<AuthoredChange>, expected: &str| {
        let diagnostics = created
            .repository
            .prepare_authored_change(
                &default_request(changes, budget),
                PublicationOptions::default(),
            )
            .expect_err("exhausted independent dimension must reject preparation");
        assert_eq!(diagnostics[0].code, expected);
        assert_eq!(created.repository.current().unwrap().head.revision, base);
    };

    type SetLimit = fn(&mut ChangeBudget, u64);
    let rename_dimensions: &[(SetLimit, u64, &str)] = &[
        (
            |budget, value| budget.canonical_edits.maximum_owner_edits = value,
            work.canonical_edits.owner_edits,
            "change_budget_canonical_owner_edits",
        ),
        (
            |budget, value| budget.canonical_reads.maximum_point_reads = value,
            work.canonical_reads.point_reads,
            "change_budget_canonical_point_reads",
        ),
        (
            |budget, value| budget.canonical_reads.maximum_map_pages = value,
            work.canonical_reads.map_pages_read,
            "change_budget_canonical_map_pages",
        ),
        (
            |budget, value| budget.canonical_reads.maximum_map_entries = value,
            work.canonical_reads.map_entries_visited,
            "change_budget_canonical_map_entries",
        ),
        (
            |budget, value| budget.canonical_reads.maximum_catalog_lookups = value,
            work.canonical_reads.catalog_lookups,
            "change_budget_canonical_catalog_lookups",
        ),
        (
            |budget, value| budget.canonical_reads.maximum_objects = value,
            work.canonical_reads.objects_read,
            "change_budget_canonical_objects",
        ),
        (
            |budget, value| budget.canonical_reads.maximum_bytes = value,
            work.canonical_reads.bytes_read,
            "change_budget_canonical_bytes",
        ),
        (
            |budget, value| budget.canonical_reads.maximum_decoded_records = value,
            work.canonical_reads.canonical_records_decoded,
            "change_budget_canonical_decoded_records",
        ),
        (
            |budget, value| budget.canonical_map_update.maximum_pages_encoded = value,
            work.canonical_map_update.pages_encoded,
            "change_budget_canonical_map_pages_encoded",
        ),
        (
            |budget, value| budget.canonical_map_update.maximum_bytes_encoded = value,
            work.canonical_map_update.bytes_encoded,
            "change_budget_canonical_map_bytes_encoded",
        ),
        (
            |budget, value| budget.witness_reads.maximum_point_reads = value,
            work.witness_reads.point_reads,
            "change_budget_witness_point_reads",
        ),
        (
            |budget, value| budget.witness_reads.maximum_map_pages = value,
            work.witness_reads.map_pages_read,
            "change_budget_witness_map_pages",
        ),
        (
            |budget, value| budget.witness_reads.maximum_map_entries = value,
            work.witness_reads.map_entries_visited,
            "change_budget_witness_map_entries",
        ),
        (
            |budget, value| budget.witness_reads.maximum_catalog_lookups = value,
            work.witness_reads.catalog_lookups,
            "change_budget_witness_catalog_lookups",
        ),
        (
            |budget, value| budget.witness_reads.maximum_objects = value,
            work.witness_reads.objects_read,
            "change_budget_witness_objects",
        ),
        (
            |budget, value| budget.witness_reads.maximum_bytes = value,
            work.witness_reads.bytes_read,
            "change_budget_witness_bytes",
        ),
        (
            |budget, value| budget.witness_reads.maximum_decoded_records = value,
            work.witness_reads.witness_records_decoded,
            "change_budget_witness_decoded_records",
        ),
        (
            |budget, value| budget.impact.maximum_affected_owners = value,
            work.affected_frontier_owners,
            "change_budget_affected_frontier_owners",
        ),
        (
            |budget, value| budget.impact.maximum_summary_owners = value,
            work.impact_summary_owners,
            "change_budget_impact_summary_owners",
        ),
        (
            |budget, value| budget.impact.maximum_summary_edits = value,
            work.impact_summary_edits,
            "change_budget_impact_summary_edits",
        ),
        (
            |budget, value| budget.impact.maximum_ownership_steps = value,
            work.impact_ownership_steps,
            "change_budget_impact_ownership_steps",
        ),
        (
            |budget, value| budget.impact.maximum_relation_edges = value,
            work.relation_edges,
            "change_budget_relation_edges",
        ),
        (
            |budget, value| budget.validation.maximum_owner_records = value,
            work.validation.owner_records,
            "change_budget_validation_owner_records",
        ),
        (
            |budget, value| budget.validation.maximum_ownership_entries = value,
            work.validation.ownership_entries,
            "change_budget_validation_ownership_entries",
        ),
        (
            |budget, value| budget.validation.maximum_type_objects = value,
            work.validation.type_objects,
            "change_budget_validation_type_objects",
        ),
        (
            |budget, value| budget.tests.maximum_ownership_steps = value,
            work.tests.ownership_steps,
            "change_budget_test_ownership_steps",
        ),
        (
            |budget, value| budget.witness_update.maximum_edits = value,
            work.witness_update.edits,
            "change_budget_witness_edits",
        ),
        (
            |budget, value| budget.witness_update.maximum_pages_encoded = value,
            work.witness_update.pages_encoded,
            "change_budget_witness_map_pages_encoded",
        ),
        (
            |budget, value| budget.witness_update.maximum_bytes_encoded = value,
            work.witness_update.bytes_encoded,
            "change_budget_witness_map_bytes_encoded",
        ),
        (
            |budget, value| budget.staging.maximum_objects = value,
            work.staging.objects,
            "change_budget_staged_objects",
        ),
        (
            |budget, value| budget.staging.maximum_bytes = value,
            work.staging.bytes,
            "change_budget_staged_bytes",
        ),
        (
            |budget, value| budget.staging.maximum_pages = value,
            work.staging.pages,
            "change_budget_staged_pages",
        ),
    ];
    for (set_limit, observed, expected) in rename_dimensions {
        assert!(*observed > 0, "baseline must exercise {expected}");
        let mut budget = ChangeBudget::default();
        set_limit(&mut budget, observed - 1);
        reject(budget, vec![rename.clone()], expected);
    }

    let mut identity_budget = ChangeBudget::default();
    identity_budget.authored.maximum_allocated_identities = 0;
    reject(
        identity_budget,
        vec![AuthoredChange::CreateModule {
            symbol: "$identity_budget".to_owned(),
            name: Name::new("identity_budget").unwrap(),
        }],
        "change_budget_allocated_identities",
    );

    let mut anonymous_identity_budget = ChangeBudget::default();
    anonymous_identity_budget
        .authored
        .maximum_allocated_identities = 1;
    reject(
        anonymous_identity_budget,
        vec![AuthoredChange::CreateFunction {
            symbol: "$anonymous_identity_budget".to_owned(),
            module: ModuleSelector::Name {
                name: Name::new("unreached_module").unwrap(),
            },
            name: Name::new("anonymous_identity_budget").unwrap(),
            visibility: DeclarationVisibility::Private,
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            result: AuthoredType::Unit {},
            effect: AuthoredFunctionEffect::Pure {},
            body: AuthoredExpression {
                symbol: None,
                operation: AuthoredExpressionOperation::Unit {},
            },
        }],
        "change_budget_allocated_identities",
    );

    let create_with_unit = AuthoredChange::CreateFunction {
        symbol: "$existing_type_budget".to_owned(),
        module: ModuleSelector::Name {
            name: Name::new("first").unwrap(),
        },
        name: Name::new("existing_type_budget").unwrap(),
        visibility: DeclarationVisibility::Private,
        type_parameters: Vec::new(),
        parameters: Vec::new(),
        result: AuthoredType::Unit {},
        effect: AuthoredFunctionEffect::Pure {},
        body: AuthoredExpression {
            symbol: None,
            operation: AuthoredExpressionOperation::Unit {},
        },
    };
    let mut existing_type_budget = ChangeBudget::default();
    existing_type_budget.canonical_edits.maximum_type_edits = 0;
    let existing_type = created
        .repository
        .prepare_authored_change(
            &default_request(vec![create_with_unit], existing_type_budget),
            PublicationOptions::default(),
        )
        .expect("accepted unit type must not consume a canonical type-edit admission");
    assert_eq!(
        existing_type
            .publication
            .budget_work
            .canonical_edits
            .type_edits,
        0
    );
    assert_eq!(existing_type.publication.budget_work.authored_type_nodes, 1);

    let mut authored_type_budget = ChangeBudget::default();
    authored_type_budget.authored.maximum_type_nodes = 0;
    reject(
        authored_type_budget,
        vec![AuthoredChange::CreateFunction {
            symbol: "$type_node_budget".to_owned(),
            module: ModuleSelector::Name {
                name: Name::new("first").unwrap(),
            },
            name: Name::new("type_node_budget").unwrap(),
            visibility: DeclarationVisibility::Private,
            type_parameters: Vec::new(),
            parameters: Vec::new(),
            result: AuthoredType::Unit {},
            effect: AuthoredFunctionEffect::Pure {},
            body: AuthoredExpression {
                symbol: None,
                operation: AuthoredExpressionOperation::Unit {},
            },
        }],
        "change_budget_authored_type_nodes",
    );

    let binding_function = owner_named(&created.initial.snapshot, "with_binding");
    let OwnerKey::Declaration(binding_function) = binding_function else {
        panic!("with_binding must be a function declaration")
    };
    let replace = AuthoredChange::ReplaceFunctionBody {
        function: DeclarationSelector::Id {
            declaration: binding_function,
        },
        body: authored_expression(AuthoredExpressionOperation::Unit {}),
    };
    let expression_baseline = created
        .repository
        .prepare_authored_change(
            &default_request(vec![replace.clone()], ChangeBudget::default()),
            PublicationOptions::default(),
        )
        .expect("baseline expression preparation");
    let expression_work = expression_baseline.publication.budget_work;
    let expression_dimensions: &[(SetLimit, u64, &str)] = &[
        (
            |budget, value| budget.validation.maximum_expression_steps = value,
            expression_work.validation.expression_steps,
            "change_budget_validation_expression_steps",
        ),
        (
            |budget, value| budget.tests.maximum_selected = value,
            expression_work.tests.selected,
            "change_budget_tests_selected",
        ),
    ];
    for (set_limit, observed, expected) in expression_dimensions {
        assert!(*observed > 0, "baseline must exercise {expected}");
        let mut budget = ChangeBudget::default();
        set_limit(&mut budget, observed - 1);
        reject(budget, vec![replace.clone()], expected);
    }
}

#[test]
fn authored_request_creates_a_typed_function_and_test_from_forward_references() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let request = AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::CreateTest {
                symbol: "$identity_test".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$authored".to_owned(),
                },
                name: Name::new("identity_test").unwrap(),
                visibility: DeclarationVisibility::Private,
                actual: AuthoredExpression {
                    symbol: Some("$test_actual".to_owned()),
                    operation: AuthoredExpressionOperation::Call {
                        function: AuthoredDeclarationReference::Local {
                            declaration: DeclarationSelector::Symbol {
                                symbol: "$identity".to_owned(),
                            },
                        },
                        type_arguments: Vec::new(),
                        arguments: vec![AuthoredExpression {
                            symbol: None,
                            operation: AuthoredExpressionOperation::Bool { value: true },
                        }],
                    },
                },
                expected: AuthoredExpression {
                    symbol: Some("$test_expected".to_owned()),
                    operation: AuthoredExpressionOperation::Bool { value: true },
                },
            },
            AuthoredChange::CreateFunction {
                symbol: "$identity".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$authored".to_owned(),
                },
                name: Name::new("identity").unwrap(),
                visibility: DeclarationVisibility::Package,
                type_parameters: Vec::new(),
                parameters: vec![AuthoredParameter {
                    symbol: "$input".to_owned(),
                    name: Name::new("input").unwrap(),
                    ty: AuthoredType::Bool {},
                }],
                result: AuthoredType::Bool {},
                effect: AuthoredFunctionEffect::Pure {},
                body: AuthoredExpression {
                    symbol: Some("$function_body".to_owned()),
                    operation: AuthoredExpressionOperation::Local {
                        value: AuthoredLocalReference::Symbol {
                            symbol: "$input".to_owned(),
                        },
                    },
                },
            },
            AuthoredChange::CreateModule {
                symbol: "$authored".to_owned(),
                name: Name::new("authored").unwrap(),
            },
        ],
    };
    let prepared = created
        .repository
        .prepare_authored_change(&request, PublicationOptions::default())
        .expect("prepare function and test creation");
    assert_eq!(prepared.publication.receipt.counts.owners_created, 8);
    assert_eq!(prepared.publication.receipt.counts.owners_updated, 0);
    assert_eq!(prepared.publication.receipt.counts.type_objects_added, 1);
    assert_eq!(prepared.publication.receipt.validation.tests_selected, 1);
    assert_eq!(prepared.allocated.len(), 7);
    assert!(matches!(
        created
            .repository
            .publish(&prepared.publication)
            .expect("publish function and test creation"),
        PublicationOutcome::Accepted { .. }
    ));

    let view = created.repository.view_current().expect("advanced view");
    let function = prepared.allocated["$identity"];
    let test = prepared.allocated["$identity_test"];
    let parameter = prepared.allocated["$input"];
    let body = prepared.allocated["$function_body"];
    let actual = prepared.allocated["$test_actual"];
    let expected = prepared.allocated["$test_expected"];
    let module = prepared.allocated["$authored"];
    let Some(OwnerRecord::Declaration(function_record)) = view.owner(function).unwrap().value
    else {
        panic!("created function must be readable")
    };
    let DeclarationPayload::Function(function_payload) = function_record.payload else {
        panic!("created declaration must retain function payload")
    };
    assert_eq!(OwnerKey::Expression(function_payload.body), body);
    assert_eq!(
        function_record.module,
        match module {
            OwnerKey::Module(module) => module,
            _ => panic!("module allocation has a foreign domain"),
        }
    );
    let Some(OwnerRecord::Parameter(parameter_record)) = view.owner(parameter).unwrap().value
    else {
        panic!("created parameter must be readable")
    };
    assert_eq!(parameter_record.ty, function_payload.result);
    assert_eq!(
        view.type_object(function_payload.result)
            .unwrap()
            .value
            .unwrap()
            .form,
        TypeForm::Bool
    );
    let Some(OwnerRecord::Expression(body_record)) = view.owner(body).unwrap().value else {
        panic!("created function body must be readable")
    };
    assert_eq!(
        body_record.operation,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(match parameter {
                OwnerKey::Parameter(parameter) => parameter,
                _ => panic!("parameter allocation has a foreign domain"),
            }),
        }
    );
    let Some(OwnerRecord::Declaration(test_record)) = view.owner(test).unwrap().value else {
        panic!("created test must be readable")
    };
    let DeclarationPayload::Test {
        actual: test_actual,
        expected: test_expected,
        ..
    } = test_record.payload
    else {
        panic!("created declaration must retain test payload")
    };
    assert_eq!(OwnerKey::Expression(test_actual), actual);
    assert_eq!(OwnerKey::Expression(test_expected), expected);
    let Some(OwnerRecord::Expression(actual_record)) = view.owner(actual).unwrap().value else {
        panic!("created test actual must be readable")
    };
    let ExpressionOperation::Call {
        function: called,
        arguments,
        ..
    } = actual_record.operation
    else {
        panic!("created test actual must remain a call")
    };
    assert_eq!(OwnerKey::Declaration(called.declaration), function);
    assert_eq!(arguments.len(), 1);
    assert!(
        view.owner(OwnerKey::Expression(arguments[0]))
            .unwrap()
            .value
            .is_some()
    );
}

#[test]
fn authored_type_builder_interns_every_graph_five_type_form() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let package = created.current.semantic_root.package_id;
    let payload =
        authored_exact_declaration(package, owner_named(&created.initial.snapshot, "Payload"));
    let parameters = vec![
        authored_parameter("$p_unit", "p_unit", AuthoredType::Unit {}),
        authored_parameter("$p_bool", "p_bool", AuthoredType::Bool {}),
        authored_parameter("$p_i64", "p_i64", AuthoredType::I64 {}),
        authored_parameter("$p_bytes", "p_bytes", AuthoredType::Bytes {}),
        authored_parameter("$p_text", "p_text", AuthoredType::Text {}),
        authored_parameter(
            "$p_static_text",
            "p_static_text",
            AuthoredType::StaticText {},
        ),
        authored_parameter("$p_secret", "p_secret", AuthoredType::Secret {}),
        authored_parameter(
            "$p_type_parameter",
            "p_type_parameter",
            AuthoredType::TypeParameter {
                parameter: AuthoredTypeParameterReference::Symbol {
                    symbol: "$type_parameter".to_owned(),
                },
            },
        ),
        authored_parameter(
            "$p_named",
            "p_named",
            AuthoredType::Named {
                declaration: payload,
            },
        ),
        authored_parameter(
            "$p_structural",
            "p_structural",
            AuthoredType::StructuralRecord {
                fields: vec![AuthoredStructuralTypeField {
                    name: Name::new("value").unwrap(),
                    ty: AuthoredType::Unit {},
                }],
            },
        ),
        authored_parameter(
            "$p_list",
            "p_list",
            AuthoredType::List {
                item: Box::new(AuthoredType::Unit {}),
            },
        ),
        authored_parameter(
            "$p_map",
            "p_map",
            AuthoredType::Map {
                key: Box::new(AuthoredType::Bool {}),
                value: Box::new(AuthoredType::Unit {}),
            },
        ),
        authored_parameter(
            "$p_option",
            "p_option",
            AuthoredType::Option {
                item: Box::new(AuthoredType::I64 {}),
            },
        ),
        authored_parameter(
            "$p_result",
            "p_result",
            AuthoredType::Result {
                ok: Box::new(AuthoredType::Text {}),
                error: Box::new(AuthoredType::Bytes {}),
            },
        ),
        authored_parameter(
            "$p_stream",
            "p_stream",
            AuthoredType::Stream {
                item: Box::new(AuthoredType::Unit {}),
            },
        ),
        authored_parameter(
            "$p_function",
            "p_function",
            AuthoredType::Function {
                parameters: vec![AuthoredType::Bool {}],
                result: Box::new(AuthoredType::Unit {}),
            },
        ),
    ];
    let request = AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::CreateFunction {
            symbol: "$type_builder".to_owned(),
            module: ModuleSelector::Name {
                name: Name::new("second").unwrap(),
            },
            name: Name::new("type_builder").unwrap(),
            visibility: DeclarationVisibility::Private,
            type_parameters: vec![AuthoredTypeParameter {
                symbol: "$type_parameter".to_owned(),
                name: Name::new("T").unwrap(),
            }],
            parameters,
            result: AuthoredType::Unit {},
            effect: AuthoredFunctionEffect::Pure {},
            body: AuthoredExpression {
                symbol: Some("$type_body".to_owned()),
                operation: AuthoredExpressionOperation::Unit {},
            },
        }],
    };
    let prepared = created
        .repository
        .prepare_authored_change(&request, PublicationOptions::default())
        .expect("every type form must lower and validate");
    // The base fixture already owns canonical `unit`; all other requested shapes are new.
    assert_eq!(prepared.publication.receipt.counts.type_objects_added, 15);
    assert_eq!(prepared.publication.receipt.counts.owners_created, 19);
    assert!(matches!(
        created
            .repository
            .publish(&prepared.publication)
            .expect("publish all type forms"),
        PublicationOutcome::Accepted { .. }
    ));

    let view = created.repository.view_current().expect("advanced view");
    let mut observed = std::collections::BTreeSet::new();
    for symbol in [
        "$p_unit",
        "$p_bool",
        "$p_i64",
        "$p_bytes",
        "$p_text",
        "$p_static_text",
        "$p_secret",
        "$p_type_parameter",
        "$p_named",
        "$p_structural",
        "$p_list",
        "$p_map",
        "$p_option",
        "$p_result",
        "$p_stream",
        "$p_function",
    ] {
        let owner = prepared.allocated[symbol];
        let Some(OwnerRecord::Parameter(parameter)) = view.owner(owner).unwrap().value else {
            panic!("created type-form parameter must remain readable")
        };
        let form = view.type_object(parameter.ty).unwrap().value.unwrap().form;
        observed.insert(match form {
            TypeForm::Unit => "unit",
            TypeForm::Bool => "bool",
            TypeForm::I64 => "i64",
            TypeForm::Bytes => "bytes",
            TypeForm::Text => "text",
            TypeForm::StaticText => "static_text",
            TypeForm::Secret => "secret",
            TypeForm::TypeParameter { .. } => "type_parameter",
            TypeForm::Named { .. } => "named",
            TypeForm::StructuralRecord { .. } => "structural_record",
            TypeForm::List { .. } => "list",
            TypeForm::Map { .. } => "map",
            TypeForm::Option { .. } => "option",
            TypeForm::Result { .. } => "result",
            TypeForm::Stream { .. } => "stream",
            TypeForm::Function { .. } => "function",
        });
    }
    assert_eq!(observed.len(), 16);
}

#[test]
fn authored_request_creates_every_foundational_owner_kind_with_forward_symbols() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let local_declaration = |symbol: &str| AuthoredDeclarationReference::Local {
        declaration: DeclarationSelector::Symbol {
            symbol: symbol.to_owned(),
        },
    };
    let request = AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::CreateTarget {
                symbol: "$target".to_owned(),
                name: Name::new("authored_command").unwrap(),
                component: local_declaration("$component"),
                port: AuthoredPortReference::Symbol {
                    symbol: "$port".to_owned(),
                },
                runner: RunnerKind::Command,
            },
            AuthoredChange::CreateDocumentation {
                symbol: "$documentation".to_owned(),
                owner: OwnerSelector::Symbol {
                    symbol: "$record".to_owned(),
                },
                class: DocumentationClass::Nonsemantic,
                text: "authored record".to_owned(),
            },
            AuthoredChange::CreateAnnotation {
                symbol: "$annotation".to_owned(),
                owner: OwnerSelector::Symbol {
                    symbol: "$field".to_owned(),
                },
                class: AnnotationClass::Semantic,
                key: Name::new("indexed").unwrap(),
                value: AuthoredAnnotationValue::Bool(true),
            },
            AuthoredChange::CreateComponent {
                symbol: "$component".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$domain".to_owned(),
                },
                name: Name::new("Application").unwrap(),
                visibility: DeclarationVisibility::Public,
                requirements: vec![AuthoredRequirement {
                    symbol: "$requirement".to_owned(),
                    name: Name::new("store").unwrap(),
                    interface: local_declaration("$interface"),
                    operations: vec![AuthoredOperationReference::Symbol {
                        symbol: "$operation".to_owned(),
                    }],
                    limits: vec![AuthoredResourceLimit {
                        name: Name::new("calls").unwrap(),
                        maximum: 1,
                        unit: ResourceUnit::Calls,
                    }],
                }],
                ports: vec![
                    AuthoredPort {
                        symbol: "$port".to_owned(),
                        name: Name::new("run").unwrap(),
                        function_type: AuthoredType::Function {
                            parameters: Vec::new(),
                            result: Box::new(AuthoredType::Unit {}),
                        },
                        implementation: AuthoredPortImplementation::Function {
                            function: local_declaration("$entry"),
                        },
                    },
                    AuthoredPort {
                        symbol: "$expression_port".to_owned(),
                        name: Name::new("invoke").unwrap(),
                        function_type: AuthoredType::Function {
                            parameters: Vec::new(),
                            result: Box::new(AuthoredType::Unit {}),
                        },
                        implementation: AuthoredPortImplementation::Expression {
                            expression: AuthoredExpression {
                                symbol: Some("$port_expression".to_owned()),
                                operation: AuthoredExpressionOperation::FunctionValue {
                                    function: local_declaration("$entry"),
                                    type_arguments: Vec::new(),
                                },
                            },
                        },
                    },
                ],
            },
            AuthoredChange::CreateConstant {
                symbol: "$constant".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$domain".to_owned(),
                },
                name: Name::new("unit").unwrap(),
                visibility: DeclarationVisibility::Package,
                ty: AuthoredType::Unit {},
                value: AuthoredExpression {
                    symbol: Some("$constant_value".to_owned()),
                    operation: AuthoredExpressionOperation::Unit {},
                },
            },
            AuthoredChange::CreateExternal {
                symbol: "$external".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$domain".to_owned(),
                },
                name: Name::new("identity_external").unwrap(),
                visibility: DeclarationVisibility::Package,
                type_parameters: vec![AuthoredTypeParameter {
                    symbol: "$external_type".to_owned(),
                    name: Name::new("T").unwrap(),
                }],
                parameters: vec![AuthoredParameter {
                    symbol: "$external_parameter".to_owned(),
                    name: Name::new("value").unwrap(),
                    ty: AuthoredType::TypeParameter {
                        parameter: AuthoredTypeParameterReference::Symbol {
                            symbol: "$external_type".to_owned(),
                        },
                    },
                }],
                result: AuthoredType::TypeParameter {
                    parameter: AuthoredTypeParameterReference::Symbol {
                        symbol: "$external_type".to_owned(),
                    },
                },
                implementation: Name::new("identity_host").unwrap(),
            },
            AuthoredChange::CreateInterface {
                symbol: "$interface".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$domain".to_owned(),
                },
                name: Name::new("Store").unwrap(),
                visibility: DeclarationVisibility::Public,
                operations: vec![AuthoredOperation {
                    symbol: "$operation".to_owned(),
                    name: Name::new("write").unwrap(),
                    parameters: vec![AuthoredParameter {
                        symbol: "$operation_parameter".to_owned(),
                        name: Name::new("payload").unwrap(),
                        ty: AuthoredType::Named {
                            declaration: local_declaration("$record"),
                        },
                    }],
                    result: AuthoredType::Unit {},
                    idempotency: Idempotency::Idempotent,
                    external_visibility: ExternalVisibility::None,
                }],
            },
            AuthoredChange::CreateVariant {
                symbol: "$variant".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$domain".to_owned(),
                },
                name: Name::new("State").unwrap(),
                visibility: DeclarationVisibility::Public,
                cases: vec![AuthoredCase {
                    symbol: "$case".to_owned(),
                    name: Name::new("Ready").unwrap(),
                    payload: Some(AuthoredType::Named {
                        declaration: local_declaration("$record"),
                    }),
                }],
            },
            AuthoredChange::CreateRecord {
                symbol: "$record".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$domain".to_owned(),
                },
                name: Name::new("Payload").unwrap(),
                visibility: DeclarationVisibility::Public,
                fields: vec![AuthoredField {
                    symbol: "$field".to_owned(),
                    name: Name::new("value").unwrap(),
                    ty: AuthoredType::Unit {},
                }],
            },
            AuthoredChange::CreateFunction {
                symbol: "$entry".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$domain".to_owned(),
                },
                name: Name::new("entry").unwrap(),
                visibility: DeclarationVisibility::Package,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: AuthoredType::Unit {},
                effect: AuthoredFunctionEffect::Pure {},
                body: AuthoredExpression {
                    symbol: Some("$entry_body".to_owned()),
                    operation: AuthoredExpressionOperation::Sequence {
                        items: vec![
                            AuthoredExpression {
                                symbol: Some("$record_literal".to_owned()),
                                operation: AuthoredExpressionOperation::Record {
                                    nominal_type: Some(local_declaration("$record")),
                                    fields: vec![AuthoredRecordExpressionField {
                                        selector: AuthoredFieldSelector::Nominal {
                                            field: AuthoredFieldReference::Symbol {
                                                symbol: "$field".to_owned(),
                                            },
                                        },
                                        value: authored_expression(
                                            AuthoredExpressionOperation::Unit {},
                                        ),
                                    }],
                                },
                            },
                            authored_expression(AuthoredExpressionOperation::Unit {}),
                        ],
                    },
                },
            },
            AuthoredChange::CreateModule {
                symbol: "$domain".to_owned(),
                name: Name::new("domain").unwrap(),
            },
            AuthoredChange::RenameOwner {
                owner: OwnerSelector::Symbol {
                    symbol: "$field".to_owned(),
                },
                name: Name::new("renamed_value").unwrap(),
            },
        ],
    };
    let prepared = created
        .repository
        .prepare_authored_change(&request, PublicationOptions::default())
        .expect("all foundational owners must lower and validate through one request");
    assert_eq!(prepared.allocated.len(), 24);
    assert_eq!(prepared.publication.receipt.counts.owners_created, 26);
    // Canonical `unit` and `() -> unit` are reused from the accepted fixture.
    assert_eq!(prepared.publication.receipt.counts.type_objects_added, 2);
    assert_eq!(
        prepared.publication.receipt.validation.profile,
        ValidationProfile::IncrementalOwnerFrontier
    );
    assert_eq!(
        prepared.publication.receipt.validation.full_oracle,
        FullOracleStatus::NotRun
    );
    // Reusing accepted type objects performs exact point reads without adding duplicate type
    // objects to the candidate authority.
    assert_eq!(prepared.lowering_work.canonical.point_reads, 4);
    assert!(matches!(
        created
            .repository
            .publish(&prepared.publication)
            .expect("publish foundational owner request"),
        PublicationOutcome::Accepted { .. }
    ));

    let view = created.repository.view_current().expect("advanced view");
    for owner in prepared.allocated.values() {
        assert!(view.owner(*owner).unwrap().value.is_some());
    }
    let component = prepared.allocated["$component"];
    let requirement = prepared.allocated["$requirement"];
    let port = prepared.allocated["$port"];
    let expression_port = prepared.allocated["$expression_port"];
    let target = prepared.allocated["$target"];
    let Some(OwnerRecord::Declaration(component_record)) = view.owner(component).unwrap().value
    else {
        panic!("component must remain a declaration")
    };
    let DeclarationPayload::Component {
        requirements,
        ports,
    } = component_record.payload
    else {
        panic!("component declaration must retain its closed payload")
    };
    let OwnerKey::Requirement(requirement_id) = requirement else {
        panic!("requirement allocation has a foreign domain")
    };
    let OwnerKey::Port(port_id) = port else {
        panic!("port allocation has a foreign domain")
    };
    let OwnerKey::Port(expression_port_id) = expression_port else {
        panic!("expression-port allocation has a foreign domain")
    };
    let mut expected_ports = vec![port_id, expression_port_id];
    expected_ports.sort_unstable();
    assert_eq!(requirements.as_slice(), &[requirement_id]);
    assert_eq!(ports, expected_ports);
    let Some(OwnerRecord::Target(target_record)) = view.owner(target).unwrap().value else {
        panic!("target must remain readable")
    };
    let OwnerKey::Declaration(component_id) = component else {
        panic!("component allocation has a foreign domain")
    };
    assert_eq!(target_record.component.declaration, component_id);
    assert_eq!(target_record.port.port, port_id);
    let Some(OwnerRecord::Documentation(documentation)) = view
        .owner(prepared.allocated["$documentation"])
        .unwrap()
        .value
    else {
        panic!("documentation must remain readable")
    };
    assert_eq!(documentation.owner, prepared.allocated["$record"]);
    let Some(OwnerRecord::Annotation(annotation)) =
        view.owner(prepared.allocated["$annotation"]).unwrap().value
    else {
        panic!("annotation must remain readable")
    };
    assert_eq!(annotation.owner, prepared.allocated["$field"]);
    let Some(OwnerRecord::Field(field_record)) =
        view.owner(prepared.allocated["$field"]).unwrap().value
    else {
        panic!("renamed field must remain readable")
    };
    assert_eq!(field_record.name.as_str(), "renamed_value");
    let Some(OwnerRecord::Expression(record_literal)) = view
        .owner(prepared.allocated["$record_literal"])
        .unwrap()
        .value
    else {
        panic!("nominal record expression must remain readable")
    };
    let ExpressionOperation::Record { fields, .. } = record_literal.operation else {
        panic!("record literal must retain its operation")
    };
    assert!(matches!(
        fields[0].selector,
        crate::platform::kernel::FieldSelector::Nominal(reference)
            if OwnerKey::Field(reference.field) == prepared.allocated["$field"]
    ));
}

#[test]
fn authored_member_and_contract_mutations_share_one_order_independent_pipeline() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let package = created.current.semantic_root.package_id;
    let declaration_id = |owner| match owner {
        OwnerKey::Declaration(declaration) => declaration,
        _ => panic!("selected owner must be a declaration"),
    };
    let second_module = ModuleSelector::Name {
        name: Name::new("second").unwrap(),
    };
    let stage_one = AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::CreateRecord {
                symbol: "$mutable_record".to_owned(),
                module: second_module.clone(),
                name: Name::new("MutableRecord").unwrap(),
                visibility: DeclarationVisibility::Package,
                fields: vec![AuthoredField {
                    symbol: "$base_field".to_owned(),
                    name: Name::new("base").unwrap(),
                    ty: AuthoredType::Unit {},
                }],
            },
            AuthoredChange::CreateVariant {
                symbol: "$mutable_variant".to_owned(),
                module: second_module.clone(),
                name: Name::new("MutableVariant").unwrap(),
                visibility: DeclarationVisibility::Package,
                cases: vec![AuthoredCase {
                    symbol: "$base_case".to_owned(),
                    name: Name::new("Base").unwrap(),
                    payload: None,
                }],
            },
            AuthoredChange::CreateFunction {
                symbol: "$mutable_function".to_owned(),
                module: second_module,
                name: Name::new("mutable_function").unwrap(),
                visibility: DeclarationVisibility::Private,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: AuthoredType::Bool {},
                effect: AuthoredFunctionEffect::Pure {},
                body: AuthoredExpression {
                    symbol: Some("$mutable_body".to_owned()),
                    operation: AuthoredExpressionOperation::Bool { value: true },
                },
            },
        ],
    };
    let stage_one = created
        .repository
        .prepare_authored_change(&stage_one, PublicationOptions::default())
        .expect("prepare isolated mutable owners");
    assert!(matches!(
        created
            .repository
            .publish(&stage_one.publication)
            .expect("publish isolated mutable owners"),
        PublicationOutcome::Accepted { .. }
    ));

    let record = stage_one.allocated["$mutable_record"];
    let base_field = stage_one.allocated["$base_field"];
    let variant = stage_one.allocated["$mutable_variant"];
    let base_case = stage_one.allocated["$base_case"];
    let function = stage_one.allocated["$mutable_function"];
    let interface = owner_named(&created.initial.snapshot, "Store");
    let external = owner_named(&created.initial.snapshot, "identity_external");
    let component = owner_named(&created.initial.snapshot, "Application");
    let caller = owner_named(&created.initial.snapshot, "caller");
    let target = target_named(&created.initial.snapshot, "command");
    let exact_declaration = |owner| AuthoredDeclarationReference::Exact {
        package,
        declaration: declaration_id(owner),
    };
    let current = created
        .repository
        .current()
        .expect("stage-one current binding");
    let request = AuthoredChangeSet {
        base: current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        // Updates intentionally precede their request-local additions. Symbol allocation and
        // member construction are request-wide passes, not authored-order dependencies.
        changes: vec![
            AuthoredChange::SetTarget {
                target: OwnerSelector::Exact { owner: target },
                component: exact_declaration(component),
                port: AuthoredPortReference::Symbol {
                    symbol: "$alternate_port".to_owned(),
                },
                runner: RunnerKind::Command,
            },
            AuthoredChange::SetRequirementContract {
                requirement: OwnerSelector::Symbol {
                    symbol: "$added_requirement".to_owned(),
                },
                interface: exact_declaration(interface),
                operations: vec![AuthoredOperationReference::Symbol {
                    symbol: "$added_operation".to_owned(),
                }],
                limits: vec![AuthoredResourceLimit {
                    name: Name::new("calls").unwrap(),
                    maximum: 7,
                    unit: ResourceUnit::Calls,
                }],
            },
            AuthoredChange::SetOperationContract {
                operation: OwnerSelector::Symbol {
                    symbol: "$added_operation".to_owned(),
                },
                result: AuthoredType::Bool {},
                idempotency: Idempotency::NonIdempotent,
                external_visibility: ExternalVisibility::Possible,
            },
            AuthoredChange::SetParameterType {
                parameter: OwnerSelector::Symbol {
                    symbol: "$external_value".to_owned(),
                },
                ty: AuthoredType::TypeParameter {
                    parameter: AuthoredTypeParameterReference::Symbol {
                        symbol: "$external_u".to_owned(),
                    },
                },
            },
            AuthoredChange::SetExternalContract {
                external: DeclarationSelector::Id {
                    declaration: declaration_id(external),
                },
                result: AuthoredType::TypeParameter {
                    parameter: AuthoredTypeParameterReference::Symbol {
                        symbol: "$external_u".to_owned(),
                    },
                },
                implementation: Name::new("identity_host_v2").unwrap(),
            },
            AuthoredChange::SetFunctionContract {
                function: DeclarationSelector::Id {
                    declaration: declaration_id(function),
                },
                result: AuthoredType::Bool {},
                effect: AuthoredFunctionEffect::Task {
                    requirements: vec![AuthoredRequirementReference::Symbol {
                        symbol: "$added_requirement".to_owned(),
                    }],
                },
            },
            AuthoredChange::SetDeclarationVisibility {
                declaration: DeclarationSelector::Id {
                    declaration: declaration_id(record),
                },
                visibility: DeclarationVisibility::Private,
            },
            AuthoredChange::SetFieldType {
                field: OwnerSelector::Exact { owner: base_field },
                ty: AuthoredType::Bool {},
            },
            AuthoredChange::SetCasePayload {
                case: OwnerSelector::Exact { owner: base_case },
                payload: Some(AuthoredType::Bool {}),
            },
            AuthoredChange::AddParameter {
                parent: ParameterParentSelector::Operation {
                    operation: OwnerSelector::Symbol {
                        symbol: "$added_operation".to_owned(),
                    },
                },
                parameter: AuthoredParameter {
                    symbol: "$added_operation_parameter".to_owned(),
                    name: Name::new("value").unwrap(),
                    ty: AuthoredType::Bool {},
                },
            },
            AuthoredChange::AddPort {
                component: DeclarationSelector::Id {
                    declaration: declaration_id(component),
                },
                port: AuthoredPort {
                    symbol: "$alternate_port".to_owned(),
                    name: Name::new("alternate").unwrap(),
                    function_type: AuthoredType::Function {
                        parameters: Vec::new(),
                        result: Box::new(AuthoredType::Unit {}),
                    },
                    implementation: AuthoredPortImplementation::Function {
                        function: exact_declaration(caller),
                    },
                },
            },
            AuthoredChange::AddRequirement {
                component: DeclarationSelector::Id {
                    declaration: declaration_id(component),
                },
                requirement: AuthoredRequirement {
                    symbol: "$added_requirement".to_owned(),
                    name: Name::new("secondary_store").unwrap(),
                    interface: exact_declaration(interface),
                    operations: Vec::new(),
                    limits: Vec::new(),
                },
            },
            AuthoredChange::AddOperation {
                interface: DeclarationSelector::Id {
                    declaration: declaration_id(interface),
                },
                operation: AuthoredOperation {
                    symbol: "$added_operation".to_owned(),
                    name: Name::new("write_v2").unwrap(),
                    parameters: Vec::new(),
                    result: AuthoredType::Unit {},
                    idempotency: Idempotency::Idempotent,
                    external_visibility: ExternalVisibility::None,
                },
            },
            AuthoredChange::AddParameter {
                parent: ParameterParentSelector::Declaration {
                    declaration: DeclarationSelector::Id {
                        declaration: declaration_id(external),
                    },
                },
                parameter: AuthoredParameter {
                    symbol: "$external_value".to_owned(),
                    name: Name::new("value").unwrap(),
                    ty: AuthoredType::Unit {},
                },
            },
            AuthoredChange::AddTypeParameter {
                declaration: DeclarationSelector::Id {
                    declaration: declaration_id(external),
                },
                parameter: AuthoredTypeParameter {
                    symbol: "$external_u".to_owned(),
                    name: Name::new("U").unwrap(),
                },
            },
            AuthoredChange::AddCase {
                variant: DeclarationSelector::Id {
                    declaration: declaration_id(variant),
                },
                case: AuthoredCase {
                    symbol: "$added_case".to_owned(),
                    name: Name::new("Added").unwrap(),
                    payload: None,
                },
            },
            AuthoredChange::AddField {
                record: DeclarationSelector::Id {
                    declaration: declaration_id(record),
                },
                field: AuthoredField {
                    symbol: "$added_field".to_owned(),
                    name: Name::new("added").unwrap(),
                    ty: AuthoredType::Unit {},
                },
            },
        ],
    };
    let operation_count = request.changes.len() as u64;
    let prepared = created
        .repository
        .prepare_authored_change(&request, PublicationOptions::default())
        .expect("prepare mixed member and contract mutations");
    assert_eq!(prepared.allocated.len(), 8);
    assert_eq!(prepared.lowering_work.operations_lowered, operation_count);
    assert_eq!(prepared.publication.receipt.counts.owners_created, 8);
    assert!(prepared.publication.receipt.counts.owners_updated >= 9);
    assert_eq!(
        prepared.publication.receipt.validation.profile,
        ValidationProfile::IncrementalOwnerFrontier
    );
    assert_eq!(
        prepared.publication.receipt.validation.full_oracle,
        FullOracleStatus::NotRun
    );
    assert!(matches!(
        created
            .repository
            .publish(&prepared.publication)
            .expect("publish mixed member and contract mutations"),
        PublicationOutcome::Accepted { .. }
    ));

    let view = created
        .repository
        .view_current()
        .expect("advanced mutation view");
    for owner in prepared.allocated.values() {
        assert!(view.owner(*owner).unwrap().value.is_some());
    }
    let added_field = prepared.allocated["$added_field"];
    let added_case = prepared.allocated["$added_case"];
    let added_operation = prepared.allocated["$added_operation"];
    let operation_parameter = prepared.allocated["$added_operation_parameter"];
    let external_parameter = prepared.allocated["$external_value"];
    let external_type_parameter = prepared.allocated["$external_u"];
    let requirement = prepared.allocated["$added_requirement"];
    let port = prepared.allocated["$alternate_port"];

    let Some(OwnerRecord::Declaration(record_owner)) = view.owner(record).unwrap().value else {
        panic!("mutated record must remain readable")
    };
    let DeclarationPayload::Record { fields } = record_owner.payload else {
        panic!("mutated declaration must remain a record")
    };
    assert_eq!(record_owner.visibility, DeclarationVisibility::Private);
    assert!(fields.contains(&match added_field {
        OwnerKey::Field(field) => field,
        _ => panic!("added field has a foreign domain"),
    }));
    let Some(OwnerRecord::Field(base_field_record)) = view.owner(base_field).unwrap().value else {
        panic!("base field must remain readable")
    };
    assert_eq!(
        view.type_object(base_field_record.ty)
            .unwrap()
            .value
            .unwrap()
            .form,
        TypeForm::Bool
    );

    let Some(OwnerRecord::Declaration(variant_owner)) = view.owner(variant).unwrap().value else {
        panic!("mutated variant must remain readable")
    };
    let DeclarationPayload::Variant { cases } = variant_owner.payload else {
        panic!("mutated declaration must remain a variant")
    };
    assert!(cases.contains(&match added_case {
        OwnerKey::Case(case) => case,
        _ => panic!("added case has a foreign domain"),
    }));
    let Some(OwnerRecord::Case(base_case_record)) = view.owner(base_case).unwrap().value else {
        panic!("base case must remain readable")
    };
    assert!(base_case_record.payload.is_some());

    let Some(OwnerRecord::Operation(operation_record)) = view.owner(added_operation).unwrap().value
    else {
        panic!("added operation must remain readable")
    };
    assert_eq!(operation_record.idempotency, Idempotency::NonIdempotent);
    assert_eq!(
        operation_record.external_visibility,
        ExternalVisibility::Possible
    );
    assert_eq!(operation_record.parameters.len(), 1);
    assert_eq!(
        OwnerKey::Parameter(operation_record.parameters[0]),
        operation_parameter
    );
    assert_eq!(
        view.type_object(operation_record.result)
            .unwrap()
            .value
            .unwrap()
            .form,
        TypeForm::Bool
    );

    let Some(OwnerRecord::Declaration(external_record)) = view.owner(external).unwrap().value
    else {
        panic!("external declaration must remain readable")
    };
    let DeclarationPayload::External(external_payload) = external_record.payload else {
        panic!("mutated declaration must remain external")
    };
    assert_eq!(external_payload.implementation.as_str(), "identity_host_v2");
    assert!(
        external_payload
            .parameters
            .contains(&match external_parameter {
                OwnerKey::Parameter(parameter) => parameter,
                _ => panic!("external parameter has a foreign domain"),
            })
    );
    assert!(
        external_payload
            .type_parameters
            .contains(&match external_type_parameter {
                OwnerKey::TypeParameter(parameter) => parameter,
                _ => panic!("external type parameter has a foreign domain"),
            })
    );
    let Some(OwnerRecord::Parameter(external_parameter_record)) =
        view.owner(external_parameter).unwrap().value
    else {
        panic!("external parameter must remain readable")
    };
    assert_eq!(external_parameter_record.ty, external_payload.result);
    assert!(matches!(
        view.type_object(external_payload.result)
            .unwrap()
            .value
            .unwrap()
            .form,
        TypeForm::TypeParameter { parameter }
            if OwnerKey::TypeParameter(parameter) == external_type_parameter
    ));

    let Some(OwnerRecord::Declaration(function_record)) = view.owner(function).unwrap().value
    else {
        panic!("mutated function must remain readable")
    };
    assert_eq!(
        function_record.header.kind,
        crate::platform::kernel::OwnerKind::TaskFunction
    );
    let DeclarationPayload::Function(function_payload) = function_record.payload else {
        panic!("mutated declaration must remain a function")
    };
    assert!(matches!(
        function_payload.effect,
        crate::platform::kernel::FunctionEffect::Task { requirements }
            if requirements.as_slice() == [crate::platform::kernel::RequirementReference {
                package,
                requirement: match requirement {
                    OwnerKey::Requirement(requirement) => requirement,
                    _ => panic!("requirement has a foreign domain"),
                },
            }]
    ));

    let Some(OwnerRecord::Requirement(requirement_record)) = view.owner(requirement).unwrap().value
    else {
        panic!("added requirement must remain readable")
    };
    assert_eq!(requirement_record.operations.len(), 1);
    assert_eq!(
        OwnerKey::Operation(requirement_record.operations[0].operation),
        added_operation
    );
    assert_eq!(requirement_record.limits[0].maximum, 7);
    let Some(OwnerRecord::Target(target_record)) = view.owner(target).unwrap().value else {
        panic!("target must remain readable")
    };
    assert_eq!(OwnerKey::Port(target_record.port.port), port);
    assert_eq!(target_record.runner, RunnerKind::Command);
}

#[test]
fn authored_member_additions_revalidate_exact_reverse_dependents() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let base = created.current.head.revision;
    let declaration_id = |owner| match owner {
        OwnerKey::Declaration(declaration) => declaration,
        _ => panic!("selected owner must be a declaration"),
    };
    let cases = [
        AuthoredChange::AddField {
            record: DeclarationSelector::Id {
                declaration: declaration_id(owner_named(&created.initial.snapshot, "Payload")),
            },
            field: AuthoredField {
                symbol: "$required_field".to_owned(),
                name: Name::new("required").unwrap(),
                ty: AuthoredType::Unit {},
            },
        },
        AuthoredChange::AddCase {
            variant: DeclarationSelector::Id {
                declaration: declaration_id(owner_named(&created.initial.snapshot, "State")),
            },
            case: AuthoredCase {
                symbol: "$new_case".to_owned(),
                name: Name::new("New").unwrap(),
                payload: None,
            },
        },
        AuthoredChange::AddParameter {
            parent: ParameterParentSelector::Declaration {
                declaration: DeclarationSelector::Id {
                    declaration: declaration_id(owner_named(&created.initial.snapshot, "callee")),
                },
            },
            parameter: AuthoredParameter {
                symbol: "$new_parameter".to_owned(),
                name: Name::new("additional").unwrap(),
                ty: AuthoredType::Unit {},
            },
        },
    ];
    for change in cases {
        let diagnostics = created
            .repository
            .prepare_authored_change(
                &AuthoredChangeSet {
                    base,
                    preconditions: Vec::new(),
                    budget: ChangeBudget::default(),
                    changes: vec![change],
                },
                PublicationOptions::default(),
            )
            .expect_err("an unadapted dependent must reject the member addition");
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.class == crate::platform::diagnostic::DiagnosticClass::Semantic
                && diagnostic.code.starts_with("kernel_type_")
        }));
        assert_eq!(created.repository.current().unwrap().head.revision, base);
    }
}

#[test]
fn authored_expression_builder_covers_every_graph_five_operation() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let package = created.current.semantic_root.package_id;
    let callee = owner_named(&created.initial.snapshot, "callee");
    let constant = owner_named(&created.initial.snapshot, "unit_constant");
    let record = owner_named(&created.initial.snapshot, "Payload");
    let field = field_named(&created.initial.snapshot, "value");
    let case = case_named(&created.initial.snapshot, "Ready");
    let requirement = requirement_named(&created.initial.snapshot, "store");
    let operation = operation_named(&created.initial.snapshot, "read");
    let callee_reference = authored_exact_declaration(package, callee);
    let record_reference = authored_exact_declaration(package, record);
    let constant_reference = authored_exact_declaration(package, constant);
    let field_reference = AuthoredFieldReference::Exact {
        package,
        field: match field {
            OwnerKey::Field(field) => field,
            _ => panic!("field helper returned a foreign owner"),
        },
    };
    let case_reference = AuthoredCaseReference::Exact {
        package,
        case: match case {
            OwnerKey::Case(case) => case,
            _ => panic!("case helper returned a foreign owner"),
        },
    };
    let requirement_reference = AuthoredRequirementReference::Exact {
        package,
        requirement: match requirement {
            OwnerKey::Requirement(requirement) => requirement,
            _ => panic!("requirement helper returned a foreign owner"),
        },
    };
    let operation_reference = AuthoredOperationReference::Exact {
        package,
        operation: match operation {
            OwnerKey::Operation(operation) => operation,
            _ => panic!("operation helper returned a foreign owner"),
        },
    };
    let nominal_record = |symbol: &str| AuthoredExpression {
        symbol: Some(symbol.to_owned()),
        operation: AuthoredExpressionOperation::Record {
            nominal_type: Some(record_reference.clone()),
            fields: vec![AuthoredRecordExpressionField {
                selector: AuthoredFieldSelector::Nominal {
                    field: field_reference.clone(),
                },
                value: authored_expression(AuthoredExpressionOperation::Unit {}),
            }],
        },
    };
    let variant = |symbol: &str| AuthoredExpression {
        symbol: Some(symbol.to_owned()),
        operation: AuthoredExpressionOperation::Variant {
            case: case_reference.clone(),
            payload: None,
        },
    };
    let pure_body = AuthoredExpression {
        symbol: Some("$pure_body".to_owned()),
        operation: AuthoredExpressionOperation::Sequence {
            items: vec![
                authored_expression(AuthoredExpressionOperation::Unit {}),
                authored_expression(AuthoredExpressionOperation::Bool { value: true }),
                authored_expression(AuthoredExpressionOperation::I64 { value: 7 }),
                authored_expression(AuthoredExpressionOperation::Text {
                    value: "text".to_owned(),
                }),
                authored_expression(AuthoredExpressionOperation::StaticText {
                    value: "static".to_owned(),
                }),
                AuthoredExpression {
                    symbol: Some("$constant".to_owned()),
                    operation: AuthoredExpressionOperation::Constant {
                        declaration: constant_reference,
                    },
                },
                AuthoredExpression {
                    symbol: Some("$if".to_owned()),
                    operation: AuthoredExpressionOperation::If {
                        condition: Box::new(authored_expression(
                            AuthoredExpressionOperation::Bool { value: true },
                        )),
                        when_true: Box::new(authored_expression(
                            AuthoredExpressionOperation::Unit {},
                        )),
                        when_false: Box::new(authored_expression(
                            AuthoredExpressionOperation::Unit {},
                        )),
                    },
                },
                AuthoredExpression {
                    symbol: Some("$let".to_owned()),
                    operation: AuthoredExpressionOperation::Let {
                        bindings: vec![AuthoredLetBinding {
                            symbol: "$local".to_owned(),
                            name: Name::new("local").unwrap(),
                            value: authored_expression(AuthoredExpressionOperation::Unit {}),
                            declared_type: Some(AuthoredType::Unit {}),
                        }],
                        body: Box::new(AuthoredExpression {
                            symbol: Some("$local_use".to_owned()),
                            operation: AuthoredExpressionOperation::Local {
                                value: AuthoredLocalReference::Symbol {
                                    symbol: "$local".to_owned(),
                                },
                            },
                        }),
                    },
                },
                AuthoredExpression {
                    symbol: Some("$call".to_owned()),
                    operation: AuthoredExpressionOperation::Call {
                        function: callee_reference.clone(),
                        type_arguments: Vec::new(),
                        arguments: vec![authored_expression(AuthoredExpressionOperation::Unit {})],
                    },
                },
                AuthoredExpression {
                    symbol: Some("$function_value".to_owned()),
                    operation: AuthoredExpressionOperation::FunctionValue {
                        function: callee_reference.clone(),
                        type_arguments: Vec::new(),
                    },
                },
                AuthoredExpression {
                    symbol: Some("$invoke".to_owned()),
                    operation: AuthoredExpressionOperation::Invoke {
                        callee: Box::new(authored_expression(
                            AuthoredExpressionOperation::FunctionValue {
                                function: callee_reference,
                                type_arguments: Vec::new(),
                            },
                        )),
                        arguments: vec![authored_expression(AuthoredExpressionOperation::Unit {})],
                    },
                },
                nominal_record("$record"),
                AuthoredExpression {
                    symbol: Some("$field".to_owned()),
                    operation: AuthoredExpressionOperation::Field {
                        value: Box::new(nominal_record("$field_record")),
                        selector: AuthoredFieldSelector::Nominal {
                            field: field_reference,
                        },
                    },
                },
                variant("$variant"),
                AuthoredExpression {
                    symbol: Some("$list".to_owned()),
                    operation: AuthoredExpressionOperation::List {
                        item_type: AuthoredType::Unit {},
                        items: vec![authored_expression(AuthoredExpressionOperation::Unit {})],
                    },
                },
                AuthoredExpression {
                    symbol: Some("$map".to_owned()),
                    operation: AuthoredExpressionOperation::Map {
                        key_type: AuthoredType::Bool {},
                        value_type: AuthoredType::Unit {},
                        entries: vec![AuthoredMapExpressionEntry {
                            key: authored_expression(AuthoredExpressionOperation::Bool {
                                value: true,
                            }),
                            value: authored_expression(AuthoredExpressionOperation::Unit {}),
                        }],
                    },
                },
                AuthoredExpression {
                    symbol: Some("$match".to_owned()),
                    operation: AuthoredExpressionOperation::Match {
                        value: Box::new(variant("$match_value")),
                        arms: vec![AuthoredMatchExpressionArm {
                            case: case_reference,
                            payload_binding: None,
                            body: authored_expression(AuthoredExpressionOperation::Unit {}),
                        }],
                    },
                },
                AuthoredExpression {
                    symbol: Some("$structural_record".to_owned()),
                    operation: AuthoredExpressionOperation::Record {
                        nominal_type: None,
                        fields: vec![AuthoredRecordExpressionField {
                            selector: AuthoredFieldSelector::Structural {
                                name: Name::new("item").unwrap(),
                            },
                            value: authored_expression(AuthoredExpressionOperation::Unit {}),
                        }],
                    },
                },
                AuthoredExpression {
                    symbol: Some("$structural_field".to_owned()),
                    operation: AuthoredExpressionOperation::Field {
                        value: Box::new(authored_expression(AuthoredExpressionOperation::Record {
                            nominal_type: None,
                            fields: vec![AuthoredRecordExpressionField {
                                selector: AuthoredFieldSelector::Structural {
                                    name: Name::new("item").unwrap(),
                                },
                                value: authored_expression(AuthoredExpressionOperation::Unit {}),
                            }],
                        })),
                        selector: AuthoredFieldSelector::Structural {
                            name: Name::new("item").unwrap(),
                        },
                    },
                },
                authored_expression(AuthoredExpressionOperation::Unit {}),
            ],
        },
    };
    let task_body = AuthoredExpression {
        symbol: Some("$task_body".to_owned()),
        operation: AuthoredExpressionOperation::Sequence {
            items: vec![
                AuthoredExpression {
                    symbol: Some("$capability".to_owned()),
                    operation: AuthoredExpressionOperation::CapabilityCall {
                        requirement: requirement_reference.clone(),
                        operation: operation_reference,
                        arguments: Vec::new(),
                    },
                },
                AuthoredExpression {
                    symbol: Some("$transaction".to_owned()),
                    operation: AuthoredExpressionOperation::Transaction {
                        requirement: requirement_reference.clone(),
                        binding: AuthoredBindingDefinition {
                            symbol: "$transaction_binding".to_owned(),
                            name: Name::new("transaction").unwrap(),
                        },
                        body: Box::new(authored_expression(AuthoredExpressionOperation::Unit {})),
                    },
                },
                authored_expression(AuthoredExpressionOperation::Unit {}),
            ],
        },
    };
    let request = AuthoredChangeSet {
        base: created.current.head.revision,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![
            AuthoredChange::CreateFunction {
                symbol: "$all_pure".to_owned(),
                module: ModuleSelector::Name {
                    name: Name::new("second").unwrap(),
                },
                name: Name::new("all_pure").unwrap(),
                visibility: DeclarationVisibility::Private,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: AuthoredType::Unit {},
                effect: AuthoredFunctionEffect::Pure {},
                body: pure_body,
            },
            AuthoredChange::CreateFunction {
                symbol: "$all_task".to_owned(),
                module: ModuleSelector::Name {
                    name: Name::new("second").unwrap(),
                },
                name: Name::new("all_task").unwrap(),
                visibility: DeclarationVisibility::Private,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: AuthoredType::Unit {},
                effect: AuthoredFunctionEffect::Task {
                    requirements: vec![requirement_reference],
                },
                body: task_body,
            },
        ],
    };
    let prepared = created
        .repository
        .prepare_authored_change(&request, PublicationOptions::default())
        .expect("every expression operation must lower and validate");
    for symbol in [
        "$pure_body",
        "$constant",
        "$if",
        "$let",
        "$local_use",
        "$call",
        "$function_value",
        "$invoke",
        "$record",
        "$field",
        "$variant",
        "$list",
        "$map",
        "$match",
        "$structural_record",
        "$structural_field",
        "$task_body",
        "$capability",
        "$transaction",
    ] {
        assert!(matches!(
            prepared.allocated[symbol],
            OwnerKey::Expression(_)
        ));
    }
    assert!(matches!(
        prepared.allocated["$transaction_binding"],
        OwnerKey::Binding(_)
    ));
    assert!(prepared.publication.receipt.validation.semantically_checked >= 2);
}

#[test]
fn locked_publication_accepts_once_reconciles_exact_retry_and_rejects_stale() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let prepared = prepare_body_publication(&created, PublicationOptions::default());
    assert!(prepared.authority.semantic.canonical_read_work.point_reads > 0);
    assert!(prepared.authority.semantic.canonical_read_work.point_reads < 16);
    assert!(
        prepared
            .authority
            .semantic
            .canonical_read_work
            .canonical_records_decoded
            > 0
    );
    assert!(
        prepared
            .authority
            .semantic
            .canonical_read_work
            .canonical_records_decoded
            < 16
    );
    assert!(prepared.authority.semantic.map_work.pages_read > 0);
    assert!(prepared.authority.semantic.map_work.pages_read < 64);

    let mut tampered = prepared.clone();
    tampered.receipt.intent = Some("changed after preparation".to_owned());
    let error = created
        .repository
        .publish(&tampered)
        .expect_err("commit must revalidate every typed prepared record");
    assert_eq!(error.code, "publication_repository_prepared_binding");
    assert_eq!(
        created.repository.current().unwrap().head,
        created.current.head
    );

    let mut forged = prepared.clone();
    forged.revision.publication.idempotency_receipts = created.current.semantic_root.owners;
    forged.receipt.result = forged.revision.core.revision_id().unwrap();
    let (receipt_digest, receipt_bytes) = forged.receipt.encode().unwrap();
    forged.revision.publication.receipt = receipt_digest;
    forged.revision = RevisionRecord::new(
        forged.revision.core.clone(),
        forged.revision.publication.clone(),
    )
    .unwrap();
    let (revision_digest, revision_bytes) = forged.revision.encode().unwrap();
    forged.head = HeadRecord {
        contract_version: contract::REVISION_CONTRACT_VERSION,
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        repository_id: forged.head.repository_id,
        revision: forged.revision.revision,
        record: revision_digest,
    };
    forged.receipt_digest = receipt_digest;
    forged.receipt_bytes = receipt_bytes.clone();
    forged.revision_digest = revision_digest;
    forged.revision_bytes = revision_bytes.clone();
    forged.head_bytes = forged.head.encode().unwrap();
    forged.accepted = AcceptedBinding::verify(
        forged.head,
        &forged.revision,
        forged.authority.witness.digest,
        &forged.authority.witness.manifest,
    )
    .unwrap();
    forged.objects.insert(
        ObjectKey::from_digest(ObjectDomain::Receipt, receipt_digest.bytes()),
        receipt_bytes,
    );
    forged.objects.insert(
        ObjectKey::from_digest(ObjectDomain::Revision, revision_digest.bytes()),
        revision_bytes,
    );
    assert_eq!(
        created
            .repository
            .reconcile(&forged)
            .expect_err("reconciliation must reject a non-derived prepared history root")
            .code,
        "publication_repository_idempotency_root"
    );
    let error = created
        .repository
        .publish(&forged)
        .expect_err("commit must derive the exact idempotency root before visibility");
    assert_eq!(error.code, "publication_repository_idempotency_root");
    assert_eq!(
        created.repository.current().unwrap().head,
        created.current.head,
        "a forged but internally consistent history root must publish nothing"
    );

    let outcome = created
        .repository
        .publish(&prepared)
        .expect("first publication");
    let PublicationOutcome::Accepted {
        current,
        seal,
        store_work,
    } = outcome
    else {
        panic!("first publication must advance HEAD")
    };
    assert_eq!(current.head, prepared.head);
    assert!(!seal.packs.is_empty());
    assert!(store_work.packs_sealed > 0);

    let retry = created
        .repository
        .publish(&prepared)
        .expect("exact retry reconciliation");
    let PublicationOutcome::AlreadyAccepted {
        accepted: replay,
        observed,
    } = retry
    else {
        panic!("exact retry must return original accepted publication")
    };
    assert_eq!(replay.head, prepared.head);
    assert_eq!(replay.receipt, prepared.receipt);
    assert_eq!(observed, prepared.head);

    let stale = prepare_rename_publication(&created, "stale_rename", None);
    let outcome = created
        .repository
        .publish(&stale)
        .expect("stale is a nonpublishing outcome");
    let PublicationOutcome::Stale {
        expected,
        current: observed,
    } = outcome
    else {
        panic!("competing publication from the old base must be stale")
    };
    assert_eq!(expected, Some(created.current.head));
    assert_eq!(observed, Some(prepared.head));
    let ReconciliationStatus::Stale {
        expected,
        current: observed,
    } = created.repository.reconcile(&stale).unwrap().status
    else {
        panic!("stale reconciliation must stay distinct")
    };
    assert_eq!(expected, Some(created.current.head));
    assert_eq!(observed, Some(prepared.head));
    assert_eq!(created.repository.current().unwrap().head, prepared.head);
}

#[test]
fn concurrent_readers_observe_only_old_or_new_complete_publications() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let prepared = prepare_body_publication(&created, PublicationOptions::default());
    let old = created.current.head;
    let new = prepared.head;
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(5));
    let mut readers = Vec::new();
    for _ in 0..4 {
        let repository = created.repository.clone();
        let barrier = barrier.clone();
        readers.push(std::thread::spawn(move || {
            barrier.wait();
            (0..16)
                .map(|_| repository.current().map(|current| current.head))
                .collect::<Result<Vec<_>, _>>()
        }));
    }
    barrier.wait();
    assert!(matches!(
        created.repository.publish(&prepared).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    for reader in readers {
        let observed = reader
            .join()
            .expect("reader thread must not panic")
            .expect("reader must observe complete authority");
        assert!(observed.into_iter().all(|head| head == old || head == new));
    }
    assert_eq!(created.repository.current().unwrap().head, new);
}

#[test]
fn idempotency_key_is_exactly_bound_to_base_and_normalized_transaction() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let options = PublicationOptions {
        idempotency_key: Some("request-1".to_owned()),
        intent: None,
    };
    let accepted = prepare_body_publication(&created, options.clone());
    let conflict = prepare_rename_publication(&created, "different_request", Some("request-1"));

    assert!(matches!(
        created.repository.publish(&accepted).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    assert!(matches!(
        created.repository.publish(&accepted).unwrap(),
        PublicationOutcome::AlreadyAccepted { .. }
    ));
    let exact_reconciliation = created
        .repository
        .reconcile(&accepted)
        .expect("current exact reconciliation");
    assert_eq!(exact_reconciliation.work.idempotency_lookup_pages_read, 0);
    assert_eq!(exact_reconciliation.work.accepted_publications_loaded, 0);
    assert!(matches!(
        exact_reconciliation.status,
        ReconciliationStatus::Accepted { .. }
    ));
    let error = created
        .repository
        .publish(&conflict)
        .expect_err("one key cannot identify two normalized transactions");
    assert_eq!(error.code, "publication_repository_idempotency_conflict");
    let conflict_reconciliation = created
        .repository
        .reconcile(&conflict)
        .expect("idempotency reconciliation");
    assert_eq!(
        conflict_reconciliation.work.idempotency_lookup_pages_read,
        0
    );
    assert_eq!(conflict_reconciliation.work.accepted_publications_loaded, 0);
    let ReconciliationStatus::ConflictingIdempotency { accepted: bound } =
        conflict_reconciliation.status
    else {
        panic!("idempotency conflict must remain a typed reconciliation outcome")
    };
    assert_eq!(bound, accepted.head);
}

#[test]
fn idempotency_reconciliation_survives_later_revisions_and_restart() {
    let temporary = tempfile::tempdir().expect("temporary idempotency history parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let callee = owner_named(&created.initial.snapshot, "callee");

    let first = prepare_body_publication(
        &created,
        PublicationOptions {
            idempotency_key: Some("request-1".to_owned()),
            intent: None,
        },
    );
    let conflict = prepare_rename_publication(&created, "conflicting_request", Some("request-1"));
    let absent = prepare_rename_publication(&created, "absent_request", Some("request-absent"));
    let PublicationOutcome::Accepted {
        current: first_current,
        ..
    } = created
        .repository
        .publish(&first)
        .expect("publish first key")
    else {
        panic!("first idempotent publication must advance HEAD")
    };
    assert_eq!(
        first_current
            .revision
            .publication
            .idempotency_receipts
            .entries(),
        0,
        "a revision excludes its own receipt from its ancestor index"
    );

    let second =
        prepare_current_rename(&created.repository, callee, "later_one", Some("request-2"));
    let mut missing_idempotency_page = second.clone();
    let idempotency_root_page = ObjectKey::from_digest(
        ObjectDomain::MapPage,
        second
            .revision
            .publication
            .idempotency_receipts
            .page()
            .bytes(),
    );
    assert!(
        missing_idempotency_page
            .objects
            .remove(&idempotency_root_page)
            .is_some(),
        "keyed parent advancement must stage its derived root page"
    );
    assert_eq!(
        created
            .repository
            .publish(&missing_idempotency_page)
            .expect_err("missing derived history page must reject before publication")
            .code,
        "publication_repository_idempotency_page_missing"
    );
    assert_eq!(created.repository.current().unwrap().head, first.head);
    let PublicationOutcome::Accepted {
        current: second_current,
        ..
    } = created
        .repository
        .publish(&second)
        .expect("publish second key")
    else {
        panic!("second idempotent publication must advance HEAD")
    };
    assert_eq!(
        second_current
            .revision
            .publication
            .idempotency_receipts
            .entries(),
        1
    );

    let third = prepare_current_rename(&created.repository, callee, "later_two", None);
    let PublicationOutcome::Accepted {
        current: third_current,
        ..
    } = created
        .repository
        .publish(&third)
        .expect("publish later revision")
    else {
        panic!("later publication must advance HEAD")
    };
    assert_eq!(
        third_current
            .revision
            .publication
            .idempotency_receipts
            .entries(),
        2
    );

    for (prepared, expected) in [(&first, first.head), (&second, second.head)] {
        let reconciliation = created
            .repository
            .reconcile(prepared)
            .expect("historical exact reconciliation");
        assert!(
            reconciliation.work.idempotency_lookup_pages_read > 0
                && reconciliation.work.idempotency_lookup_pages_read < 32,
            "historical reconciliation must use one bounded persistent-map lookup: {:?}",
            reconciliation.work
        );
        assert!(
            reconciliation.work.idempotency_lookup_entries_visited < 32,
            "historical reconciliation must not scan accepted history: {:?}",
            reconciliation.work
        );
        assert_eq!(reconciliation.work.accepted_publications_loaded, 1);
        assert!(
            reconciliation.work.objects_read < 64,
            "historical reconciliation must load bounded exact authority: {:?}",
            reconciliation.work
        );
        let ReconciliationStatus::Accepted { accepted, observed } = reconciliation.status else {
            panic!("historical idempotency key must resolve its original publication")
        };
        assert_eq!(accepted.head, expected);
        assert_eq!(observed, third.head);
    }
    let PublicationOutcome::AlreadyAccepted { accepted, observed } = created
        .repository
        .publish(&first)
        .expect("historical exact replay")
    else {
        panic!("historical replay must return the original acceptance")
    };
    assert_eq!(accepted.head, first.head);
    assert_eq!(observed, third.head);
    assert_eq!(created.repository.current().unwrap().head, third.head);

    let absent_reconciliation = created
        .repository
        .reconcile(&absent)
        .expect("absent historical key reconciliation");
    assert!(
        absent_reconciliation.work.idempotency_lookup_pages_read > 0
            && absent_reconciliation.work.idempotency_lookup_pages_read < 32
    );
    assert!(
        absent_reconciliation
            .work
            .idempotency_lookup_entries_visited
            < 32
    );
    assert!(absent_reconciliation.work.objects_read < 64);
    assert_eq!(absent_reconciliation.work.accepted_publications_loaded, 0);
    assert!(matches!(
        absent_reconciliation.status,
        ReconciliationStatus::Stale { .. }
    ));

    let conflict_reconciliation = created
        .repository
        .reconcile(&conflict)
        .expect("historical idempotency conflict");
    assert!(
        conflict_reconciliation.work.idempotency_lookup_pages_read > 0
            && conflict_reconciliation.work.idempotency_lookup_pages_read < 32
    );
    assert!(
        conflict_reconciliation
            .work
            .idempotency_lookup_entries_visited
            < 32
    );
    assert!(conflict_reconciliation.work.objects_read < 64);
    assert_eq!(
        conflict_reconciliation.work.accepted_publications_loaded, 0,
        "a conflicting historical binding does not need to load the accepted publication"
    );
    let ReconciliationStatus::ConflictingIdempotency { accepted } = conflict_reconciliation.status
    else {
        panic!("historical key reuse must remain a typed conflict")
    };
    assert_eq!(accepted, first.head);
    assert_eq!(
        created
            .repository
            .publish(&conflict)
            .expect_err("historical key reuse cannot publish")
            .code,
        "publication_repository_idempotency_conflict"
    );

    let reopened = GraphRepository::open(&destination).expect("reopen idempotency history");
    assert_eq!(reopened.current().unwrap().head, third.head);
    let reconciliation = reopened
        .reconcile(&first)
        .expect("reconcile historical key after restart");
    assert!(
        reconciliation.work.idempotency_lookup_pages_read > 0
            && reconciliation.work.idempotency_lookup_pages_read < 32
    );
    assert!(reconciliation.work.idempotency_lookup_entries_visited < 32);
    assert!(reconciliation.work.objects_read < 64);
    assert_eq!(reconciliation.work.accepted_publications_loaded, 1);
    let ReconciliationStatus::Accepted { accepted, observed } = reconciliation.status else {
        panic!("reopened repository must retain exact idempotency history")
    };
    assert_eq!(accepted.head, first.head);
    assert_eq!(observed, third.head);
}

#[test]
fn deterministic_owned_closure_publication_interruptions_reopen_old_or_new_complete_head() {
    for (ordinal, point, expects_new, leaves_head_stage) in [
        (0, PublicationPoint::BeforeObjectStage, false, false),
        (1, PublicationPoint::AfterFirstObjectStage, false, false),
        (2, PublicationPoint::AfterPacksSealed, false, false),
        (3, PublicationPoint::AfterHeadFileSynced, false, true),
        (4, PublicationPoint::AfterHeadRenamed, true, false),
        (5, PublicationPoint::AfterHeadDirectorySynced, true, false),
    ] {
        let temporary = tempfile::tempdir().expect("temporary repository parent");
        let destination = temporary.path().join(format!("meaning-{ordinal}"));
        let logical = crate::platform::kernel::tests::witness_snapshot();
        let created =
            GraphRepository::create(&destination, &logical, None).expect("create repository");
        let root = logical
            .owners
            .iter()
            .find_map(|(owner, record)| match record {
                OwnerRecord::Module(module) if module.name.as_str() == "second" => Some(*owner),
                _ => None,
            })
            .expect("second module");
        let request = AuthoredChangeSet {
            base: created.current.head.revision,
            preconditions: Vec::new(),
            budget: ChangeBudget::default(),
            changes: vec![AuthoredChange::DeleteOwner {
                owner: OwnerSelector::Exact { owner: root },
                policy: AuthoredDeletePolicy::OwnedClosure,
            }],
        };
        let prepared = created
            .repository
            .prepare_authored_change(&request, PublicationOptions::default())
            .expect("prepare interrupted ownership closure")
            .publication;
        let error = created
            .repository
            .publish_with_fault(&prepared, point)
            .expect_err("injected interruption");
        assert_eq!(
            error.code, "publication_repository_injected_interruption",
            "unexpected diagnostic at {point:?}"
        );

        let reopened = GraphRepository::open(&destination).expect("interrupted repository reopens");
        let current = reopened.current().expect("complete accepted current");
        assert_eq!(
            current.head,
            if expects_new {
                prepared.head
            } else {
                created.current.head
            },
            "wrong visible side of interruption at {point:?}"
        );
        assert_eq!(
            reopened.head_staging_leftovers().unwrap().len(),
            usize::from(leaves_head_stage),
            "unexpected HEAD-stage classification at {point:?}"
        );
        match reopened
            .reconcile(&prepared)
            .expect("exact reconciliation")
            .status
        {
            ReconciliationStatus::Accepted { accepted, observed } if expects_new => {
                assert_eq!(accepted.head, prepared.head);
                assert_eq!(observed, prepared.head);
            }
            ReconciliationStatus::NotStarted { current } if !expects_new => {
                assert_eq!(current, Some(created.current.head));
            }
            other => panic!("unexpected reconciliation {other:?} at {point:?}"),
        }
        if expects_new {
            assert!(matches!(
                reopened.publish(&prepared).unwrap(),
                PublicationOutcome::AlreadyAccepted { .. }
            ));
        }
    }
}

#[test]
fn packed_store_interruptions_keep_head_old_until_exact_retry() {
    for checkpoint in SealCheckpoint::ALL {
        let temporary = tempfile::tempdir().expect("temporary storage interruption parent");
        let destination = temporary.path().join(checkpoint.name());
        let logical = crate::platform::kernel::tests::witness_snapshot();
        let created =
            GraphRepository::create(&destination, &logical, None).expect("create repository");
        let prepared = prepare_body_publication(
            &created,
            PublicationOptions {
                idempotency_key: Some(format!("storage-{}", checkpoint.name())),
                intent: None,
            },
        );

        let error = created
            .repository
            .publish_with_fault(&prepared, PublicationPoint::Storage(checkpoint))
            .expect_err("storage checkpoint must interrupt before HEAD publication");
        assert_eq!(error.code, "pack_store_injected_interruption");

        let reopened = GraphRepository::open(&destination).expect("interrupted repository reopens");
        assert_eq!(
            reopened.current().unwrap().head,
            created.current.head,
            "storage interruption advanced HEAD at {checkpoint:?}"
        );
        assert!(reopened.head_staging_leftovers().unwrap().is_empty());
        let reconciliation = reopened
            .reconcile(&prepared)
            .expect("interrupted publication must reconcile exactly");
        assert!(matches!(
            reconciliation.status,
            ReconciliationStatus::NotStarted {
                current: Some(current)
            } if current == created.current.head
        ));

        let PublicationOutcome::Accepted { current, .. } = reopened
            .publish(&prepared)
            .expect("exact retry must publish safely")
        else {
            panic!("exact retry did not publish at {checkpoint:?}")
        };
        assert_eq!(current.head, prepared.head);
        assert_eq!(reopened.current().unwrap().head, prepared.head);
    }
}

#[test]
fn repository_rejects_predecessor_head_and_missing_accepted_pack() {
    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let predecessor = temporary.path().join("predecessor");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let _ = GraphRepository::create(&predecessor, &logical, None).expect("create repository");
    let mut head = std::fs::read(predecessor.join("HEAD")).expect("HEAD bytes");
    head[..8].copy_from_slice(b"LKJHEAD6");
    std::fs::write(predecessor.join("HEAD"), head).expect("replace test HEAD");
    assert_eq!(
        GraphRepository::open(&predecessor)
            .expect_err("predecessor HEAD must reject")
            .code,
        "packed_contract"
    );

    let missing = temporary.path().join("missing-pack");
    let _ = GraphRepository::create(&missing, &logical, None).expect("create repository");
    for entry in std::fs::read_dir(missing.join("packs")).expect("pack directory") {
        let path = entry.expect("pack entry").path();
        std::fs::remove_file(path).expect("remove accepted test pack");
    }
    assert_eq!(
        GraphRepository::open(&missing)
            .expect_err("missing accepted pack must reject")
            .code,
        "publication_repository_object_missing"
    );
}

#[cfg(unix)]
#[test]
fn repository_rejects_symlinked_lock_and_head_stage() {
    use std::os::unix::fs::symlink;

    let temporary = tempfile::tempdir().expect("temporary repository parent");
    let destination = temporary.path().join("meaning");
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create(&destination, &logical, None).expect("create repository");
    let target = destination.join("lock-target");
    std::fs::rename(destination.join("LOCK"), &target).expect("move real lock");
    symlink(&target, destination.join("LOCK")).expect("symlink lock");
    assert!(created.repository.current().is_err());

    std::fs::remove_file(destination.join("LOCK")).expect("remove lock symlink");
    std::fs::rename(&target, destination.join("LOCK")).expect("restore lock");
    let stage_target = destination.join("stage-target");
    std::fs::write(&stage_target, b"not a HEAD").expect("stage target");
    symlink(
        &stage_target,
        destination.join(".HEAD-stage-hostile-symlink"),
    )
    .expect("stage symlink");
    assert_eq!(
        created
            .repository
            .head_staging_leftovers()
            .expect_err("symlinked HEAD stage must reject")
            .code,
        "publication_repository_stage_type"
    );
}

fn prepare_body_publication(
    created: &CreatedRepository,
    options: PublicationOptions,
) -> PreparedPublication {
    let body = function_body(&created.initial.snapshot, "callee");
    let mut replacement = created.initial.snapshot.owners[&body].clone();
    let OwnerRecord::Expression(expression) = &mut replacement else {
        panic!("callee body must be an expression")
    };
    expression.operation = ExpressionOperation::Unit {};
    let expected = encode_owner(&created.initial.snapshot.owners[&body])
        .expect("base body encoding")
        .0;
    created
        .repository
        .prepare_change(
            vec![PrimitiveEdit::ReplaceOwner {
                expected,
                record: replacement,
            }],
            options,
        )
        .expect("prepare current repository change")
}

fn prepare_rename_publication(
    created: &CreatedRepository,
    name: &str,
    idempotency_key: Option<&str>,
) -> PreparedPublication {
    let callee = owner_named(&created.initial.snapshot, "callee");
    let mut replacement = created.initial.snapshot.owners[&callee].clone();
    let OwnerRecord::Declaration(declaration) = &mut replacement else {
        panic!("callee must be a declaration")
    };
    declaration.name = Name::new(name).expect("replacement name");
    let expected = encode_owner(&created.initial.snapshot.owners[&callee])
        .expect("base declaration encoding")
        .0;
    prepare_publication(
        created,
        vec![PrimitiveEdit::ReplaceOwner {
            expected,
            record: replacement,
        }],
        PublicationOptions {
            idempotency_key: idempotency_key.map(str::to_owned),
            intent: None,
        },
    )
}

fn prepare_current_rename(
    repository: &GraphRepository,
    owner: OwnerKey,
    name: &str,
    idempotency_key: Option<&str>,
) -> PreparedPublication {
    let view = repository.view_current().expect("current rename view");
    let Some(mut replacement) = view.owner(owner).unwrap().value else {
        panic!("current rename owner must exist")
    };
    let expected = encode_owner(&replacement)
        .expect("current owner encoding")
        .0;
    let OwnerRecord::Declaration(declaration) = &mut replacement else {
        panic!("current rename owner must be a declaration")
    };
    declaration.name = Name::new(name).expect("current replacement name");
    view.prepare_change(
        vec![PrimitiveEdit::ReplaceOwner {
            expected,
            record: replacement,
        }],
        PublicationOptions {
            idempotency_key: idempotency_key.map(str::to_owned),
            intent: None,
        },
    )
    .expect("prepare current rename")
}

fn prepare_publication(
    created: &CreatedRepository,
    edits: Vec<PrimitiveEdit>,
    options: PublicationOptions,
) -> PreparedPublication {
    let view = super::RepositoryView::new(
        created.current.clone(),
        created
            .repository
            .object_store()
            .expect("pinned repository store"),
    );
    view.prepare_change(edits, options)
        .expect("prepared repository publication")
}

fn empty_snapshot(seed: &[u8]) -> crate::platform::kernel::KernelSnapshot {
    let empty = MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        0,
        crate::platform::persistent_map::MapContentDigest::from_bytes([0; 32]),
    );
    crate::platform::kernel::KernelSnapshot {
        root: SemanticRoot {
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(seed, 0),
            package_id: PackageId::migrate(seed, 0),
            package_name: Name::new("empty").expect("empty fixture package name"),
            owners: empty,
            dependencies: empty,
            retirements: empty,
        },
        owners: std::collections::BTreeMap::new(),
        types: std::collections::BTreeMap::new(),
        dependency_interfaces: std::collections::BTreeMap::new(),
        dependency_types: std::collections::BTreeMap::new(),
        blobs: std::collections::BTreeMap::new(),
        dependencies: std::collections::BTreeMap::new(),
        retirements: std::collections::BTreeMap::new(),
    }
}

fn owner_named(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Declaration(declaration) if declaration.name.as_str() == name => {
                Some(*owner)
            }
            _ => None,
        })
        .expect("named declaration")
}

fn field_named(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Field(field) if field.name.as_str() == name => Some(*owner),
            _ => None,
        })
        .expect("named field")
}

fn case_named(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Case(case) if case.name.as_str() == name => Some(*owner),
            _ => None,
        })
        .expect("named case")
}

fn requirement_named(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Requirement(requirement) if requirement.name.as_str() == name => {
                Some(*owner)
            }
            _ => None,
        })
        .expect("named requirement")
}

fn operation_named(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Operation(operation) if operation.name.as_str() == name => Some(*owner),
            _ => None,
        })
        .expect("named operation")
}

fn target_named(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Target(target) if target.name.as_str() == name => Some(*owner),
            _ => None,
        })
        .expect("named target")
}

fn authored_exact_declaration(package: PackageId, owner: OwnerKey) -> AuthoredDeclarationReference {
    let OwnerKey::Declaration(declaration) = owner else {
        panic!("declaration helper returned a foreign owner")
    };
    AuthoredDeclarationReference::Exact {
        package,
        declaration,
    }
}

fn authored_expression(operation: AuthoredExpressionOperation) -> AuthoredExpression {
    AuthoredExpression {
        symbol: None,
        operation,
    }
}

fn authored_parameter(symbol: &str, name: &str, ty: AuthoredType) -> AuthoredParameter {
    AuthoredParameter {
        symbol: symbol.to_owned(),
        name: Name::new(name).expect("parameter name"),
        ty,
    }
}

fn function_body(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    let declaration = owner_named(snapshot, name);
    let OwnerRecord::Declaration(record) = &snapshot.owners[&declaration] else {
        panic!("named owner must be a declaration")
    };
    let DeclarationPayload::Function(function) = &record.payload else {
        panic!("named declaration must be a function")
    };
    OwnerKey::Expression(function.body)
}

fn test_actual(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    let declaration = owner_named(snapshot, name);
    let OwnerRecord::Declaration(record) = &snapshot.owners[&declaration] else {
        panic!("named owner must be a declaration")
    };
    let DeclarationPayload::Test { actual, .. } = &record.payload else {
        panic!("named declaration must be a test")
    };
    OwnerKey::Expression(*actual)
}

fn binding_named(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Binding(binding) if binding.name.as_str() == name => Some(*owner),
            _ => None,
        })
        .expect("named binding")
}

fn expression_calling(
    snapshot: &crate::platform::kernel::KernelSnapshot,
    declaration: OwnerKey,
) -> OwnerKey {
    let OwnerKey::Declaration(declaration) = declaration else {
        panic!("call target must be a declaration")
    };
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Expression(expression)
                if matches!(
                    expression.operation,
                    ExpressionOperation::Call { function, .. }
                        if function.declaration == declaration
                ) =>
            {
                Some(*owner)
            }
            _ => None,
        })
        .expect("exact declaration call")
}

fn expression_using_binding(
    snapshot: &crate::platform::kernel::KernelSnapshot,
    binding: OwnerKey,
) -> OwnerKey {
    let OwnerKey::Binding(binding) = binding else {
        panic!("local reference target must be a binding")
    };
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Expression(expression)
                if expression.operation
                    == (ExpressionOperation::Local {
                        value: LocalValueReference::LexicalBinding(binding),
                    }) =>
            {
                Some(*owner)
            }
            _ => None,
        })
        .expect("exact binding use")
}

fn complete_ownership_descendants(
    witness: &crate::platform::witness::FullWitness,
    root: OwnerKey,
) -> BTreeSet<OwnerKey> {
    witness
        .entries
        .ownership
        .keys()
        .copied()
        .filter(|owner| {
            let mut current = *owner;
            for _ in 0..=witness.entries.ownership.len() {
                if current == root {
                    return true;
                }
                let Some(entry) = witness.entries.ownership.get(&current) else {
                    return false;
                };
                let OwnershipParent::Owner(parent) = entry.parent else {
                    return false;
                };
                current = parent;
            }
            false
        })
        .collect()
}

fn logical_plan_decode_error(bytes: &[u8]) -> String {
    crate::platform::control::decode_logical_change_plan(std::io::Cursor::new(bytes))
        .expect_err("mutated logical plan must reject")
        .code
}
