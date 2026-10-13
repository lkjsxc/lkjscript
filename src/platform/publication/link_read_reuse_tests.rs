//! Actual accepted repositories/exports exercise the scoped reader; faults alter only owned copies.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
use super::*;
use crate::platform::change::{AuthoredChange, AuthoredChangeSet, ChangeBudget};
use crate::platform::execution::ExecutionControl;
use crate::platform::kernel::{Name, SemanticRoot};
use crate::platform::persistent_map::{MapContentDigest, PageDigest};
use crate::platform::publication::GraphRepository;
use crate::platform::semantic_id::RepositoryId;

fn empty(seed: &[u8]) -> KernelSnapshot {
    let map = MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        0,
        MapContentDigest::from_bytes([0; 32]),
    );
    KernelSnapshot {
        root: SemanticRoot {
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(seed, 0),
            package_id: PackageId::migrate(seed, 0),
            package_name: Name::new("scoped_link_fixture").unwrap(),
            owners: map,
            dependencies: map,
            retirements: map,
        },
        owners: BTreeMap::new(),
        types: BTreeMap::new(),
        dependency_interfaces: BTreeMap::new(),
        dependency_types: BTreeMap::new(),
        representation_dependencies: BTreeMap::new(),
        blobs: BTreeMap::new(),
        dependencies: BTreeMap::new(),
        retirements: BTreeMap::new(),
    }
}

fn author(repository: &GraphRepository, body: &str) {
    let input = format!(
        "request base={}\n{body}",
        repository.current().unwrap().head.revision
    );
    let decoded =
        crate::platform::control::decode_compact_change("link-reader", input.as_bytes()).unwrap();
    let prepared = repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap();
    repository.publish(&prepared.publication).unwrap();
}

fn import(
    repository: &GraphRepository,
    exported: &super::super::ExportedPackageTransport,
) -> DependencyRecord {
    repository
        .stage_package_transport(exported.transport_digest, &exported.container)
        .unwrap();
    let dependency = DependencyRecord {
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        package: exported.revision.package,
        semantic_revision: exported.revision.revision.revision_id().unwrap(),
        package_revision: exported.revision_digest,
    };
    let prepared = repository
        .prepare_authored_change(
            &AuthoredChangeSet {
                base: repository.current().unwrap().head.revision,
                preconditions: Vec::new(),
                budget: ChangeBudget::default(),
                changes: vec![AuthoredChange::AddDependency {
                    package: dependency.package,
                    semantic_revision: dependency.semantic_revision,
                    package_revision: dependency.package_revision,
                }],
            },
            PublicationOptions::default(),
        )
        .unwrap();
    repository.publish(&prepared.publication).unwrap();
    dependency
}

struct Fixture {
    directory: tempfile::TempDir,
    repository: GraphRepository,
    supplier: super::super::ExportedPackageTransport,
    dependency: DependencyRecord,
    owner: OwnerKey,
}

fn fixture() -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let supplier_repository = GraphRepository::create(
        &directory.path().join("supplier"),
        &empty(b"link-supplier"),
        None,
    )
    .unwrap()
    .repository;
    author(
        &supplier_repository,
        "declarations.begin\n(units (module create library (function create value (visibility public) (returns I64) (effect pure) (body (i64 41))) (function create unused (visibility public) (returns I64) (effect pure) (body (i64 77)))))\ndeclarations.end\n",
    );
    let source = supplier_repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    let owner = source
        .owners
        .iter()
        .find_map(|(key, value)| {
            matches!(value, OwnerRecord::Declaration(d) if d.name.as_str()=="value").then_some(*key)
        })
        .unwrap();
    let supplier = supplier_repository.export_package_transport().unwrap();
    let repository = GraphRepository::create(
        &directory.path().join("consumer"),
        &empty(b"link-consumer"),
        None,
    )
    .unwrap()
    .repository;
    let dependency = import(&repository, &supplier);
    Fixture {
        directory,
        repository,
        supplier,
        dependency,
        owner,
    }
}

