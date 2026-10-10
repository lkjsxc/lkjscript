//! Exact ownership work admission and invalid-path refusal.
use super::*;

#[test]
fn shared_checked_unit_projection_charges_one_walk_and_refuses_before_overrun() {
    let mut witness =
        rebuild_full_witness(&crate::platform::kernel::tests::witness_snapshot()).unwrap();
    let port = OwnerKey::Port(PortId::migrate(b"projection-budget", 0));
    let root = OwnerKey::Declaration(DeclarationId::migrate(b"projection-budget", 0));
    witness.entries.ownership =
        BTreeMap::from([(port, edge(root, OwnershipRole::DeclarationPort))]);
    let derived = DerivedDelta::default();
    for limit in 0..=4 {
        let mut ownership = CandidateOwnership::new(&derived, &witness);
        let mut work = ImpactWork::default();
        let (mut units, mut semantic, mut admitted) =
            (BTreeSet::new(), BTreeSet::from([port]), BTreeSet::new());
        let result = add_checked_units(
            port,
            &mut ownership,
            &mut units,
            &mut semantic,
            &mut work,
            &mut admitted,
            OwningUnitAdmission {
                maximum_affected: 1,
                maximum_ownership_steps: limit,
            },
        );
        assert_eq!(work.ownership_steps, limit);
        if limit < 4 {
            let error = result.unwrap_err();
            assert_eq!(error.class, DiagnosticClass::Resource);
            assert_eq!(error.code, "change_budget_impact_ownership_steps");
            assert!(units.is_empty() && admitted.is_empty());
            assert_eq!(semantic, BTreeSet::from([port]));
        } else {
            result.unwrap();
            assert_eq!(units, BTreeSet::from([root]));
            assert_eq!(semantic, BTreeSet::from([port, root]));
            assert_eq!(admitted, units);
        }
    }
}

#[test]
fn before_and_candidate_ownership_cycles_never_become_unit_sets() {
    let mut witness =
        rebuild_full_witness(&crate::platform::kernel::tests::witness_snapshot()).unwrap();
    let a = OwnerKey::Port(PortId::migrate(b"projection-cycle", 0));
    let b = OwnerKey::Port(PortId::migrate(b"projection-cycle", 1));
    let cycle = BTreeMap::from([
        (a, edge(b, OwnershipRole::DeclarationPort)),
        (b, edge(a, OwnershipRole::DeclarationPort)),
    ]);
    for candidate_cycle in [false, true] {
        witness.entries.ownership = if candidate_cycle {
            BTreeMap::new()
        } else {
            cycle.clone()
        };
        let after = if candidate_cycle {
            cycle.clone()
        } else {
            BTreeMap::new()
        };
        let derived = delta(&witness.entries.ownership, &after);
        let mut ownership = CandidateOwnership::new(&derived, &witness);
        let error = owning_units(a, &mut ownership, &mut ImpactWork::default(), 8).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Corrupt);
        assert_eq!(error.code, "change_impact_unit_cycle");
    }
}
