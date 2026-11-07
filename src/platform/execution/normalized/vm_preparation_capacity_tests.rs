//! Preparation observations do not grant finite execution capacity.
//! Counter probes are synthetic; the authored VM control checks real attachment and cleanup.
use super::super::shared_budget::SharedBudget;
use super::*;
use crate::platform::execution::normalized::tests::byte_buffer_tests;
use crate::platform::kernel::{OwnerKey, OwnerRecord};
use crate::platform::publication::GraphRepository;

fn policy(maximum: Option<u64>) -> NormalizedRunPolicy {
    NormalizedRunPolicy {
        maximum_allocated_bytes: maximum,
        ..NormalizedRunPolicy::foreground()
    }
}

fn resource_refusal(result: Result<(), ExecutionError>) {
    let error = result.expect_err("finite preparation attachment must refuse");
    assert_eq!(error.class, ExecutionFailureClass::Resource);
    assert_eq!(error.code, "normalized_allocation");
}

#[test]
fn preparation_storage_attachment_preserves_optional_exact_allowances() {
    for maximum in [None, Some(0), Some(6), Some(7), Some(8), Some(u64::MAX)] {
        let mut observed = NormalizedRunObservation {
            calls: 11,
            type_derivation_steps: 13,
            type_metadata_bytes: 5,
            ..NormalizedRunObservation::default()
        };
        let before = observed.clone();
        let result = reserve_preparation_allocation(
            policy(maximum),
            None,
            &mut observed,
            Observation::exact(5),
            2,
        );
        if maximum.is_some_and(|value| value < 7) {
            resource_refusal(result);
            assert_eq!(observed, before);
        } else {
            assert!(result.is_ok());
            assert_eq!(observed.allocated_bytes, 7);
            assert!(!observed.allocated_bytes_saturated);
            assert_eq!(observed.allocation_charges, 1);
            assert_eq!(observed.calls, 11);
            assert_eq!(observed.type_derivation_steps, 13);
            assert_eq!(observed.type_metadata_bytes, 5);
        }
    }
    let mut zero = NormalizedRunObservation::default();
    let before = zero.clone();
    assert!(
        reserve_preparation_allocation(policy(Some(0)), None, &mut zero, Observation::exact(0), 0,)
            .is_ok()
    );
    assert_eq!(zero, before);
}

#[test]
fn preparation_storage_exact_maximum_and_saturation_are_distinct() {
    let mut observed = NormalizedRunObservation::default();
    assert!(
        reserve_preparation_allocation(
            policy(Some(u64::MAX)),
            None,
            &mut observed,
            Observation::exact(u64::MAX),
            0,
        )
        .is_ok()
    );
    assert_eq!(observed.allocated_bytes, u64::MAX);
    assert!(!observed.allocated_bytes_saturated);
    let before = observed.clone();
    resource_refusal(reserve_preparation_allocation(
        policy(Some(u64::MAX)),
        None,
        &mut observed,
        Observation::exact(0),
        1,
    ));
    assert_eq!(observed, before);
    assert!(
        reserve_preparation_allocation(
            policy(None),
            None,
            &mut observed,
            Observation::exact(0),
            1,
        )
        .is_ok()
    );
    assert_eq!(observed.allocated_bytes, u64::MAX);
    assert!(observed.allocated_bytes_saturated);

    for maximum in [None, Some(0), Some(7), Some(u64::MAX)] {
        for prepared in [
            Observation::exact(u64::MAX),
            Observation {
                consumed: 7,
                saturated: true,
            },
        ] {
            let mut observed = NormalizedRunObservation::default();
            let before = observed.clone();
            let result =
                reserve_preparation_allocation(policy(maximum), None, &mut observed, prepared, 1);
            if maximum.is_some() {
                resource_refusal(result);
                assert_eq!(observed, before);
            } else {
                assert!(result.is_ok());
                assert!(observed.allocated_bytes_saturated);
                assert_eq!(
                    observed.allocated_bytes,
                    prepared.consumed.saturating_add(1)
                );
                assert_eq!(observed.allocation_charges, 1);
            }
        }
    }
}

