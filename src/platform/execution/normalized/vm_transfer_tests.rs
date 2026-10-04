use super::*;
use crate::platform::execution::normalized::{
    byte_buffer::{ByteBuffer, StorageObservation as Buffers},
    owned_choice::OwnedChoice,
    owned_i64_cell::{OwnedI64Cell, StorageObservation as Cells},
    owned_product::{OwnedProduct, StorageObservation as Products},
    owned_sequence::OwnedSequence,
    tests::byte_buffer_tests,
};
use crate::platform::kernel::{DeclarationReference, OwnerKey, OwnerRecord};
use crate::platform::publication::GraphRepository;
use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

const SOURCE: &str = r#"declarations.begin
(units (module create custody
  (function create tree (visibility public) (effect (task))
    (parameter create payload (type (owned-choice (case empty Unit)
      (case packet (owned-product (field bytes ByteBuffer) (field cell OwnedI64Cell) (field label Text))))) (use consume))
    (returns Unit) (body (unit)))
  (function create buffer (visibility public) (effect (task))
    (parameter create payload (type ByteBuffer) (use consume)) (returns Unit) (body (unit)))
  (function create tree-result (visibility public) (effect (task))
    (parameter create payload (type (owned-choice (case empty Unit)
      (case packet (owned-product (field bytes ByteBuffer) (field cell OwnedI64Cell) (field label Text))))) (use consume))
    (returns (owned-choice (case empty Unit)
      (case packet (owned-product (field bytes ByteBuffer) (field cell OwnedI64Cell) (field label Text)))))
    (body (local payload)))
  (function create buffer-result (visibility public) (effect (task))
    (parameter create payload (type ByteBuffer) (use consume))
    (returns ByteBuffer) (body (local payload)))
  (function create ordered (visibility public) (effect (task))
    (parameter create label (type Text))
    (parameter create payload (type ByteBuffer) (use consume)) (returns Unit) (body (unit)))
  (function create fixed-result (visibility public) (effect (task))
    (type-parameter create Tag)
    (parameter create payload (type ByteBuffer) (use consume)) (returns ByteBuffer)
    (body (local payload)))
  (function create hidden-callback (visibility public) (effect (task))
    (parameter create payload (type (option (function () Unit))))
    (returns Unit) (body (unit)))))
declarations.end"#;

fn fixture() -> (Arc<NormalizedProgram>, BTreeMap<String, FunctionIndex>) {
    static PREPARED: OnceLock<(Arc<NormalizedProgram>, BTreeMap<String, FunctionIndex>)> =
        OnceLock::new();
    let (program, functions) = PREPARED.get_or_init(|| {
        let source = byte_buffer_tests::author_only(SOURCE).unwrap();
        let temporary = tempfile::tempdir().unwrap();
        let repository = GraphRepository::create(&temporary.path().join("custody"), &source, None)
            .unwrap()
            .repository;
        let application =
            crate::platform::normalized_lifecycle::prepare_repository(repository).unwrap();
        let program = application.program;
        let functions = source
            .owners
            .iter()
            .filter_map(|(key, owner)| {
                let (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(owner)) =
                    (key, owner)
                else {
                    return None;
                };
                let function = program.function(DeclarationReference {
                    package: source.root.package_id,
                    declaration: *declaration,
                })?;
                Some((owner.name.to_string(), function))
            })
            .collect();
        (program, functions)
    });
    (Arc::clone(program), functions.clone())
}

