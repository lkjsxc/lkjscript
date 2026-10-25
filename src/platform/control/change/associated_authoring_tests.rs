//! Native/compact proposal correspondence and exact prepared-candidate authority.
use super::*;
use crate::platform::change::{
    AuthoredLowering, CanonicalBaseRead, CanonicalRead, CanonicalReadAdmission, CanonicalReadWork,
    PrimitiveEdit, canonical_authored_intent_bytes, lower_authored_changes,
};
use crate::platform::execution::ExecutionControl;
use crate::platform::kernel::{self as k, DeclarationPayload as D, OwnerRecord as O};
use crate::platform::publication::{GraphRepository, RepositoryView};
use crate::platform::semantic_id::{AssociatedTypeMemberId, RepositoryId};

const FIRST: &str = "assoc_type_95000000000000000000000000000009";
const SECOND: &str = "assoc_type_95000000000000000000000000000003";
const WITNESS: &str = "implparam_95000000000000000000000000000005";

fn compact_body() -> String {
    format!(
        r#"create.module as=$module name=associated_surface
type.parameter as=@self parameter=$self
type.parameter as=@consumer-self parameter=$consumer-self
type.associated-contract-member as=@member contract=$contract member={FIRST}
type.owned-sequence as=@block item=byte-buffer
type.associated-witness-projection as=@projection scope=$consumer parameter={WITNESS} contract=$contract member={FIRST}
create.owned-contract as=$contract module=$module name=Storage visibility=public self=@self
owned.associated-member parent=$contract index=0 id={FIRST} name=Block kind=owned
owned.associated-member parent=$contract index=1 id={SECOND} name=Spare kind=owned
owned.method parent=$contract index=0 as=%method id=method_95000000000000000000000000000001 name=produce result=@member
owned.parameter parent=%method index=0 type=@self use=consume
add.type-parameter as=$self declaration=$contract name=Self constraint=owned
create.owned-implementation as=$implementation module=$module name=Cells visibility=public contract=$contract self=@block
owned.associated-binding parent=$implementation index=0 member={FIRST} kind=owned type=@block
owned.associated-binding parent=$implementation index=1 member={SECOND} kind=owned type=owned-i64-cell
owned.mapping parent=$implementation index=0 method=method_95000000000000000000000000000001 function=$make
add.parameter as=$cell function=$make name=cell type=@block use=consume
create.function as=$make module=$module name=make visibility=public result=@block effect=pure body=$cell-value
expression.local as=$cell-value value=$cell
add.type-parameter as=$consumer-self declaration=$consumer name=W constraint=owned
add.parameter as=$block-parameter function=$consumer name=block type=@projection use=consume
set.implementations as=%consumer-witnesses declaration=$consumer
owned.witness parent=%consumer-witnesses index=0 id={WITNESS} name=storage contract=$contract self=@consumer-self
create.function as=$consumer module=$module name=consumer visibility=private result=@projection effect=pure body=$block-value
expression.local as=$block-value value=$block-parameter
"#
    )
}

fn native_body() -> String {
    format!(
        r#"declarations.begin
(units (module create associated_surface (as $module)
  (owned-contract create Storage (as $contract) (visibility public)
    (self Self) (type-parameter create Self (as $self) (constraint owned))
    (associated-type {FIRST} Block (bound owned))
    (associated-type {SECOND} Spare (bound owned))
    (method method_95000000000000000000000000000001 produce
      (parameters (Self consume)) (returns (associated-contract-member Storage {FIRST}))))
  (owned-implementation create Cells (as $implementation) (visibility public)
    (contract Storage) (self (owned-sequence ByteBuffer))
    (associated-type {FIRST} (owned-sequence ByteBuffer))
    (associated-type {SECOND} OwnedI64Cell)
    (method method_95000000000000000000000000000001 make))
  (function create make (as $make) (visibility public) (effect pure)
    (parameter create cell (as $cell) (type (owned-sequence ByteBuffer)) (use consume))
    (returns (owned-sequence ByteBuffer)) (body (local cell)))
  (function create consumer (as $consumer) (visibility private) (effect pure)
    (type-parameter create W (as $consumer-self) (constraint owned))
    (implementation-parameter {WITNESS} storage Storage W)
    (parameter create block (as $block-parameter)
      (type (associated-witness-projection consumer {WITNESS} Storage {FIRST})) (use consume))
    (returns (associated-witness-projection consumer {WITNESS} Storage {FIRST}))
    (body (local block)))))
declarations.end
"#
    )
}

