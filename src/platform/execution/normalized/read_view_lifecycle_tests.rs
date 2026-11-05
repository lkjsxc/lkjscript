//! Shared metadata teardown and rejected raw loan packets join the common drain.

use super::*;
use crate::platform::execution::normalized::child_storage::Captures;
use crate::platform::execution::normalized::value::FunctionIndex;
use std::sync::atomic::AtomicUsize;
use std::thread;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn chain(depth: usize, mut value: NormalizedValue) -> TestResult<NormalizedValue> {
    for _ in 0..depth {
        value = NormalizedValue::Function {
            function: FunctionIndex(0, ValueOrigin::default()),
            type_arguments: Default::default(),
            effect_arguments: Default::default(),
            requirement_arguments: Default::default(),
            read_scope_arguments: Default::default(),
            bound_arguments: Some(Captures::try_new(vec![value])?),
        };
    }
    Ok(value)
}

#[test]
fn read_view_ready_metadata_teardown_preserves_shared_survivor() -> TestResult {
    let payload: Arc<str> = Arc::from("read-view-metadata-leaf");
    let metadata = Arc::new(chain(16_384, NormalizedValue::Text(Arc::clone(&payload)))?);
    let finalized = Arc::new(AtomicUsize::new(0));
    let mut work = DropQueue::default();
    for index in [1_u8, 2] {
        work.enqueue_view(Box::new(ReadyCleanup {
            fields: vec![StoredField {
                descriptor: FieldDescriptor {
                    field: FieldId::from_bytes([index; 16]).ok_or("invalid test field identity")?,
                    ty: TypeObjectDigest::from_bytes([213; 32]),
                    source: None,
                    kind: FieldKind::Metadata,
                },
                value: StoredValue::Metadata(Arc::clone(&metadata)),
            }],
            custody: None,
            next: None,
            finalized: Some(Arc::clone(&finalized)),
        }));
    }
    thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(move || work.drain())?
        .join()
        .map_err(|_| "read-view metadata queue worker panicked")?;
    assert_eq!(finalized.load(Ordering::Relaxed), 2);
    assert_eq!(Arc::strong_count(&metadata), 1);
    assert_eq!(Arc::strong_count(&payload), 2);
    thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(move || drop(metadata))?
        .join()
        .map_err(|_| "read-view metadata final owner worker panicked")?;
    assert_eq!(Arc::strong_count(&payload), 1);
    Ok(())
}

#[test]
fn read_view_invalid_loan_packet_drains_deep_raw_value() -> TestResult {
    let payload: Arc<str> = Arc::from("rejected-loan-leaf");
    let mut raw = NormalizedValue::Text(Arc::clone(&payload));
    for _ in 0..20_000 {
        raw = NormalizedValue::Option(Some(Box::new(raw)));
    }
    let rejected = thread::Builder::new()
        .stack_size(64 * 1024)
        .spawn(move || {
            SourceLoans::new(
                HeldRead {
                    origin: ValueOrigin::default(),
                    value: raw,
                },
                0,
                &ExecutionControl::uncancelled(),
                &mut |_| Ok(()),
            )
            .is_err()
        })?
        .join()
        .map_err(|_| "rejected raw loan worker panicked")?;
    assert!(rejected);
    assert_eq!(Arc::strong_count(&payload), 1);
    Ok(())
}
