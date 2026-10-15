//! Open task groups retain exact bounds, witnesses, and symbolic pair meaning.
use super::*;
use crate::platform::kernel::{ImplementationOperand, TypeParameterConstraints};

const OPEN_SOURCE: &str = r#"declarations.begin
(units (module create open-artifact
  (function create ordinary (visibility public) (effect (task))
    (type-parameter create U (constraint transferable))
    (parameter create value (type U)) (returns U) (body (local value)))
  (function create owned (visibility public) (effect (task))
    (type-parameter create T (constraint owned transferable))
    (parameter create value (type T) (use consume)) (returns T) (body (local value)))
  (function create joined (visibility public) (effect (task))
    (type-parameter create T (constraint owned transferable))
    (type-parameter create U (constraint transferable))
    (parameter create data (type U))
    (parameter create value (type T) (use consume)) (returns I64)
    (body (sequence
      (parallel (call owned (types T) (local value))
                (call ordinary (types U) (local data)))
      (i64 41))))))
declarations.end
"#;

fn open_fixture(literal: &str) -> (LoadedArtifact, ObjectKey, CompilationUnit, TypeObjectDigest) {
    let source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(literal)
            .unwrap();
    let loaded = artifact_for_source(&source);
    let (key, unit, pair) = parallel_result(&loaded, &source);
    (loaded, key, unit, pair)
}

#[test]
fn parallel_open_pair_inference_is_preserved_without_any_concrete_caller() {
    for (literal, owned) in [
        (OPEN_SOURCE.to_owned(), true),
        (
            OPEN_SOURCE.replace(
                "(parallel (call owned (types T) (local value))\n                (call ordinary (types U) (local data)))",
                "(parallel (call ordinary (types U) (local data))\n                (call owned (types T) (local value)))",
            ),
            true,
        ),
        (
            OPEN_SOURCE
                .replace("(constraint owned transferable)", "(constraint transferable)")
                .replace(" (use consume)", ""),
            false,
        ),
    ] {
        let (loaded, key, unit, pair) = open_fixture(&literal);
        let object = decode_type_object(
            &loaded.objects[&ObjectKey::from_digest(ObjectDomain::Type, pair.bytes())],
            pair,
        )
        .unwrap();
        let fields = match object.form {
            TypeForm::OwnedProduct { fields } if owned => fields,
            TypeForm::StructuralRecord { fields } if !owned => fields,
            other => panic!("wrong symbolic pair: {other:?}"),
        };
        assert_eq!(fields[0].name.as_str(), "left");
        assert_eq!(fields[1].name.as_str(), "right");
        for field in fields {
            assert!(matches!(
                decode_type_object(
                    &loaded.objects[&ObjectKey::from_digest(ObjectDomain::Type, field.ty.bytes())],
                    field.ty,
                )
                .unwrap()
                .form,
                TypeForm::TypeParameter { .. }
            ));
        }
        let CompilationPayload::Function { signature, .. } = &unit.payload else {
            panic!("function");
        };
        assert_ne!(unit.tables.types[signature.result.ty as usize], pair);
        load_artifact(&effect_tests::replace_unit(&loaded, key, &unit, vec![])).unwrap();
    }
}

#[test]
fn parallel_open_artifact_rejects_rehashed_erased_transfer_bounds() {
    for (before, after) in [
        (
            TypeParameterConstraints::OwnedTransferable,
            TypeParameterConstraints::Owned,
        ),
        (
            TypeParameterConstraints::Transferable,
            TypeParameterConstraints::None,
        ),
        (
            TypeParameterConstraints::CaptureSafeTransferable,
            TypeParameterConstraints::CaptureSafe,
        ),
    ] {
        let literal = if before == TypeParameterConstraints::CaptureSafeTransferable {
            OPEN_SOURCE.replace(
                "(constraint transferable)",
                "(constraint capture-safe transferable)",
            )
        } else {
            OPEN_SOURCE.to_owned()
        };
        let (loaded, key, mut unit, _) = open_fixture(&literal);
        let CompilationPayload::Function { signature, .. } = &mut unit.payload else {
            panic!("function");
        };
        *signature
            .type_parameter_constraints
            .iter_mut()
            .find(|c| **c == before)
            .unwrap() = after;
        let error = load_artifact(&effect_tests::replace_unit(&loaded, key, &unit, vec![]))
            .expect_err("enclosing rehashes cannot erase a canonical transfer obligation");
        assert!(
            matches!(
                error.class,
                DiagnosticClass::Corrupt | DiagnosticClass::Semantic
            ),
            "{error:?}"
        );
    }
}

