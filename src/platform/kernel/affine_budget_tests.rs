//! Independent boundary counts, not an oracle built from the production meter.
use super::*;
use crate::platform::kernel::{KernelSnapshot, TypeObject, encode_type_object};
use std::cell::Cell;

struct CountingRead {
    snapshot: KernelSnapshot,
    reads: Cell<usize>,
}

impl CountingRead {
    fn new(snapshot: KernelSnapshot) -> Self {
        Self {
            snapshot,
            reads: Cell::new(0),
        }
    }

    fn observed(&self) {
        self.reads.set(self.reads.get() + 1);
    }
}

impl ExpressionRead for CountingRead {
    fn package_id(&self) -> PackageId {
        self.snapshot.root.package_id
    }

    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.observed();
        self.snapshot.owner(owner)
    }

    fn type_object(&self, digest: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        self.observed();
        self.snapshot.type_object(digest)
    }

    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.observed();
        self.snapshot.package_interface_owner(package, owner)
    }

    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.observed();
        self.snapshot.has_dependency(package)
    }
}

fn constant_fixture() -> (KernelSnapshot, OwnerKey, TypeObjectDigest) {
    let snapshot = super::super::tests::witness_snapshot();
    let (root, ty) = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Declaration(record) if record.name.as_str() == "unit_constant" => {
                let DeclarationPayload::Constant { ty, .. } = record.payload else {
                    panic!("unit constant");
                };
                Some((*owner, ty))
            }
            _ => None,
        })
        .expect("maintained unit constant");
    (snapshot, root, ty)
}

/// An isolated affine task: one declaration, an ordinary result and one Unit body.
/// Structural/type acceptance is exercised separately through the full/change owners.
fn task_fixture() -> (KernelSnapshot, OwnerKey) {
    let (mut snapshot, _, unit) = constant_fixture();
    let body = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Expression(id), OwnerRecord::Expression(record))
                if matches!(record.operation, ExpressionOperation::Unit {}) =>
            {
                Some(*id)
            }
            _ => None,
        })
        .expect("unit expression");
    let root = snapshot
        .owners
        .iter_mut()
        .find_map(|(owner, record)| {
            let OwnerRecord::Declaration(record) = record else {
                return None;
            };
            let DeclarationPayload::Function(function) = &mut record.payload else {
                return None;
            };
            function.type_parameters.clear();
            function.effect_parameters.clear();
            function.requirement_parameters.clear();
            function.parameters.clear();
            function.result = unit;
            function.effect = FunctionEffect::Task {
                effect_parameters: Vec::new(),
                requirements: Vec::new(),
            };
            function.body = body;
            Some(*owner)
        })
        .expect("function declaration");
    snapshot
        .owners
        .retain(|owner, _| *owner == root || *owner == OwnerKey::Expression(body));
    (snapshot, root)
}

fn run(
    read: &impl ExpressionRead,
    root: OwnerKey,
    work: &mut usize,
    maximum_steps: usize,
    maximum_diagnostics: usize,
) -> (Result<(), ExpressionValidationExhaustion>, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let outcome = validate_affine_roots_with_limits(
        read,
        [root],
        &mut diagnostics,
        work,
        ExpressionValidationLimits {
            maximum_steps,
            maximum_diagnostics,
        },
    );
    (outcome, diagnostics)
}

#[test]
fn affine_budget_stops_before_every_read_and_expression_phase() {
    // Independently enumerated work: root, result type, shape signature,
    // initial-state signature, expression visit, expression record.
    for (limit, expected_reads) in [0, 1, 2, 3, 4, 4].into_iter().enumerate() {
        let (snapshot, root) = task_fixture();
        let read = CountingRead::new(snapshot);
        let mut work = 0;
        let (outcome, diagnostics) = run(&read, root, &mut work, limit, 0);
        assert_eq!(
            outcome,
            Err(ExpressionValidationExhaustion::Steps),
            "limit {limit}"
        );
        assert_eq!(work, limit);
        assert_eq!(read.reads.get(), expected_reads);
        assert!(diagnostics.is_empty(), "exhaustion is not invalid meaning");
    }
    let (snapshot, root) = task_fixture();
    let read = CountingRead::new(snapshot);
    let mut work = 0;
    let (outcome, diagnostics) = run(&read, root, &mut work, 6, 0);
    assert_eq!(outcome, Ok(()));
    assert_eq!(work, 6);
    assert_eq!(read.reads.get(), 5);
    assert!(diagnostics.is_empty());
}