fn payload(
    program: &NormalizedProgram,
    function: FunctionIndex,
    source: ValueOrigin,
    leaf_source: ValueOrigin,
    borrowed: bool,
    invalid_metadata: bool,
) -> (NormalizedValue, [usize; 3], Option<ByteBuffer>) {
    let ty = program.functions[function.0 as usize].parameters[0].ty;
    let TypeForm::OwnedChoice { cases } = &program.types[&ty].form else {
        panic!("choice");
    };
    let product_type = cases[1].ty;
    assert_eq!(cases[1].name.as_str(), "packet");
    let TypeForm::OwnedProduct { fields } = &program.types[&product_type].form else {
        panic!("product");
    };
    assert_eq!(
        fields.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["bytes", "cell", "label"]
    );
    let control = ExecutionControl::uncancelled();
    let buffer = ByteBuffer::empty(leaf_source)
        .push(197, &control, &mut |_| Ok(()))
        .unwrap();
    let loan = borrowed.then(|| buffer.borrow().unwrap());
    let cell = OwnedI64Cell::new(source, -137);
    let label: Arc<str> = Arc::from("metadata");
    let identities = [
        buffer.allocation_identity(),
        cell.allocation_identity(),
        label.as_ptr() as usize,
    ];
    let fields = vec![
        NormalizedValue::ByteBuffer(buffer),
        NormalizedValue::OwnedI64Cell(cell),
        if invalid_metadata {
            NormalizedValue::Bool(false)
        } else {
            NormalizedValue::Text(label)
        },
    ];
    let product =
        OwnedProduct::create(source, product_type, fields, &control, &mut |_| Ok(())).unwrap();
    let choice = OwnedChoice::create(
        source,
        ty,
        1,
        NormalizedValue::OwnedProduct(product),
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    (NormalizedValue::OwnedChoice(choice), identities, loan)
}

fn ordinary(
    program: &NormalizedProgram,
    value: &NormalizedValue,
    ty: TypeObjectDigest,
) -> Result<(), ExecutionError> {
    // Independent small expected shape for this fixture; production and reference
    // integration tests exercise their respective complete borrowed admissions.
    match (value, &program.types[&ty].form) {
        (NormalizedValue::Text(text), TypeForm::Text) if &**text == "metadata" => Ok(()),
        _ => Err(ExecutionError::new(
            ExecutionFailureClass::Trap,
            "test_metadata",
            "unexpected metadata",
        )),
    }
}

fn sequence_fixture() -> (NormalizedProgram, FunctionIndex, TypeObjectDigest) {
    use crate::platform::kernel::{TypeObject, encode_type_object};
    let (prepared, functions) = fixture();
    let mut program = (*prepared).clone();
    let item = *program
        .types
        .iter()
        .find(|(_, object)| matches!(object.form, TypeForm::OwnedI64Cell))
        .unwrap()
        .0;
    let object = TypeObject::new(TypeForm::OwnedSequence { item }).unwrap();
    let ty = encode_type_object(&object).unwrap().0;
    program.types.insert(ty, object);
    let function = functions["buffer-result"];
    let target = &mut Arc::make_mut(&mut program.functions)[function.0 as usize];
    Arc::make_mut(&mut target.parameters)[0].ty = ty;
    target.result = ty;
    (program, function, ty)
}

fn sequence_payload(
    origin: ValueOrigin,
    ty: TypeObjectDigest,
    scalars: &[i64],
) -> (NormalizedValue, Vec<usize>) {
    let control = ExecutionControl::uncancelled();
    let mut sequence = OwnedSequence::create(origin, ty, &control, &mut |_| Ok(())).unwrap();
    let mut identities = Vec::new();
    for scalar in scalars {
        let cell = OwnedI64Cell::new(origin, *scalar);
        identities.push(cell.allocation_identity());
        sequence = sequence
            .push(
                origin,
                NormalizedValue::OwnedI64Cell(cell),
                &control,
                &mut |_| Ok(()),
            )
            .unwrap();
    }
    (NormalizedValue::OwnedSequence(sequence), identities)
}

#[test]
fn sequence_transfer_preserves_empty_affinity_and_every_item_allocation() {
    let (program, function, ty) = sequence_fixture();
    let control = ExecutionControl::uncancelled();
    for scalars in [&[][..], &[2, 3, 7][..]] {
        let cells = Cells::start();
        let products = Products::start();
        let parent = ValueOrigin::fresh().unwrap();
        let child = ValueOrigin::fresh().unwrap();
        let (value, identities) = sequence_payload(parent, ty, scalars);
        let inert = value.clone();
        let input = TransferArguments::seal(
            &program,
            parent,
            child,
            function,
            vec![value],
            &control,
            &mut |_, _| panic!("sequence elements are owners"),
        )
        .unwrap();
        let (_, mut values) = input.adopt(&program, child, &control).unwrap();
        let value = values.pop().unwrap();
        value.memory_validate(child, true).unwrap();
        assert!(value.memory_validate(parent, true).is_err());
        let output = TransferResult::seal(
            &program,
            child,
            parent,
            function,
            ty,
            value,
            &control,
            &mut |_, _| panic!("sequence elements are owners"),
        )
        .unwrap();
        let value = output
            .adopt(&program, parent, function, ty, &control)
            .unwrap();
        assert!(inert.memory_validate(parent, false).is_err());
        assert!(inert.memory_validate(child, false).is_err());
        let NormalizedValue::OwnedSequence(sequence) = &value else {
            panic!("sequence");
        };
        assert_eq!(sequence.len(parent, &control).unwrap(), scalars.len());
        sequence
            .inspect_transfer(parent, |values| {
                for ((value, scalar), identity) in values.iter().zip(scalars).zip(&identities) {
                    let NormalizedValue::OwnedI64Cell(cell) = value else {
                        panic!("cell");
                    };
                    cell.validate(parent, true).unwrap();
                    assert_eq!(cell.read().unwrap(), *scalar);
                    assert_eq!(cell.allocation_identity(), *identity);
                }
                Ok(())
            })
            .unwrap();
        super::super::super::value::release_raw_value(value);
        drop(inert);
        assert_eq!(cells.created(), scalars.len());
        assert_eq!(products.created(), 1);
        assert_eq!((cells.live(), products.live()), ((0, 0), (0, 0)));
    }
}

#[test]
fn cancelled_sequence_transfer_reclaims_items_with_intermediate_origins() {
    let (program, function, ty) = sequence_fixture();
    let control = ExecutionControl::uncancelled();
    let mut failed = 0;
    let mut completed = 0;
    for checks in 0..10 {
        let cells = Cells::start();
        let products = Products::start();
        let source = ValueOrigin::fresh().unwrap();
        let destination = ValueOrigin::fresh().unwrap();
        let unrelated = OwnedI64Cell::new(ValueOrigin::fresh().unwrap(), 911);
        let (value, _) = sequence_payload(source, ty, &[2, 3]);
        let output = TransferResult::seal(
            &program,
            source,
            destination,
            function,
            ty,
            value,
            &control,
            &mut |_, _| panic!("sequence elements are owners"),
        )
        .unwrap();
        let cancelled = ExecutionControl::cancel_after_checks(checks);
        match output.adopt(&program, destination, function, ty, &cancelled) {
            Ok(value) => {
                completed += 1;
                super::super::super::value::release_raw_value(value);
            }
            Err(error) => {
                failed += 1;
                assert_eq!(error.class, ExecutionFailureClass::Cancelled);
            }
        }
        // Check four interrupts the second item after the first item has been
        // retagged, while the sequence still belongs to the source invocation.
        if checks == 3 {
            assert!(cancelled.is_cancelled());
        }
        assert_eq!((cells.live(), products.live()), ((1, 0), (0, 0)));
        assert_eq!(unrelated.extract().unwrap(), 911);
    }
    assert!(failed > 0 && completed > 0);
}

#[test]
fn sequence_transfer_rejects_borrowed_envelopes_and_inert_clones() {
    let (program, function, ty) = sequence_fixture();
    let control = ExecutionControl::uncancelled();
    for clone in [false, true] {
        let cells = Cells::start();
        let products = Products::start();
        let source = ValueOrigin::fresh().unwrap();
        let (value, _) = sequence_payload(source, ty, &[7]);
        let NormalizedValue::OwnedSequence(sequence) = &value else {
            panic!("sequence");
        };
        let loan = (!clone).then(|| sequence.borrow().unwrap());
        let (value, retained) = if clone {
            (value.clone(), Some(value))
        } else {
            (value, None)
        };
        assert!(
            TransferResult::seal(
                &program,
                source,
                ValueOrigin::fresh().unwrap(),
                function,
                ty,
                value,
                &control,
                &mut |_, _| panic!("sequence elements are owners"),
            )
            .is_err()
        );
        if let Some(value) = retained {
            value.memory_validate(source, true).unwrap();
            super::super::super::value::release_raw_value(value);
        }
        if let Some(loan) = &loan {
            assert!(loan.len(source, &control).is_err());
        }
        drop(loan);
        assert_eq!((cells.live(), products.live()), ((0, 0), (0, 0)));
    }
}

#[test]
fn sealed_transfer_adopts_every_nested_owner_without_copying_storage_or_metadata() {
    let buffers = Buffers::start();
    let cells = Cells::start();
    let products = Products::start();
    let (program, functions) = fixture();
    let source = ValueOrigin::fresh().unwrap();
    let destination = ValueOrigin::fresh().unwrap();
    let function = functions["tree"];
    let (value, identities, _) = payload(&program, function, source, source, false, false);
    let inert = value.clone();
    let control = ExecutionControl::uncancelled();
    let envelope = TransferArguments::seal(
        &program,
        source,
        destination,
        function,
        vec![value],
        &control,
        &mut |value, ty| ordinary(&program, value, ty),
    )
    .unwrap();
    let queued = &envelope.values.as_ref().unwrap()[0];
    queued.memory_validate(source, true).unwrap();
    assert!(queued.memory_validate(destination, true).is_err());
    let argument_storage = envelope.values.as_ref().unwrap().as_ptr() as usize;
    let (selected, mut values) = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let buffers = Buffers::start();
                let cells = Cells::start();
                let products = Products::start();
                let adopted = envelope.adopt(&program, destination, &control);
                assert_eq!(
                    (buffers.created(), cells.created(), products.created()),
                    (0, 0, 0)
                );
                adopted
            })
            .join()
            .expect("adopting child must be joined")
    })
    .unwrap();
    assert_eq!(selected, function);
    assert_eq!(values.as_ptr() as usize, argument_storage);
    let value = values.pop().unwrap();
    value.memory_validate(destination, true).unwrap();
    assert!(value.memory_validate(source, true).is_err());
    assert!(inert.memory_validate(source, false).is_err());
    assert!(inert.memory_validate(destination, false).is_err());
    let NormalizedValue::OwnedChoice(choice) = value else {
        panic!("choice");
    };
    let choice_type = choice.ty();
    let (case, product) = choice.select(destination, choice_type, &control).unwrap();
    assert_eq!(case, 1);
    let NormalizedValue::OwnedProduct(product) = product else {
        panic!("product");
    };
    let product_type = product.ty();
    let fields = product.unpack(destination, product_type, &control).unwrap();
    let mut fields = fields.into_iter();
    let NormalizedValue::ByteBuffer(buffer) = fields.next().unwrap() else {
        panic!("buffer");
    };
    let NormalizedValue::OwnedI64Cell(cell) = fields.next().unwrap() else {
        panic!("cell");
    };
    let NormalizedValue::Text(label) = fields.next().unwrap() else {
        panic!("label");
    };
    assert_eq!(
        [
            buffer.allocation_identity(),
            cell.allocation_identity(),
            label.as_ptr() as usize
        ],
        identities
    );
    assert!(buffer.validate(source, true).is_err());
    assert!(cell.validate(source, true).is_err());
    buffer.validate(destination, true).unwrap();
    cell.validate(destination, true).unwrap();
    assert_eq!(&*buffer.freeze().unwrap(), &[197]);
    assert_eq!(cell.extract().unwrap(), -137);
    assert_eq!(
        (buffers.created(), cells.created(), products.created()),
        (1, 1, 2)
    );
    assert_eq!(
        (buffers.live(), cells.live(), products.live()),
        ((0, 0), (0, 0), (0, 0))
    );
}