fn decode(body: &str) -> NormalizedChangeRequest {
    decode_compact_change(
        "associated-proposal.lkjc",
        format!("request base=rev_{}\n{body}", "82".repeat(32)).as_bytes(),
    )
    .unwrap()
}

#[test]
fn associated_surface_native_intent_matches_literal_compact_complete_coordinates() {
    let compact = decode(&compact_body());
    let native = decode(&native_body());
    let bytes = canonical_authored_intent_bytes(&compact.semantic).unwrap();
    assert_eq!(&bytes[..8], b"LKJACHNG");
    assert_eq!(
        bytes,
        canonical_authored_intent_bytes(&native.semantic).unwrap()
    );
    assert_eq!(compact.request_commitment, native.request_commitment);
    assert_eq!(
        &bytes[8..10],
        &crate::platform::canonical_schema::SCHEMA_EPOCH.to_le_bytes()
    );
    let AuthoredChange::CreateOwnedContract {
        associated_types,
        methods,
        ..
    } = &compact.semantic.changes[1]
    else {
        panic!("contract")
    };
    assert_eq!(
        associated_types
            .iter()
            .map(|m| m.id.to_string())
            .collect::<Vec<_>>(),
        [FIRST, SECOND]
    );
    assert!(
        matches!(&methods[0].result.ty, AuthoredType::AssociatedContractMember { contract: AuthoredDeclarationReference::Local { declaration: DeclarationSelector::Symbol { symbol } }, member } if symbol == "$contract" && member.to_string() == FIRST)
    );
    let AuthoredChange::CreateFunction { result, .. } = compact.semantic.changes.iter().find(|change| matches!(change, AuthoredChange::CreateFunction { symbol, .. } if symbol == "$consumer")).unwrap() else { panic!("consumer") };
    assert!(
        matches!(&result.ty, AuthoredType::AssociatedWitnessProjection { scope: AuthoredDeclarationReference::Local { declaration: DeclarationSelector::Symbol { symbol } }, parameter, contract: AuthoredDeclarationReference::Local { declaration: DeclarationSelector::Symbol { symbol: contract } }, member } if symbol == "$consumer" && parameter.to_string() == WITNESS && contract == "$contract" && member.to_string() == FIRST)
    );
    assert_eq!(result.relation, AuthoredResultRelation::Value);
}

#[test]
fn associated_surface_explicit_coordinates_and_complete_inventory_order_bind_intent() {
    let original = compact_body();
    let expected = decode(&original).request_commitment;
    for changed in [
        original.replace("scope=$consumer", "scope=$make"),
        original.replace(&format!("parameter={WITNESS}"), "parameter=implparam_95000000000000000000000000000006"),
        original.replace(&format!("contract=$contract member={FIRST}"), &format!("contract=$implementation member={FIRST}")),
        original.replace(&format!("as=@projection scope=$consumer parameter={WITNESS} contract=$contract member={FIRST}"), &format!("as=@projection scope=$consumer parameter={WITNESS} contract=$contract member={SECOND}")),
        original.replace(&format!("index=0 id={FIRST}"), &format!("index=1 id={FIRST}")).replace(&format!("index=1 id={SECOND}"), &format!("index=0 id={SECOND}")),
        original.replace(&format!("index=0 member={FIRST}"), &format!("index=1 member={FIRST}")).replace(&format!("index=1 member={SECOND}"), &format!("index=0 member={SECOND}")),
    ] {
        assert_ne!(expected, decode(&changed).request_commitment, "{changed}");
    }
}

