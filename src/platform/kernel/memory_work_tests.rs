//! Independent state-size accounting: unused live slots must still cost proof work.
use super::*;
use std::cell::Cell;

struct Counted<'a> {
    source: &'a KernelSnapshot,
    used: Cell<usize>,
    maximum: usize,
}
impl ExpressionRead for Counted<'_> {
    fn package_id(&self) -> PackageId {
        self.source.package_id()
    }
    fn owner(&self, key: OwnerKey) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.source.owner(key)
    }
    fn type_object(&self, ty: TypeObjectDigest) -> Result<Option<TypeObject>, Diagnostic> {
        self.source.type_object(ty)
    }
    fn package_interface_owner(
        &self,
        package: PackageId,
        key: OwnerKey,
    ) -> Result<Option<PackageInterfaceRecord>, Diagnostic> {
        self.source.package_interface_owner(package, key)
    }
    fn has_dependency(&self, package: PackageId) -> Result<bool, Diagnostic> {
        self.source.has_dependency(package)
    }
    fn validation_work(&self) -> Result<(), Diagnostic> {
        if self.used.get() == self.maximum {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "test_proof_work",
                "finite proof work exhausted",
            ));
        }
        self.used.set(self.used.get() + 1);
        Ok(())
    }
}

#[test]
fn owned_choice_branch_state_work_includes_unused_slots_and_exhaustion() {
    let source = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(r#"declarations.begin
(units (module create branch-work
  (function create inspect (visibility private) (effect pure)
    (parameter create p (type (owned-choice (case accepted Unit) (case rejected ByteBuffer))) (use consume))
    (returns Unit)
    (body (match-owned (type (owned-choice (case accepted Unit) (case rejected ByteBuffer))) (local p)
      (case accepted (binding value (type Unit)) (in (unit)))
      (case rejected (binding value (type ByteBuffer)) (in (unit))))))))
declarations.end"#).unwrap();
    let (expression, input) = source
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Expression(e) => match e.operation {
                ExpressionOperation::MatchOwned { source, .. } => Some((e.id, source)),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    let OwnerRecord::Expression(input) = &source.owners[&OwnerKey::Expression(input)] else {
        panic!("match source");
    };
    let ExpressionOperation::Local { value: input } = input.operation else {
        panic!("direct source");
    };
    let run = |extra: usize, maximum| {
        let read = Counted {
            source: &source,
            used: Cell::new(0),
            maximum,
        };
        let mut state = State::from([(
            input,
            Slot {
                borrowed: false,
                live: true,
            },
        )]);
        // Model otherwise-unused live analysis slots; they are deliberately not
        // referenced by the expression and cannot add owner-read/inference cost.
        for i in 0..extra {
            let key = LocalValueReference::FunctionParameter(
                format!("param_{:032x}", i + 1).parse().unwrap(),
            );
            assert!(
                state
                    .insert(
                        key,
                        Slot {
                            borrowed: false,
                            live: true
                        }
                    )
                    .is_none()
            );
        }
        let result = Check {
            read: &read,
            scope: None,
        }
        .eval(expression, &mut state, ParameterUse::Unrestricted, 0);
        (result, read.used.get())
    };
    let (small, small_work) = run(4, usize::MAX);
    let (large, large_work) = run(260, usize::MAX);
    assert!(!small.unwrap());
    assert!(!large.unwrap());
    // Two complete state forks and one complete join precede completion.
    assert!(
        large_work - small_work >= 3 * 256,
        "branch state size was not charged: small={small_work}, large={large_work}"
    );
    let (result, used) = run(260, large_work - 1);
    assert_eq!(used, large_work - 1);
    let error = result.unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(error.code, "test_proof_work");
}
