//! Runtime envelope guards run before consuming the sequence source.
use super::super::{owned_i64_cell, owned_product};
use super::*;
use crate::platform::kernel::{KernelSnapshot, TypeObject};

const SOURCE: &str = r#"declarations.begin
(units (module create sequence-eval
  (type-alias Seq (owned-sequence OwnedI64Cell))
  (type-alias Item (owned-product (field rest Seq) (field value OwnedI64Cell)))
  (type-alias Pop (owned-choice (case empty Seq) (case item Item)))
  (external create new-cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create finish-cell (visibility private) (implementation core.cell.extract)
    (parameter create value (type OwnedI64Cell) (use consume)) (returns I64))
  (function create finish-sequence (visibility private) (effect pure)
    (parameter create values (type Seq) (use consume)) (returns I64)
    (body (let
      (binding outcome (type Pop) (sequence-pop (type Seq) (local values)))
      (in (match-owned (type Pop) (local outcome)
        (case empty (binding rest (type Seq))
          (in (sequence-length (type Seq) (local rest))))
        (case item (binding item (type Item))
          (in (unpack-owned (type Item) (local item)
            (field rest (binding rest (type Seq)))
            (field value (binding value (type OwnedI64Cell)))
            (in (call finish-cell (local value)))))))))))
  (function create empty-main (visibility private) (effect pure) (returns I64)
    (body (let
      (binding values (type Seq) (sequence-empty (type Seq)))
      (in (call finish-sequence (local values))))))
  (function create item-main (visibility private) (effect pure) (returns I64)
    (body (let
      (binding values (type Seq) (sequence-empty (type Seq)))
      (binding value (type OwnedI64Cell) (call new-cell (i64 41)))
      (binding values (type Seq) (sequence-push (type Seq) (local value) (local values)))
      (in (call finish-sequence (local values))))))))
declarations.end"#;

#[derive(Clone, Copy, Debug)]
enum Fault {
    ResultForm,
    CaseArity,
    CaseName,
    CaseOrder,
    EmptyType,
    ItemType,
    ProductForm,
    FieldArity,
    FieldName,
    FieldOrder,
    RestType,
    ValueType,
}

fn pop_types(source: &KernelSnapshot) -> (TypeObjectDigest, TypeObjectDigest, TypeObjectDigest) {
    source
        .owners
        .values()
        .find_map(|owner| {
            let OwnerRecord::Expression(expression) = owner else {
                return None;
            };
            let ExpressionOperation::SequencePop {
                sequence_type,
                result_type,
                ..
            } = expression.operation
            else {
                return None;
            };
            let TypeForm::OwnedChoice { cases } = &source.types[&result_type].form else {
                panic!("pop result")
            };
            Some((sequence_type, result_type, cases[1].ty))
        })
        .unwrap()
}

fn scalar_type(types: &BTreeMap<TypeObjectDigest, TypeObject>) -> TypeObjectDigest {
    types
        .iter()
        .find_map(|(digest, object)| (object.form == TypeForm::I64).then_some(*digest))
        .unwrap()
}

fn mutate_shape(
    types: &mut BTreeMap<TypeObjectDigest, TypeObject>,
    result: TypeObjectDigest,
    product: TypeObjectDigest,
    fault: Fault,
) {
    let scalar = scalar_type(types);
    match fault {
        Fault::ResultForm => types.get_mut(&result).unwrap().form = TypeForm::I64,
        Fault::ProductForm => types.get_mut(&product).unwrap().form = TypeForm::I64,
        Fault::CaseArity
        | Fault::CaseName
        | Fault::CaseOrder
        | Fault::EmptyType
        | Fault::ItemType => {
            let TypeForm::OwnedChoice { cases } = &mut types.get_mut(&result).unwrap().form else {
                panic!("choice")
            };
            match fault {
                Fault::CaseArity => {
                    cases.pop();
                }
                Fault::CaseName => cases[0].name = Name::new("absent").unwrap(),
                Fault::CaseOrder => cases.swap(0, 1),
                Fault::EmptyType => cases[0].ty = scalar,
                Fault::ItemType => cases[1].ty = scalar,
                _ => unreachable!(),
            }
        }
        Fault::FieldArity
        | Fault::FieldName
        | Fault::FieldOrder
        | Fault::RestType
        | Fault::ValueType => {
            let TypeForm::OwnedProduct { fields } = &mut types.get_mut(&product).unwrap().form
            else {
                panic!("product")
            };
            match fault {
                Fault::FieldArity => {
                    fields.pop();
                }
                Fault::FieldName => fields[0].name = Name::new("remainder").unwrap(),
                Fault::FieldOrder => fields.swap(0, 1),
                Fault::RestType => fields[0].ty = scalar,
                Fault::ValueType => fields[1].ty = scalar,
                _ => unreachable!(),
            }
        }
    }
}