#[test]
fn associated_surface_rejects_missing_wrong_domain_and_unconsumed_coordinates() {
    let input = compact_body();
    for invalid in [
        input.replace(" kind=owned", ""),
        input.replace("kind=owned", "kind=transferable"),
        input.replace("scope=$consumer ", ""),
        input.replace(&format!("parameter={WITNESS} "), ""),
        input.replace(
            &format!("id={FIRST}"),
            "id=method_95000000000000000000000000000009",
        ),
        input.replace(
            &format!("member={FIRST}"),
            "member=implparam_95000000000000000000000000000009",
        ),
        input.replace(
            &format!("parameter={WITNESS}"),
            &format!("parameter={FIRST}"),
        ),
        input.replace(FIRST, "assoc_type_00000000000000000000000000000000"),
        input.replace("index=1 id=", "index=2 id="),
        input.replace("index=1 member=", "index=0 member="),
        input.replace(
            "owned.associated-member parent=$contract",
            "owned.associated-member parent=$implementation",
        ),
        input.replace(
            "owned.associated-binding parent=$implementation",
            "owned.associated-binding parent=$contract",
        ),
        input.replace(
            "as=@projection scope=$consumer",
            "as=@projection unknown=$consumer scope=$consumer",
        ),
    ] {
        let text = format!("request base=rev_{}\n{invalid}", "82".repeat(32));
        assert!(
            decode_compact_change("invalid-associated.lkjc", text.as_bytes()).is_err(),
            "{text}"
        );
    }
    for invalid in [
        native_body().replace("(bound owned)", "(bound owned transferable)"),
        native_body().replace("(bound owned)", "owned"),
        native_body().replace(
            &format!("consumer {WITNESS} Storage {FIRST}"),
            &format!("{WITNESS} Storage {FIRST}"),
        ),
        native_body().replace(
            &format!("associated-contract-member Storage {FIRST}"),
            "associated-contract-member Storage",
        ),
    ] {
        assert!(
            decode_compact_change(
                "invalid-native-associated.lkjc",
                format!("request base=rev_{}\n{invalid}", "82".repeat(32)).as_bytes()
            )
            .is_err(),
            "{invalid}"
        );
    }
}

fn candidate(
    lowered: &AuthoredLowering,
) -> (
    BTreeMap<OwnerKey, O>,
    BTreeMap<k::TypeObjectDigest, k::TypeObject>,
) {
    let mut owners = BTreeMap::new();
    let mut types = BTreeMap::new();
    for edit in &lowered.edits {
        match edit {
            PrimitiveEdit::InsertOwner { record } | PrimitiveEdit::ReplaceOwner { record, .. } => {
                owners.insert(record.owner(), record.clone());
            }
            PrimitiveEdit::AddTypeObject { digest, object } => {
                types.insert(*digest, object.clone());
            }
            other => panic!("unexpected raw candidate edit {other:?}"),
        }
    }
    (owners, types)
}

fn canonical_bytes(
    owners: &BTreeMap<OwnerKey, O>,
    types: &BTreeMap<k::TypeObjectDigest, k::TypeObject>,
) -> (
    BTreeMap<OwnerKey, Vec<u8>>,
    BTreeMap<k::TypeObjectDigest, Vec<u8>>,
) {
    (
        owners
            .iter()
            .map(|(id, record)| (*id, k::encode_owner(record).unwrap().1))
            .collect(),
        types
            .iter()
            .map(|(id, object)| {
                let (digest, bytes) = k::encode_type_object(object).unwrap();
                assert_eq!(*id, digest);
                (*id, bytes)
            })
            .collect(),
    )
}

/// A raw proposal projection for lowering correspondence. Original ordinary base/witness reads
/// retain their real binding; the overlaid candidate receives no validation or admission token.
struct UnadmittedCandidateRead<'a> {
    ordinary: &'a RepositoryView,
    owners: BTreeMap<OwnerKey, O>,
    types: BTreeMap<k::TypeObjectDigest, k::TypeObject>,
}

