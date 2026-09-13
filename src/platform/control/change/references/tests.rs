use super::*;
use crate::platform::kernel::*;
use crate::platform::publication::{GraphRepository, PreparedAuthoredPublication};
use crate::platform::semantic_id::*;

const SEED: &[u8] = b"graph-5-normalization-prototype";

fn empty(seed: &[u8]) -> KernelSnapshot {
    use crate::platform::persistent_map::{MapContentDigest, MapRoot, PageDigest};
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    snapshot.owners.clear();
    snapshot.types.clear();
    snapshot.blobs.clear();
    snapshot.root.repository_id = RepositoryId::migrate(seed, 0);
    snapshot.root.package_id = PackageId::migrate(seed, 0);
    snapshot.root.owners = MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        0,
        MapContentDigest::from_bytes([0; 32]),
    );
    snapshot
}

fn prepare(
    repository: &GraphRepository,
    prelude: &str,
    edits: &str,
) -> Result<(NormalizedChangeRequest, PreparedAuthoredPublication), Vec<Diagnostic>> {
    let request = format!(
        "request base={} idempotency=named-test\n{prelude}{edits}",
        repository.current().unwrap().head.revision
    );
    let request = decode_compact_change("named.lkjc", request.as_bytes())?;
    let prepared =
        repository.prepare_authored_change(&request.semantic, request.options.clone())?;
    Ok((request, prepared))
}

fn encoded(
    request: &NormalizedChangeRequest,
    prepared: &PreparedAuthoredPublication,
) -> (String, String) {
    use crate::platform::control::{LogicalChangePlan, encode_logical_change_plan};
    let plan = LogicalChangePlan::new(request.request_commitment, prepared).unwrap();
    let mut bytes = Vec::new();
    let encoding = encode_logical_change_plan(&plan, |part| {
        bytes.extend_from_slice(part);
        Ok(())
    })
    .unwrap();
    (
        encoding.token.to_string(),
        String::from_utf8(bytes).unwrap(),
    )
}