#[test]
fn sealed_transfer_rejects_nested_foreign_loan_and_wrong_metadata_before_adoption() {
    let (program, functions) = fixture();
    for failure in ["foreign", "loan", "metadata"] {
        let buffers = Buffers::start();
        let cells = Cells::start();
        let products = Products::start();
        let source = ValueOrigin::fresh().unwrap();
        let leaf_source = if failure == "foreign" {
            ValueOrigin::fresh().unwrap()
        } else {
            source
        };
        let (value, _, loan) = payload(
            &program,
            functions["tree"],
            source,
            leaf_source,
            failure == "loan",
            failure == "metadata",
        );
        let result = TransferArguments::seal(
            &program,
            source,
            ValueOrigin::fresh().unwrap(),
            functions["tree"],
            vec![value],
            &ExecutionControl::uncancelled(),
            &mut |value, ty| ordinary(&program, value, ty),
        );
        assert!(result.is_err(), "{failure}");
        if let Some(loan) = &loan {
            assert!(loan.get(0).is_err());
        }
        drop(loan);
        assert_eq!(
            (buffers.live(), cells.live(), products.live()),
            ((0, 0), (0, 0), (0, 0)),
            "{failure}"
        );
    }
}

#[test]
fn sealed_transfer_never_promotes_raw_clones_or_hidden_callable_types() {
    let (program, functions) = fixture();
    let buffers = Buffers::start();
    let source = ValueOrigin::fresh().unwrap();
    let buffer = ByteBuffer::empty(source);
    let result = TransferArguments::seal(
        &program,
        source,
        ValueOrigin::fresh().unwrap(),
        functions["buffer"],
        vec![NormalizedValue::ByteBuffer(buffer.clone())],
        &ExecutionControl::uncancelled(),
        &mut |_, _| panic!("owned token"),
    );
    assert!(result.is_err());
    buffer.validate(source, true).unwrap();
    drop(buffer);
    assert_eq!(buffers.live(), (0, 0));
    let result = TransferArguments::seal(
        &program,
        source,
        ValueOrigin::fresh().unwrap(),
        functions["hidden-callback"],
        vec![NormalizedValue::Option(None)],
        &ExecutionControl::uncancelled(),
        &mut |_, _| panic!("forbidden type precedes values"),
    );
    assert!(result.is_err());
}