fn change_vm_pop(
    program: &mut NormalizedProgram,
    entry: DeclarationReference,
    scalar: Option<TypeObjectDigest>,
) {
    let function = Arc::make_mut(&mut program.functions)
        .iter_mut()
        .find(|f| f.declaration == entry)
        .unwrap();
    let NormalizedFunctionBody::Code(code) = &mut function.body else {
        panic!("graph function")
    };
    let pop = Arc::make_mut(&mut code.instructions)
        .iter_mut()
        .find(|i| matches!(i, NormalizedInstruction::SequencePop { .. }))
        .unwrap();
    let NormalizedInstruction::SequencePop {
        source_local,
        result_type,
        ..
    } = pop
    else {
        unreachable!()
    };
    *source_local = u32::MAX;
    if let Some(scalar) = scalar {
        *result_type = scalar;
    }
}

fn change_reference_pop(source: &mut KernelSnapshot, scalar: Option<TypeObjectDigest>) {
    let expression = source
        .owners
        .values_mut()
        .find_map(|owner| match owner {
            OwnerRecord::Expression(expression)
                if matches!(
                    expression.operation,
                    ExpressionOperation::SequencePop { .. }
                ) =>
            {
                Some(expression)
            }
            _ => None,
        })
        .unwrap();
    let ExpressionOperation::SequencePop {
        source: input,
        result_type,
        ..
    } = &mut expression.operation
    else {
        unreachable!()
    };
    *input = ExpressionId::from_bytes([0xee; 16]).unwrap();
    if let Some(scalar) = scalar {
        *result_type = scalar;
    }
}

fn run_clean(
    reference: bool,
    source: &KernelSnapshot,
    schema: Arc<super::super::NormalizedReferenceSchema>,
    program: &NormalizedProgram,
    name: &str,
) -> (Result<NormalizedValue, ExecutionError>, usize, usize) {
    let products = owned_product::StorageObservation::start();
    let cells = owned_i64_cell::StorageObservation::start();
    let entry = declaration_named(source, name);
    let control = ExecutionControl::uncancelled();
    let result = if reference {
        let reader = FaultedReferenceRead { source, schema };
        let sink = Mutex::new(None);
        let result = NormalizedReferenceInterpreter::from_reader(
            &reader,
            program,
            NormalizedRunPolicy::foreground(),
        )
        .observing_checked(&sink)
        .invoke(entry, vec![], None, &control)
        .map(|pair| pair.0);
        let observed = sink
            .into_inner()
            .unwrap()
            .expect("runtime guard was reached");
        assert_eq!(observed.live_call_frames_after, 0);
        assert_eq!(observed.live_local_scopes_after, 0);
        assert_eq!(observed.live_handles_after, 0);
        result
    } else {
        let sink = Mutex::new(None);
        let result = NormalizedVm::for_test(program, NormalizedRunPolicy::foreground())
            .observing_checked(&sink)
            .invoke(entry, vec![], None, &control)
            .map(|pair| pair.0);
        let observed = sink
            .into_inner()
            .unwrap()
            .expect("runtime guard was reached");
        assert_eq!(observed.live_call_frames_after, 0);
        assert_eq!(observed.live_operands_after, 0);
        assert_eq!(observed.live_handles_after, 0);
        result
    };
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
    products.assert_owners_released_after_loans();
    (result, products.created(), cells.created())
}

fn assert_failure(
    reference: bool,
    source: &KernelSnapshot,
    schema: Arc<super::super::NormalizedReferenceSchema>,
    program: &NormalizedProgram,
    name: &str,
) {
    let (failed, composites, cells) = run_clean(reference, source, schema, program, name);
    let error = failed.unwrap_err();
    assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
    assert_eq!(
        error.code,
        if reference {
            "normalized_reference_type"
        } else {
            "normalized_runtime_type"
        }
    );
    assert!(
        error.message.starts_with("sequence pop"),
        "shape rejection precedes the counterfeit source lookup: {error:?}"
    );
    assert_eq!(composites, 1, "no pop envelope was allocated");
    assert_eq!(cells, usize::from(name == "item-main"));
}

#[test]
fn owned_sequence_evaluators_reject_every_malformed_pop_shape_before_source_consumption() {
    let original = byte_buffer_tests::author_only(SOURCE).unwrap();
    let program = prepare_snapshot(&original);
    let schema =
        Arc::new(super::super::NormalizedReferenceSchema::reconstruct([&original]).unwrap());
    let (_, result, product) = pop_types(&original);
    let helper = declaration_named(&original, "finish-sequence");
    for fault in [
        Fault::ResultForm,
        Fault::CaseArity,
        Fault::CaseName,
        Fault::CaseOrder,
        Fault::EmptyType,
        Fault::ItemType,
        Fault::ProductForm,
        Fault::FieldArity,
        Fault::FieldName,
        Fault::FieldOrder,
        Fault::RestType,
        Fault::ValueType,
    ] {
        let mut invalid_program = program.clone();
        mutate_shape(&mut invalid_program.types, result, product, fault);
        change_vm_pop(&mut invalid_program, helper, None);
        let mut invalid_source = original.clone();
        mutate_shape(&mut invalid_source.types, result, product, fault);
        change_reference_pop(&mut invalid_source, None);
        let mut invalid_schema = (*schema).clone();
        mutate_shape(&mut invalid_schema.types, result, product, fault);
        let invalid_schema = Arc::new(invalid_schema);
        for reference in [false, true] {
            for (name, expected) in [("empty-main", 0), ("item-main", 41)] {
                assert_failure(
                    reference,
                    &invalid_source,
                    Arc::clone(&invalid_schema),
                    &invalid_program,
                    name,
                );
                let (healthy, _, _) =
                    run_clean(reference, &original, Arc::clone(&schema), &program, name);
                assert_eq!(
                    healthy.unwrap(),
                    NormalizedValue::I64(expected),
                    "{fault:?}"
                );
            }
        }
    }
}