#[test]
fn named_reference_local_namespace_oracle_alpha_order_and_base_rename() {
    let temporary = tempfile::tempdir().unwrap();
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &logical, None).unwrap();
    // Expected identities come from the fixture's independently declared ID ordinals. Neither
    // production resolution nor a query traversal computes this namespace model.
    let model = [
        (
            "m",
            "module",
            "first",
            None,
            OwnerKey::Module(ModuleId::migrate(SEED, 0)),
        ),
        (
            "m2",
            "module",
            "second",
            None,
            OwnerKey::Module(ModuleId::migrate(SEED, 1)),
        ),
        (
            "record",
            "declaration",
            "Payload",
            Some("m"),
            OwnerKey::Declaration(DeclarationId::migrate(SEED, 0)),
        ),
        (
            "variant",
            "declaration",
            "State",
            Some("m"),
            OwnerKey::Declaration(DeclarationId::migrate(SEED, 1)),
        ),
        (
            "interface",
            "declaration",
            "Store",
            Some("m"),
            OwnerKey::Declaration(DeclarationId::migrate(SEED, 2)),
        ),
        (
            "component",
            "declaration",
            "Application",
            Some("m"),
            OwnerKey::Declaration(DeclarationId::migrate(SEED, 3)),
        ),
        (
            "function",
            "declaration",
            "callee",
            Some("m"),
            OwnerKey::Declaration(DeclarationId::migrate(SEED, 4)),
        ),
        (
            "external",
            "declaration",
            "identity_external",
            Some("m2"),
            OwnerKey::Declaration(DeclarationId::migrate(SEED, 8)),
        ),
        (
            "field",
            "field",
            "value",
            Some("record"),
            OwnerKey::Field(FieldId::migrate(SEED, 0)),
        ),
        (
            "case",
            "case",
            "Ready",
            Some("variant"),
            OwnerKey::Case(CaseId::migrate(SEED, 0)),
        ),
        (
            "operation",
            "operation",
            "read",
            Some("interface"),
            OwnerKey::Operation(OperationId::migrate(SEED, 0)),
        ),
        (
            "requirement",
            "requirement",
            "store",
            Some("component"),
            OwnerKey::Requirement(RequirementId::migrate(SEED, 0)),
        ),
        (
            "port",
            "port",
            "run",
            Some("component"),
            OwnerKey::Port(PortId::migrate(SEED, 0)),
        ),
        (
            "parameter",
            "parameter",
            "input",
            Some("function"),
            OwnerKey::Parameter(ParameterId::migrate(SEED, 0)),
        ),
        (
            "type",
            "type-parameter",
            "T",
            Some("external"),
            OwnerKey::TypeParameter(TypeParameterId::migrate(SEED, 0)),
        ),
        (
            "target",
            "target",
            "command",
            None,
            OwnerKey::Target(TargetId::migrate(SEED, 0)),
        ),
    ];
    let rows = model
        .iter()
        .map(|(alias, class, name, parent, _)| {
            format!(
                "reference.owner as=${alias} package=local class={class} name={name}{}\n",
                parent
                    .map(|parent| format!(" parent=${parent}"))
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>();
    let edits = "expression.local as=$read value=$parameter\nreplace.body function=$function body=$read\nrename.owner owner=$function name=renamed\nexpression.unit as=$fresh-body\ncreate.function as=$fresh module=$m name=fresh visibility=private result=unit effect=pure body=$fresh-body\n";
    let (request, prepared) = prepare(&created.repository, &rows.concat(), edits).unwrap();
    let AuthoredChange::ReferenceBindings { bindings } = &request.semantic.changes[0] else {
        panic!()
    };
    for (alias, _, _, _, expected) in model {
        let origin = bindings
            .origins
            .iter()
            .find(|origin| origin.alias == format!("${alias}"))
            .unwrap();
        assert_eq!(
            prepared.logical_plan.resolutions.owners[origin.owner.unwrap().0 as usize].owner,
            expected
        );
        assert!(!prepared.allocated.values().any(|owner| *owner == expected));
    }
    let (token, plan) = encoded(&request, &prepared);
    crate::platform::control::decode_logical_change_plan(std::io::Cursor::new(plan.as_bytes()))
        .unwrap();
    let reordered = rows.iter().rev().cloned().collect::<String>();
    let (alpha, alpha_prepared) = prepare(
        &created.repository,
        &reordered
            .replace("as=$", "as=$ref_")
            .replace("parent=$", "parent=$ref_"),
        &edits
            .replace("value=$parameter", "value=$ref_parameter")
            .replace("function=$function", "function=$ref_function")
            .replace("owner=$function", "owner=$ref_function")
            .replace("module=$m ", "module=$ref_m "),
    )
    .unwrap();
    assert_eq!(
        crate::platform::change::canonical_authored_intent_bytes(&request.semantic).unwrap(),
        crate::platform::change::canonical_authored_intent_bytes(&alpha.semantic).unwrap()
    );
    assert_eq!(request.request_commitment, alpha.request_commitment);
    assert_eq!(prepared.allocated, alpha_prepared.allocated);
    assert_eq!(token, encoded(&alpha, &alpha_prepared).0);
    let duplicated = format!(
        "{}reference.owner as=$another-m package=local class=module name=first\n",
        rows.concat()
    );
    let (duplicate, duplicate_prepared) = prepare(&created.repository, &duplicated, edits).unwrap();
    assert_eq!(request.request_commitment, duplicate.request_commitment);
    assert_eq!(token, encoded(&duplicate, &duplicate_prepared).0);
    let changed = plan.replace("name=callee", "name=absent");
    assert_ne!(changed, plan);
    assert!(
        crate::platform::control::decode_logical_change_plan(std::io::Cursor::new(
            changed.as_bytes()
        ))
        .is_err()
    );
    // A newly hashed, structurally valid substituted resolution is still a different review.
    // Apply re-prepares the healthy exact base and compares its token, not this claimed table.
    let mut substituted = prepared.clone();
    let selected = substituted
        .logical_plan
        .resolutions
        .owners
        .iter_mut()
        .find(|owner| owner.owner == OwnerKey::Parameter(ParameterId::migrate(SEED, 0)))
        .unwrap();
    selected.owner = OwnerKey::Parameter(ParameterId::migrate(SEED, 99));
    assert_ne!(token, encoded(&request, &substituted).0);
    let initial = created.repository.current().unwrap().head.revision;
    created.repository.publish(&prepared.publication).unwrap();
    let view = created
        .repository
        .view_idempotency_base("named-test", initial)
        .unwrap()
        .unwrap();
    let retry = view
        .prepare_authored_change(&request.semantic, request.options.clone())
        .unwrap();
    assert_eq!(token, encoded(&request, &retry).0);
    let accepted = created.repository.current().unwrap().head.revision;
    created.repository.publish(&retry.publication).unwrap();
    assert_eq!(
        accepted,
        created.repository.current().unwrap().head.revision
    );
}

#[test]
fn named_reference_decoding_rejects_cycles_collisions_wrong_classes_and_raw_exhaustion() {
    let base = RevisionId::from_digest([7; 32]);
    for (prelude, expected) in [
        (
            "reference.owner as=$a package=local class=declaration name=x parent=$b\nreference.owner as=$b package=local class=declaration name=y parent=$a\n",
            "change_reference_cycle",
        ),
        (
            "reference.package as=$p source=builtin extra=bad\n",
            "change_field_unknown",
        ),
        (
            "reference.owner as=$fresh package=local class=module name=first\n",
            "change_reference_symbol_collision",
        ),
        (
            "reference.owner as=$m package=local class=module name=first\nreference.owner as=$m package=local class=module name=first\n",
            "change_reference_duplicate",
        ),
        (
            "reference.package as=$p source=builtin\nreference.owner as=$m package=$p class=module name=first\n",
            "change_reference_parent_class",
        ),
    ] {
        let text = format!("request base={base}\n{prelude}create.module as=$fresh name=new\n");
        let error = decode_compact_change("invalid.lkjc", text.as_bytes()).unwrap_err();
        assert_eq!(error[0].code, expected, "{error:?}");
    }
    let rows = (0..1_000)
        .map(|i| format!("reference.owner as=$m{i} package=local class=module name=first\n"))
        .collect::<String>();
    let error = decode_compact_change(
        "over-budget.lkjc",
        format!("request base={base}\n{rows}create.module as=$fresh name=new\n").as_bytes(),
    )
    .unwrap_err();
    assert_eq!(error[0].code, "change_budget_operations");
    assert!(
        decode_compact_change(
            "bindings-only.lkjc",
            format!("request base={base}\nreference.package as=$p source=builtin\n").as_bytes()
        )
        .is_err()
    );
    let package = PackageId::migrate(b"wrong reference parent package", 0);
    let revision = PackageRevisionDigest::from_bytes([1; 32]);
    let other_revision = PackageRevisionDigest::from_bytes([2; 32]);
    let text = format!(
        "request base={base}\nreference.package as=$p package={package} package-revision={revision}\nreference.package as=$q package={package} package-revision={other_revision}\nreference.owner as=$record package=$p class=declaration name=cell\nreference.owner as=$field package=$q class=field name=value parent=$record\ncreate.module as=$fresh name=new\n"
    );
    assert_eq!(
        decode_compact_change("different-revision.lkjc", text.as_bytes()).unwrap_err()[0].code,
        "change_reference_parent_package"
    );
}

#[test]
fn named_reference_labels_in_values_remain_literal() {
    let temporary = tempfile::tempdir().unwrap();
    let created = GraphRepository::create(
        &temporary.path().join("meaning"),
        &crate::platform::kernel::tests::witness_snapshot(),
        None,
    )
    .unwrap();
    let (request, _) = prepare(
        &created.repository,
        "reference.owner as=$m package=local class=module name=first\n",
        "expression.text as=$literal value=\"$m and $undefined\"\ncreate.function as=$fresh module=$m name=literal visibility=private result=text effect=pure body=$literal\n",
    )
    .unwrap();
    let AuthoredChange::CreateFunction { body, .. } = &request.semantic.changes[1] else {
        panic!()
    };
    assert!(
        matches!(&body.operation, AuthoredExpressionOperation::Text { value } if value == "$m and $undefined")
    );
}

#[test]
fn named_reference_builtin_resolution_is_exact_and_unused_names_are_checked() {
    let temporary = tempfile::tempdir().unwrap();
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &logical, None).unwrap();
    let standard = crate::platform::builtin_standard::BuiltinStandard::load().unwrap();
    let transport = standard.transport();
    created
        .repository
        .stage_package_transport(standard.package_transport, &transport.container)
        .unwrap();
    let prelude = "reference.package as=$std source=builtin\nreference.owner as=$add package=$std class=declaration name=add\nreference.owner as=$m package=local class=module name=first\n";
    let edits = format!(
        "expression.i64 as=$a value=41\nexpression.i64 as=$b value=1\nexpression.call as=$body function=$add\nexpression.argument parent=$body index=0 expression=$a\nexpression.argument parent=$body index=1 expression=$b\ncreate.function as=$fresh module=$m name=answer visibility=private result=i64 effect=pure body=$body\nadd.dependency package={} semantic-revision={} package-revision={}\n",
        standard.package, standard.semantic_revision, standard.package_revision
    );
    let (request, prepared) = prepare(&created.repository, prelude, &edits).unwrap();
    let mut changed_supplier = request.semantic.clone();
    let AuthoredChange::ReferenceBindings { bindings } = &mut changed_supplier.changes[0] else {
        panic!()
    };
    for package in &mut bindings.packages {
        if let AuthoredReferencePackage::Exact {
            semantic_revision, ..
        } = package
        {
            *semantic_revision = Some(RevisionId::from_digest([9; 32]));
        }
    }
    let changed_commitment =
        change_request_commitment(&changed_supplier, &request.options).unwrap();
    assert_ne!(request.request_commitment, changed_commitment);
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&changed_supplier, request.options.clone())
            .unwrap_err()[0]
            .code,
        "change_reference_supplier_mismatch"
    );
    let selected = &prepared.logical_plan.resolutions.owners;
    assert!(selected.iter().any(|owner| owner.owner.to_string()
        == "decl_72b38e6cd864cb4239b329b7e337577f"
        && owner.package == standard.package));
    let (_, plan) = encoded(&request, &prepared);
    crate::platform::control::decode_logical_change_plan(std::io::Cursor::new(plan.as_bytes()))
        .unwrap();
    let initial = created.repository.current().unwrap().head.revision;
    for (prelude, code) in [
        (
            prelude.replace("name=add", "name=absent"),
            "change_reference_not_exposed",
        ),
        (
            format!(
                "{prelude}reference.owner as=$unused package=$std class=declaration name=absent\n"
            ),
            "change_reference_not_exposed",
        ),
    ] {
        assert_eq!(
            prepare(&created.repository, &prelude, &edits).unwrap_err()[0].code,
            code
        );
        assert_eq!(initial, created.repository.current().unwrap().head.revision);
    }
    let mut duplicate_dependency = request.semantic.clone();
    duplicate_dependency
        .changes
        .push(AuthoredChange::ReplaceDependency {
            package: standard.package,
            semantic_revision: standard.semantic_revision,
            package_revision: standard.package_revision,
        });
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&duplicate_dependency, request.options.clone())
            .unwrap_err()[0]
            .code,
        "change_reference_dependency_edits"
    );
    created.repository.publish(&prepared.publication).unwrap();
    let accepted = created.repository.current().unwrap().head.revision;
    // Dependency deletion is already part of typed authored intent; this does not invent a
    // compact delete.dependency command. No selector may fall back to the now-deleted base binding.
    let removed = AuthoredChangeSet {
        base: accepted,
        preconditions: vec![],
        changes: vec![
            request.semantic.changes[0].clone(),
            AuthoredChange::DeleteDependency {
                package: standard.package,
            },
        ],
        budget: Default::default(),
    };
    assert_eq!(
        created
            .repository
            .prepare_authored_change(&removed, Default::default())
            .unwrap_err()[0]
            .code,
        "change_reference_dependency_missing"
    );
    assert_eq!(
        accepted,
        created.repository.current().unwrap().head.revision
    );
}