#[test]
fn sealed_transfer_requires_the_exact_ordinary_prefix_and_owned_suffix() {
    let (program, functions) = fixture();
    let buffers = Buffers::start();
    let source = ValueOrigin::fresh().unwrap();
    let destination = ValueOrigin::fresh().unwrap();
    let function = functions["ordered"];
    let accepted = TransferArguments::seal(
        &program,
        source,
        destination,
        function,
        vec![
            NormalizedValue::Text(Arc::from("metadata")),
            NormalizedValue::ByteBuffer(ByteBuffer::empty(source)),
        ],
        &ExecutionControl::uncancelled(),
        &mut |value, ty| ordinary(&program, value, ty),
    )
    .unwrap();
    drop(accepted);
    assert_eq!(buffers.live(), (0, 0));

    let mut reordered = (*program).clone();
    let selected = &mut Arc::make_mut(&mut reordered.functions)[function.0 as usize];
    Arc::make_mut(&mut selected.parameters).swap(0, 1);
    let rejected = TransferArguments::seal(
        &reordered,
        source,
        destination,
        function,
        vec![
            NormalizedValue::ByteBuffer(ByteBuffer::empty(source)),
            NormalizedValue::Text(Arc::from("metadata")),
        ],
        &ExecutionControl::uncancelled(),
        &mut |_, _| panic!("invalid parameter order must precede ordinary admission"),
    );
    assert!(rejected.is_err());
    assert_eq!(buffers.live(), (0, 0));
}

