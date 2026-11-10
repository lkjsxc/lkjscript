//! Raw lifecycle fixtures deliberately bypass type/copyability admission.
//! Their rejected ownership shapes must still be destructible safely.

use super::*;
use crate::platform::execution::normalized::child_storage::Captures;
use crate::platform::execution::normalized::value::FunctionIndex;
use std::sync::Weak;
use std::thread;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn raw_owner(fields: Vec<NormalizedValue>) -> OwnedStorage {
    OwnedStorage {
        origin: ValueOrigin::default(),
        backing_origin: ValueOrigin::default(),
        ty: TypeObjectDigest::from_bytes([211; 32]),
        storage: Arc::new(Mutex::new(Storage {
            fields: Some(fields),
            loans: 0,
            admitted_program: None,
            admission_valid: false,
            view_free: false,
            retired_fields: None,
            cleanup_next: None,
        })),
        mode: Mode::Owner,
    }
}

#[test]
fn owned_retirement_mixed_raw_capture_chain_releases_exactly_once() -> TestResult {
    let payload: Arc<str> = Arc::from("affine-retirement-leaf");
    let mut value = NormalizedValue::Text(Arc::clone(&payload));
    let mut observed: Vec<Weak<Mutex<Storage>>> = Vec::new();
    for _ in 0..4_096 {
        let owner = raw_owner(vec![value]);
        observed.push(Arc::downgrade(&owner.storage));
        value = NormalizedValue::Function {
            function: FunctionIndex(0, ValueOrigin::default()),
            type_arguments: Default::default(),
            effect_arguments: Default::default(),
            requirement_arguments: Default::default(),
            read_scope_arguments: Default::default(),
            bound_arguments: Some(Captures::try_new(vec![NormalizedValue::OwnedProduct(
                owner,
            )])?),
        };
    }
    let survivor = value.clone();
    drop(value);
    assert!(observed.iter().all(|storage| storage.strong_count() == 1));
    assert_eq!(Arc::strong_count(&payload), 2);
    thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(move || drop(survivor))?
        .join()
        .map_err(|_| "mixed affine retirement worker panicked")?;
    assert!(observed.iter().all(|storage| storage.strong_count() == 0));
    assert_eq!(Arc::strong_count(&payload), 1);
    Ok(())
}

#[test]
fn owned_create_and_rejection_drain_deep_raw_unary_fields() -> TestResult {
    crate::platform::stack::run("owned-raw-fields", || -> TestResult {
        for depth in [256, 2048, 20_000] {
            for refuse in [false, true] {
                let payload: Arc<str> = Arc::from("raw-field-leaf");
                let mut raw = NormalizedValue::Text(Arc::clone(&payload));
                for _ in 0..depth {
                    raw = NormalizedValue::Option(Some(Box::new(raw)));
                }
                let result = OwnedStorage::create(
                    ValueOrigin::default(),
                    TypeObjectDigest::from_bytes([212; 32]),
                    vec![raw],
                    &ExecutionControl::uncancelled(),
                    &mut |_| {
                        if refuse {
                            Err(ExecutionError::resource(
                                "test_allocation_failure",
                                "injected allocation failure",
                            ))
                        } else {
                            Ok(())
                        }
                    },
                );
                if refuse {
                    assert!(
                        matches!(result, Err(error) if error.code == "test_allocation_failure")
                    );
                } else {
                    let value = result.unwrap();
                    assert_eq!(Arc::strong_count(&payload), 2);
                    drop(value);
                }
                assert_eq!(Arc::strong_count(&payload), 1);
            }
        }
        Ok(())
    })?
}
