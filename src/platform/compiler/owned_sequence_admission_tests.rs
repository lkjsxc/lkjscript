//! Independent artifact admission binds variable-cardinality custody to accepted syntax.
use super::*;
use crate::platform::kernel::KernelSnapshot;

// Loan syntax is deliberately inactive. A runtime-selected path cannot bypass admission.
const SOURCE: &str = r#"declarations.begin
(units (module create sequence-artifact
  (external create buffer-length (visibility private) (implementation core.buffer.length)
    (parameter create value (type ByteBuffer) (use borrow)) (returns I64))
  (function create empty (visibility private) (effect pure)
    (type-parameter create T (constraint owned))
    (returns (owned-sequence T))
    (body (sequence-empty (type (owned-sequence T)))))
  (function create length (visibility private) (effect pure)
    (parameter create values (type (owned-sequence ByteBuffer)) (use borrow))
    (parameter create other (type (owned-sequence ByteBuffer)) (use borrow))
    (returns I64)
    (body (sequence-length (type (owned-sequence ByteBuffer)) (local values))))
  (function create push (visibility private) (effect pure)
    (parameter create value (type ByteBuffer) (use consume))
    (parameter create other (type ByteBuffer) (use consume))
    (parameter create values (type (owned-sequence ByteBuffer)) (use consume))
    (parameter create other-values (type (owned-sequence ByteBuffer)) (use consume))
    (returns (owned-sequence ByteBuffer))
    (body (sequence-push (type (owned-sequence ByteBuffer)) (local value) (local values))))
  (function create pop (visibility private) (effect pure)
    (parameter create values (type (owned-sequence ByteBuffer)) (use consume))
    (parameter create other (type (owned-sequence ByteBuffer)) (use consume))
    (returns (owned-choice (case empty (owned-sequence ByteBuffer))
      (case item (owned-product (field rest (owned-sequence ByteBuffer)) (field value ByteBuffer)))))
    (body (sequence-pop (type (owned-sequence ByteBuffer)) (local values))))
  (function create inspect (visibility private) (effect pure)
    (parameter create values (type (owned-sequence ByteBuffer)) (use borrow))
    (parameter create other (type (owned-sequence ByteBuffer)) (use borrow))
    (returns I64)
    (body (if (bool true) (i64 7)
      (borrow-owned-item (type (owned-sequence ByteBuffer)) (local values)
        (index (i64 3)) (binding view (type ByteBuffer))
        (in (call buffer-length (local view)))))))))
declarations.end
"#;

fn fixture() -> (KernelSnapshot, LoadedArtifact) {
    let source =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(SOURCE)
            .expect("sequence fixture authored through the native change surface");
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let directory = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&directory.path().join("sequence"), &source, None)
        .unwrap()
        .repository;
    let first = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    let second = build_clean(&repository, OptimizationPolicy::DeterministicBaseline).unwrap();
    assert_eq!(first.manifest_digest, second.manifest_digest);
    let artifact = link_artifact(&repository, first.manifest_digest, &[]).unwrap();
    (source, load_artifact(&artifact.artifact.bytes).unwrap())
}

fn function_unit(
    source: &KernelSnapshot,
    loaded: &LoadedArtifact,
    name: &str,
) -> (ObjectKey, CompilationUnit) {
    let owner = OwnerKey::Declaration(declaration_named(source, name));
    loaded
        .objects
        .iter()
        .filter(|(key, _)| key.domain == ObjectDomain::CompilerUnit)
        .map(|(key, bytes)| (*key, CompilationUnit::decode(bytes, *key).unwrap()))
        .find(|(_, unit)| unit.source.owner == owner)
        .unwrap()
}

fn code(unit: &CompilationUnit) -> &CompiledCode {
    let CompilationPayload::Function { code, .. } = &unit.payload else {
        panic!("function");
    };
    code
}

fn code_mut(unit: &mut CompilationUnit) -> &mut CompiledCode {
    let CompilationPayload::Function { code, .. } = &mut unit.payload else {
        panic!("function");
    };
    code
}