impl CanonicalBaseRead for UnadmittedCandidateRead<'_> {
    fn semantic_root(&self) -> &k::SemanticRoot {
        self.ordinary.semantic_root()
    }
    fn repository_id(&self) -> RepositoryId {
        self.ordinary.repository_id()
    }
    fn package_id(&self) -> PackageId {
        self.ordinary.package_id()
    }
    fn exact_revision(&self) -> Option<RevisionId> {
        self.ordinary.exact_revision()
    }
    fn owner_count(&self) -> u64 {
        self.ordinary.owner_count()
    }
    fn dependency_count(&self) -> u64 {
        self.ordinary.dependency_count()
    }
    fn retirement_count(&self) -> u64 {
        self.ordinary.retirement_count()
    }
    fn read_owner(&self, id: OwnerKey) -> Result<CanonicalRead<Option<O>>, Diagnostic> {
        self.read_owner_admitted(id, CanonicalReadAdmission::unbounded())
    }
    fn read_owner_admitted(
        &self,
        id: OwnerKey,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<O>>, Diagnostic> {
        if let Some(record) = self.owners.get(&id) {
            let work = CanonicalReadWork {
                point_reads: 1,
                canonical_records_decoded: 1,
                bytes_read: k::encode_owner(record)?.1.len() as u64,
                ..Default::default()
            };
            admission.check_work(work, "raw candidate owner")?;
            Ok(CanonicalRead {
                value: Some(record.clone()),
                work,
            })
        } else {
            CanonicalBaseRead::read_owner_admitted(self.ordinary, id, admission)
        }
    }
    fn read_type_object(
        &self,
        id: k::TypeObjectDigest,
    ) -> Result<CanonicalRead<Option<k::TypeObject>>, Diagnostic> {
        self.read_type_object_admitted(id, CanonicalReadAdmission::unbounded())
    }
    fn read_type_object_admitted(
        &self,
        id: k::TypeObjectDigest,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<k::TypeObject>>, Diagnostic> {
        if let Some(object) = self.types.get(&id) {
            let work = CanonicalReadWork {
                point_reads: 1,
                canonical_records_decoded: 1,
                bytes_read: k::encode_type_object(object)?.1.len() as u64,
                ..Default::default()
            };
            admission.check_work(work, "raw candidate type")?;
            Ok(CanonicalRead {
                value: Some(object.clone()),
                work,
            })
        } else {
            CanonicalBaseRead::read_type_object_admitted(self.ordinary, id, admission)
        }
    }
    fn read_package_interface_owner(
        &self,
        dependency: &k::DependencyRecord,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<k::PackageInterfaceRecord>>, Diagnostic> {
        CanonicalBaseRead::read_package_interface_owner(self.ordinary, dependency, owner)
    }
    fn read_package_interface_owner_admitted(
        &self,
        dependency: &k::DependencyRecord,
        owner: OwnerKey,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<k::PackageInterfaceRecord>>, Diagnostic> {
        CanonicalBaseRead::read_package_interface_owner_admitted(
            self.ordinary,
            dependency,
            owner,
            admission,
        )
    }
    fn read_dependency(
        &self,
        package: PackageId,
    ) -> Result<CanonicalRead<Option<k::DependencyRecord>>, Diagnostic> {
        CanonicalBaseRead::read_dependency(self.ordinary, package)
    }
    fn read_dependency_admitted(
        &self,
        package: PackageId,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<k::DependencyRecord>>, Diagnostic> {
        CanonicalBaseRead::read_dependency_admitted(self.ordinary, package, admission)
    }
    fn read_dependencies_admitted(
        &self,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Vec<k::DependencyRecord>>, Diagnostic> {
        CanonicalBaseRead::read_dependencies_admitted(self.ordinary, admission)
    }
    fn read_retirement(
        &self,
        owner: OwnerKey,
    ) -> Result<CanonicalRead<Option<k::RetirementRecord>>, Diagnostic> {
        CanonicalBaseRead::read_retirement(self.ordinary, owner)
    }
    fn read_retirement_admitted(
        &self,
        owner: OwnerKey,
        admission: CanonicalReadAdmission,
    ) -> Result<CanonicalRead<Option<k::RetirementRecord>>, Diagnostic> {
        CanonicalBaseRead::read_retirement_admitted(self.ordinary, owner, admission)
    }
}

#[test]
fn associated_surface_raw_candidate_draft_normalization_preserves_full_bytes() {
    let temporary = tempfile::tempdir().unwrap();
    let initial = k::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &initial, None).unwrap();
    let view = created.repository.view_current().unwrap();
    let request = decode_compact_change(
        "raw-associated.lkjc",
        format!("request base={}\n{}", view.revision(), native_body()).as_bytes(),
    )
    .unwrap();
    let lowered = lower_authored_changes(&view, &view, &request.semantic).unwrap();
    let (owners, types) = candidate(&lowered);
    assert!(
        owners
            .values()
            .all(|owner| owner.header().contract_version == k::contract::GRAPH_CONTRACT_VERSION)
    );
    let expected = canonical_bytes(&owners, &types);
    let selected: Vec<_> = ["$contract", "$implementation", "$make", "$consumer"]
        .iter()
        .map(|name| lowered.allocated[*name].into())
        .collect();
    let draft = draft::render_raw_lowered_candidate(
        &view,
        &selected,
        Some(4 * 1_048_576),
        ExecutionControl::uncancelled(),
        owners.clone(),
        types.clone(),
    )
    .unwrap();
    let text = std::str::from_utf8(&draft).unwrap();
    assert!(
        text.contains(FIRST) && text.contains(SECOND) && text.contains(WITNESS),
        "{text}"
    );
    for name in ["$contract", "$implementation", "$make", "$consumer"] {
        assert!(
            text.contains(&lowered.allocated[name].to_string()),
            "{name}: {text}"
        );
    }
    let mut reader = canonical::Reader::for_raw_lowered_candidate(
        &view,
        ExecutionControl::uncancelled(),
        owners.clone(),
        types.clone(),
    );
    let normalized = decode_parsed_change(
        input::parse("raw-associated-draft.lkjc", &draft).unwrap(),
        Some(&mut reader),
    )
    .unwrap();
    let candidate_read = UnadmittedCandidateRead {
        ordinary: &view,
        owners,
        types,
    };
    let second = lower_authored_changes(&candidate_read, &view, &normalized.semantic).unwrap();
    let (second_owners, second_types) = candidate(&second);
    let mut final_owners = candidate_read.owners.clone();
    let mut final_types = candidate_read.types.clone();
    final_owners.extend(second_owners);
    final_types.extend(second_types);
    assert_eq!(canonical_bytes(&final_owners, &final_types), expected);
    let O::Declaration(contract) = &final_owners[&lowered.allocated["$contract"]] else {
        panic!("contract")
    };
    let D::OwnedContract(contract) = &contract.payload else {
        panic!("contract")
    };
    assert_eq!(
        contract
            .associated_types
            .iter()
            .map(|m| m.id.to_string())
            .collect::<Vec<_>>(),
        [FIRST, SECOND]
    );
    let O::Declaration(implementation) = &final_owners[&lowered.allocated["$implementation"]]
    else {
        panic!("implementation")
    };
    let D::OwnedImplementation(implementation) = &implementation.payload else {
        panic!("implementation")
    };
    assert_eq!(
        implementation
            .associated_types
            .iter()
            .map(|b| b.member.to_string())
            .collect::<Vec<_>>(),
        [SECOND, FIRST]
    );
    let first: AssociatedTypeMemberId = FIRST.parse().unwrap();
    let second: AssociatedTypeMemberId = SECOND.parse().unwrap();
    assert_eq!(implementation.contract.package, view.package());
    assert_eq!(
        OwnerKey::Declaration(implementation.contract.declaration),
        lowered.allocated["$contract"]
    );
    assert_eq!(implementation.methods[0].function.package, view.package());
    assert_eq!(
        OwnerKey::Declaration(implementation.methods[0].function.declaration),
        lowered.allocated["$make"]
    );
    assert!(
        matches!(&final_types[&contract.methods[0].result.ty].form, k::TypeForm::AssociatedContractMember { contract, member } if contract.package == view.package() && OwnerKey::Declaration(contract.declaration) == lowered.allocated["$contract"] && *member == first)
    );
    let O::Declaration(consumer) = &final_owners[&lowered.allocated["$consumer"]] else {
        panic!("consumer")
    };
    let D::Function(consumer) = &consumer.payload else {
        panic!("consumer")
    };
    assert!(
        matches!(&final_types[&consumer.result.ty].form, k::TypeForm::AssociatedWitnessProjection { scope, parameter, contract, member } if OwnerKey::Declaration(scope.declaration) == lowered.allocated["$consumer"] && scope.package == view.package() && parameter.to_string() == WITNESS && OwnerKey::Declaration(contract.declaration) == lowered.allocated["$contract"] && contract.package == view.package() && *member == first)
    );
    assert_eq!(implementation.associated_types[0].member, second);
    assert!(matches!(
        final_types[&implementation.associated_types[0].value.ty()].form,
        k::TypeForm::OwnedI64Cell
    ));
    // Omitting a complete inventory in an edit means empty replacement, never retention.
    let mut empty = String::new();
    let mut remainder = text;
    let mut removed = 0;
    while let Some(start) = remainder.find("(associated-type ") {
        empty.push_str(&remainder[..start]);
        let clause = &remainder[start..];
        let mut depth = 0;
        let end = clause
            .char_indices()
            .find_map(|(index, ch)| {
                if ch == '(' {
                    depth += 1;
                }
                if ch == ')' {
                    depth -= 1;
                }
                (depth == 0).then_some(index + ch.len_utf8())
            })
            .unwrap();
        remainder = &clause[end..];
        removed += 1;
    }
    empty.push_str(remainder);
    assert_eq!(removed, 4);
    let mut reader = canonical::Reader::for_raw_lowered_candidate(
        &view,
        ExecutionControl::uncancelled(),
        final_owners,
        final_types,
    );
    let empty = decode_parsed_change(
        input::parse("empty-associated-inventory.lkjc", empty.as_bytes()).unwrap(),
        Some(&mut reader),
    )
    .unwrap();
    let mut empty_inventories = 0;
    for change in &empty.semantic.changes {
        match change {
            AuthoredChange::SetOwnedContract {
                associated_types, ..
            } => {
                assert!(associated_types.is_empty());
                empty_inventories += 1;
            }
            AuthoredChange::SetOwnedImplementation {
                associated_types, ..
            } => {
                assert!(associated_types.is_empty());
                empty_inventories += 1;
            }
            _ => {}
        }
    }
    assert_eq!(empty_inventories, 2);
}

#[test]
fn associated_surface_duplicates_survive_parser_and_fail_lowering() {
    let temporary = tempfile::tempdir().unwrap();
    let created = GraphRepository::create(
        &temporary.path().join("meaning"),
        &k::tests::witness_snapshot(),
        None,
    )
    .unwrap();
    let view = created.repository.view_current().unwrap();
    for (body, code) in [
        (
            compact_body().replace(&format!("id={SECOND}"), &format!("id={FIRST}")),
            "change_authored_associated_member_duplicate",
        ),
        (
            compact_body().replace("name=Spare", "name=Block"),
            "change_authored_associated_member_duplicate",
        ),
        (
            compact_body().replace(
                &format!("index=1 member={SECOND}"),
                &format!("index=1 member={FIRST}"),
            ),
            "change_authored_associated_binding_duplicate",
        ),
    ] {
        let parsed = decode_compact_change(
            "duplicate-associated.lkjc",
            format!("request base={}\n{body}", view.revision()).as_bytes(),
        )
        .unwrap();
        let error = lower_authored_changes(&view, &view, &parsed.semantic).unwrap_err();
        assert_eq!(error.code, code, "{error:#?}");
    }
}

/// Reuse the established complete logical overlay oracle. Its map page digests
/// are test placeholders; this is a semantic agreement control, not source authority.
fn full_candidate_for_request(
    view: &RepositoryView,
    request: &NormalizedChangeRequest,
) -> (k::KernelSnapshot, crate::platform::change::CanonicalDelta) {
    assert_eq!(view.revision(), request.semantic.base);
    let base = view
        .reconstruct_full_oracle()
        .expect("exact accepted baseline")
        .value;
    let lowered = lower_authored_changes(view, view, &request.semantic)
        .expect("real request lowers against the same accepted baseline");
    let delta = crate::platform::change::CanonicalDelta::normalize(&base, lowered.edits)
        .expect("same complete canonical candidate");
    let candidate =
        crate::platform::change::KernelOverlay::new(&base, &delta).materialize_logical_oracle();
    (candidate, delta)
}

#[test]
fn associated_surface_actual_preparation_accepts_complete_inventory_and_refuses_unused_binding_without_publication()
 {
    let temporary = tempfile::tempdir().expect("temporary repository");
    let created = GraphRepository::create(
        &temporary.path().join("meaning"),
        &k::tests::witness_snapshot(),
        None,
    )
    .expect("accepted ordinary baseline");
    let before = created.repository.current().expect("baseline identity");
    let view = created
        .repository
        .view_current()
        .expect("exact baseline view");
    let healthy = native_body();
    let request = decode_compact_change(
        "healthy-associated.lkjc",
        format!("request base={}\n{healthy}", before.head.revision).as_bytes(),
    )
    .expect("complete native request");
    let prepared = created
        .repository
        .prepare_authored_change(&request.semantic, request.options.clone())
        .expect("complete associated candidate prepares through the actual publication route");
    assert_eq!(prepared.publication.expected_base, Some(before.head));
    assert_ne!(prepared.publication.head.revision, before.head.revision);
    assert!(prepared.allocated.contains_key("$implementation"));
    assert!(prepared.allocated.contains_key("$consumer-self"));

    let (candidate, delta) = full_candidate_for_request(&view, &request);
    for edit in delta.owners.values() {
        assert!(
            edit.before.is_none(),
            "this literal only creates new owners"
        );
        let (digest, record) = edit.after.as_ref().expect("new complete owner");
        let (encoded_digest, bytes) = k::encode_owner(record).expect("canonical candidate owner");
        assert_eq!(*digest, encoded_digest);
        let key = crate::platform::storage::object::ObjectKey::from_digest(
            crate::platform::storage::object::ObjectDomain::Owner,
            digest.bytes(),
        );
        assert_eq!(
            prepared
                .publication
                .objects
                .get(&key)
                .expect("real staged owner"),
            &bytes,
        );
    }
    let full = k::validate_full(&candidate)
        .expect("complete incremental candidate agrees with the full semantic oracle");
    assert_eq!(
        full.owners_checked,
        prepared
            .publication
            .authority
            .semantic
            .root
            .owners
            .entries(),
    );
    let after_healthy = created
        .repository
        .current()
        .expect("still accepted baseline");
    assert_eq!(before.head, after_healthy.head);
    assert_eq!(before.revision, after_healthy.revision);
    assert_eq!(before.receipt, after_healthy.receipt);
    assert_eq!(before.semantic_root, after_healthy.semantic_root);
    assert_eq!(before.witness, after_healthy.witness);

    // Spare is never named in a method signature or body. Replace only its RHS
    // in a second complete proposal; retaining all other bindings cannot hide it.
    let spare = format!("(associated-type {SECOND} OwnedI64Cell)");
    assert_eq!(healthy.matches(&spare).count(), 1);
    let invalid = healthy.replacen(&spare, &format!("(associated-type {SECOND} Secret)"), 1);
    let invalid_request = decode_compact_change(
        "invalid-unused-associated-binding.lkjc",
        format!("request base={}\n{invalid}", before.head.revision).as_bytes(),
    )
    .expect("invalid binding still parses as a complete proposal");
    let errors = created
        .repository
        .prepare_authored_change(&invalid_request.semantic, invalid_request.options.clone())
        .expect_err("unused Secret cannot satisfy the member's Owned bound");
    assert!(
        errors
            .iter()
            .any(|error| error.class == DiagnosticClass::Semantic
                && error.code == "kernel_owned_contract"),
        "{errors:#?}",
    );
    assert!(
        errors
            .iter()
            .all(|error| error.code != "owned_associated_semantics_unsupported")
    );
    let (invalid_candidate, _) = full_candidate_for_request(&view, &invalid_request);
    let full_errors = k::validate_full(&invalid_candidate)
        .expect_err("full semantic proof also checks the unused binding");
    assert!(
        full_errors
            .iter()
            .any(|error| error.class == DiagnosticClass::Semantic
                && error.code == "kernel_owned_contract"),
        "{full_errors:#?}",
    );
    let after_invalid = created.repository.current().expect("unchanged acceptance");
    assert_eq!(before.head, after_invalid.head);
    assert_eq!(before.revision, after_invalid.revision);
    assert_eq!(before.receipt, after_invalid.receipt);
    assert_eq!(before.semantic_root, after_invalid.semantic_root);
    assert_eq!(before.witness, after_invalid.witness);
}
