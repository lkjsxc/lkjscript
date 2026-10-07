use super::*;
use std::cell::Cell;

fn call(from: usize, to: usize) -> CallSite {
    CallSite {
        from,
        to,
        expression: None,
    }
}

fn empty() -> Inputs {
    Inputs::Application {
        arguments: Vec::new(),
        operands: Vec::new(),
    }
}

#[test]
fn one_exact_incoming_call_loads_once_for_many_provenance_requests() -> Result<(), Diagnostic> {
    let loads = Cell::new(0);
    let mut incoming = Incoming::new(call(7, 11));
    let first = incoming.resolve(|site| {
        assert_eq!((site.from, site.to), (7, 11));
        loads.set(loads.get() + 1);
        Ok(empty())
    })?;
    for _ in 0..1024 {
        let next = incoming.resolve(|_| {
            loads.set(loads.get() + 1);
            Ok(empty())
        })?;
        assert!(Rc::ptr_eq(&first, &next));
    }
    assert_eq!(loads.get(), 1);
    Ok(())
}

#[test]
fn equal_inputs_never_merge_distinct_call_contexts_or_new_proof_operations()
-> Result<(), Diagnostic> {
    let loads = Cell::new(0);
    let mut first = Incoming::new(call(1, 2));
    let mut second = Incoming::new(call(3, 2));
    let mut renewed = Incoming::new(call(1, 2));
    let mut load = |_: CallSite| {
        loads.set(loads.get() + 1);
        Ok(empty())
    };
    let a = first.resolve(&mut load)?;
    let b = second.resolve(&mut load)?;
    let c = renewed.resolve(&mut load)?;
    assert!(!Rc::ptr_eq(&a, &b));
    assert!(!Rc::ptr_eq(&a, &c));
    assert_eq!(loads.get(), 3);
    Ok(())
}

#[test]
fn failed_input_load_never_becomes_reusable_partial_admission() -> Result<(), Diagnostic> {
    let mut incoming = Incoming::new(call(1, 2));
    for class in [DiagnosticClass::Resource, DiagnosticClass::Semantic] {
        let refused = incoming.resolve(|_| {
            Err(Diagnostic::new(
                class,
                "input_load_refused",
                "controlled input refusal",
            ))
        });
        assert!(refused.is_err());
        assert!(incoming.inputs.is_none());
    }
    let accepted = incoming.resolve(|_| Ok(empty()))?;
    let reused = incoming.resolve(|_| Err(changed_target()))?;
    assert!(Rc::ptr_eq(&accepted, &reused));
    Ok(())
}

#[test]
fn input_storage_ends_with_its_last_operation_local_handle() -> Result<(), Diagnostic> {
    let mut incoming = Incoming::new(call(1, 2));
    let held = incoming.resolve(|_| Ok(empty()))?;
    let weak = Rc::downgrade(&held);
    drop(incoming);
    assert!(weak.upgrade().is_some());
    drop(held);
    assert!(weak.upgrade().is_none());
    Ok(())
}
