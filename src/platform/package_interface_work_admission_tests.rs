use super::*;
use crate::platform::package_transport::source::SourceAllowance;

#[test]
fn associated_interface_work_uses_only_the_callers_exact_optional_allowance() {
    const COUNT: usize = 10_000_001;
    for maximum in [
        None,
        Some(0),
        Some(COUNT as u64 - 1),
        Some(COUNT as u64),
        Some(u64::MAX),
    ] {
        let mut allowance = SourceAllowance::new(maximum);
        let mut calls = Vec::new();
        let result = {
            let mut visit = |count| {
                calls.push(count);
                if allowance.reserve(count) {
                    Ok(())
                } else {
                    Err(interface_error(
                        DiagnosticClass::Resource,
                        "test_selected_interface_work",
                        "selected work allowance exhausted",
                    ))
                }
            };
            let mut admission = AssociatedInterfaceAdmission::new(&mut visit);
            admission
                .checkpoint()
                .and_then(|()| admission.charge(COUNT))
        };
        let admitted = maximum.is_none_or(|limit| limit >= COUNT as u64);
        assert_eq!(result.is_ok(), admitted);
        assert_eq!(calls, vec![0, COUNT as u64]);
        assert_eq!(
            allowance.remaining(),
            maximum.map(|limit| if admitted {
                limit - COUNT as u64
            } else {
                limit
            })
        );
        if let Err(error) = result {
            assert_eq!(error.code, "test_selected_interface_work");
        }
    }
}

#[test]
fn associated_interface_cancellation_callback_is_preserved_at_zero_work() {
    let mut observed = Vec::new();
    let mut visit = |count| {
        observed.push(count);
        Err(interface_error(
            DiagnosticClass::Cancelled,
            "test_interface_cancelled",
            "cancelled before interface work",
        ))
    };
    let result = AssociatedInterfaceAdmission::new(&mut visit).checkpoint();
    assert!(matches!(
        result,
        Err(Diagnostic {
            class: DiagnosticClass::Cancelled,
            ..
        })
    ));
    assert_eq!(observed, vec![0]);
}