fn associated_fixture() -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let supplier_repository = GraphRepository::create(
        &directory.path().join("associated-supplier"),
        &empty(b"link-associated-supplier"),
        None,
    )
    .unwrap()
    .repository;
    author(
        &supplier_repository,
        r#"declarations.begin
(units (module create library
  (owned-contract create Carrier (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (associated-type assoc_type_a4000000000000000000000000000001 Spare (bound owned))
    (method method_a4000000000000000000000000000001 pass
      (parameters (Self consume)) (returns Self) (effect pure)))
  (function create pass (visibility public) (effect pure)
    (parameter create data (type (owned-sequence I64)) (use consume))
    (returns (owned-sequence I64)) (body (local data)))
  (owned-implementation create Bound (visibility public)
    (contract Carrier) (self (owned-sequence I64))
    (associated-type assoc_type_a4000000000000000000000000000001 ByteBuffer)
    (method method_a4000000000000000000000000000001 pass))))
declarations.end
"#,
    );
    let source = supplier_repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    let owner = source
        .owners
        .iter()
        .find_map(|(key, value)| {
            matches!(value, OwnerRecord::Declaration(d) if d.name.as_str()=="Bound").then_some(*key)
        })
        .unwrap();
    let supplier = supplier_repository.export_package_transport().unwrap();
    let repository = GraphRepository::create(
        &directory.path().join("associated-consumer"),
        &empty(b"link-associated-consumer"),
        None,
    )
    .unwrap()
    .repository;
    let dependency = import(&repository, &supplier);
    Fixture {
        directory,
        repository,
        supplier,
        dependency,
        owner,
    }
}

#[test]
fn scoped_link_reads_reuse_complete_exact_closure_without_changing_records() {
    let f = fixture();
    let view = f.repository.view_current().unwrap();
    let cold = view
        .package_interface_owner_from_dependency(&f.dependency, f.owner)
        .unwrap();
    let ((first, second), final_work, observation) = view
        .with_link_read_reuse(|| {
            let first = view.package_interface_owner_from_dependency(&f.dependency, f.owner)?;
            let second = view.package_interface_owner_from_dependency(&f.dependency, f.owner)?;
            assert_eq!(
                view.link_transport_reuse
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .entries
                    .len(),
                1
            );
            let closure = view.link_transport_reuse.borrow();
            assert_eq!(
                closure.as_ref().unwrap().entries[&f.dependency.package_revision]
                    .closure
                    .root_interface
                    .owners
                    .len(),
                2,
                "unused public member retained"
            );
            Ok((first, second))
        })
        .unwrap();
    assert_eq!(first.value, cold.value);
    assert_eq!(second.value, cold.value);
    assert!(first.work.store.objects_read > 0);
    assert_eq!(
        second.work.store.objects_read, 0,
        "warm selector never re-admits the complete closure"
    );
    assert_eq!(second.work.canonical_records_decoded, 1);
    assert_eq!(observation.transport_misses, 1);
    assert_eq!(observation.transport_hits, 1);
    assert_eq!(observation.retained_closures, 1);
    assert_eq!(observation.rechecked_objects, final_work.store.objects_read);
    assert!(second.work.map.entries_visited > 0);
    assert!(
        final_work.store.objects_read > 0,
        "original read set rechecked physically"
    );
    assert!(view.link_transport_reuse.borrow().is_none());
    view.with_link_read_reuse(|| {
        view.package_interface_owner_from_dependency(&f.dependency, f.owner)?;
        let OwnerKey::Declaration(_) = f.owner else {
            panic!("declaration");
        };
        let absent = view.package_interface_owner_from_dependency(
            &f.dependency,
            OwnerKey::Declaration(crate::platform::semantic_id::DeclarationId::migrate(
                b"missing-owner",
                0,
            )),
        )?;
        assert!(absent.value.is_none());
        assert_eq!(absent.work.canonical_records_decoded, 0);
        assert!(absent.work.map.entries_visited > 0);
        Ok(())
    })
    .unwrap();
    let again = view
        .package_interface_owner_from_dependency(&f.dependency, f.owner)
        .unwrap();
    assert_eq!(again.value, cold.value);
    assert!(
        again.work.store.objects_read > 0,
        "proof does not escape invocation"
    );
    view.with_link_read_reuse(|| {
        // Capacity controls optional retention, never source validity.
        view.link_transport_reuse
            .borrow_mut()
            .as_mut()
            .unwrap()
            .retained_bytes = MAXIMUM_RETAINED_BYTES;
        let uncached = view.package_interface_owner_from_dependency(&f.dependency, f.owner)?;
        assert_eq!(uncached.value, cold.value);
        assert!(
            view.link_transport_reuse
                .borrow()
                .as_ref()
                .unwrap()
                .entries
                .is_empty()
        );
        Ok(())
    })
    .unwrap();
    f.directory.close().unwrap();

    // A complete publicly authored nonempty associated contract and implementation
    // require the explicit canonical envelope. Generic logical DTO encoding is
    // intentionally forbidden; neither unused inventory may break copy accounting.
    let a = associated_fixture();
    let view = a.repository.view_current().unwrap();
    let cold = view
        .package_interface_owner_from_dependency(&a.dependency, a.owner)
        .unwrap();
    let ((first, second), _, observation) = view.with_link_read_reuse(|| {
        let first = view.package_interface_owner_from_dependency(&a.dependency, a.owner)?;
        let second = view.package_interface_owner_from_dependency(&a.dependency, a.owner)?;
        let cache = view.link_transport_reuse.borrow();
        let entry = &cache.as_ref().unwrap().entries[&a.dependency.package_revision];
        let mut associated_contracts = 0;
        let mut associated_implementations = 0;
        for (owner, record) in &entry.closure.root_interface.owners {
            let associated = match &record.record {
                PackageInterfaceRecord::Declaration(declaration) => match &declaration.payload {
                    crate::platform::kernel::PackageInterfaceDeclarationPayload::OwnedContract(c) => {
                        assert_eq!(c.associated_types.len(), 1);
                        assert_eq!(c.associated_types[0].name.as_str(), "Spare");
                        associated_contracts += 1;
                        true
                    }
                    crate::platform::kernel::PackageInterfaceDeclarationPayload::OwnedImplementation(i) => {
                        assert_eq!(i.associated_types.len(), 1);
                        let byte_buffer = crate::platform::kernel::encode_type_object(
                            &crate::platform::kernel::TypeObject::new(
                                crate::platform::kernel::TypeForm::ByteBuffer,
                            ).unwrap(),
                        )?.0;
                        assert_eq!(i.associated_types[0].ty(), byte_buffer, "unused binding retained");
                        associated_implementations += 1;
                        true
                    }
                    _ => false,
                },
                _ => false,
            };
            if associated {
                assert!(bincode::encode_to_vec(record, bincode::config::standard()).is_err());
                let (_, encoded) = record.encode()?;
                assert_eq!(&encoded[..8], &crate::platform::package_interface::ASSOCIATED_INTERFACE_MAGIC);
                assert_eq!(entry.owner_copy_work[owner], encoded.len() as u64);
            }
        }
        assert_eq!((associated_contracts, associated_implementations), (1, 1));
        Ok((first, second))
    }).unwrap();
    assert_eq!(first.value, cold.value);
    assert_eq!(second.value, cold.value);
    assert_eq!(second.work.store.objects_read, 0);
    assert_eq!(second.work.canonical_records_decoded, 1);
    assert_eq!(
        (observation.transport_misses, observation.transport_hits),
        (1, 1)
    );
    assert!(view.link_transport_reuse.borrow().is_none());
    a.directory.close().unwrap();
}

