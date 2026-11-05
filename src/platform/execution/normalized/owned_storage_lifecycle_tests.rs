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
fn owned_create_rejection_drains_deep_raw_unary_fields() -> TestResult {
    let payload: Arc<str> = Arc::from("rejected-field-leaf");
    let mut raw = NormalizedValue::Text(Arc::clone(&payload));
    for _ in 0..20_000 {
        raw = NormalizedValue::Option(Some(Box::new(raw)));
    }
    let result = thread::Builder::new()
        // Admission still uses the separately bounded structural escape walk;
        // this control targets the much deeper rejected-value teardown.
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            OwnedStorage::create(
                ValueOrigin::default(),
                TypeObjectDigest::from_bytes([212; 32]),
                vec![raw],
                &ExecutionControl::uncancelled(),
                &mut |_| Ok(()),
            )
        })?
        .join()
        .map_err(|_| "rejected raw affine field worker panicked")?;
    assert!(matches!(result, Err(error) if error.code == "normalized_value_depth"));
    assert_eq!(Arc::strong_count(&payload), 1);
    Ok(())
}
