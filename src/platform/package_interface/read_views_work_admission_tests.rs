use super::*;
use crate::platform::package_transport::source::SourceAllowance;

#[test]
fn read_view_interface_work_crosses_former_work_bound_and_preserves_depth_check()
-> Result<(), Diagnostic> {
    let owners = BTreeMap::new();
    let types = BTreeMap::new();
    let mut resolve = |_| Ok(None);
    let mut observed = 0_u64;
    let mut visit = |count| {
        observed += count;
        Ok(())
    };
    let mut admission = Admission {
        package: PackageId::migrate(b"read-view-work-admission", 0),
        owners: &owners,
        types: &types,
        resolve: &mut resolve,
        storage: RepresentationBudget::default(),
        deferred: false,
        visit: &mut visit,
    };
    admission.charge(10_000_001, 0)?;
    admission.charge(10_000_001, 0)?;
    let refused = admission.step(crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH + 1);
    assert!(matches!(
        refused,
        Err(Diagnostic {
            class: DiagnosticClass::Resource,
            ..
        })
    ));
    assert_eq!(observed, 20_000_003);
    Ok(())
}

#[test]
fn read_view_interface_work_retains_the_callers_finite_refusal() -> Result<(), Diagnostic> {
    let owners = BTreeMap::new();
    let types = BTreeMap::new();
    let mut resolve = |_| Ok(None);
    let mut allowance = SourceAllowance::new(Some(3));
    let mut visit = |count| {
        if allowance.reserve(count) {
            Ok(())
        } else {
            Err(interface_error(
                DiagnosticClass::Resource,
                "test_selected_view_work",
                "selected work allowance exhausted",
            ))
        }
    };
    let mut admission = Admission {
        package: PackageId::migrate(b"read-view-work-admission", 0),
        owners: &owners,
        types: &types,
        resolve: &mut resolve,
        storage: RepresentationBudget::default(),
        deferred: false,
        visit: &mut visit,
    };
    admission.charge(3, 0)?;
    match admission.step(0) {
        Err(error) => assert_eq!(error.code, "test_selected_view_work"),
        Ok(()) => return Err(reject("finite caller allowance was not enforced")),
    }
    assert_eq!(allowance.remaining(), Some(0));
    Ok(())
}
