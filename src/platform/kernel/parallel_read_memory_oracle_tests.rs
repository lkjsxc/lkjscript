//! Canonical read-sharing proof independent of production admission and runtime leases.
use super::*;
use crate::platform::semantic_id::DeclarationId;

const SOURCE: &str = r#"declarations.begin
(units (module create read-oracle
  (external create extract (visibility private) (implementation core.cell.extract)
    (parameter create cell (type OwnedI64Cell) (use consume)) (returns I64))
  (function create read (visibility public) (effect (task))
    (type-parameter create T (constraint owned shareable))
    (type-parameter create Phantom (constraint owned))
    (parameter create number (type I64))
    (parameter create value (type T) (use borrow))
    (returns I64) (body (local number)))
  (function create consume (visibility public) (effect (task))
    (parameter create number (type I64))
    (parameter create value (type OwnedI64Cell) (use consume))
    (returns I64) (body (call extract (local value))))
  (function create joined (visibility public) (effect (task))
    (type-parameter create T (constraint owned shareable))
    (type-parameter create Phantom (constraint owned))
    (parameter create value (type T) (use consume))
    (returns T)
    (body (let
      (binding results (type (record (left I64) (right I64)))
        (parallel (call read (types T Phantom) (i64 17) (local value))
                  (call read (types T Phantom) (i64 29) (local value))))
      (in (local value)))))
  (function create hidden (visibility public) (effect (task))
    (parameter create owner (type OwnedI64Cell) (use consume))
    (parameter create spare (type OwnedI64Cell) (use consume))
    (returns (record (left I64) (right I64)))
    (body (parallel
      (call read (types OwnedI64Cell ByteBuffer) (i64 7) (local owner))
      (call read (types OwnedI64Cell ByteBuffer)
        (call extract (local spare)) (local owner)))))))
declarations.end
"#;

fn source() -> KernelSnapshot {
    crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE).unwrap()
}

fn declaration(snapshot: &KernelSnapshot, name: &str) -> DeclarationId {
    snapshot
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(d)) if d.name.as_str() == name => {
                Some(*id)
            }
            _ => None,
        })
        .unwrap()
}

fn function(snapshot: &KernelSnapshot, name: &str) -> FunctionDeclaration {
    let id = declaration(snapshot, name);
    let OwnerRecord::Declaration(d) = &snapshot.owners[&OwnerKey::Declaration(id)] else {
        unreachable!()
    };
    let DeclarationPayload::Function(f) = &d.payload else {
        unreachable!()
    };
    f.clone()
}

#[test]
fn parallel_read_oracle_admits_shareable_only_generic_and_unused_owned_actual() {
    assert!(
        accepts(&source()),
        "read sharing neither moves its owner nor strengthens unused formal bounds"
    );
}

#[test]
fn parallel_read_oracle_requires_shareability_separately_from_transferability() {
    let baseline = source();
    assert!(accepts(&baseline));
    for bounds in [
        TypeParameterConstraints::Owned,
        TypeParameterConstraints::OwnedTransferable,
    ] {
        let mut changed = baseline.clone();
        let joined = declaration(&changed, "joined");
        for owner in changed.owners.values_mut() {
            if let OwnerRecord::TypeParameter(p) = owner
                && p.declaration == joined
                && p.name.as_str() == "T"
            {
                p.constraints = bounds;
            }
        }
        assert!(
            !accepts(&changed),
            "unexecuted generic parallel capture requires independent share proof: {bounds:?}"
        );
    }
}

#[test]
fn parallel_read_oracle_checks_capture_permission_at_the_caller_boundary() {
    let mut changed = source();
    let read = declaration(&changed, "read");
    for owner in changed.owners.values_mut() {
        if let OwnerRecord::TypeParameter(p) = owner
            && p.declaration == read
            && p.name.as_str() == "T"
        {
            p.constraints = TypeParameterConstraints::Owned;
        }
    }
    assert!(
        accepts(&changed),
        "a synchronous task reader needs owned, while its caller independently proves shareability"
    );
}

