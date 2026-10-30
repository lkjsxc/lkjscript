//! Raw candidate readback only. The ordinary repository view does not admit these types.
use super::*;
use crate::platform::kernel::{
    AssociatedTypeBinding, AssociatedTypeBound, AssociatedTypeMember, DeclarationReference,
    TypeObject,
};
use crate::platform::publication::GraphRepository;
use crate::platform::semantic_id::{AssociatedTypeMemberId, ImplementationParameterId};

#[test]
fn raw_candidate_associated_readback_preserves_exact_coordinates_unused_inventory_and_observations()
{
    let temporary = tempfile::tempdir().unwrap();
    let base = k::tests::witness_snapshot();
    let created = GraphRepository::create(&temporary.path().join("ordinary"), &base, None).unwrap();
    let view = created.repository.view_current().unwrap();
    let contract = DeclarationReference {
        package: PackageId::migrate(b"readback-supplier", 1),
        declaration: DeclarationId::migrate(b"readback-supplier", 1),
    };
    let scope = DeclarationReference {
        package: PackageId::migrate(b"readback-consumer", 1),
        declaration: DeclarationId::migrate(b"readback-consumer", 1),
    };
    let parameter = ImplementationParameterId::migrate(b"readback-consumer", 1);
    let members = vec![
        AssociatedTypeMember {
            id: AssociatedTypeMemberId::migrate(b"readback", 2),
            name: Name::new("Unused").unwrap(),
            bound: AssociatedTypeBound::Owned,
        },
        AssociatedTypeMember {
            id: AssociatedTypeMemberId::migrate(b"readback", 1),
            name: Name::new("Out").unwrap(),
            bound: AssociatedTypeBound::Owned,
        },
    ];
    let forms = [
        TypeForm::AssociatedContractMember {
            contract,
            member: members[0].id,
        },
        TypeForm::AssociatedWitnessProjection {
            scope,
            parameter,
            contract,
            member: members[1].id,
        },
    ];
    let mut types = BTreeMap::new();
    let mut bindings = Vec::new();
    for (member, form) in members.iter().zip(forms) {
        let (digest, bytes) = k::encode_type_object(&TypeObject::new(form).unwrap()).unwrap();
        types.insert(digest, k::decode_type_object(&bytes, digest).unwrap());
        bindings.push(AssociatedTypeBinding {
            member: member.id,
            value: k::AssociatedTypeBindingValue::Owned { ty: digest },
        });
    }
    let mut reader = Reader::for_raw_lowered_candidate(
        &view,
        ExecutionControl::uncancelled(),
        BTreeMap::new(),
        types.clone(),
    );
    let authored_members = reader.associated_members(&members).unwrap();
    assert_eq!(
        authored_members
            .iter()
            .map(|m| (m.id, &m.name, m.bound.clone()))
            .collect::<Vec<_>>(),
        members
            .iter()
            .map(|m| (m.id, &m.name, m.bound.clone()))
            .collect::<Vec<_>>()
    );
    let authored_bindings = reader.associated_bindings(&bindings).unwrap();
    assert_eq!(
        authored_bindings[0],
        AuthoredAssociatedTypeBinding {
            member: members[0].id,
            value: AuthoredAssociatedTypeBindingValue::Owned {
                ty: AuthoredType::AssociatedContractMember {
                    contract: declaration(contract),
                    member: members[0].id
                }
            },
        }
    );
    assert_eq!(
        authored_bindings[1],
        AuthoredAssociatedTypeBinding {
            member: members[1].id,
            value: AuthoredAssociatedTypeBindingValue::Owned {
                ty: AuthoredType::AssociatedWitnessProjection {
                    scope: declaration(scope),
                    parameter,
                    contract: declaration(contract),
                    member: members[1].id
                }
            },
        }
    );
    // Two member entries, two bindings, and each of the two raw type occurrences.
    assert_eq!(
        reader.type_nodes,
        crate::platform::change::budget::BudgetCounter::exact(6)
    );
    for available in [5, 6] {
        let mut reader = Reader::for_raw_lowered_candidate(
            &view,
            ExecutionControl::uncancelled(),
            BTreeMap::new(),
            types.clone(),
        );
        reader.type_nodes =
            crate::platform::change::budget::BudgetCounter::exact(u64::MAX - available);
        reader.associated_members(&members).unwrap();
        assert_eq!(
            reader.associated_bindings(&bindings).unwrap(),
            authored_bindings
        );
        assert_eq!(reader.type_nodes.consumed, u64::MAX);
        assert_eq!(reader.type_nodes.saturated, available < 6);
    }
    let mut cancelled = Reader::for_raw_lowered_candidate(
        &view,
        ExecutionControl::cancel_after_checks(0),
        BTreeMap::new(),
        types,
    );
    assert_eq!(
        cancelled.associated_members(&members).unwrap_err().class,
        DiagnosticClass::Cancelled
    );
    // The real view remains feature-free: raw readback never publishes candidate objects.
    assert_eq!(
        created.repository.view_current().unwrap().revision(),
        view.revision()
    );
}
