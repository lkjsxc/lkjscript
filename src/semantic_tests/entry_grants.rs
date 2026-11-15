//! Entry capabilities are validated before acquisition in both execution APIs.
use super::interpreter_contracts::FailingHost;
use crate::{check, frontend, interpreter, ir, ir_text, native, runtime, source};
use interpreter::{ErrorKind, InterpreterError};

fn prepared() -> runtime::Runtime {
    let input = source::SourceInput {
        path: "runtime.rs".to_string(),
        text: include_str!("../../library/runtime.rs").to_string(),
    };
    let bundle = source::SourceBundle {
        root: "runtime.rs".to_string(),
        library: None,
        files: vec![input],
    };
    runtime::seal(frontend::check_bundle(&bundle).unwrap()).unwrap()
}
fn entry() -> ir::CheckedProgram {
    check::check(
        ir_text::read(include_str!(
            "../../tests/fixtures/authority/grant_pair.lkjir"
        ))
        .unwrap(),
    )
    .unwrap()
}
fn directory() -> ir::EntryGrant {
    ir::EntryGrant::Directory("docs\0".as_bytes().to_vec())
}
fn listener() -> ir::EntryGrant {
    ir::EntryGrant::Listener(vec![2u8, 0, 0, 0, 127, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0])
}
fn mismatched_grants_do_not_acquire_authority() {
    let checked = entry();
    let runtime = prepared();
    let cases: Vec<Vec<ir::EntryGrant>> = vec![
        Vec::new(),
        vec![directory()],
        vec![directory(), listener(), ir::EntryGrant::Clock],
        // The first Directory would acquire successfully if validation were incremental.
        vec![directory(), ir::EntryGrant::Stdout],
        vec![ir::EntryGrant::Clock, listener()],
    ];
    for grants in cases {
        let mut host = FailingHost(Vec::new());
        let error = interpreter::run_granted(checked.program(), 0, &grants, &mut host)
            .err()
            .expect("interpreter accepted mismatched entry grants");
        assert!(error.kind == ErrorKind::Interpreter(InterpreterError::InvalidIr));
        assert!(
            host.0.is_empty(),
            "entry mismatch acquired or closed host authority"
        );
        let error = native::encode_granted(&checked, &runtime, 0, &grants, None, None)
            .err()
            .expect("native emission accepted mismatched entry grants");
        assert_eq!(error.code, "E_ENTRY");
    }
}
fn acquisition_failure_closes_the_preceding_directory() {
    let checked = entry();
    let mut host = FailingHost(Vec::new());
    let error = interpreter::run_granted(
        checked.program(),
        0,
        &vec![directory(), listener()],
        &mut host,
    )
    .err()
    .expect("injected Listener acquisition failure was ignored");
    assert!(error.kind == ErrorKind::Trap(7));
    // D acquires handle 1, L refuses, C checks and closes that exact handle once.
    assert_eq!(host.0.as_slice(), b"DLC");
}
fn listener_cannot_be_forged_from_raw_scalar_arguments() {
    let checked = entry();
    let runtime = prepared();
    for argument in vec![0u64, 41u64] {
        let mut host = FailingHost(Vec::new());
        let error = interpreter::run(checked.program(), 1, &[argument], &mut host)
            .err()
            .expect("interpreter accepted raw Listener bits");
        assert!(error.kind == ErrorKind::Interpreter(InterpreterError::InvalidIr));
        assert!(host.0.is_empty());
        let selected = native::Entry {
            function: 1,
            arguments: vec![argument as i64],
            result_fd: None,
        };
        let error = native::encode(&checked, &runtime, selected)
            .err()
            .expect("native emission accepted raw Listener bits");
        assert_eq!(error.code, "E_ENTRY");
    }
}
fn non_scalar_result_rejects_before_acquisition() {
    let checked = entry();
    let runtime = prepared();
    let grants = vec![directory()];
    let mut host = FailingHost(Vec::new());
    let error = interpreter::run_granted(checked.program(), 2, &grants, &mut host)
        .err()
        .expect("interpreter accepted a capability entry result");
    assert!(error.kind == ErrorKind::Interpreter(InterpreterError::InvalidIr));
    assert!(host.0.is_empty(), "invalid entry result acquired authority");
    let error = native::encode_granted(&checked, &runtime, 2, &grants, None, None)
        .err()
        .expect("native emission accepted a capability entry result");
    assert_eq!(error.code, "E_ENTRY");
}
fn connector_grants_are_exact_and_unforgeable() {
    let checked = check::check(ir_text::read(include_str!(
        "../../tests/fixtures/authority/connector_entry.lkjir"
    )).unwrap()).unwrap();
    let runtime = prepared();
    let mut host = FailingHost(Vec::new());
    let endpoint = vec![2u8, 0, 1, 187, 127, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0];
    let grants = crate::entry_grants::prepare(checked.program(), 0, &[
        "target=127.000.0.1:443".to_string(),
    ]).unwrap();
    assert!(matches!(&grants[0], ir::EntryGrant::Connector(address) if *address == endpoint));
    interpreter::run_granted(checked.program(), 0, &grants, &mut host).unwrap();
    assert!(host.0.is_empty(), "granting an endpoint performs no socket acquisition");
    native::encode_granted(&checked, &runtime, 0, &grants, None, None).unwrap();
    for value in vec![0u64, 1u64, 65535u64] {
        let error = interpreter::run(checked.program(), 0, &[value], &mut host)
            .err().expect("raw bits forged Connector");
        assert!(error.kind == ErrorKind::Interpreter(InterpreterError::InvalidIr));
        let selected = native::Entry { function: 0, arguments: vec![value as i64], result_fd: None };
        assert_eq!(native::encode(&checked, &runtime, selected).err().unwrap().code, "E_ENTRY");
    }
    for text in vec![
        "target".to_string(), "target=localhost:443".to_string(),
        "target=127.0.0.1:65536".to_string(), "target=127.0.0.256:443".to_string(),
    ] {
        assert!(crate::entry_grants::prepare(checked.program(), 0, &[text]).is_err());
    }
    let mismatched = vec![ir::EntryGrant::Listener(endpoint)];
    assert!(interpreter::run_granted(checked.program(), 0, &mismatched, &mut host).is_err());
    assert!(native::encode_granted(&checked, &runtime, 0, &mismatched, None, None).is_err());
    assert!(host.0.is_empty());
}
pub fn probe() -> i64 {
    mismatched_grants_do_not_acquire_authority();
    acquisition_failure_closes_the_preceding_directory();
    listener_cannot_be_forged_from_raw_scalar_arguments();
    non_scalar_result_rejects_before_acquisition();
    connector_grants_are_exact_and_unforgeable();
    5
}
pub fn run(world: &mut World, trace: bool) -> usize {
    if trace {
        world
            .stdout()
            .write(b"entry_grants_validate_acquire_rollback_and_reject_raw_authority\n");
    }
    probe() as usize
}