#[test]
fn sealed_transfer_checks_owned_choice_cases_that_the_value_does_not_select() {
    let (program, functions) = fixture();
    let buffers = Buffers::start();
    let cells = Cells::start();
    let products = Products::start();
    let source = ValueOrigin::fresh().unwrap();
    let function = functions["tree"];
    let (value, _, _) = payload(&program, function, source, source, false, false);
    let mut malformed = (*program).clone();
    let ty = malformed.functions[function.0 as usize].parameters[0].ty;
    let forbidden = malformed.functions[functions["hidden-callback"].0 as usize].parameters[0].ty;
    let TypeForm::OwnedChoice { cases } = &mut malformed.types.get_mut(&ty).unwrap().form else {
        panic!("choice");
    };
    // The actual packet case is unchanged. The unselected case is enough to
    // invalidate the target's complete transfer contract before payload admission.
    cases[0].ty = forbidden;
    assert!(
        TransferArguments::seal(
            &malformed,
            source,
            ValueOrigin::fresh().unwrap(),
            function,
            vec![value],
            &ExecutionControl::uncancelled(),
            &mut |_, _| panic!("forbidden case must precede ordinary admission"),
        )
        .is_err()
    );
    assert_eq!(
        (buffers.live(), cells.live(), products.live()),
        ((0, 0), (0, 0), (0, 0))
    );
}