fn reject_changed_unit(loaded: &LoadedArtifact, key: ObjectKey, unit: &CompilationUnit) {
    let bytes = match unit.validate() {
        Ok(()) => effect_tests::replace_unit(loaded, key, unit, vec![]),
        Err(error) => effect_tests::replace_rejected_unit(loaded, key, unit, &error.code),
    };
    load_artifact(&bytes).expect_err("repaired digests cannot change accepted sequence meaning");
}

#[test]
fn owned_sequence_lowering_binds_generic_empty_and_consuming_local_operands() {
    let (source, loaded) = fixture();
    let (empty_key, empty) = function_unit(&source, &loaded, "empty");
    let CompiledInstruction::SequenceEmpty { sequence_type } = code(&empty).instructions[0] else {
        panic!("generic sequence empty lowers directly");
    };
    let sequence = source.types[&empty.tables.types[sequence_type as usize]].clone();
    let TypeForm::OwnedSequence { item } = sequence.form else {
        panic!("sequence type");
    };
    assert!(matches!(
        source.types[&item].form,
        TypeForm::TypeParameter { .. }
    ));
    let mut forged_empty = empty.clone();
    let item_type = forged_empty
        .tables
        .types
        .iter()
        .position(|ty| *ty == item)
        .unwrap() as u32;
    code_mut(&mut forged_empty).instructions[0] = CompiledInstruction::SequenceEmpty {
        sequence_type: item_type,
    };
    reject_changed_unit(&loaded, empty_key, &forged_empty);

    for (name, faults) in [("length", 1), ("push", 2), ("pop", 2)] {
        let (key, original) = function_unit(&source, &loaded, name);
        load_artifact(&effect_tests::replace_unit(&loaded, key, &original, vec![]))
            .expect("neutral digest repair preserves sequence meaning");
        for fault in 0..faults {
            let mut changed = original.clone();
            match &mut code_mut(&mut changed).instructions[0] {
                CompiledInstruction::SequenceLength { source_local, .. } => *source_local = 1,
                CompiledInstruction::SequencePush {
                    value_local,
                    source_local,
                    ..
                } => {
                    assert_eq!((*value_local, *source_local), (0, 2));
                    if fault == 0 {
                        *value_local = 1;
                    } else {
                        *source_local = 3;
                    }
                }
                CompiledInstruction::SequencePop {
                    sequence_type,
                    result_type,
                    source_local,
                } => {
                    if fault == 0 {
                        *source_local = 1;
                    } else {
                        *result_type = *sequence_type;
                    }
                }
                _ => panic!("expected direct sequence instruction"),
            }
            reject_changed_unit(&loaded, key, &changed);
        }
    }
}

#[test]
fn owned_sequence_item_artifact_rejects_index_source_type_and_cleanup_forgery() {
    let (source, loaded) = fixture();
    let (key, original) = function_unit(&source, &loaded, "inspect");
    let begin = code(&original)
        .instructions
        .iter()
        .position(|i| matches!(i, CompiledInstruction::BorrowOwnedItem { .. }))
        .unwrap();
    let end = code(&original)
        .instructions
        .iter()
        .position(|i| matches!(i, CompiledInstruction::EndOwnedBorrow { .. }))
        .unwrap();
    assert!(matches!(
        code(&original).instructions[begin - 1],
        CompiledInstruction::I64(3)
    ));
    for fault in [
        "index",
        "source",
        "type",
        "binding",
        "erase-end",
        "erase-begin",
    ] {
        let mut changed = original.clone();
        let instructions = &mut code_mut(&mut changed).instructions;
        match fault {
            "index" => instructions[begin - 1] = CompiledInstruction::I64(4),
            "erase-end" => instructions[end] = CompiledInstruction::Jump(end as u32 + 1),
            "erase-begin" => instructions[begin] = CompiledInstruction::Drop,
            _ => {
                let CompiledInstruction::BorrowOwnedItem {
                    sequence_type,
                    source_local,
                    binding_type,
                    binding_local,
                } = &mut instructions[begin]
                else {
                    unreachable!();
                };
                match fault {
                    "source" => *source_local = 1,
                    "type" => *binding_type = *sequence_type,
                    "binding" => *binding_local = *source_local,
                    _ => unreachable!(),
                }
            }
        }
        reject_changed_unit(&loaded, key, &changed);
    }
}
