use super::*;
use crate::platform::kernel::encode_type_object;
use crate::platform::storage::memory::MemoryPackedStore;
use crate::platform::witness::rebuild_full_witness;

#[test]
fn ordinary_sequence_interface_closures_admit_scalars_and_generic_elements() {
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
    }
}