#[test]
fn parallel_read_oracle_keeps_task_borrowed_results_unsupported() {
    let mut changed = source();
    let read = function(&changed, "read");
    let OwnerRecord::Parameter(source) = &changed.owners[&OwnerKey::Parameter(read.parameters[1])]
    else {
        unreachable!()
    };
    let ty = source.ty;
    let key = OwnerKey::Declaration(declaration(&changed, "read"));
    let OwnerRecord::Declaration(d) = changed.owners.get_mut(&key).unwrap() else {
        unreachable!()
    };
    let DeclarationPayload::Function(f) = &mut d.payload else {
        unreachable!()
    };
    f.result = ty;
    f.result_borrow = Some(read.parameters[1]);
    let OwnerRecord::Expression(e) = changed
        .owners
        .get_mut(&OwnerKey::Expression(read.body))
        .unwrap()
    else {
        unreachable!()
    };
    e.operation = ExpressionOperation::Local {
        value: LocalValueReference::FunctionParameter(read.parameters[1]),
    };
    assert!(
        !accepts(&changed),
        "valid read inputs do not authorize an escaping task result"
    );
}

#[test]
fn parallel_read_oracle_holds_left_loans_through_hidden_right_preparation() {
    let mut changed = source();
    assert!(accepts(&changed));
    let hidden = function(&changed, "hidden");
    let oracle = Oracle(&changed, None);
    let ExpressionOperation::Parallel { right, .. } = oracle.expression(hidden.body).unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::Call { arguments, .. } = oracle.expression(*right).unwrap() else {
        unreachable!()
    };
    let ExpressionOperation::Call { arguments, .. } = oracle.expression(arguments[0]).unwrap()
    else {
        unreachable!()
    };
    let local = arguments[0];
    let OwnerRecord::Expression(e) = changed
        .owners
        .get_mut(&OwnerKey::Expression(local))
        .unwrap()
    else {
        unreachable!()
    };
    e.operation = ExpressionOperation::Local {
        value: LocalValueReference::FunctionParameter(hidden.parameters[0]),
    };
    assert!(
        !accepts(&changed),
        "a later ordinary expression cannot consume captured storage before either child starts"
    );
}

#[test]
fn parallel_read_oracle_rejects_read_consume_aliases_in_both_child_orders() {
    let baseline = source();
    assert!(accepts(&baseline));
    let hidden = function(&baseline, "hidden");
    let ExpressionOperation::Parallel { left, right } =
        *Oracle(&baseline, None).expression(hidden.body).unwrap()
    else {
        unreachable!()
    };
    for child in [left, right] {
        let mut changed = baseline.clone();
        let consume = declaration(&changed, "consume");
        let OwnerRecord::Expression(e) = changed
            .owners
            .get_mut(&OwnerKey::Expression(child))
            .unwrap()
        else {
            unreachable!()
        };
        let ExpressionOperation::Call {
            function,
            type_arguments,
            ..
        } = &mut e.operation
        else {
            unreachable!()
        };
        function.declaration = consume;
        type_arguments.clear();
        assert!(
            !accepts(&changed),
            "alias conflict must not depend on which child reads first"
        );
    }
}

#[test]
fn parallel_read_oracle_rejects_generation_downgrades_at_each_new_boundary() {
    let baseline = source();
    assert!(accepts(&baseline));
    let hidden = function(&baseline, "hidden");
    let read = function(&baseline, "read");
    for key in [
        OwnerKey::Expression(hidden.body),
        OwnerKey::Declaration(declaration(&baseline, "read")),
        OwnerKey::Parameter(read.parameters[1]),
        OwnerKey::TypeParameter(read.type_parameters[0]),
    ] {
        let mut changed = baseline.clone();
        changed
            .owners
            .get_mut(&key)
            .unwrap()
            .set_encoding_for_edit(29);
        assert!(
            !accepts(&changed),
            "new read-sharing contract cannot appear under old owner version: {key:?}"
        );
    }
}

