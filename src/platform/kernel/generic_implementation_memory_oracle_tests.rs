//! Independent scheme admission and borrowed-result provenance over canonical source.
use super::*;
use crate::platform::execution::normalized::NormalizedReferenceSchema;
use crate::platform::semantic_id::DeclarationId;

const SOURCE: &str = r#"declarations.begin
(units (module create schemes
  (external create make (visibility private) (implementation core.cell.create)
    (parameter create value (type I64)) (returns OwnedI64Cell))
  (external create read (visibility private) (implementation core.cell.read)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64))
  (owned-contract create Reader (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (type-parameter create Item (constraint owned))
    (type-parameter create Phantom (constraint owned))
    (method method_a8000000000000000000000000000001 choose
      (parameters (Bool unrestricted) (Self borrow) (Self borrow))
      (returns Item (borrow-from 1))))
  (function create choose-first (visibility public) (effect pure)
    (type-parameter create Element (constraint owned))
    (type-parameter create Unused (constraint owned))
    (parameter create flag (type Bool))
    (parameter create selected (type (owned-product (field item Element))) (use borrow))
    (parameter create other (type (owned-product (field item Element))) (use borrow))
    (returns Element (borrow-from selected))
    (body (borrow-owned-field (type (owned-product (field item Element))) (local selected)
      (field item (binding view (type Element))) (in (local view)))))
  (owned-implementation create ReaderScheme (visibility public)
    (type-parameter create T (constraint owned))
    (type-parameter create P (constraint owned))
    (contract Reader) (self (owned-product (field item T))) (types T P)
    (method method_a8000000000000000000000000000001 choose-first (types T P)))
  (function create forward (visibility public) (effect pure)
    (type-parameter create Storage (constraint owned))
    (type-parameter create Item (constraint owned))
    (type-parameter create Phantom (constraint owned))
    (implementation-parameter implparam_a8000000000000000000000000000001 reader Reader Storage (types Item Phantom))
    (parameter create selected (type Storage) (use borrow))
    (parameter create other (type Storage) (use borrow))
    (returns Item (borrow-from selected))
    (body (borrow-call
      (method-call parameter@forward@implparam_a8000000000000000000000000000001
        Reader method_a8000000000000000000000000000001 (bool true) (local selected) (local other))
      (binding view (type Item)) (in (local view)))))
  (function create main (visibility public) (effect pure) (returns I64)
    (body (let
      (binding a-cell (type OwnedI64Cell) (call make (i64 17)))
      (binding a (type (owned-product (field item OwnedI64Cell)))
        (pack-owned (type (owned-product (field item OwnedI64Cell))) (field item (local a-cell))))
      (binding b-cell (type OwnedI64Cell) (call make (i64 29)))
      (binding b (type (owned-product (field item OwnedI64Cell)))
        (pack-owned (type (owned-product (field item OwnedI64Cell))) (field item (local b-cell))))
      (in (borrow-call
        (implementation-call forward (types (owned-product (field item OwnedI64Cell)) OwnedI64Cell ByteBuffer)
          (implementations (implementation ReaderScheme (types OwnedI64Cell ByteBuffer))) (local a) (local b))
        (binding view (type OwnedI64Cell)) (in (call read (local view))))))))))
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
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(owner))
                if owner.name.as_str() == name =>
            {
                Some(*id)
            }
            _ => None,
        })
        .unwrap()
}

fn implementation(snapshot: &mut KernelSnapshot) -> &mut OwnedImplementation {
    let id = declaration(snapshot, "ReaderScheme");
    let OwnerRecord::Declaration(owner) =
        snapshot.owners.get_mut(&OwnerKey::Declaration(id)).unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(implementation) = &mut owner.payload else {
        unreachable!()
    };
    implementation
}

fn intern(snapshot: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let ty = encode_type_object(&object).unwrap().0;
    snapshot.types.insert(ty, object);
    ty
}