#[test]
fn named_reference_preconditions_resolve_only_existing_local_base_owners() {
    let temporary = tempfile::tempdir().unwrap();
    let created = GraphRepository::create(
        &temporary.path().join("meaning"),
        &crate::platform::kernel::tests::witness_snapshot(),
        None,
    )
    .unwrap();
    let prelude = "reference.owner as=$module package=local class=module name=first\nreference.owner as=$function package=local class=declaration name=callee parent=$module\nprecondition.owner-exists owner=$function\nprecondition.owner-name owner=$function name=callee\nprecondition.owner-parent owner=$function parent=$module\nprecondition.namespace-points-to parent=$module class=declaration name=callee owner=$function\nprecondition.namespace-absent parent=$module class=declaration name=renamed\n";
    let (_, prepared) = prepare(
        &created.repository,
        prelude,
        "rename.owner owner=$function name=renamed\n",
    )
    .unwrap();
    assert_eq!(prepared.lowering_work.preconditions_checked, 5);
    let absent = format!("{prelude}precondition.owner-absent owner=$function\n");
    assert_eq!(
        prepare(
            &created.repository,
            &absent,
            "rename.owner owner=$function name=renamed\n"
        )
        .unwrap_err()[0]
            .code,
        "change_precondition_owner_present"
    );
    created.repository.publish(&prepared.publication).unwrap();
}

