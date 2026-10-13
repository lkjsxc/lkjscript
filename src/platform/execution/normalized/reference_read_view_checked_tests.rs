//! Actual old reference packets must survive the new descriptor custody boundary.
use super::super::super::owned_i64_cell::OwnedI64Cell;
use super::*;

fn owner(domain: ValueOrigin, program: ValueOrigin) -> OwnedI64Cell {
    let owner = OwnedI64Cell::new(domain, 17);
    owner.establish_admission(program).unwrap();
    owner
}

fn chain(owner: &OwnedI64Cell, domain: ValueOrigin, depth: usize) -> Arc<AncestorCustody> {
    let mut chain = None;
    for _ in 0..depth {
        let mut custody = AncestorCustody::default();
        custody.inner = chain.take();
        custody.token = Some(NormalizedValue::OwnedI64Cell(owner.borrow().unwrap()));
        custody.token_origin = Some(domain);
        chain = Some(Arc::new(custody));
    }
    chain.unwrap()
}

#[test]
fn reference_read_view_old_ancestor_finalization_is_iterative_and_shared_exactly_once() {
    let domain = ValueOrigin::fresh().unwrap();
    let program = ValueOrigin::fresh().unwrap();
    let owner = owner(domain, program);
    let shared = chain(&owner, domain, 24_000);
    let mut custody = AncestorCustody::default();
    custody.inner = Some(Arc::clone(&shared));
    custody.token = Some(NormalizedValue::OwnedI64Cell(owner.borrow().unwrap()));
    custody.token_origin = Some(domain);
    custody.outer = Some(shared);
    let packet = Arc::new(custody);
    assert!(owner.validate(domain, true).is_err());
    drop(packet);
    owner.validate(domain, true).unwrap();
}

#[test]
fn reference_read_view_flattened_old_ancestors_keep_real_loans_after_packet_drop() {
    let domain = ValueOrigin::fresh().unwrap();
    let program = ValueOrigin::fresh().unwrap();
    let owner = owner(domain, program);
    let value = Value {
        datum: NormalizedValue::OwnedI64Cell(owner.borrow().unwrap()),
        preparation: program,
        ownership: Ownership::Memory,
        provenance: None,
        guards: Some(chain(&owner, domain, 32)),
        reservation: None,
        source_anchor: None,
        source_scope: None,
        view_borrowed: false,
    };
    let mut reserved = 0;
    let (reads, packets) = value
        .flatten_read_custody(domain, &ExecutionControl::uncancelled(), &mut |bytes| {
            reserved += bytes;
            Ok(())
        })
        .unwrap();
    assert_eq!(reads.len(), 32);
    assert!(packets.is_empty());
    assert!(reserved > 0);
    drop(value);
    assert!(owner.validate(domain, true).is_err());
    drop(reads);
    owner.validate(domain, true).unwrap();
}

#[test]
fn reference_read_view_failed_flatten_reservation_preserves_original_custody_and_retry() {
    let domain = ValueOrigin::fresh().unwrap();
    let program = ValueOrigin::fresh().unwrap();
    let owner = owner(domain, program);
    let value = Value {
        datum: NormalizedValue::OwnedI64Cell(owner.borrow().unwrap()),
        preparation: program,
        ownership: Ownership::Memory,
        provenance: None,
        guards: Some(chain(&owner, domain, 16)),
        reservation: None,
        source_anchor: None,
        source_scope: None,
        view_borrowed: false,
    };
    let mut reservations = 0;
    let refused = value.flatten_read_custody(domain, &ExecutionControl::uncancelled(), &mut |_| {
        reservations += 1;
        if reservations == 5 {
            Err(ExecutionError::resource(
                "reference_test_reservation",
                "controlled reservation refusal",
            ))
        } else {
            Ok(())
        }
    });
    assert!(refused.is_err());
    assert!(owner.validate(domain, true).is_err());
    let (reads, _) = value
        .flatten_read_custody(domain, &ExecutionControl::uncancelled(), &mut |_| Ok(()))
        .unwrap();
    assert_eq!(reads.len(), 16);
    drop(reads);
    drop(value);
    owner.validate(domain, true).unwrap();
}