#[test]
fn preparation_storage_shared_refusal_preserves_both_ledgers() {
    for maximum in [Some(0), Some(6), Some(7), Some(8), Some(u64::MAX)] {
        let shared = SharedBudget::new(policy(maximum), None, 0, 0);
        let mut observed = NormalizedRunObservation::default();
        let before = observed.clone();
        let result = reserve_preparation_allocation(
            policy(None),
            Some(&shared),
            &mut observed,
            Observation::exact(5),
            2,
        );
        let maximum = maximum.expect("finite matrix");
        if maximum < 7 {
            resource_refusal(result);
            assert_eq!(observed, before);
            // The complete original allowance remains after refusal.
            assert!(shared.reserve(maximum, 0).is_ok());
        } else {
            assert!(result.is_ok());
            assert_eq!(observed.allocated_bytes, 7);
            assert!(!observed.allocated_bytes_saturated);
            assert!(shared.reserve(maximum - 7, 0).is_ok());
        }
        resource_refusal(shared.reserve(1, 0));
    }

    let shared = SharedBudget::new(policy(Some(u64::MAX)), None, 0, 0);
    let mut observed = NormalizedRunObservation::default();
    let before = observed.clone();
    resource_refusal(reserve_preparation_allocation(
        policy(Some(0)),
        Some(&shared),
        &mut observed,
        Observation::exact(5),
        2,
    ));
    assert_eq!(observed, before);
    // Local refusal happens before the shared finite reservation.
    assert!(shared.reserve(u64::MAX, 0).is_ok());
    resource_refusal(shared.reserve(1, 0));
}

#[test]
fn preparation_storage_saturation_never_grants_shared_finite_permission() {
    for maximum in [0, 7, u64::MAX] {
        let shared = SharedBudget::new(policy(Some(maximum)), None, 0, 0);
        let mut observed = NormalizedRunObservation::default();
        let before = observed.clone();
        resource_refusal(reserve_preparation_allocation(
            policy(None),
            Some(&shared),
            &mut observed,
            Observation {
                consumed: u64::MAX,
                saturated: true,
            },
            0,
        ));
        assert_eq!(observed, before);
        assert!(shared.reserve(maximum, 0).is_ok());
        resource_refusal(shared.reserve(1, 0));
    }
    let shared = SharedBudget::new(policy(Some(u64::MAX)), None, 0, 0);
    let mut observed = NormalizedRunObservation::default();
    assert!(
        reserve_preparation_allocation(
            policy(None),
            Some(&shared),
            &mut observed,
            Observation::exact(u64::MAX),
            0,
        )
        .is_ok()
    );
    assert!(!observed.allocated_bytes_saturated);
    resource_refusal(shared.reserve(1, 0));
}

#[test]
fn preparation_storage_unbounded_bytes_keep_collection_permission_independent() {
    let shared = SharedBudget::new(
        NormalizedRunPolicy {
            maximum_collection_items: Some(0),
            ..policy(None)
        },
        None,
        0,
        0,
    );
    let mut observed = NormalizedRunObservation::default();
    assert!(
        reserve_preparation_allocation(
            policy(None),
            Some(&shared),
            &mut observed,
            Observation::exact(u64::MAX),
            1,
        )
        .is_ok()
    );
    assert_eq!(observed.allocated_bytes, u64::MAX);
    assert!(observed.allocated_bytes_saturated);
    assert_eq!(observed.collection_items, 0);
    assert!(!observed.collection_items_saturated);
    let error = shared.reserve(0, 1).expect_err("zero item allowance");
    assert_eq!(error.code, "normalized_collection_items");
    assert!(shared.reserve(u64::MAX, 0).is_ok());
}