#[test]
fn named_reference_resolution_propagates_cancellation_without_publication() {
    use crate::platform::change::{
        CanonicalBaseRead, CanonicalRead, WitnessBaseRead, lower_authored_changes,
    };
    use crate::platform::execution::ExecutionControl;
    struct CancelledBase<'a> {
        view: &'a crate::platform::publication::RepositoryView,
        control: ExecutionControl,
    }
    impl CanonicalBaseRead for CancelledBase<'_> {
        fn semantic_root(&self) -> &SemanticRoot {
            self.view.semantic_root()
        }
        fn repository_id(&self) -> RepositoryId {
            self.view.repository_id()
        }
        fn package_id(&self) -> PackageId {
            self.view.package_id()
        }
        fn exact_revision(&self) -> Option<RevisionId> {
            self.view.exact_revision()
        }
        fn owner_count(&self) -> u64 {
            self.view.owner_count()
        }
        fn dependency_count(&self) -> u64 {
            self.view.dependency_count()
        }
        fn retirement_count(&self) -> u64 {
            self.view.retirement_count()
        }
        fn read_owner(
            &self,
            owner: OwnerKey,
        ) -> Result<CanonicalRead<Option<OwnerRecord>>, Diagnostic> {
            self.control.check().map_err(|_| {
                Diagnostic::new(
                    DiagnosticClass::Cancelled,
                    "named_reference_cancelled",
                    "owning preparation was cancelled at a canonical read boundary",
                )
            })?;
            self.view.read_owner(owner)
        }
        fn read_type_object(
            &self,
            digest: TypeObjectDigest,
        ) -> Result<CanonicalRead<Option<TypeObject>>, Diagnostic> {
            self.view.read_type_object(digest)
        }
        fn read_package_interface_owner(
            &self,
            dependency: &DependencyRecord,
            owner: OwnerKey,
        ) -> Result<CanonicalRead<Option<PackageInterfaceRecord>>, Diagnostic> {
            self.view.read_package_interface_owner(dependency, owner)
        }
        fn read_dependency(
            &self,
            package: PackageId,
        ) -> Result<CanonicalRead<Option<DependencyRecord>>, Diagnostic> {
            self.view.read_dependency(package)
        }
        fn read_retirement(
            &self,
            owner: OwnerKey,
        ) -> Result<CanonicalRead<Option<RetirementRecord>>, Diagnostic> {
            self.view.read_retirement(owner)
        }
    }
    let temporary = tempfile::tempdir().unwrap();
    let created = GraphRepository::create(
        &temporary.path().join("meaning"),
        &crate::platform::kernel::tests::witness_snapshot(),
        None,
    )
    .unwrap();
    let before = created.repository.current().unwrap().head.revision;
    let request = decode_compact_change("cancel.lkjc", format!("request base={before}\nreference.owner as=$m package=local class=module name=first\nreference.owner as=$f package=local class=declaration name=callee parent=$m\nrename.owner owner=$f name=renamed\n").as_bytes()).unwrap();
    let view = created.repository.view_current().unwrap();
    assert!(view.witness_contract_is_current());
    let base = CancelledBase {
        view: &view,
        control: ExecutionControl::cancel_after_checks(1),
    };
    let error = lower_authored_changes(&base, &view, &request.semantic).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Cancelled);
    assert!(base.control.is_cancelled());
    assert_eq!(created.repository.current().unwrap().head.revision, before);
    let prepared = created
        .repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap();
    created.repository.publish(&prepared.publication).unwrap();
}