#[test]
fn scoped_link_reads_preserve_rooted_representation_and_exact_dependency_authority() {
    let f = fixture();
    let wrapper = GraphRepository::create(
        &f.directory.path().join("wrapper"),
        &empty(b"link-wrapper"),
        None,
    )
    .unwrap()
    .repository;
    import(&wrapper, &f.supplier);
    let wrapper_export = wrapper.export_package_transport().unwrap();
    let root = GraphRepository::create(
        &f.directory.path().join("outer"),
        &empty(b"link-outer"),
        None,
    )
    .unwrap()
    .repository;
    let direct = import(&root, &wrapper_export);
    let view = root.view_current().unwrap();
    view.with_link_read_reuse(|| {
        let represented = view
            .read_representation_dependency(&direct, f.dependency.package)?
            .value
            .unwrap();
        assert_eq!(represented.package_revision, f.dependency.package_revision);
        assert!(
            view.read_package_interface_owner(&represented, f.owner)?
                .value
                .is_some()
        );
        assert!(view.read_dependency(f.dependency.package)?.value.is_none());
        assert!(
            view.package_interface_owner(f.dependency.package, f.owner)?
                .value
                .is_none(),
            "representation is not a direct import"
        );
        assert!(
            view.read_representation_dependency(&direct, PackageId::migrate(b"unrelated", 0))?
                .value
                .is_none()
        );
        let mut wrong = direct.clone();
        wrong.semantic_revision = RevisionId::from_digest([0xdd; 32]);
        let error = view
            .read_representation_dependency(&wrong, f.dependency.package)
            .expect_err("wrong retained root must refuse");
        assert_eq!(error.class, DiagnosticClass::Semantic);
        assert_eq!(error.code, "package_revision_dependency_binding");
        Ok(())
    })
    .unwrap();
}

