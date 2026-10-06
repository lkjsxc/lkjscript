//! Exact allocation/work admission of the whole-node interner, before publication.
use super::*;
use crate::platform::execution::{
    ExecutionControl, normalized::tests::owned_implementation_scheme_tests::duplicate_dag_fixture,
};
use crate::platform::kernel::{DeclarationPayload, KernelSnapshot, OwnerRecord};

struct Probe {
    result: Result<(), Diagnostic>,
    steps: usize,
    bytes: usize,
    applied: usize,
    normalized: usize,
}

fn named(source: &KernelSnapshot, name: &str) -> DeclarationReference {
    source
        .owners
        .iter()
        .find_map(|(key, record)| match (key, record) {
            (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record))
                if record.name.as_str() == name =>
            {
                Some(DeclarationReference {
                    package: source.root.package_id,
                    declaration: *declaration,
                })
            }
            _ => None,
        })
        .unwrap()
}

fn probe(
    source: &KernelSnapshot,
    program: &NormalizedProgram,
    bytes: usize,
    steps: usize,
    control: &ExecutionControl,
) -> Probe {
    let mut types = program.types.clone();
    let mut work = Budget {
        steps,
        bytes,
        control,
    };
    let implementations = source
        .owners
        .iter()
        .filter_map(|(key, record)| {
            let (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record)) =
                (key, record)
            else {
                return None;
            };
            let DeclarationPayload::OwnedImplementation(scheme) = &record.payload else {
                return None;
            };
            Some((
                DeclarationReference {
                    package: source.root.package_id,
                    declaration: *declaration,
                },
                scheme,
            ))
        })
        .collect();
    let mut closing = Closing {
        templates: Arc::from([]),
        functions: Vec::new(),
        implementations,
        targets: BTreeMap::new(),
        instances: BTreeMap::new(),
        normalized: BTreeMap::new(),
        witnesses: BTreeMap::new(),
        pending: Vec::new(),
        types: &mut types,
        work: &mut work,
    };
    let implementation = named(source, "Plain");
    let cell = program
        .implementation_applications
        .iter()
        .find(|node| node.implementation == named(source, "Both"))
        .unwrap()
        .self_type;
    let result = closing
        .operand(
            &ImplementationOperand::Concrete {
                implementation,
                type_arguments: vec![cell],
                implementations: Vec::new(),
            },
            None,
            &Bindings::new(),
            &BTreeMap::new(),
        )
        .map(|_| ());
    Probe {
        result,
        steps: closing.work.steps,
        bytes: closing.work.bytes,
        applied: closing.witnesses.len(),
        normalized: closing.normalized.len(),
    }
}

#[test]
fn complete_witness_nodes_reserve_exactly_and_refuse_one_byte_short_before_publication() {
    let (source, program) = duplicate_dag_fixture(1);
    let control = ExecutionControl::uncancelled();
    let base = probe(&source, &program, 0, 0, &control);
    assert!(base.result.is_ok());
    assert_eq!((base.applied, base.normalized), (1, 1));
    assert!(base.bytes > 0);
    let maximum = super::MAXIMUM_METADATA_BYTES;
    let exact = probe(&source, &program, maximum - base.bytes, 0, &control);
    assert!(exact.result.is_ok());
    assert_eq!(exact.bytes, maximum);
    let short = probe(&source, &program, maximum - base.bytes + 1, 0, &control);
    let error = short.result.unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(error.code, "normalized_instantiation_storage");
    assert_eq!((short.applied, short.normalized), (0, 0));
}

#[test]
fn complete_witness_nodes_keep_work_and_cancellation_separate_from_semantic_refusal() {
    let (source, program) = duplicate_dag_fixture(1);
    let control = ExecutionControl::uncancelled();
    let base = probe(&source, &program, 0, 0, &control);
    assert!(base.result.is_ok());
    assert!(base.steps > 2);
    let exact = probe(&source, &program, 0, MAXIMUM_WORK - base.steps, &control);
    assert!(exact.result.is_ok());
    assert_eq!(exact.steps, MAXIMUM_WORK);
    let short = probe(
        &source,
        &program,
        0,
        MAXIMUM_WORK - base.steps + 1,
        &control,
    );
    let error = short.result.unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(error.code, "normalized_instantiation_work");
    assert_eq!((short.applied, short.normalized), (0, 0));
    for after in [0, 1, base.steps as u64 - 1] {
        let stopped = probe(
            &source,
            &program,
            0,
            0,
            &ExecutionControl::cancel_after_checks(after),
        );
        assert_eq!(
            stopped.result.unwrap_err().class,
            DiagnosticClass::Cancelled
        );
        assert_eq!((stopped.applied, stopped.normalized), (0, 0));
    }
}