#[test]
fn named_reference_wrong_parents_scope_and_large_shared_prelude_leave_head_unchanged() {
    let temporary = tempfile::tempdir().unwrap();
    let created = GraphRepository::create(
        &temporary.path().join("meaning"),
        &crate::platform::kernel::tests::witness_snapshot(),
        None,
    )
    .unwrap();
    let base = created.repository.current().unwrap().head.revision;
    let prelude = "reference.owner as=$m package=local class=module name=first\nreference.owner as=$record package=local class=declaration name=Payload parent=$m\nreference.owner as=$function package=local class=declaration name=callee parent=$m\nreference.owner as=$parameter package=local class=parameter name=input parent=$function\n";
    let bad_parent = format!(
        "{prelude}reference.owner as=$field package=local class=field name=value parent=$function\n"
    );
    assert_eq!(
        prepare(
            &created.repository,
            &bad_parent,
            "create.module as=$new name=fresh\n"
        )
        .unwrap_err()[0]
            .code,
        "change_reference_parent_kind"
    );
    let bad_scope = "expression.local as=$body value=$parameter\ncreate.function as=$new module=$m name=fresh visibility=private result=unit effect=pure body=$body\n";
    assert!(prepare(&created.repository, prelude, bad_scope).is_err());
    assert_eq!(created.repository.current().unwrap().head.revision, base);
    let rows = (0..999)
        .map(|i| format!("reference.owner as=$m{i} package=local class=module name=first\n"))
        .collect::<String>();
    let (request, prepared) = prepare(
        &created.repository,
        &rows,
        "create.module as=$new name=fresh\n",
    )
    .unwrap();
    assert_eq!(prepared.logical_plan.resolutions.owners.len(), 1);
    assert_eq!(prepared.lowering_work.operations_lowered, 1_000);
    assert_eq!(prepared.lowering_work.witness.point_reads, 1);
    assert_eq!(prepared.allocated.len(), 1);
    let mut exhausted = request.semantic;
    exhausted.budget.witness_reads.maximum_point_reads = 0;
    assert!(
        created
            .repository
            .prepare_authored_change(&exhausted, request.options)
            .is_err()
    );
    assert_eq!(created.repository.current().unwrap().head.revision, base);
    created.repository.publish(&prepared.publication).unwrap();
}

