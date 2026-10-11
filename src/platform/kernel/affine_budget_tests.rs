//! Observed read/traversal prefixes and independently accounted boundary costs.
use super::*;
use crate::platform::kernel::{KernelSnapshot, TypeObject, encode_type_object};
use std::cell::{Cell, RefCell};

#[derive(Clone, Debug, Eq, PartialEq)]
enum ObservedPhase {
    Owner(OwnerKey),
    Type(TypeObjectDigest),
    Interface(PackageId, OwnerKey),
    Dependency(PackageId),
    Representation(PackageId, OwnerKey),
    RepresentationDependency(PackageId),
    Traversal,
}

struct CountingRead {
    snapshot: KernelSnapshot,
    reads: Cell<usize>,
    phases: RefCell<Vec<ObservedPhase>>,
}

impl CountingRead {
    fn new(snapshot: KernelSnapshot) -> Self {
        Self {
            snapshot,
            reads: Cell::new(0),
            phases: RefCell::new(Vec::new()),
        }
    }

    fn observed(&self, phase: ObservedPhase) {
        self.reads.set(self.reads.get() + 1);
        self.phases.borrow_mut().push(phase);
    }

    fn trace(&self) -> Vec<ObservedPhase> {
        self.phases.borrow().clone()
    }
}

impl ExpressionRead for CountingRead {
    fn package_id(&self) -> PackageId {
        self.snapshot.root.package_id
    }

    fn owner(&self, owner: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.observed(ObservedPhase::Owner(owner));
        self.snapshot.owner(owner)
    }

    fn type_object(&self, digest: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        self.observed(ObservedPhase::Type(digest));
        self.snapshot.type_object(digest)
    }