#[test]
fn sealed_transfer_cancellation_at_each_walk_phase_cleans_all_owners_and_preserves_another_task() {
    let (program, functions) = fixture();
    let control = ExecutionControl::uncancelled();
    for preparing in [true, false] {
        let mut failed = 0;
        let mut completed = 0;
        for checks in 0..24 {
            let buffers = Buffers::start();
            let cells = Cells::start();
            let products = Products::start();
            let source = ValueOrigin::fresh().unwrap();
            let destination = ValueOrigin::fresh().unwrap();
            let unrelated = OwnedI64Cell::new(ValueOrigin::fresh().unwrap(), 911);
            let (value, _, _) = payload(&program, functions["tree"], source, source, false, false);
            let cancelled = ExecutionControl::cancel_after_checks(checks);
            let envelope = TransferArguments::seal(
                &program,
                source,
                destination,
                functions["tree"],
                vec![value],
                if preparing { &cancelled } else { &control },
                &mut |value, ty| ordinary(&program, value, ty),
            );
            match envelope.and_then(|envelope| {
                envelope.adopt(
                    &program,
                    destination,
                    if preparing { &control } else { &cancelled },
                )
            }) {
                Ok((_, values)) => {
                    completed += 1;
                    release_raw_values(values);
                }
                Err(error) => {
                    failed += 1;
                    assert_eq!(error.class, ExecutionFailureClass::Cancelled);
                }
            }
            assert_eq!(
                (buffers.live(), cells.live(), products.live()),
                ((0, 0), (1, 0), (0, 0))
            );
            assert_eq!(unrelated.extract().unwrap(), 911);
        }
        assert!(failed > 0 && completed > 0, "preparing={preparing}");
    }
}

#[test]
fn sealed_transfer_destination_mismatch_disposes_instead_of_returning_a_foreign_owner() {
    let (program, functions) = fixture();
    let buffers = Buffers::start();
    let source = ValueOrigin::fresh().unwrap();
    let destination = ValueOrigin::fresh().unwrap();
    let buffer = ByteBuffer::empty(source);
    let envelope = TransferArguments::seal(
        &program,
        source,
        destination,
        functions["buffer"],
        vec![NormalizedValue::ByteBuffer(buffer)],
        &ExecutionControl::uncancelled(),
        &mut |_, _| panic!("owned token"),
    )
    .unwrap();
    assert!(
        envelope
            .adopt(
                &program,
                ValueOrigin::fresh().unwrap(),
                &ExecutionControl::uncancelled()
            )
            .is_err()
    );
    assert_eq!(buffers.live(), (0, 0));
}

#[test]
fn sealed_result_returns_nested_custody_without_copying_allocations() {
    let (program, functions) = fixture();
    let buffers = Buffers::start();
    let cells = Cells::start();
    let products = Products::start();
    let parent = ValueOrigin::fresh().unwrap();
    let child = ValueOrigin::fresh().unwrap();
    let function = functions["tree-result"];
    let ty = program.functions[function.0 as usize].result;
    let control = ExecutionControl::uncancelled();
    let (value, identities, _) = payload(&program, function, parent, parent, false, false);
    let input = TransferArguments::seal(
        &program,
        parent,
        child,
        function,
        vec![value],
        &control,
        &mut |value, ty| ordinary(&program, value, ty),
    )
    .unwrap();
    let (_, mut values) = input.adopt(&program, child, &control).unwrap();
    let value = values.pop().unwrap();
    let inert = value.clone();
    let output = TransferResult::seal(
        &program,
        child,
        parent,
        function,
        ty,
        value,
        &control,
        &mut |value, ty| ordinary(&program, value, ty),
    )
    .unwrap();
    let value = output
        .adopt(&program, parent, function, ty, &control)
        .unwrap();
    value.memory_validate(parent, true).unwrap();
    assert!(value.memory_validate(child, true).is_err());
    assert!(inert.memory_validate(parent, false).is_err());
    assert!(inert.memory_validate(child, false).is_err());
    let NormalizedValue::OwnedChoice(choice) = value else {
        panic!("choice")
    };
    let (_, product) = choice.select(parent, ty, &control).unwrap();
    let NormalizedValue::OwnedProduct(product) = product else {
        panic!("product")
    };
    let product_type = product.ty();
    let fields = product.unpack(parent, product_type, &control).unwrap();
    let mut fields = fields.into_iter();
    let NormalizedValue::ByteBuffer(buffer) = fields.next().unwrap() else {
        panic!("buffer")
    };
    let NormalizedValue::OwnedI64Cell(cell) = fields.next().unwrap() else {
        panic!("cell")
    };
    let NormalizedValue::Text(label) = fields.next().unwrap() else {
        panic!("metadata")
    };
    assert_eq!(
        [
            buffer.allocation_identity(),
            cell.allocation_identity(),
            label.as_ptr() as usize,
        ],
        identities
    );
    assert_eq!(&*buffer.freeze().unwrap(), &[197]);
    assert_eq!(cell.extract().unwrap(), -137);
    assert_eq!(
        (buffers.created(), cells.created(), products.created()),
        (1, 1, 2)
    );
    assert_eq!(
        (buffers.live(), cells.live(), products.live()),
        ((0, 0), (0, 0), (0, 0))
    );
}