fn fixture() -> Result<(NormalizedProgram, DeclarationReference), String> {
    let source = byte_buffer_tests::author_only(
        "declarations.begin\n(units (module create preparation\n         (function create answer (visibility public) (effect pure) (returns I64)\n         (body (i64 42)))))\ndeclarations.end",
    )?;
    let declaration = source
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(owner))
                if owner.name.as_str() == "answer" =>
            {
                Some(DeclarationReference {
                    package: source.root.package_id,
                    declaration: *declaration,
                })
            }
            _ => None,
        })
        .ok_or("missing authored answer")?;
    let memory = crate::platform::MemProjectStore::new(None).map_err(|e| format!("{e:?}"))?;
    let created =
        GraphRepository::create_memory(&memory, &source, None).map_err(|e| format!("{e:?}"))?;
    let compilation = crate::platform::compiler::build_clean(
        &created.repository,
        crate::platform::compiler::OptimizationPolicy::DeterministicBaseline,
    )
    .map_err(|e| format!("{e:?}"))?;
    let linked = crate::platform::compiler::link_artifact(
        &created.repository,
        compilation.manifest_digest,
        &[],
    )
    .map_err(|e| format!("{e:?}"))?;
    let loaded = crate::platform::compiler::load_artifact(&linked.artifact.bytes)
        .map_err(|e| format!("{e:?}"))?;
    let program = NormalizedProgram::prepare(loaded).map_err(|e| format!("{e:?}"))?;
    Ok((program, declaration))
}

#[test]
fn preparation_storage_real_vm_retains_flags_refuses_finite_and_cancels() -> Result<(), String> {
    let (mut program, declaration) = fixture()?;
    // Seed passive counters only. This does not claim a u64::MAX-sized allocation.
    program.work.type_derivation_steps = u64::MAX;
    program.work.type_derivation_saturated = true;
    program.work.type_metadata_bytes = u64::MAX;
    program.work.type_metadata_saturated = true;
    let (value, observed) = NormalizedVm::for_test(&program, policy(None))
        .invoke(
            declaration,
            Vec::new(),
            None,
            &ExecutionControl::uncancelled(),
        )
        .map_err(|e| format!("{e:?}"))?;
    assert!(matches!(value, NormalizedValue::I64(42)));
    assert_eq!(observed.type_derivation_steps, u64::MAX);
    assert!(observed.type_derivation_saturated);
    assert_eq!(observed.type_metadata_bytes, u64::MAX);
    assert!(observed.type_metadata_saturated);
    assert!(observed.allocated_bytes_saturated);
    assert_eq!(observed.live_call_frames_after, 0);
    assert_eq!(observed.live_operands_after, 0);
    assert_eq!(observed.live_handles_after, 0);

    let observation = std::sync::Mutex::new(None);
    let error = NormalizedVm::for_test(&program, policy(Some(u64::MAX)))
        .observing_checked(&observation)
        .invoke(
            declaration,
            Vec::new(),
            None,
            &ExecutionControl::uncancelled(),
        )
        .expect_err("saturated preparation cannot fit a finite byte allowance");
    assert_eq!(error.class, ExecutionFailureClass::Resource);
    assert_eq!(error.code, "normalized_allocation");
    let failed = observation
        .lock()
        .map_err(|e| e.to_string())?
        .clone()
        .ok_or("missing failed VM observation")?;
    assert_eq!(failed.allocated_bytes, 0);
    assert!(!failed.allocated_bytes_saturated);
    assert_eq!(failed.allocation_charges, 0);
    assert_eq!(failed.live_call_frames_after, 0);
    assert_eq!(failed.live_operands_after, 0);
    assert_eq!(failed.live_handles_after, 0);

    let error = NormalizedVm::for_test(&program, policy(Some(u64::MAX)))
        .invoke(
            declaration,
            Vec::new(),
            None,
            &ExecutionControl::cancel_after_checks(0),
        )
        .expect_err("cancellation precedes attachment");
    assert_eq!(error.class, ExecutionFailureClass::Cancelled);
    Ok(())
}
