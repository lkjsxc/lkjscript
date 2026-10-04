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
        let mut state = State::from([(input, Slot::owner(false))]);
        // Model otherwise-unused live analysis slots; they are deliberately not
        // referenced by the expression and cannot add owner-read/inference cost.
        for i in 0..extra {
            let key = LocalValueReference::FunctionParameter(
                format!("param_{:032x}", i + 1).parse().unwrap(),
            );
            assert!(state.insert(key, Slot::owner(false)).is_none());
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

fn retained_ancestry(input: LocalValueReference, ancestors: usize) -> State {
    let mut state = State::from([(input, Slot::owner(true))]);
    let mut child = input;
    for ordinal in 0..ancestors {
        let parent =
            LocalValueReference::LexicalBinding(crate::platform::semantic_id::BindingId::migrate(
                b"borrow-proof-retained-ancestor",
                ordinal as u64,
            ));
        *state.get_mut(&child).unwrap() = Slot::view(parent);
        assert!(state.insert(parent, Slot::owner(false)).is_none());
        child = parent;
    }
    state
}

fn expression_input(source: &KernelSnapshot, expression: ExpressionId) -> LocalValueReference {
    let OwnerRecord::Expression(record) = &source.owners[&OwnerKey::Expression(expression)] else {
        panic!("scope source expression");
    };
    let ExpressionOperation::Local { value } = record.operation else {
        panic!("exact local scope source");
    };
    value
}

#[test]
fn nested_field_borrow_proof_work_charges_ancestry_and_exhausts_as_resource() {
    let source = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create loan-work
  (function create inspect (visibility private) (effect pure)
    (parameter create packet
      (type (owned-product (field inner (owned-product (field payload ByteBuffer))))) (use borrow))
    (returns Unit)
    (body (borrow-owned-field
      (type (owned-product (field inner (owned-product (field payload ByteBuffer))))) (local packet)
      (field inner (binding inner (type (owned-product (field payload ByteBuffer)))))
      (in (borrow-owned-field (type (owned-product (field payload ByteBuffer))) (local inner)
        (field payload (binding view (type ByteBuffer))) (in (unit)))))))))
declarations.end"#,
    )
    .unwrap();
    let expression = source
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Declaration(d) => match &d.payload {
                DeclarationPayload::Function(f) => Some(f.body),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    let OwnerRecord::Expression(record) = &source.owners[&OwnerKey::Expression(expression)] else {
        panic!("field borrow");
    };
    let ExpressionOperation::BorrowOwnedField { source: input, .. } = record.operation else {
        panic!("outer field borrow");
    };
    let input = expression_input(&source, input);
    let run = |ancestors, maximum| {
        let read = Counted {
            source: &source,
            used: Cell::new(0),
            maximum,
        };
        // Keep authored syntax and all metadata identical. Only the already
        // retained lexical custody chain changes, so added work must come from
        // loan acquisition/release rather than expression or type traversal.
        let mut state = retained_ancestry(input, ancestors);
        let result = Check {
            read: &read,
            scope: None,
        }
        .eval(expression, &mut state, ParameterUse::Unrestricted, 0);
        if result.is_ok() {
            assert!(state.values().all(|slot| slot.live && slot.loans == 0));
            assert_eq!(state.len(), ancestors + 1);
        }
        (result, read.used.get())
    };
    let (shallow, shallow_work) = run(0, usize::MAX);
    let (nested, nested_work) = run(128, usize::MAX);
    assert!(!shallow.unwrap());
    assert!(!nested.unwrap());
    // Both nested scopes acquire and release every retained ancestor.
    assert_eq!(nested_work - shallow_work, 4 * 128);
    for maximum in [0, nested_work - 1] {
        let (result, used) = run(128, maximum);
        assert_eq!(used, maximum);
        let error = result.unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Resource);
        assert_eq!(error.code, "test_proof_work");
    }
    let (result, used) = run(128, nested_work);
    assert!(!result.unwrap());
    assert_eq!(used, nested_work);
}

#[test]
fn borrowed_choice_proof_work_counts_loans_and_complete_branch_inventories() {
    let source = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create borrowed-branch-work
  (function create inspect (visibility private) (effect pure)
    (parameter create p (type (owned-choice (case accepted Unit) (case rejected ByteBuffer))) (use borrow))
    (returns Unit)
    (body (match-borrowed-owned (type (owned-choice (case accepted Unit) (case rejected ByteBuffer))) (local p)
      (case accepted (binding value (type Unit)) (in (unit)))
      (case rejected (binding value (type ByteBuffer)) (in (unit))))))))
declarations.end"#,
    )
    .unwrap();
    let (expression, input) = source
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Expression(e) => match e.operation {
                ExpressionOperation::MatchBorrowedOwned { source, .. } => Some((e.id, source)),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    let input = expression_input(&source, input);
    let run = |ancestors, maximum| {
        let read = Counted {
            source: &source,
            used: Cell::new(0),
            maximum,
        };
        let mut state = retained_ancestry(input, ancestors);
        let result = Check {
            read: &read,
            scope: None,
        }
        .eval(expression, &mut state, ParameterUse::Unrestricted, 0);
        if result.is_ok() {
            assert!(state.values().all(|slot| slot.live && slot.loans == 0));
        }
        (result, read.used.get())
    };
    let (shallow, shallow_work) = run(0, usize::MAX);
    let (large, large_work) = run(128, usize::MAX);
    assert!(!shallow.unwrap());
    assert!(!large.unwrap());
    // Every slot participates in source acquisition/release, two branch forks
    // and the branch join; an ordinary selected payload does not skip custody.
    assert_eq!(large_work - shallow_work, 5 * 128);
    let (result, used) = run(128, large_work - 1);
    assert_eq!(used, large_work - 1);
    let error = result.unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(error.code, "test_proof_work");
    let (result, used) = run(128, large_work);
    assert!(!result.unwrap());
    assert_eq!(used, large_work);
}