const SCHEME: &str = r#"declarations.begin
(units (module create read-schemes
  (owned-contract create Reader (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_ba000000000000000000000000000001 read
      (parameters (Self borrow)) (returns I64) (effect (task))))
  (function create generic-read (visibility public) (effect (task))
    (type-parameter create T (constraint owned shareable))
    (parameter create value (type T) (use borrow))
    (returns I64) (body (i64 11)))
  (owned-implementation create ReadScheme (visibility public)
    (type-parameter create T (constraint owned shareable))
    (contract Reader) (self T)
    (method method_ba000000000000000000000000000001 generic-read (types T)))
  (function create method-use (visibility public) (effect (task))
    (type-parameter create T (constraint owned shareable))
    (parameter create value (type T) (use borrow))
    (returns I64)
    (body (method-call (implementation ReadScheme (types T)) Reader
      method_ba000000000000000000000000000001 (local value))))))
declarations.end
"#;

#[test]
fn parallel_read_oracle_checks_stronger_scheme_and_mapped_target_bounds_exactly() {
    let baseline =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SCHEME)
            .unwrap();
    assert!(
        accepts(&baseline),
        "synchronous borrowed task methods compose with shareable schemes"
    );
    for name in ["ReadScheme", "method-use"] {
        for bounds in [
            TypeParameterConstraints::Owned,
            TypeParameterConstraints::OwnedTransferable,
        ] {
            let mut changed = baseline.clone();
            let id = declaration(&changed, name);
            for owner in changed.owners.values_mut() {
                if let OwnerRecord::TypeParameter(p) = owner
                    && p.declaration == id
                {
                    p.constraints = bounds;
                }
            }
            assert!(
                !accepts(&changed),
                "{name} loses the exact mapped/application share bound: {bounds:?}"
            );
        }
    }
    let mut changed = baseline;
    let key = OwnerKey::Declaration(declaration(&changed, "Reader"));
    changed
        .owners
        .get_mut(&key)
        .unwrap()
        .set_encoding_for_edit(29);
    assert!(
        !accepts(&changed),
        "a task method's borrowed mode belongs to the new contract generation"
    );
}

#[test]
fn parallel_read_oracle_checks_nested_sequence_items_and_every_owned_choice_case() {
    let mut snapshot = source();
    let joined = function(&snapshot, "joined");
    let owned_type = |id| {
        let object = TypeObject::new(TypeForm::TypeParameter { parameter: id }).unwrap();
        encode_type_object(&object).unwrap().0
    };
    let good = owned_type(joined.type_parameters[0]);
    let bad = owned_type(joined.type_parameters[1]);
    let choice = TypeObject::new(TypeForm::OwnedChoice {
        cases: vec![
            StructuralTypeField {
                name: Name::new("accepted").unwrap(),
                ty: good,
            },
            StructuralTypeField {
                name: Name::new("pending").unwrap(),
                ty: bad,
            },
        ],
    })
    .unwrap();
    let choice_ty = encode_type_object(&choice).unwrap().0;
    snapshot.types.insert(choice_ty, choice);
    let sequence = TypeObject::new(TypeForm::OwnedSequence { item: choice_ty }).unwrap();
    let sequence_ty = encode_type_object(&sequence).unwrap().0;
    snapshot.types.insert(sequence_ty, sequence);
    let oracle = Oracle(
        &snapshot,
        Some(DeclarationReference {
            package: snapshot.root.package_id,
            declaration: declaration(&snapshot, "joined"),
        }),
    );
    let mut remaining = contract::MAXIMUM_VALIDATION_WORK;
    assert_eq!(
        oracle.read_shape(sequence_ty, &BTreeMap::new(), 0, &mut remaining),
        None,
        "an unused case inside a dynamic item retains its independent share requirement"
    );
}
