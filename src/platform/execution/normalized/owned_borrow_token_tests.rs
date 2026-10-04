//! Sealed projection tokens and whole-parent custody, independently of syntax.
use super::super::{
    byte_buffer::ByteBuffer,
    owned_choice::OwnedChoice,
    owned_i64_cell::OwnedI64Cell,
    owned_product::{OwnedProduct, StorageObservation},
    value::ValueOrigin,
};
use super::*;

#[test]
fn owned_field_projection_retains_parent_and_child_allocation_identity() {
    let products = StorageObservation::start();
    let buffers = super::super::byte_buffer::StorageObservation::start();
    let origin = ValueOrigin::fresh().unwrap();
    let control = ExecutionControl::uncancelled();
    let ty = TypeObjectDigest::from_bytes([103; 32]);
    let buffer = ByteBuffer::create(origin, &control, &mut |_| Ok(()))
        .unwrap()
        .push(211, &control, &mut |_| Ok(()))
        .unwrap();
    let identity = buffer.allocation_identity();
    let owner = OwnedProduct::create(
        origin,
        ty,
        vec![
            NormalizedValue::ByteBuffer(buffer),
            NormalizedValue::I64(-41),
        ],
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    assert!(
        owner
            .borrow_field(origin, 0, &control, &mut |_| Ok(()))
            .is_err()
    );
    let parent = owner.borrow().unwrap();
    assert!(
        parent
            .clone()
            .borrow_field(origin, 0, &control, &mut |_| Ok(()))
            .is_err()
    );
    assert!(
        parent
            .borrow_field(ValueOrigin::fresh().unwrap(), 0, &control, &mut |_| Ok(()))
            .is_err()
    );
    assert!(
        parent
            .borrow_field(origin, 2, &control, &mut |_| Ok(()))
            .is_err()
    );
    let NormalizedValue::ByteBuffer(child) = parent
        .borrow_field(origin, 0, &control, &mut |_| Ok(()))
        .unwrap()
    else {
        panic!("buffer read token")
    };
    assert_eq!(child.allocation_identity(), identity);
    assert_eq!(child.get(0).unwrap(), 211);
    assert!(child.validate(origin, true).is_err());
    assert!(owner.validate(origin, true).is_err());
    // Reading a sibling after projection proves no product mutex is retained.
    assert_eq!(
        parent
            .read_metadata(origin, 1, &control, &mut |_| Ok(()))
            .unwrap(),
        NormalizedValue::I64(-41)
    );
    let reborrow = child.borrow().unwrap();
    drop(reborrow);
    drop(child);
    drop(parent);
    let mut fields = owner.unpack(origin, ty, &control).unwrap();
    let NormalizedValue::ByteBuffer(child) = fields.remove(0) else {
        panic!("same owner")
    };
    assert_eq!(child.allocation_identity(), identity);
    assert_eq!(&*child.freeze().unwrap(), &[211]);
    products.assert_transfers_preserve_allocations();
    products.assert_owners_released_after_loans();
    assert_eq!(products.live(), (0, 0));
    assert_eq!(buffers.live(), (0, 0));
}

#[test]
fn nested_projection_and_sibling_reads_finish_without_retaining_storage_locks() {
    let (sent, received) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        let origin = ValueOrigin::fresh().unwrap();
        let control = ExecutionControl::uncancelled();
        let outer_ty = TypeObjectDigest::from_bytes([104; 32]);
        let inner_ty = TypeObjectDigest::from_bytes([105; 32]);
        let inner = OwnedProduct::create(
            origin,
            inner_ty,
            vec![
                NormalizedValue::OwnedI64Cell(OwnedI64Cell::new(origin, 137)),
                NormalizedValue::I64(19),
            ],
            &control,
            &mut |_| Ok(()),
        )
        .unwrap();
        let outer = OwnedProduct::create(
            origin,
            outer_ty,
            vec![
                NormalizedValue::OwnedProduct(inner),
                NormalizedValue::I64(23),
            ],
            &control,
            &mut |_| Ok(()),
        )
        .unwrap();
        let outer_read = outer.borrow().unwrap();
        let NormalizedValue::OwnedProduct(inner_read) = outer_read
            .borrow_field(origin, 0, &control, &mut |_| Ok(()))
            .unwrap()
        else {
            panic!("inner view")
        };
        let inner_reborrow = inner_read.borrow().unwrap();
        let NormalizedValue::OwnedI64Cell(cell_read) = inner_reborrow
            .borrow_field(origin, 0, &control, &mut |_| Ok(()))
            .unwrap()
        else {
            panic!("cell view")
        };
        assert_eq!(cell_read.read().unwrap(), 137);
        assert_eq!(
            inner_read
                .read_metadata(origin, 1, &control, &mut |_| Ok(()))
                .unwrap(),
            NormalizedValue::I64(19)
        );
        assert_eq!(
            outer_read
                .read_metadata(origin, 1, &control, &mut |_| Ok(()))
                .unwrap(),
            NormalizedValue::I64(23)
        );
        assert!(outer.validate(origin, true).is_err());
        drop(cell_read);
        drop(inner_reborrow);
        drop(inner_read);
        drop(outer_read);
        assert!(outer.validate(origin, true).is_ok());
        sent.send(()).unwrap();
    });
    received
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("nested projections and sibling reads must complete without a retained mutex");
    worker.join().unwrap();
}

#[test]
fn borrowed_choice_copies_ordinary_payload_and_cleans_projection_refusal() {
    let products = StorageObservation::start();
    let origin = ValueOrigin::fresh().unwrap();
    let control = ExecutionControl::uncancelled();
    let ty = TypeObjectDigest::from_bytes([106; 32]);
    let owner = OwnedChoice::create(
        origin,
        ty,
        1,
        NormalizedValue::Option(Some(Box::new(NormalizedValue::I64(73)))),
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    let parent = owner.borrow().unwrap();
    assert!(
        parent
            .borrow_payload(origin, &control, &mut |_| Err(ExecutionError::resource(
                "test_quota",
                "refused"
            )))
            .is_err()
    );
    let child = parent
        .borrow_payload(origin, &control, &mut |_| Ok(()))
        .unwrap();
    assert_eq!(
        child,
        NormalizedValue::Option(Some(Box::new(NormalizedValue::I64(73))))
    );
    assert!(owner.validate(origin, true).is_err());
    control.cancel();
    assert!(
        parent
            .borrow_payload(origin, &control, &mut |_| Ok(()))
            .is_err()
    );
    drop(parent);
    let (case, payload) = owner
        .select(origin, ty, &ExecutionControl::uncancelled())
        .unwrap();
    assert_eq!((case, payload), (1, child));
    products.assert_owners_released_after_loans();
    assert_eq!(products.live(), (0, 0));
}