#[test]
fn scoped_link_reads_keep_cumulative_limits_cancellation_and_cleanup() {
    let f = fixture();
    let control = ExecutionControl::uncancelled();
    let view = f.repository.view_current_with_control(&control).unwrap();
    let (_, observed, _) = view
        .with_link_read_reuse(|| {
            view.package_interface_owner_from_dependency(&f.dependency, f.owner)?;
            view.package_interface_owner_from_dependency(&f.dependency, f.owner)?;
            Ok(())
        })
        .unwrap();
    let copy_cost = view
        .with_link_read_reuse(|| {
            view.package_interface_owner_from_dependency(&f.dependency, f.owner)?;
            let weight = view.link_transport_reuse.borrow().as_ref().unwrap().entries
                [&f.dependency.package_revision]
                .owner_copy_work[&f.owner];
            Ok(weight)
        })
        .unwrap()
        .0;
    let exact = usize::try_from(
        copy_cost + 3 + observed.store.objects_read + observed.map.entries_visited + 1,
    )
    .unwrap();
    for allowance in [exact, exact - 1] {
        let result = view.with_link_read_reuse(|| {
            view.package_interface_owner_from_dependency(&f.dependency, f.owner)?;
            view.link_transport_reuse
                .borrow_mut()
                .as_mut()
                .unwrap()
                .remaining_work = allowance;
            view.package_interface_owner_from_dependency(&f.dependency, f.owner)?;
            Ok(())
        });
        if allowance == exact {
            assert!(result.is_ok());
        } else {
            assert_eq!(result.err().unwrap().class, DiagnosticClass::Resource);
        }
        assert!(view.link_transport_reuse.borrow().is_none());
    }
    let cancelled = view
        .with_link_read_reuse(|| {
            view.package_interface_owner_from_dependency(&f.dependency, f.owner)?;
            control.cancel();
            view.package_interface_owner_from_dependency(&f.dependency, f.owner)
        })
        .err()
        .unwrap();
    assert_eq!(cancelled.class, DiagnosticClass::Cancelled);
    assert!(view.link_transport_reuse.borrow().is_none());
    let healthy = f.repository.view_current().unwrap();
    healthy
        .with_link_read_reuse(|| {
            healthy.package_interface_owner_from_dependency(&f.dependency, f.owner)
        })
        .unwrap();
    let missing = PackageRevisionDigest::from_bytes([0xce; 32]);
    healthy
        .with_link_read_reuse(|| {
            let mut wrong = f.dependency.clone();
            wrong.package_revision = missing;
            let error = healthy
                .package_interface_owner_from_dependency(&wrong, f.owner)
                .expect_err("failed first miss");
            assert!(
                healthy
                    .link_transport_reuse
                    .borrow()
                    .as_ref()
                    .unwrap()
                    .entries
                    .is_empty()
            );
            Err::<(), _>(error)
        })
        .err()
        .unwrap();
    assert!(healthy.link_transport_reuse.borrow().is_none());
    healthy
        .with_link_read_reuse(|| {
            healthy.package_interface_owner_from_dependency(&f.dependency, f.owner)
        })
        .unwrap();
}

