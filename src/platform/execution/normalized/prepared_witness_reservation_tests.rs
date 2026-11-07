//! Exact allocation/work admission of the whole-node interner, before publication.
use super::*;
use crate::platform::execution::{
    ExecutionControl,
    normalized::{
        prepare::NormalizedPreparationLimits,
        tests::owned_implementation_scheme_tests::duplicate_dag_fixture,
    },
};
use crate::platform::kernel::{DeclarationPayload, KernelSnapshot, OwnerRecord};

struct Probe {
    result: Result<(), Diagnostic>,
    steps: u64,
    bytes: u64,
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
    maximum_bytes: Option<u64>,
    maximum_steps: Option<u64>,
    control: &ExecutionControl,
) -> Probe {
    let mut types = program.types.clone();
    let mut work = Budget::with_limits(
        control,
        NormalizedPreparationLimits {
            maximum_type_derivation_steps: maximum_steps,
            maximum_type_metadata_bytes: maximum_bytes,
        },
    );
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
    let contracts = source
        .owners
        .iter()
        .filter_map(|(key, record)| {
            let (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record)) =
                (key, record)
            else {
                return None;
            };
            let DeclarationPayload::OwnedContract(contract) = &record.payload else {
                return None;
            };
            Some((
                DeclarationReference {
                    package: source.root.package_id,
                    declaration: *declaration,
                },
                contract,
            ))
        })
        .collect();
    let runtime_owners = source
        .owners
        .iter()
        .map(|(key, record)| ((source.root.package_id, *key), record.clone()))
        .collect();
    let mut closing = Closing {
        templates: Arc::from([]),
        functions: Vec::new(),
        implementations,
        contracts,
        runtime_owners: &runtime_owners,
        producer_profiles: None,
        owned_records: program.owned_record_declarations.clone(),
        targets: BTreeMap::new(),
        instances: BTreeMap::new(),
        normalized: BTreeMap::new(),
        witnesses: BTreeMap::new(),
        pending: Vec::new(),
        pending_methods: Vec::new(),
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
    assert!(!closing.work.steps.observation().saturated);
    assert!(!closing.work.bytes.observation().saturated);
    Probe {
        result,
        steps: closing.work.steps.observation().consumed,
        bytes: closing.work.bytes.observation().consumed,
        applied: closing.witnesses.len(),
        normalized: closing.normalized.len(),
    }
}

#[test]
fn complete_witness_nodes_reserve_exactly_and_refuse_one_byte_short_before_publication() {
    let (source, program) = duplicate_dag_fixture(1);
    let control = ExecutionControl::uncancelled();
    let base = probe(&source, &program, None, None, &control);
    assert!(base.result.is_ok());
    assert_eq!((base.applied, base.normalized), (1, 1));
    assert!(base.bytes > 0);
    let exact = probe(&source, &program, Some(base.bytes), None, &control);
    assert!(exact.result.is_ok());
    assert_eq!(exact.bytes, base.bytes);
    let short = probe(&source, &program, Some(base.bytes - 1), None, &control);
    let error = short.result.unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(error.code, "normalized_instantiation_storage");
    assert_eq!((short.applied, short.normalized), (0, 0));
}

#[test]
fn complete_witness_nodes_keep_work_and_cancellation_separate_from_semantic_refusal() {
    let (source, program) = duplicate_dag_fixture(1);
    let control = ExecutionControl::uncancelled();
    let base = probe(&source, &program, None, None, &control);
    assert!(base.result.is_ok());
    assert!(base.steps > 2);
    let exact = probe(&source, &program, None, Some(base.steps), &control);
    assert!(exact.result.is_ok());
    assert_eq!(exact.steps, base.steps);
    let short = probe(&source, &program, None, Some(base.steps - 1), &control);
    let error = short.result.unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(error.code, "normalized_instantiation_work");
    assert_eq!((short.applied, short.normalized), (0, 0));
    for after in [0, 1, base.steps - 1] {
        let stopped = probe(
            &source,
            &program,
            None,
            None,
            &ExecutionControl::cancel_after_checks(after),
        );
        assert_eq!(
            stopped.result.unwrap_err().class,
            DiagnosticClass::Cancelled
        );
        assert_eq!((stopped.applied, stopped.normalized), (0, 0));
    }
}