#[test]
fn affine_budget_preserves_shared_counter_and_integer_ceiling() {
    let (snapshot, root) = task_fixture();
    let read = CountingRead::new(snapshot);
    let mut work = 7;
    assert_eq!(run(&read, root, &mut work, 13, 0).0, Ok(()));
    assert_eq!(work, 13);
    let mut near_ceiling = usize::MAX - 1;
    let near_read = CountingRead::new(read.snapshot.clone());
    assert_eq!(
        run(&near_read, root, &mut near_ceiling, usize::MAX, 0).0,
        Err(ExpressionValidationExhaustion::Steps)
    );
    assert_eq!(near_ceiling, usize::MAX);
    assert_eq!(near_read.reads.get(), 1);
    for initial in [13, usize::MAX - 1, usize::MAX] {
        let read = CountingRead::new(read.snapshot.clone());
        let mut work = initial;
        assert_eq!(
            run(&read, root, &mut work, initial, 0).0,
            Err(ExpressionValidationExhaustion::Steps),
        );
        assert_eq!(work, initial);
        assert_eq!(read.reads.get(), 0);
    }
}

#[test]
fn affine_budget_meters_repeated_type_paths_without_changing_meaning() {
    for (depth, limit, succeeds) in [(5, 64, true), (14, 32, false)] {
        let (mut snapshot, root, mut ty) = constant_fixture();
        for _ in 0..depth {
            let object = TypeObject::new(TypeForm::Result { ok: ty, error: ty }).unwrap();
            ty = encode_type_object(&object).unwrap().0;
            snapshot.types.insert(ty, object);
        }
        let OwnerRecord::Declaration(record) = snapshot.owners.get_mut(&root).unwrap() else {
            panic!("constant");
        };
        let DeclarationPayload::Constant { ty: result, .. } = &mut record.payload else {
            panic!("constant payload");
        };
        *result = ty;
        let read = CountingRead::new(snapshot);
        let mut work = 0;
        let (outcome, diagnostics) = run(&read, root, &mut work, limit, 0);
        assert_eq!(outcome.is_ok(), succeeds);
        if !succeeds {
            assert_eq!(outcome, Err(ExpressionValidationExhaustion::Steps));
        }
        assert_eq!(work, limit);
        assert_eq!(
            read.reads.get(),
            limit,
            "stop before the first unadmitted read"
        );
        assert!(diagnostics.is_empty());
    }
}

#[test]
fn affine_budget_keeps_semantic_and_diagnostic_exhaustion_distinct() {
    let (mut snapshot, root, _) = constant_fixture();
    let interface = DeclarationReference {
        package: snapshot.root.package_id,
        declaration: DeclarationId::migrate(b"affine-work-interface", 0),
    };
    let object = TypeObject::new(TypeForm::CapabilityResource { interface }).unwrap();
    let ty = encode_type_object(&object).unwrap().0;
    snapshot.types.insert(ty, object);
    let OwnerRecord::Declaration(record) = snapshot.owners.get_mut(&root).unwrap() else {
        panic!("constant");
    };
    let DeclarationPayload::Constant { ty: result, .. } = &mut record.payload else {
        panic!("constant payload");
    };
    *result = ty;
    for (steps, sink, expected) in [
        (1, 0, Err(ExpressionValidationExhaustion::Steps)),
        (2, 0, Err(ExpressionValidationExhaustion::Diagnostics)),
        (2, 1, Ok(())),
    ] {
        let mut work = 0;
        let (outcome, diagnostics) = run(&snapshot, root, &mut work, steps, sink);
        assert_eq!(outcome, expected);
        assert_eq!(work, steps);
        if sink == 1 {
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].code, "kernel_affine_constant");
            assert_eq!(diagnostics[0].class, DiagnosticClass::Semantic);
        } else {
            assert!(diagnostics.is_empty());
        }
    }
}

#[test]
fn affine_budget_whole_snapshot_wrapper_never_silences_exhaustion() {
    let (snapshot, _) = task_fixture();
    let mut work = 0;
    let mut diagnostics = Vec::new();
    validate_affine_meaning(&snapshot, &snapshot, &mut diagnostics, &mut work, 0);
    assert_eq!(work, 0);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].class, DiagnosticClass::Resource);
    assert_eq!(diagnostics[0].code, "kernel_affine_work");
}