#[test]
fn scoped_link_reads_recheck_original_objects_and_selected_readiness_only() {
    let f = fixture();
    let view = f.repository.view_current().unwrap();
    let ready_path = f.repository.root().join("PACKAGE-TRANSPORTS/CURRENT");
    let original = std::fs::read(&ready_path).unwrap();
    let error = view
        .with_link_read_reuse(|| {
            view.package_interface_owner_from_dependency(&f.dependency, f.owner)?;
            let mut ready =
                super::super::super::repository::read_package_readiness(f.repository.root())?;
            ready.bindings.remove(&f.dependency.package_revision);
            std::fs::write(&ready_path, ready.encode()?).unwrap();
            Ok(())
        })
        .err()
        .unwrap();
    assert_eq!(error.code, "publication_link_read_selection_changed");
    assert!(view.link_transport_reuse.borrow().is_none());
    std::fs::write(&ready_path, &original).unwrap();
    view.with_link_read_reuse(|| {
        view.package_interface_owner_from_dependency(&f.dependency, f.owner)?;
        let mut ready =
            super::super::super::repository::read_package_readiness(f.repository.root())?;
        ready.bindings.insert(
            PackageRevisionDigest::from_bytes([0xca; 32]),
            PackageTransportDigest::from_bytes([0xba; 32]),
        );
        std::fs::write(&ready_path, ready.encode()?).unwrap();
        Ok(())
    })
    .unwrap();
    std::fs::write(&ready_path, &original).unwrap();
    let packs = f.repository.root().join("packs");
    let names = std::fs::read_dir(&packs)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect::<Vec<_>>();
    let error = view
        .with_link_read_reuse(|| {
            view.package_interface_owner_from_dependency(&f.dependency, f.owner)?;
            for file in &names {
                std::fs::remove_file(file).unwrap();
            }
            Ok(())
        })
        .err()
        .unwrap();
    assert!(matches!(
        error.class,
        DiagnosticClass::Corrupt | DiagnosticClass::Infrastructure
    ));
    assert!(view.link_transport_reuse.borrow().is_none());
}

#[test]
fn scoped_link_reads_do_not_consult_readiness_without_retained_imports() {
    let directory = tempfile::tempdir().unwrap();
    let repository =
        GraphRepository::create(&directory.path().join("empty"), &empty(b"link-empty"), None)
            .unwrap()
            .repository;
    let view = repository.view_current().unwrap();
    let ready = repository.root().join("PACKAGE-TRANSPORTS");
    std::fs::create_dir_all(&ready).unwrap();
    std::fs::write(ready.join("CURRENT"), b"unrelated malformed readiness").unwrap();
    let (_, work, observation) = view.with_link_read_reuse(|| Ok(())).unwrap();
    assert_eq!(work.store.objects_read, 0);
    assert_eq!(work.map.entries_visited, 0);
    assert_eq!(observation, LinkReadReuseObservation::default());
    assert!(view.link_transport_reuse.borrow().is_none());
}

#[test]
fn scoped_link_reuse_preserves_native_immutable_bytes_and_actual_control_forwarding() {
    let f = fixture();
    let OwnerKey::Declaration(declaration) = f.owner else {
        panic!("function declaration");
    };
    author(
        &f.repository,
        &format!(
            "declarations.begin\n(units (module create app (function create invoke (visibility public) (returns I64) (effect pure) (body (call {}/{})))))\ndeclarations.end\n",
            f.dependency.package, declaration
        ),
    );
    let closure = f.repository.export_package_container().unwrap();
    let supplier = &closure.packages[&f.dependency.package_revision];
    let dependency = crate::platform::compiler::load_artifact(
        &crate::platform::compiler::compile_immutable(supplier, &closure.container.objects, &[])
            .unwrap()
            .artifact
            .bytes,
    )
    .unwrap();
    let root = &closure.packages[&closure.container.root.package_revision];
    let expected = crate::platform::compiler::compile_immutable(
        root,
        &closure.container.objects,
        std::slice::from_ref(&dependency),
    )
    .unwrap();
    let built = crate::platform::compiler::build_clean(
        &f.repository,
        crate::platform::compiler::OptimizationPolicy::DeterministicBaseline,
    )
    .unwrap();
    let linked = crate::platform::compiler::link_artifact(
        &f.repository,
        built.manifest_digest,
        std::slice::from_ref(&dependency),
    )
    .unwrap();
    assert_eq!(linked.artifact.bytes, expected.artifact.bytes);
    crate::platform::compiler::load_artifact(&linked.artifact.bytes).unwrap();
    let cancelled = ExecutionControl::cancel_after_checks(0);
    let error = crate::platform::compiler::link_artifact_with_control(
        &f.repository,
        built.manifest_digest,
        std::slice::from_ref(&dependency),
        &cancelled,
    )
    .err()
    .unwrap();
    assert_eq!(error.class, DiagnosticClass::Cancelled);
    let linked = crate::platform::compiler::link_artifact_with_control(
        &f.repository,
        built.manifest_digest,
        std::slice::from_ref(&dependency),
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert_eq!(linked.artifact.bytes, expected.artifact.bytes);
}