#[test]
fn sealed_result_rejects_foreign_domains_loans_metadata_and_inert_clones() {
    let (program, functions) = fixture();
    let function = functions["tree-result"];
    let ty = program.functions[function.0 as usize].result;
    for failure in ["foreign", "loan", "metadata", "clone"] {
        let buffers = Buffers::start();
        let cells = Cells::start();
        let products = Products::start();
        let source = ValueOrigin::fresh().unwrap();
        let leaf = if failure == "foreign" {
            ValueOrigin::fresh().unwrap()
        } else {
            source
        };
        let (value, _, loan) = payload(
            &program,
            function,
            source,
            leaf,
            failure == "loan",
            failure == "metadata",
        );
        let (value, retained) = if failure == "clone" {
            (value.clone(), Some(value))
        } else {
            (value, None)
        };
        let sealed = TransferResult::seal(
            &program,
            source,
            ValueOrigin::fresh().unwrap(),
            function,
            ty,
            value,
            &ExecutionControl::uncancelled(),
            &mut |value, ty| ordinary(&program, value, ty),
        );
        assert!(sealed.is_err(), "{failure}");
        if let Some(retained) = retained {
            retained.memory_validate(source, true).unwrap();
            drop(retained);
        }
        if let Some(loan) = &loan {
            assert!(loan.get(0).is_err());
        }
        drop(loan);
        assert_eq!(
            (buffers.live(), cells.live(), products.live()),
            ((0, 0), (0, 0), (0, 0)),
            "{failure}"
        );
    }
}

#[test]
fn sealed_result_binds_program_target_result_type_and_destination() {
    let (program, functions) = fixture();
    let function = functions["buffer-result"];
    let ty = program.functions[function.0 as usize].result;
    let control = ExecutionControl::uncancelled();
    for mismatch in ["program", "target", "type", "destination", "source"] {
        let buffers = Buffers::start();
        let child = ValueOrigin::fresh().unwrap();
        let parent = ValueOrigin::fresh().unwrap();
        let raw = NormalizedValue::ByteBuffer(ByteBuffer::empty(child));
        if mismatch == "source" {
            assert!(
                TransferResult::seal(
                    &program,
                    ValueOrigin::fresh().unwrap(),
                    parent,
                    function,
                    ty,
                    raw,
                    &control,
                    &mut |_, _| panic!("owned result"),
                )
                .is_err()
            );
        } else {
            let output = TransferResult::seal(
                &program,
                child,
                parent,
                function,
                ty,
                raw,
                &control,
                &mut |_, _| panic!("owned result"),
            )
            .unwrap();
            let mut foreign = (*program).clone();
            foreign.value_origin = ValueOrigin::fresh().unwrap();
            let actual_program: &NormalizedProgram = if mismatch == "program" {
                &foreign
            } else {
                &program
            };
            let actual_function = if mismatch == "target" {
                functions["buffer"]
            } else {
                function
            };
            let actual_type = if mismatch == "type" {
                program.functions[functions["buffer"].0 as usize].result
            } else {
                ty
            };
            let actual_parent = if mismatch == "destination" {
                ValueOrigin::fresh().unwrap()
            } else {
                parent
            };
            assert!(
                output
                    .adopt(
                        actual_program,
                        actual_parent,
                        actual_function,
                        actual_type,
                        &control
                    )
                    .is_err()
            );
        }
        assert_eq!(buffers.live(), (0, 0), "{mismatch}");
    }
    let buffers = Buffers::start();
    let child = ValueOrigin::fresh().unwrap();
    assert!(
        TransferResult::seal(
            &program,
            child,
            ValueOrigin::fresh().unwrap(),
            functions["buffer"],
            ty,
            NormalizedValue::ByteBuffer(ByteBuffer::empty(child)),
            &control,
            &mut |_, _| panic!("wrong result type precedes payload admission"),
        )
        .is_err()
    );
    assert_eq!(buffers.live(), (0, 0));
}