#[test]
fn owned_sequence_evaluators_reject_counterfeit_pop_result_operand_before_source_consumption() {
    let original = byte_buffer_tests::author_only(SOURCE).unwrap();
    let program = prepare_snapshot(&original);
    let schema =
        Arc::new(super::super::NormalizedReferenceSchema::reconstruct([&original]).unwrap());
    let scalar = scalar_type(&program.types);
    let mut invalid_program = program.clone();
    change_vm_pop(
        &mut invalid_program,
        declaration_named(&original, "finish-sequence"),
        Some(scalar),
    );
    let mut invalid_source = original.clone();
    change_reference_pop(&mut invalid_source, Some(scalar));
    for reference in [false, true] {
        for (name, expected) in [("empty-main", 0), ("item-main", 41)] {
            assert_failure(
                reference,
                &invalid_source,
                Arc::clone(&schema),
                &invalid_program,
                name,
            );
            let (healthy, _, _) =
                run_clean(reference, &original, Arc::clone(&schema), &program, name);
            assert_eq!(healthy.unwrap(), NormalizedValue::I64(expected));
        }
    }
}

#[test]
fn owned_sequence_evaluators_tail_destructure_and_nested_borrows_remain_bounded_at_513_items() {
    let source = byte_buffer_tests::author_only(include_str!(
        "../../../../tests/fixtures/owned-sequence-tail.lkjc"
    ))
    .unwrap();
    let program = prepare_snapshot(&source);
    let schema = Arc::new(super::super::NormalizedReferenceSchema::reconstruct([&source]).unwrap());
    let reader = FaultedReferenceRead {
        source: &source,
        schema,
    };
    let entry = declaration_named(&source, "main");
    let result_type = program.functions[program.function(entry).unwrap().0 as usize].result;
    for reference in [false, true] {
        let products = owned_product::StorageObservation::start();
        let cells = owned_i64_cell::StorageObservation::start();
        for count in [0_i64, 1, 17, 513] {
            let mut policy = NormalizedRunPolicy::foreground();
            // A missing graph-tail handoff fails before recursion can approach
            // the host stack bound; correct execution stays below this limit.
            policy.maximum_call_depth = 16;
            let control = ExecutionControl::uncancelled();
            let arguments = vec![NormalizedValue::I64(count)];
            let (value, depth, frames, tails) = if reference {
                let sink = Mutex::new(None);
                let value = NormalizedReferenceInterpreter::from_reader(&reader, &program, policy)
                    .observing_checked(&sink)
                    .invoke(entry, arguments, None, &control)
                    .unwrap()
                    .0;
                let observed = sink.into_inner().unwrap().unwrap();
                assert_eq!(observed.live_call_frames_after, 0);
                assert_eq!(observed.live_local_scopes_after, 0);
                assert_eq!(observed.live_handles_after, 0);
                (
                    value,
                    observed.maximum_call_depth,
                    observed.maximum_control_frames,
                    observed.tail_transfers,
                )
            } else {
                let sink = Mutex::new(None);
                let value = NormalizedVm::for_test(&program, policy)
                    .observing_checked(&sink)
                    .invoke(entry, arguments, None, &control)
                    .unwrap()
                    .0;
                let observed = sink.into_inner().unwrap().unwrap();
                assert_eq!(observed.live_call_frames_after, 0);
                assert_eq!(observed.live_operands_after, 0);
                assert_eq!(observed.live_handles_after, 0);
                (
                    value,
                    observed.maximum_call_depth,
                    observed.maximum_control_frames,
                    observed.tail_transfers,
                )
            };
            let actual = super::super::codec::encode_value(
                &program,
                &value,
                result_type,
                JsonLimits::default(),
            )
            .unwrap();
            let expected = serde_json::json!({"count": count, "sum": count * (count - 1) / 2,
                "values": (0..count).rev().collect::<Vec<_>>()});
            assert_eq!(actual, expected, "reference={reference} count={count}");
            assert!(
                depth <= 5,
                "reference={reference} count={count} depth={depth}"
            );
            assert!(
                frames <= 24,
                "reference={reference} count={count} frames={frames}"
            );
            if count == 513 {
                assert!(
                    tails >= 1536,
                    "all three collection loops must transfer tail ownership: reference={reference} tails={tails}"
                );
            }
            assert_eq!(products.live(), (0, 0));
            assert_eq!(cells.live(), (0, 0));
            products.assert_owners_released_after_loans();
        }
    }
}