fn reencode(snapshot: &mut KernelSnapshot) {
    for (key, owner) in &mut snapshot.owners {
        let kind = owner.header().kind;
        let (digest, bytes) = encode_owner(owner).unwrap();
        *owner = decode_owner(&bytes, *key, kind, digest).unwrap();
    }
}

fn rejected(snapshot: &KernelSnapshot) {
    assert!(
        !accepts(snapshot),
        "independent memory oracle must reject forged scheme source"
    );
    assert!(
        NormalizedReferenceSchema::independently_admit_witness_source(snapshot).is_err(),
        "canonical reference must independently reject forged scheme source"
    );
}

#[test]
fn generic_implementation_oracles_admit_symbolic_scheme_and_applied_borrowed_mapping() {
    let snapshot = source();
    assert!(accepts(&snapshot));
    NormalizedReferenceSchema::reconstruct([&snapshot]).unwrap();
    NormalizedReferenceSchema::independently_admit_witness_source(&snapshot).unwrap();
}

#[test]
fn generic_implementation_oracles_reject_unused_mapping_arguments_and_parameter_scopes() {
    let baseline = source();
    let mut changed = baseline.clone();
    let ordinary = intern(&mut changed, TypeForm::I64);
    implementation(&mut changed).methods[0].type_arguments[1] = ordinary;
    reencode(&mut changed);
    rejected(&changed);

    let mut changed = baseline.clone();
    implementation(&mut changed).methods[0].type_arguments.pop();
    reencode(&mut changed);
    rejected(&changed);

    let mut changed = baseline.clone();
    let foreign = declaration(&changed, "choose-first");
    let parameter = implementation(&mut changed).type_parameters[1];
    let OwnerRecord::TypeParameter(owner) = changed
        .owners
        .get_mut(&OwnerKey::TypeParameter(parameter))
        .unwrap()
    else {
        unreachable!()
    };
    owner.declaration = foreign;
    reencode(&mut changed);
    rejected(&changed);

    let mut changed = baseline.clone();
    let parameter = implementation(&mut changed).type_parameters[1];
    let OwnerRecord::TypeParameter(owner) = changed
        .owners
        .get_mut(&OwnerKey::TypeParameter(parameter))
        .unwrap()
    else {
        unreachable!()
    };
    owner.constraints = TypeParameterConstraints::None;
    reencode(&mut changed);
    rejected(&changed);
}

#[test]
fn generic_implementation_oracles_reject_wrong_generic_borrowed_result_source() {
    let mut changed = source();
    let id = declaration(&changed, "choose-first");
    let OwnerRecord::Declaration(owner) =
        changed.owners.get_mut(&OwnerKey::Declaration(id)).unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut owner.payload else {
        unreachable!()
    };
    function.result_borrow = Some(function.parameters[2]);
    reencode(&mut changed);
    rejected(&changed);
}

#[test]
fn generic_implementation_oracles_reject_wrong_input_body_after_generic_mapping() {
    for aliases in [false, true] {
        let authored = if aliases {
            SOURCE.replacen("(local a) (local b))", "(local a) (local a))", 1)
        } else {
            SOURCE.to_owned()
        };
        let mut changed =
            crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
                &authored,
            )
            .unwrap();
        assert!(accepts(&changed));
        let id = declaration(&changed, "choose-first");
        let OwnerRecord::Declaration(owner) = &changed.owners[&OwnerKey::Declaration(id)] else {
            unreachable!()
        };
        let DeclarationPayload::Function(function) = &owner.payload else {
            unreachable!()
        };
        let body = function.body;
        let other = function.parameters[2];
        let OwnerRecord::Expression(expression) = &changed.owners[&OwnerKey::Expression(body)]
        else {
            unreachable!()
        };
        let ExpressionOperation::BorrowOwnedField { source, .. } = expression.operation else {
            unreachable!()
        };
        let OwnerRecord::Expression(expression) = changed
            .owners
            .get_mut(&OwnerKey::Expression(source))
            .unwrap()
        else {
            unreachable!()
        };
        expression.operation = ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(other),
        };
        reencode(&mut changed);
        rejected(&changed);
    }
}
