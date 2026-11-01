//! Independent payload, exact reservation, cancellation and custody controls.
use super::*;
use crate::platform::admission::{Meter, Observation};
use crate::platform::execution::ExecutionFailureClass;
type Check = Result<(), String>;
fn checked<T, E: std::fmt::Debug>(result: Result<T, E>) -> Result<T, String> {
    result.map_err(|error| format!("{error:?}"))
}
#[test]
fn bulk_buffer_append_reserves_exact_optional_growth_before_copy() -> Check {
    let data = [11_u8; 13];
    for limit in [None, Some(0), Some(12), Some(13), Some(u64::MAX)] {
        let observed = StorageObservation::start();
        let domain = ValueOrigin::fresh().ok_or("value origin exhausted")?;
        let buffer = ByteBuffer::empty(domain);
        let mut meter = Meter::new(limit);
        let result = buffer.append_slice(
            &data,
            0,
            13,
            &ExecutionControl::uncancelled(),
            &mut |bytes| {
                assert_eq!(bytes, 13);
                meter.charge(Observation::exact(bytes)).map_err(|_| {
                    ExecutionError::resource("bulk_test_allowance", "selected allowance")
                })
            },
        );
        if limit.is_some_and(|n| n < 13) {
            let error = result.err().ok_or("insufficient allowance was accepted")?;
            assert_eq!(error.code, "bulk_test_allowance");
            assert_eq!(meter.remaining(), limit);
        } else {
            let buffer = checked(result)?;
            assert_eq!(meter.remaining(), limit.map(|n| n - 13));
            assert_eq!(checked(buffer.freeze())?.as_ref(), &data);
        }
        assert_eq!(observed.live(), (0, 0));
    }
    Ok(())
}
#[test]
fn bulk_buffer_append_empty_windows_and_word_bits_preserve_source() -> Check {
    let control = ExecutionControl::uncancelled();
    let observed = StorageObservation::start();
    for value in [i64::MIN, -1, 0, 1, 0x0102030405060708, i64::MAX] {
        let buffer = ByteBuffer::empty(ValueOrigin::fresh().ok_or("value origin exhausted")?);
        let buffer = checked(buffer.append_i64_le(value, &control, &mut |_| Ok(())))?;
        let original = buffer.allocation_identity();
        let buffer = checked(buffer.append_slice(b"abc", 3, 3, &control, &mut |_| {
            Err(ExecutionError::resource(
                "unexpected",
                "empty span reserved",
            ))
        }))?;
        assert_eq!(buffer.allocation_identity(), original);
        assert_eq!(checked(buffer.freeze())?.as_ref(), &value.to_le_bytes());
    }
    let source = vec![37; 131_073];
    let mut charges = Vec::new();
    let buffer = ByteBuffer::empty(ValueOrigin::fresh().ok_or("value origin exhausted")?);
    let buffer =
        checked(
            buffer.append_slice(&source, 0, source.len() as i64, &control, &mut |bytes| {
                charges.push(bytes);
                Ok(())
            }),
        )?;
    assert_eq!(charges, [131_073]);
    assert_eq!(checked(buffer.freeze())?.as_ref(), source.as_slice());
    assert!(source.iter().all(|b| *b == 37));
    assert_eq!(observed.live(), (0, 0));
    Ok(())
}
#[test]
fn bulk_buffer_append_rejects_bounds_and_live_loans_before_reservation() -> Check {
    let control = ExecutionControl::uncancelled();
    let observed = StorageObservation::start();
    for (start, end) in [
        (-1, 0),
        (1, 0),
        (0, 3),
        (i64::MIN, i64::MAX),
        (0, i64::MAX),
        (i64::MAX, i64::MAX),
    ] {
        let buffer = ByteBuffer::empty(ValueOrigin::fresh().ok_or("value origin exhausted")?);
        let error = buffer
            .append_slice(&[0, 255], start, end, &control, &mut |_| {
                Err(ExecutionError::resource(
                    "unexpected",
                    "invalid bounds reserved",
                ))
            })
            .err()
            .ok_or("invalid window accepted")?;
        assert_eq!(error.class, ExecutionFailureClass::Trap);
        assert_eq!(error.code, "normalized_bytes_range");
        assert_eq!(observed.live(), (0, 0));
    }
    let buffer = ByteBuffer::empty(ValueOrigin::fresh().ok_or("value origin exhausted")?);
    let loan = checked(buffer.borrow())?;
    let error = buffer
        .append_slice(b"abc", 0, 3, &control, &mut |_| {
            Err(ExecutionError::resource(
                "unexpected",
                "loaned builder reserved",
            ))
        })
        .err()
        .ok_or("loaned builder was mutated")?;
    assert_eq!(error.code, "normalized_buffer_token");
    drop(loan);
    assert_eq!(observed.live(), (0, 0));
    Ok(())
}
#[test]
fn bulk_buffer_append_cancellation_drops_every_partial_copy() -> Check {
    let source = vec![93; 131_073];
    let observed = StorageObservation::start();
    let mut cancelled = 0;
    let mut completed = 0;
    for checks in 0..16 {
        let buffer = ByteBuffer::empty(ValueOrigin::fresh().ok_or("value origin exhausted")?);
        let control = ExecutionControl::cancel_after_checks(checks);
        match buffer.append_slice(&source, 0, source.len() as i64, &control, &mut |_| Ok(())) {
            Ok(buffer) => {
                assert_eq!(checked(buffer.freeze())?.as_ref(), source.as_slice());
                completed += 1;
            }
            Err(error) => {
                assert_eq!(error.class, ExecutionFailureClass::Cancelled);
                cancelled += 1;
            }
        }
        assert_eq!(observed.live(), (0, 0));
    }
    assert!(cancelled > 0 && completed > 0);
    assert!(source.iter().all(|b| *b == 93));
    Ok(())
}
