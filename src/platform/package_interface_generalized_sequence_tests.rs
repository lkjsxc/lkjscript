use super::*;
use crate::platform::kernel::{
    OwnerHeader, OwnerKind, PackageInterfaceRecord, TypeParameterConstraints, TypeParameterRecord,
    encode_type_object,
};
use crate::platform::storage::memory::MemoryPackedStore;
use crate::platform::witness::rebuild_full_witness;

#[test]
fn interface19_rejects_graph31_owners_in_rehashed_interface18_envelopes() {
    let owner = OwnerKey::TypeParameter(TypeParameterId::migrate(b"interface31", 0));
    let value = PackageInterfaceOwner {
        contract_version: 19,
        record: PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(owner, OwnerKind::TypeParameter),
            declaration: DeclarationId::migrate(b"interface31", 0),
            name: crate::platform::kernel::Name::new("T").unwrap(),
            constraints: TypeParameterConstraints::Transferable,
        }),
    };
    let (digest, bytes) = value.encode().unwrap();
    assert_eq!(&bytes[..8], b"LKJPIF19");
    assert_eq!(
        PackageInterfaceOwner::decode(&bytes, owner, digest).unwrap(),
        value
    );
    let mut old = value;
    old.contract_version = 18;
    assert!(old.encode().is_err());
    let raw = crate::platform::packed::encode(
        *b"LKJPIF18",
        "lkjscript.package-interface-owner-envelope.v18",
        &old,
        MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
    )
    .unwrap();
    let error = PackageInterfaceOwner::decode(&raw, owner, PackageInterfaceOwnerDigest::of(&raw))
        .unwrap_err();
    assert!(
        error
            .message
            .contains("Graph 31 owners require interface generation 19")
    );
}

#[test]
fn ordinary_sequence_interface_closures_require_graph31_for_each_signature_owner() {
    for declarations in [
        r#"(function create relay (visibility public) (effect pure)
          (parameter create p (type (owned-sequence I64)) (use consume))
          (returns (owned-sequence I64)) (body (local p)))"#,
        r#"(function create relay (visibility public) (effect pure)
          (type-parameter create T (constraint transferable))
          (parameter create p (type (owned-sequence T)) (use consume))
          (returns (owned-sequence T)) (body (local p)))"#,
    ] {
        let source = format!(
            "declarations.begin\n(units (module create sequence-interface {declarations}))\ndeclarations.end"
        );
        let snapshot =
            crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(&source)
                .unwrap();
        let witness = rebuild_full_witness(&snapshot).unwrap();
        let selection =
            PackageInterfaceSelection::from_records(snapshot.root.package_id, &snapshot.owners)
                .unwrap();
        let owners = snapshot
            .owners
            .iter()
            .filter_map(|(key, record)| {
                PackageInterfaceOwner::project(record, &witness.summaries[key], &selection)
                    .unwrap()
                    .map(|owner| (*key, owner))
            })
            .collect::<BTreeMap<_, _>>();
        let types = snapshot
            .types
            .iter()
            .map(|(digest, object)| (*digest, encode_type_object(object).unwrap().1))
            .collect();
        let validate = |owners: &BTreeMap<OwnerKey, PackageInterfaceOwner>| {
            let build = build_package_interface(owners, &types).unwrap();
            let mut store = MemoryPackedStore::default();
            let mut work = StoreWork::default();
            for (key, bytes) in &build.objects {
                store.stage(*key, bytes, &mut work).unwrap();
            }
            validate_package_interface(snapshot.root.package_id, build.root, &store, &mut work)
        };
        assert!(validate(&owners).is_ok());
        for key in owners.keys() {
            let mut hostile = owners.clone();
            let changed = hostile.get_mut(key).unwrap();
            match &mut changed.record {
                PackageInterfaceRecord::Declaration(record) => record.header.contract_version = 30,
                PackageInterfaceRecord::Parameter(record) => record.header.contract_version = 30,
                _ => continue,
            }
            changed.contract_version = 18;
            assert_eq!(
                validate(&hostile).unwrap_err().code,
                "kernel_sequence_generation"
            );
        }
    }
}