#[test]
fn named_reference_exact_interface_model_ambiguity_escape_and_all_exposed_children() {
    let temporary = tempfile::tempdir().unwrap();
    let producer = GraphRepository::create(
        &temporary.path().join("producer"),
        &empty(b"named-producer"),
        None,
    )
    .unwrap();
    let consumer = GraphRepository::create(
        &temporary.path().join("consumer"),
        &empty(b"named-consumer"),
        None,
    )
    .unwrap();
    let authored = "create.module as=$a name=a\ncreate.module as=$b name=b\ncreate.record as=$a-cell module=$a name=cell visibility=public\nadd.type-parameter as=$T declaration=$a-cell name=T\ntype.parameter as=@T parameter=$T\nadd.field as=$a-value record=$a-cell name=value type=@T\ncreate.record as=$b-cell module=$b name=cell visibility=public\nadd.field as=$b-value record=$b-cell name=value type=unit\ncreate.variant as=$variant module=$a name=choice visibility=public\nadd.case as=$case variant=$variant name=empty\ncreate.interface as=$interface module=$a name=service visibility=public\nadd.operation as=$operation interface=$interface name=perform result=unit idempotency=idempotent external-visibility=none\nadd.parameter as=$operation-parameter operation=$operation name=input type=unit\nexpression.unit as=$body\ncreate.function as=$function module=$a name=identity visibility=public result=unit effect=pure body=$body\nadd.effect-parameter as=$E declaration=$function name=E\nadd.parameter as=$parameter function=$function name=input type=unit\ncreate.component as=$component module=$a name=component visibility=public\nadd.requirement as=$requirement component=$component name=service interface=$interface\nrequirement.operation parent=$requirement index=0 operation=$operation\nexpression.unit as=$port-body\ncreate.function as=$port-function module=$a name=hidden visibility=private result=unit effect=pure body=$port-body\ntype.function as=@port-type result=unit\nadd.port as=$port component=$component name=run type=@port-type function=$port-function\n";
    let (_, produced) = prepare(&producer.repository, "", authored).unwrap();
    producer.repository.publish(&produced.publication).unwrap();
    let exported = producer.repository.export_package_transport().unwrap();
    consumer
        .repository
        .stage_package_transport(exported.transport_digest, &exported.container)
        .unwrap();
    let package = exported.revision.package;
    let selection = format!(
        "reference.package as=$p package={package} package-revision={}\n",
        exported.revision_digest
    );
    let dependency = format!(
        "add.dependency package={package} semantic-revision={} package-revision={}\n",
        exported.revision.revision.revision_id().unwrap(),
        exported.revision_digest
    );
    let ambiguous =
        format!("{selection}reference.owner as=$cell package=$p class=declaration name=cell\n");
    assert_eq!(
        prepare(&consumer.repository, &ambiguous, &dependency).unwrap_err()[0].code,
        "change_reference_ambiguous"
    );
    let rows = [
        ("$a-cell", "declaration", None, None),
        ("$b-cell", "declaration", None, None),
        ("$T", "type-parameter", Some("T"), Some("$a-cell")),
        ("$a-value", "field", Some("value"), Some("$a-cell")),
        ("$b-value", "field", Some("value"), Some("$b-cell")),
        ("$variant", "declaration", Some("choice"), None),
        ("$case", "case", Some("empty"), Some("$variant")),
        ("$interface", "declaration", Some("service"), None),
        (
            "$operation",
            "operation",
            Some("perform"),
            Some("$interface"),
        ),
        (
            "$operation-parameter",
            "parameter",
            Some("input"),
            Some("$operation"),
        ),
        ("$function", "declaration", Some("identity"), None),
        ("$E", "effect-parameter", Some("E"), Some("$function")),
        ("$parameter", "parameter", Some("input"), Some("$function")),
        ("$component", "declaration", Some("component"), None),
        (
            "$requirement",
            "requirement",
            Some("service"),
            Some("$component"),
        ),
        ("$port", "port", Some("run"), Some("$component")),
    ];
    let mut prelude = selection.clone();
    for (alias, class, name, parent) in rows {
        prelude.push_str(&format!(
            "reference.owner as={alias} package=$p class={class}{}{}\n",
            name.map(|name| format!(" name={name}"))
                .unwrap_or_else(|| format!(" owner={}", produced.allocated[alias])),
            parent
                .map(|parent| format!(" parent={parent}"))
                .unwrap_or_default()
        ));
    }
    let (request, selected) = prepare(&consumer.repository, &prelude, &dependency).unwrap();
    let AuthoredChange::ReferenceBindings { bindings } = &request.semantic.changes[0] else {
        panic!()
    };
    for (alias, _, _, _) in rows {
        let origin = bindings
            .origins
            .iter()
            .find(|origin| origin.alias == alias)
            .unwrap();
        assert_eq!(
            selected.logical_plan.resolutions.owners[origin.owner.unwrap().0 as usize].owner,
            produced.allocated[alias]
        );
    }
    assert!(selected.allocated.is_empty());
    for extra in [
        "reference.owner as=$private package=$p class=declaration name=hidden\n".to_owned(),
        format!(
            "reference.owner as=$private package=$p class=declaration owner={}\n",
            produced.allocated["$port-function"]
        ),
    ] {
        assert_eq!(
            prepare(
                &consumer.repository,
                &format!("{prelude}{extra}"),
                &dependency
            )
            .unwrap_err()[0]
                .code,
            "change_reference_not_exposed"
        );
    }
    assert!(
        prepare(
            &consumer.repository,
            &prelude,
            &format!("{dependency}rename.owner owner=$a-cell name=stolen\n")
        )
        .is_err()
    );
    let local = prelude.replace(&selection, "reference.owner as=$a package=local class=module name=a\nreference.owner as=$b package=local class=module name=b\n")
        .replace("package=$p", "package=local");
    // Local effect/operation parameters use the same exact namespace model, with local module
    // parents and named declarations instead of the foreign declaration escape hatch.
    let mut local_rows = String::new();
    for line in local.lines() {
        if line.contains("class=declaration") {
            let line = if line.contains("as=$a-cell ") || line.contains("as=$b-cell ") {
                line.split(" owner=").next().unwrap().to_owned() + " name=cell"
            } else {
                line.to_owned()
            };
            local_rows.push_str(&line);
            local_rows.push_str(if line.contains("as=$b-cell ") {
                " parent=$b\n"
            } else {
                " parent=$a\n"
            });
        } else {
            local_rows.push_str(line);
            local_rows.push('\n');
        }
    }
    let (_, selected) = prepare(&producer.repository, &local_rows, "type.parameter as=@selected-T parameter=$T\nset.field-type field=$a-value type=@selected-T\nset.function-contract as=%changed function=$function result=unit effect=task\neffect.parameter parent=%changed index=0 parameter=$E\nrename.owner owner=$E name=Effects\nrename.owner owner=$operation-parameter name=argument\n").unwrap();
    assert!(selected.allocated.is_empty());
    assert_eq!(
        selected.logical_plan.resolutions.owners.len(),
        rows.len() + 2
    );
}