#[test]
fn sealed_result_checks_unselected_cases_before_accepting_custody() {
    let (program, functions) = fixture();
    let function = functions["tree-result"];
    let ty = program.functions[function.0 as usize].result;
    let buffers = Buffers::start();
    let cells = Cells::start();
    let products = Products::start();
    let source = ValueOrigin::fresh().unwrap();
    let (value, _, _) = payload(&program, function, source, source, false, false);
    let mut malformed = (*program).clone();
    let hidden = malformed.functions[functions["hidden-callback"].0 as usize].parameters[0].ty;
    let TypeForm::OwnedChoice { cases } = &mut malformed.types.get_mut(&ty).unwrap().form else {
        panic!("choice")
    };
    cases[0].ty = hidden;
    assert!(
        TransferResult::seal(
            &malformed,
            source,
            ValueOrigin::fresh().unwrap(),
            function,
            ty,
            value,
            &ExecutionControl::uncancelled(),
            &mut |_, _| panic!("unselected type precedes value"),
        )
        .is_err()
    );
    assert_eq!(
        (buffers.live(), cells.live(), products.live()),
        ((0, 0), (0, 0), (0, 0))
    );
}

#[test]
fn sealed_result_cancellation_cleans_partial_adoption_and_preserves_unrelated_owner() {
    let (program, functions) = fixture();
    let function = functions["tree-result"];
    let ty = program.functions[function.0 as usize].result;
    let control = ExecutionControl::uncancelled();
    for preparing in [true, false] {
        let mut failed = 0;
        let mut completed = 0;
        for checks in 0..24 {
            let buffers = Buffers::start();
            let cells = Cells::start();
            let products = Products::start();
            let source = ValueOrigin::fresh().unwrap();
            let destination = ValueOrigin::fresh().unwrap();
            let unrelated = OwnedI64Cell::new(ValueOrigin::fresh().unwrap(), 911);
            let (value, _, _) = payload(&program, function, source, source, false, false);
            let cancelled = ExecutionControl::cancel_after_checks(checks);
            let output = TransferResult::seal(
                &program,
                source,
                destination,
                function,
                ty,
                value,
                if preparing { &cancelled } else { &control },
                &mut |value, ty| ordinary(&program, value, ty),
            );
            let adopted = output.and_then(|output| {
                output.adopt(
                    &program,
                    destination,
                    function,
                    ty,
                    if preparing { &control } else { &cancelled },
                )
            });
            match adopted {
                Ok(value) => {
                    completed += 1;
                    super::super::super::value::release_raw_value(value);
                }
                Err(error) => {
                    failed += 1;
                    assert_eq!(error.class, ExecutionFailureClass::Cancelled);
                }
            }
            // Check 4 during adoption has already retagged the buffer; the next
            // leaf check cancels. Mixed intermediate domains cannot escape custody.
            if !preparing && checks == 4 {
                assert!(cancelled.is_cancelled());
            }
            assert_eq!(
                (buffers.live(), cells.live(), products.live()),
                ((0, 0), (1, 0), (0, 0))
            );
            assert_eq!(unrelated.extract().unwrap(), 911);
        }
        assert!(failed > 0 && completed > 0, "preparing={preparing}");
    }
}

#[test]
fn joined_failure_disposes_a_successful_owned_result_envelope() {
    let (program, functions) = fixture();
    let function = functions["buffer-result"];
    let ty = program.functions[function.0 as usize].result;
    for successful_left in [true, false] {
        let buffers = Buffers::start();
        let source = ValueOrigin::fresh().unwrap();
        let output = TransferResult::seal(
            &program,
            source,
            ValueOrigin::fresh().unwrap(),
            function,
            ty,
            NormalizedValue::ByteBuffer(ByteBuffer::empty(source)),
            &ExecutionControl::uncancelled(),
            &mut |_, _| panic!("owned result"),
        )
        .unwrap();
        let error = ExecutionError::resource("test_sibling_failure", "original failure");
        let joined = if successful_left {
            super::super::super::parallel::results::<_, TransferResult>(
                Ok(output),
                Err(error.clone()),
            )
        } else {
            super::super::super::parallel::results::<TransferResult, _>(
                Err(error.clone()),
                Ok(output),
            )
        };
        assert_eq!(joined.err().unwrap(), error);
        assert_eq!(buffers.live(), (0, 0));
    }
}

#[path = "vm_transfer_generic_tests.rs"]
mod generic;