#[test]
fn parallel_open_artifact_rejects_rehashed_symbolic_and_concrete_pair_substitution() {
    let (loaded, key, unit, pair) = open_fixture(OPEN_SOURCE);
    let object = decode_type_object(
        &loaded.objects[&ObjectKey::from_digest(ObjectDomain::Type, pair.bytes())],
        pair,
    )
    .unwrap();
    let TypeForm::OwnedProduct { fields } = object.form else {
        panic!("owned symbolic pair");
    };
    let (cell, cell_bytes) =
        encode_type_object(&TypeObject::new(TypeForm::OwnedI64Cell).unwrap()).unwrap();
    for attack in 0..3 {
        let mut fields = fields.clone();
        let forged = match attack {
            0 => TypeObject::new(TypeForm::StructuralRecord { fields }).unwrap(),
            1 => {
                let left = fields[0].ty;
                fields[0].ty = fields[1].ty;
                fields[1].ty = left;
                TypeObject::new(TypeForm::OwnedProduct { fields }).unwrap()
            }
            _ => {
                fields[0].ty = cell;
                TypeObject::new(TypeForm::OwnedProduct { fields }).unwrap()
            }
        };
        let (forged, bytes) = encode_type_object(&forged).unwrap();
        let mut changed = unit.clone();
        changed.tables.types.push(forged);
        let CompilationPayload::Function { code, .. } = &mut changed.payload else {
            panic!("function");
        };
        let CompiledInstruction::Parallel { result_type, .. } = code
            .instructions
            .iter_mut()
            .find(|i| matches!(i, CompiledInstruction::Parallel { .. }))
            .unwrap()
        else {
            unreachable!();
        };
        *result_type = (changed.tables.types.len() - 1) as u32;
        let error = load_artifact(&effect_tests::replace_unit(
            &loaded,
            key,
            &changed,
            vec![
                (
                    ObjectKey::from_digest(ObjectDomain::Type, forged.bytes()),
                    bytes,
                ),
                (
                    ObjectKey::from_digest(ObjectDomain::Type, cell.bytes()),
                    cell_bytes.clone(),
                ),
            ],
        ))
        .expect_err("symbolic pair meaning cannot be replaced by a rehashed type operand");
        assert!(
            matches!(
                error.class,
                DiagnosticClass::Corrupt | DiagnosticClass::Semantic
            ),
            "{attack}: {error:?}"
        );
    }
}

#[test]
fn parallel_open_artifact_rejects_rehashed_lexical_witness_substitution() {
    let group = r#"declarations.begin
(units (module create open-witness-artifact
  (function create joined (visibility public) (effect (task))
    (type-parameter create T (constraint owned transferable))
    (implementation-parameter implparam_94000000000000000000000000000001 left abstraction::Storage T)
    (implementation-parameter implparam_94000000000000000000000000000002 right abstraction::Storage T)
    (parameter create n (type I64))
    (parameter create a (type T) (use consume))
    (parameter create b (type T) (use consume))
    (returns (owned-product (field left T) (field right T)))
    (body (parallel
      (implementation-call generic-workers::forward (types T)
        (implementations parameter@joined@implparam_94000000000000000000000000000001)
        (local n) (local a))
      (implementation-call generic-workers::forward (types T)
        (implementations parameter@joined@implparam_94000000000000000000000000000002)
        (local n) (local b)))))))
declarations.end
"#;
    let literal = [
        include_str!("../../../tests/fixtures/owned-witness-library.lkjc"),
        include_str!("../../../tests/fixtures/parallel-generic-workers.lkjc"),
        group,
    ]
    .join("\n");
    let (loaded, key, unit, _) = open_fixture(&literal);
    for attack in 0..3 {
        let mut changed = unit.clone();
        let CompilationPayload::Function { code, .. } = &mut changed.payload else {
            panic!("function");
        };
        let CompiledInstruction::Parallel {
            left_implementations,
            right_implementations,
            ..
        } = code
            .instructions
            .iter_mut()
            .find(|i| matches!(i, CompiledInstruction::Parallel { .. }))
            .unwrap()
        else {
            unreachable!();
        };
        match attack {
            0 => *left_implementations = right_implementations.clone(),
            1 => left_implementations.clear(),
            _ => {
                let ImplementationOperand::Parameter { scope, .. } = &mut left_implementations[0]
                else {
                    panic!("lexical witness");
                };
                *scope = changed
                    .tables
                    .declarations
                    .iter()
                    .copied()
                    .find(|r| *r != *scope)
                    .unwrap();
            }
        }
        let bytes = if attack == 2 {
            effect_tests::replace_rejected_unit(
                &loaded,
                key,
                &changed,
                "compiler_unit_witness_scope",
            )
        } else {
            effect_tests::replace_unit(&loaded, key, &changed, vec![])
        };
        let error = load_artifact(&bytes)
            .expect_err("lexical witness identity survives every enclosing rehash");
        if attack == 2 {
            assert_eq!(error.code, "compiler_unit_witness_scope");
        }
        assert!(
            matches!(
                error.class,
                DiagnosticClass::Corrupt | DiagnosticClass::Semantic
            ),
            "{attack}: {error:?}"
        );
    }
}
