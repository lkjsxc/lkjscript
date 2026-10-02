//! Storage-level custody only: this deliberately does not admit graph task handoff.
use super::{
    byte_buffer::{ByteBuffer, StorageObservation as Buffers},
    owned_choice::OwnedChoice,
    owned_i64_cell::{OwnedI64Cell, StorageObservation as Cells},
    owned_product::{OwnedProduct, StorageObservation as Products},
    value::{NormalizedValue, ValueOrigin},
};
use crate::platform::execution::{ExecutionControl, mailbox};
use crate::platform::kernel::TypeObjectDigest;
use std::future::Future;
use std::task::{Context, Waker};

fn payload(origin: ValueOrigin) -> (NormalizedValue, usize, usize) {
    let control = ExecutionControl::uncancelled();
    let buffer = ByteBuffer::empty(origin)
        .push(197, &control, &mut |_| Ok(()))
        .unwrap();
    let cell = OwnedI64Cell::new(origin, -137);
    let identities = (buffer.allocation_identity(), cell.allocation_identity());
    // Test-only exact identities, not a published graph or a semantic certificate.
    let product = OwnedProduct::create(
        origin,
        TypeObjectDigest::from_bytes([61; 32]),
        vec![
            NormalizedValue::ByteBuffer(buffer),
            NormalizedValue::OwnedI64Cell(cell),
        ],
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    let choice = OwnedChoice::create(
        origin,
        TypeObjectDigest::from_bytes([62; 32]),
        1,
        NormalizedValue::OwnedProduct(product),
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    (
        NormalizedValue::OwnedChoice(choice),
        identities.0,
        identities.1,
    )
}

#[tokio::test]
async fn owned_mailbox_preserves_nested_allocations_without_granting_foreign_origin_admission() {
    let buffers = Buffers::start();
    let cells = Cells::start();
    let products = Products::start();
    let origin = ValueOrigin::fresh().unwrap();
    let foreign = ValueOrigin::fresh().unwrap();
    let (value, buffer_id, cell_id) = payload(origin);
    let inert = value.clone();
    let (tx, mut rx) = mailbox::channel(1).unwrap();
    tx.send(value).await.unwrap();
    let received = rx.recv().await.unwrap();
    received.memory_validate(origin, true).unwrap();
    assert!(received.memory_validate(foreign, true).is_err());
    assert!(inert.memory_validate(origin, false).is_err());
    let NormalizedValue::OwnedChoice(choice) = received else {
        panic!("owned choice");
    };
    let (case, product) = choice
        .select(
            origin,
            TypeObjectDigest::from_bytes([62; 32]),
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
    assert_eq!(case, 1);
    let NormalizedValue::OwnedProduct(product) = product else {
        panic!("owned product");
    };
    let mut fields = product
        .unpack(
            origin,
            TypeObjectDigest::from_bytes([61; 32]),
            &ExecutionControl::uncancelled(),
        )
        .unwrap()
        .into_iter();
    let NormalizedValue::ByteBuffer(buffer) = fields.next().unwrap() else {
        panic!("buffer");
    };
    let NormalizedValue::OwnedI64Cell(cell) = fields.next().unwrap() else {
        panic!("cell");
    };
    assert_eq!(buffer.allocation_identity(), buffer_id);
    assert_eq!(cell.allocation_identity(), cell_id);
    assert_eq!(&*buffer.freeze().unwrap(), &[197]);
    assert_eq!(cell.extract().unwrap(), -137);
    assert_eq!(buffers.created(), 1);
    assert_eq!(cells.created(), 1);
    assert_eq!(products.created(), 2);
    assert_eq!(buffers.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
    assert_eq!(products.live(), (0, 0));
}

#[tokio::test]
async fn owned_mailbox_cancellation_and_receiver_failure_clean_nested_storage_without_touching_another_owner()
 {
    let buffers = Buffers::start();
    let cells = Cells::start();
    let products = Products::start();
    let origin = ValueOrigin::fresh().unwrap();
    let unrelated = OwnedI64Cell::new(ValueOrigin::fresh().unwrap(), 911);
    let (tx, mut rx) = mailbox::channel(1).unwrap();
    let permit = tx.try_reserve().unwrap();
    let (value, _, _) = payload(origin);
    let mut pending = Box::pin(tx.send(value));
    assert!(
        pending
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    drop(pending);
    assert_eq!(buffers.live(), (0, 0));
    assert_eq!(cells.live(), (1, 0));
    assert_eq!(products.live(), (0, 0));
    drop(permit);

    let (value, _, _) = payload(origin);
    tx.send(value).await.unwrap();
    let active = rx.recv().await.unwrap();
    let (value, _, _) = payload(origin);
    tx.send(value).await.unwrap();
    rx.close(); // Queue-owned nested values clean up; the active receiver still owns its value.
    assert_eq!(buffers.live(), (1, 0));
    assert_eq!(cells.live(), (2, 0));
    assert_eq!(products.live(), (2, 0));
    drop(active); // Models the already-dequeued consumer's failure after its work is joined.
    assert_eq!(buffers.live(), (0, 0));
    assert_eq!(cells.live(), (1, 0));
    assert_eq!(products.live(), (0, 0));
    assert_eq!(unrelated.extract().unwrap(), 911);
    assert_eq!(cells.live(), (0, 0));
}

#[tokio::test]
async fn owned_mailbox_transport_does_not_disable_existing_loan_or_raw_clone_guards() {
    let buffers = Buffers::start();
    let origin = ValueOrigin::fresh().unwrap();
    let owner = ByteBuffer::empty(origin);
    let loan = owner.borrow().unwrap();
    let inert = owner.clone();
    let (tx, mut rx) = mailbox::channel(1).unwrap();
    // Internal transport is not language admission. A sealed custodian is still
    // required before this can be exposed as a cross-task graph operation.
    tx.send(owner).await.unwrap();
    let owner = rx.recv().await.unwrap();
    assert!(owner.validate(origin, true).is_err());
    assert!(loan.validate(origin, true).is_err());
    assert!(inert.validate(origin, false).is_err());
    drop(loan);
    owner.validate(origin, true).unwrap();
    drop(owner);
    assert_eq!(buffers.live(), (0, 0));
}
