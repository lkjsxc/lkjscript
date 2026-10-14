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
        source_binding: None,
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
        source_binding: None,
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

#[test]
fn reference_source_backing_refusal_sharing_and_expiry_preserve_exact_custody() {
    use super::super::super::read_view_storage::{
        AdmissionRequest, ReadViewAdmission, RuntimeScopeLease, RuntimeScopeOwner, SourceAnchor,
        SourceCoordinate,
    };
    use crate::platform::kernel::ReadScopeArgument;
    use crate::platform::semantic_id::DeclarationId;
    const SEED: &[u8] = b"reference-source-binding-compaction";

    struct ExactScope<'a> {
        key: ReadScopeArgument,
        source: &'a SourceAnchor,
    }
    impl ReadViewAdmission for ExactScope<'_> {
        fn validate(
            &self,
            request: AdmissionRequest<'_>,
            control: &ExecutionControl,
        ) -> Result<(), ExecutionError> {
            control.check()?;
            match request {
                AdmissionRequest::Scope {
                    key,
                    source,
                    parent,
                } if key == self.key && source == self.source && parent.is_none() => Ok(()),
                _ => Err(ExecutionError::resource(
                    "reference_test_scope",
                    "unexpected scope request",
                )),
            }
        }
    }

    let domain = ValueOrigin::fresh().unwrap();
    let program = ValueOrigin::fresh().unwrap();
    let owner = owner(domain, program);
    let control = ExecutionControl::uncancelled();
    let anchor = SourceAnchor {
        program,
        application: ValueOrigin::fresh().unwrap(),
        invocation: ValueOrigin::fresh().unwrap(),
        coordinate: SourceCoordinate::Parameter(0),
        origin: domain,
        path: Arc::from([]),
    };
    let key = ReadScopeArgument::Lexical {
        callable: DeclarationReference {
            package: PackageId::migrate(SEED, 3001),
            declaration: DeclarationId::migrate(SEED, 3002),
        },
        binding: BindingId::migrate(SEED, 3003),
    };
    let admission = ExactScope {
        key,
        source: &anchor,
    };
    let (scope_owner, scope) =
        RuntimeScopeOwner::open(key, &anchor, None, &admission, &control, &mut |_| Ok(())).unwrap();
    let mut reservations = Vec::new();
    let original = Value::reserve_source_binding(
        anchor.clone(),
        Some(scope.reborrow(&control).unwrap()),
        &mut |bytes| {
            reservations.push(bytes);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(reservations.len(), 1);
    // Independently count the payload and the standard Arc strong/weak counter storage.
    let expected_heap = (std::mem::size_of::<SourceBinding>()
        + 2 * std::mem::size_of::<std::sync::atomic::AtomicUsize>()) as u64;
    assert_eq!(reservations[0], expected_heap);
    let mut value = Value {
        datum: NormalizedValue::OwnedI64Cell(owner.borrow().unwrap()),
        preparation: program,
        ownership: Ownership::Memory,
        provenance: Some(LocalValueReference::LexicalBinding(BindingId::migrate(
            SEED, 3004,
        ))),
        guards: None,
        reservation: None,
        source_binding: Some(Arc::clone(&original)),
        view_borrowed: false,
    };
    let new_anchor = SourceAnchor {
        coordinate: SourceCoordinate::Parameter(1),
        ..anchor.clone()
    };
    let mut refused_reservations = 0;
    let mut remaining = expected_heap - 1;
    let refusal = Value::reserve_source_binding(
        new_anchor.clone(),
        Some(scope.reborrow(&control).unwrap()),
        &mut |bytes| {
            refused_reservations += 1;
            if bytes > remaining {
                return Err(ExecutionError::resource(
                    "normalized_reference_allocation",
                    "controlled backing reservation refusal",
                ));
            }
            remaining -= bytes;
            Ok(())
        },
    )
    .err()
    .unwrap();
    assert_eq!(refusal.code, "normalized_reference_allocation");
    assert_eq!(refused_reservations, 1);
    assert_eq!(value.source_anchor(), Some(&anchor));
    assert!(value.source_scope().unwrap().same_live_scope(&scope));
    assert!(Arc::ptr_eq(&value.source_binding().unwrap(), &original));
    assert!(owner.validate(domain, true).is_err());

    // A different placement mints a new charged backing, without changing the retained scope.
    let new_binding = Value::reserve_source_binding(
        new_anchor.clone(),
        Some(scope.reborrow(&control).unwrap()),
        &mut |bytes| {
            reservations.push(bytes);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(reservations, [expected_heap, expected_heap]);
    value.bind_source(new_binding);
    assert_eq!(value.source_anchor(), Some(&new_anchor));
    value.restore_source_binding(Some(Arc::clone(&original)));
    assert_eq!(value.source_anchor(), Some(&anchor));
    assert_eq!(
        reservations.len(),
        2,
        "restoration allocates no new backing"
    );
    let borrowed = value.duplicate(ParameterUse::Borrow).unwrap();
    assert!(Arc::ptr_eq(&borrowed.source_binding().unwrap(), &original));
    assert_eq!(borrowed.source_anchor(), Some(&anchor));
    assert_eq!(borrowed.provenance(), value.provenance());
    assert_eq!(
        reservations.len(),
        2,
        "borrowing shares immutable placement metadata"
    );

    let ordinary = Value {
        datum: NormalizedValue::I64(17),
        preparation: program,
        ownership: Ownership::Ordinary,
        provenance: None,
        guards: None,
        reservation: None,
        source_binding: None,
        view_borrowed: false,
    };
    let copy = ordinary.duplicate(ParameterUse::Unrestricted).unwrap();
    assert!(ordinary.source_binding().is_none() && copy.source_binding().is_none());
    assert!(ordinary.source_anchor().is_none() && ordinary.source_scope().is_none());
    assert!(
        std::mem::size_of::<Value>()
            < std::mem::size_of::<NormalizedValue>()
                + std::mem::size_of::<SourceAnchor>()
                + std::mem::size_of::<Option<RuntimeScopeLease>>()
    );

    // A shared backing never extends lifetime: owner invalidation still rejects the next read.
    drop(scope_owner);
    assert_eq!(
        value.duplicate(ParameterUse::Borrow).err().unwrap().code,
        "normalized_read_view_token"
    );
    assert!(owner.validate(domain, true).is_err());
    drop(borrowed);
    drop(value);
    drop(original);
    owner.validate(domain, true).unwrap();
}