    fn package_interface_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.observed(ObservedPhase::Interface(package, owner));
        self.snapshot.package_interface_owner(package, owner)
    }

    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.observed(ObservedPhase::Dependency(package));
        self.snapshot.has_dependency(package)
    }

    fn package_representation_owner(
        &self,
        package: PackageId,
        owner: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.observed(ObservedPhase::Representation(package, owner));
        self.snapshot.package_representation_owner(package, owner)
    }

    fn has_representation_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.observed(ObservedPhase::RepresentationDependency(package));
        self.snapshot.has_representation_dependency(package)
    }

    fn validation_work(&self) -> Result<(), Diagnostic> {
        self.phases.borrow_mut().push(ObservedPhase::Traversal);
        Ok(())
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

fn healthy_task() -> (CountingRead, OwnerKey, usize, Vec<(usize, ObservedPhase)>) {
    let (snapshot, root) = task_fixture();
    let read = CountingRead::new(snapshot);
    let mut used = 0;
    let (outcome, diagnostics) = run(&read, root, &mut used, usize::MAX, 0);
    assert_eq!(outcome, Ok(()));
    assert!(diagnostics.is_empty());
    let mut expressions = BTreeSet::new();
    let mut accounted = 0;
    let mut resource_visits = 0;
    let phases = read
        .trace()
        .into_iter()
        .map(|phase| {
            // The Unit body has two independent consumers. Memory reports its visit
            // through validation_work; affine's second visit admits one meter step
            // immediately before its expression metadata read.
            if let ObservedPhase::Owner(OwnerKey::Expression(id)) = &phase
                && !expressions.insert(*id)
            {
                accounted += 1;
                resource_visits += 1;
            }
            accounted += 1;
            (accounted, phase)
        })
        .collect::<Vec<_>>();
    assert_eq!(resource_visits, 1);
    assert_eq!(
        used, accounted,
        "every delegated read and traversal requires prior work admission"
    );
    (read, root, used, phases)
}

fn prefix(phases: &[(usize, ObservedPhase)], admitted: usize) -> Vec<ObservedPhase> {
    phases
        .iter()
        .take_while(|(position, _)| *position <= admitted)
        .map(|(_, phase)| phase.clone())
        .collect()
}

#[test]
fn affine_budget_stops_before_every_read_and_expression_phase() {
    let (healthy, root, required, phases) = healthy_task();
    for limit in 0..=required {
        let read = CountingRead::new(healthy.snapshot.clone());
        let mut work = 0;
        let (outcome, diagnostics) = run(&read, root, &mut work, limit, 0);
        assert_eq!(
            outcome,
            if limit == required {
                Ok(())
            } else {
                Err(ExpressionValidationExhaustion::Steps)
            },
            "limit {limit}"
        );
        assert_eq!(work, limit);
        assert_eq!(
            read.trace(),
            prefix(&phases, limit),
            "denied phase delegated side effects at limit {limit}"
        );
        assert!(diagnostics.is_empty(), "exhaustion is not invalid meaning");
    }
}

#[test]
fn affine_budget_preserves_shared_counter_and_integer_ceiling() {
    let (healthy, root, required, _) = healthy_task();
    let read = CountingRead::new(healthy.snapshot.clone());
    let initial = 7;
    let maximum = initial + required;
    let mut work = initial;
    assert_eq!(run(&read, root, &mut work, maximum, 0).0, Ok(()));
    assert_eq!(work, maximum);
    assert_eq!(read.trace(), healthy.trace());
    let short = CountingRead::new(healthy.snapshot.clone());
    let mut short_work = initial;
    assert_eq!(
        run(&short, root, &mut short_work, maximum - 1, 0).0,
        Err(ExpressionValidationExhaustion::Steps)
    );
    assert_eq!(short_work, maximum - 1);
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
    for depth in [5, 14] {
        let (mut snapshot, root, mut ty) = constant_fixture();
        let leaf = ty;
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
        let healthy = CountingRead::new(snapshot);
        let mut required = 0;
        let (outcome, diagnostics) = run(&healthy, root, &mut required, usize::MAX, 0);
        assert_eq!(outcome, Ok(()));
        assert!(diagnostics.is_empty());
        let trace = healthy.trace();
        // Constant validation has no affine task-body visit outside the reader.
        assert_eq!(
            required,
            trace.len(),
            "each repeated path admits its work before delegation"
        );
        assert!(
            trace
                .iter()
                .filter(|phase| **phase == ObservedPhase::Type(leaf))
                .count()
                > 1
        );
        for limit in [0, 1, 32, 64, required - 1, required] {
            let read = CountingRead::new(healthy.snapshot.clone());
            let mut work = 0;
            let (outcome, diagnostics) = run(&read, root, &mut work, limit, 0);
            assert_eq!(
                outcome,
                if limit == required {
                    Ok(())
                } else {
                    Err(ExpressionValidationExhaustion::Steps)
                }
            );
            assert_eq!(work, limit);
            assert_eq!(
                read.trace(),
                trace[..limit],
                "depth {depth}, denied phase delegated a read"
            );
            assert!(diagnostics.is_empty());
        }
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
    let mut required = 0;
    let (outcome, diagnostics) = run(&snapshot, root, &mut required, usize::MAX, 1);
    assert_eq!(outcome, Ok(()));
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, "kernel_affine_constant");
    assert_eq!(diagnostics[0].class, DiagnosticClass::Semantic);
    for (steps, sink, expected) in [
        (0, 0, Err(ExpressionValidationExhaustion::Steps)),
        (1, 0, Err(ExpressionValidationExhaustion::Steps)),
        (required - 1, 0, Err(ExpressionValidationExhaustion::Steps)),
        (
            required,
            0,
            Err(ExpressionValidationExhaustion::Diagnostics),
        ),
        (required, 1, Ok(())),
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
        let meter = work::Meter::new(&mut used, 7);
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
        assert!(
            read.package_representation_owner(foreign, root)
                .unwrap()
                .is_none()
        );
        assert!(!read.has_representation_dependency(foreign).unwrap());
        read.validation_work().unwrap();
        for diagnostic in [
            read.owner(root).unwrap_err(),
            read.type_object(unit).unwrap_err(),
            read.package_interface_owner(foreign, root).unwrap_err(),
            read.has_dependency(foreign).unwrap_err(),
            read.package_representation_owner(foreign, root)
                .unwrap_err(),
            read.has_representation_dependency(foreign).unwrap_err(),
            read.validation_work().unwrap_err(),
        ] {
            assert_eq!(diagnostic.code, "kernel_affine_work");
            assert_eq!(diagnostic.class, DiagnosticClass::Resource);
        }
        assert_eq!(inner.reads.get(), 6);
        assert_eq!(inner.trace().len(), 7);
    }
    assert_eq!(used, 7);
}

#[test]
fn affine_budget_preserves_cancellation_at_each_read_phase() {
    use crate::platform::kernel::infer::CheckedExpressionRead;
    let (healthy, root, required, phases) = healthy_task();
    let baseline = CountingRead::new(healthy.snapshot.clone());
    let before = RefCell::new(Vec::new());
    let observe = || {
        before.borrow_mut().push(baseline.trace());
        Ok(())
    };
    let checked = CheckedExpressionRead {
        read: &baseline,
        checkpoint: &observe,
    };
    let mut work = 0;
    let (outcome, diagnostics) = run(&checked, root, &mut work, required, 1);
    assert_eq!(outcome, Ok(()));
    assert!(diagnostics.is_empty());
    assert_eq!(work, required);
    let before = before.into_inner();
    assert_eq!(
        before.len(),
        phases.len(),
        "every admitted owning phase has a checkpoint"
    );
    for (index, expected_prefix) in before.into_iter().enumerate() {
        let stop = index + 1;
        let inner = CountingRead::new(healthy.snapshot.clone());
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
        let (outcome, diagnostics) = run(&read, root, &mut used, required, 1);
        assert_eq!(outcome, Ok(()));
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, "affine_fixture_cancelled");
        assert_eq!(diagnostics[0].class, DiagnosticClass::Resource);
        assert_eq!(checkpoints.get(), stop);
        assert_eq!(
            inner.trace(),
            expected_prefix,
            "cancelled phase delegated side effects at checkpoint {stop}"
        );
        assert_eq!(used, phases[index].0);
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
    let (healthy, root, required, phases) = healthy_task();
    for limit in [0, 1, required - 1] {
        let read = CountingRead::new(healthy.snapshot.clone());
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
                    maximum_steps: limit,
                    maximum_diagnostics: 0
                },
            ),
            Err(ExpressionValidationExhaustion::Steps)
        );
        assert_eq!(visited.get(), 1);
        assert_eq!(work, limit);
        assert_eq!(read.trace(), prefix(&phases, limit));
        assert!(diagnostics.is_empty());
    }
}