#[test]
fn affine_budget_reader_covers_all_metadata_methods_before_delegation() {
    let (snapshot, root, unit) = constant_fixture();
    let inner = CountingRead::new(snapshot);
    let foreign = PackageId::migrate(b"affine-budget-foreign", 0);
    let mut used = 0;
    {
        let meter = work::Meter::new(&mut used, 4);
        let read = work::Read {
            inner: &inner,
            meter: &meter,
        };
        assert_eq!(read.package_id(), inner.package_id());
        read.validation_checkpoint().unwrap();
        assert!(read.owner(root).unwrap().is_some());
        assert!(read.type_object(unit).unwrap().is_some());
        assert!(
            read.package_interface_owner(foreign, root)
                .unwrap()
                .is_none()
        );
        assert!(!read.has_dependency(foreign).unwrap());
        for diagnostic in [
            read.owner(root).unwrap_err(),
            read.type_object(unit).unwrap_err(),
            read.package_interface_owner(foreign, root).unwrap_err(),
            read.has_dependency(foreign).unwrap_err(),
        ] {
            assert_eq!(diagnostic.code, "kernel_affine_work");
            assert_eq!(diagnostic.class, DiagnosticClass::Resource);
        }
        assert_eq!(inner.reads.get(), 4);
    }
    assert_eq!(used, 4);
}

#[test]
fn affine_budget_preserves_cancellation_at_each_read_phase() {
    use crate::platform::kernel::infer::CheckedExpressionRead;
    for stop in 1..=5 {
        let (snapshot, root) = task_fixture();
        let inner = CountingRead::new(snapshot);
        let checkpoints = Cell::new(0);
        let cancellation = || {
            checkpoints.set(checkpoints.get() + 1);
            if checkpoints.get() == stop {
                Err(Diagnostic::new(
                    DiagnosticClass::Resource,
                    "affine_fixture_cancelled",
                    "test owning operation cancelled",
                ))
            } else {
                Ok(())
            }
        };
        let read = CheckedExpressionRead {
            read: &inner,
            checkpoint: &cancellation,
        };
        let mut used = 0;
        let (outcome, diagnostics) = run(&read, root, &mut used, 100, 1);
        assert_eq!(outcome, Ok(()));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "affine_fixture_cancelled");
        assert_eq!(diagnostics[0].class, DiagnosticClass::Resource);
        assert_eq!(checkpoints.get(), stop);
        assert_eq!(inner.reads.get(), stop - 1);
        assert_eq!(used, if stop == 5 { 6 } else { stop });
    }
    let (snapshot, _, _) = constant_fixture();
    let cancellation = || {
        Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "affine_fixture_cancelled",
            "test owning operation cancelled",
        ))
    };
    let inner = CheckedExpressionRead {
        read: &snapshot,
        checkpoint: &cancellation,
    };
    let mut used = 0;
    {
        let meter = work::Meter::new(&mut used, 0);
        let read = work::Read {
            inner: &inner,
            meter: &meter,
        };
        assert_eq!(
            read.validation_checkpoint().unwrap_err().code,
            "affine_fixture_cancelled"
        );
    }
    assert_eq!(
        used, 0,
        "an explicit checkpoint is not an extra read-work unit"
    );
}

#[test]
fn affine_budget_does_not_visit_another_root_after_exhaustion() {
    let (snapshot, root) = task_fixture();
    let read = CountingRead::new(snapshot);
    let visited = Cell::new(0);
    let roots = [root, root]
        .into_iter()
        .inspect(|_| visited.set(visited.get() + 1));
    let mut work = 0;
    let mut diagnostics = Vec::new();
    assert_eq!(
        validate_affine_roots_with_limits(
            &read,
            roots,
            &mut diagnostics,
            &mut work,
            ExpressionValidationLimits {
                maximum_steps: 2,
                maximum_diagnostics: 0
            },
        ),
        Err(ExpressionValidationExhaustion::Steps)
    );
    assert_eq!(visited.get(), 1);
    assert_eq!(work, 2);
    assert_eq!(read.reads.get(), 2);
    assert!(diagnostics.is_empty());
}
