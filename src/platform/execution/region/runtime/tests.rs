use super::*;

fn layout() -> RegionLayout<i64> {
    RegionLayout::new([19; 32], 8, 8).expect("scalar layout")
}

#[test]
fn lexical_slots_reuse_storage_without_reviving_old_pointers_or_tokens() {
    let arena = Arena::new().expect("arena");
    let control = ExecutionControl::uncancelled();
    let cap = arena.open(1, 256, &control, |_| Ok(())).expect("region");
    let pointer = arena
        .allocate(&cap, 1, layout(), &42_i64.to_le_bytes(), &control)
        .expect("allocation");
    let copied = cap.clone();
    assert!(arena.validate(&copied, 1, false).is_err());
    assert!(arena.validate(&cap, 2, false).is_err());
    assert!(arena.borrow(&cap, 1, 2).is_err());
    let loan = arena.borrow(&cap, 1, 1).expect("synchronous unique read");
    assert!(arena.validate(&loan, 1, false).is_ok());
    assert!(arena.validate(&loan, 1, true).is_err());
    let identity = cap.identity();
    arena.close(identity).expect("bulk release");
    assert!(arena.is_empty());
    let next = arena
        .open(1, 256, &control, |_| Ok(()))
        .expect("next region");
    assert_eq!(next.identity.slot, identity.slot);
    assert_ne!(next.identity.generation, identity.generation);
    let next_pointer = arena
        .allocate(&next, 1, layout(), &99_i64.to_le_bytes(), &control)
        .expect("next allocation");
    assert_eq!(pointer.offset(), next_pointer.offset());
    assert!(
        arena
            .read(&next, 1, layout(), pointer, &control, |_| Ok(()))
            .is_err()
    );
    assert!(arena.validate(&cap, 1, true).is_err());
    assert!(arena.close(identity).is_err());
}

#[test]
fn transfer_freeze_and_scoped_workers_keep_the_exact_payload_address() {
    let arena = Arena::new().expect("arena");
    let control = ExecutionControl::uncancelled();
    let cap = arena.open(1, 256, &control, |_| Ok(())).expect("region");
    let pointer = arena
        .allocate(&cap, 1, layout(), &11_i64.to_le_bytes(), &control)
        .expect("allocation");
    let address = arena
        .read(&cap, 1, layout(), pointer, &control, |bytes| {
            Ok(bytes.as_ptr() as usize)
        })
        .expect("address");
    let cap = arena.transfer(cap, 1, 2).expect("unique transfer");
    let cap = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                arena.write(&cap, 2, layout(), pointer, &29_i64.to_le_bytes(), &control)?;
                arena.freeze(cap, 2, &control)
            })
            .join()
            .expect("joined")
            .expect("frozen")
    });
    let left = arena.borrow(&cap, 2, 3).expect("left read");
    let right = arena.borrow(&cap, 2, 4).expect("right read");
    assert!(left.is_borrowed());
    assert_eq!(left.access(), Access::Frozen);
    assert!(arena.validate(&left, 3, true).is_err());
    assert!(
        arena
            .write(&cap, 2, layout(), pointer, &0_i64.to_le_bytes(), &control)
            .is_err()
    );
    let read = |cap: &Capability<i32>, domain| {
        arena
            .read(cap, domain, layout(), pointer, &control, |bytes| {
                assert_eq!(bytes.as_ptr() as usize, address);
                Ok(i64::from_le_bytes(bytes.try_into().expect("i64")))
            })
            .expect("read")
    };
    let sum = std::thread::scope(|scope| {
        let left = scope.spawn(|| read(&left, 3));
        let right = scope.spawn(|| read(&right, 4));
        left.join().expect("left joined") + right.join().expect("right joined")
    });
    assert_eq!(sum, 58);
    arena.close(cap.identity()).expect("release");
    assert!(arena.validate(&left, 3, false).is_err());
    assert!(arena.validate(&right, 4, false).is_err());
}

#[test]
fn loaded_image_has_a_fresh_activation_and_no_node_reconstruction() {
    let arena = Arena::new().expect("arena");
    let control = ExecutionControl::uncancelled();
    let cap = arena.open(1, 256, &control, |_| Ok(())).expect("region");
    let pointer = arena
        .allocate(&cap, 1, layout(), &42_i64.to_le_bytes(), &control)
        .expect("allocation");
    let (image, digest) = arena
        .with_read(&cap, 1, &control, |read| {
            Ok((
                read.image.to_vec(),
                persistence::digest(read.image, &control)?,
            ))
        })
        .expect("save image");
    let loaded = arena
        .open(1, 0, &control, |_| Ok(()))
        .expect("fresh binder");
    let loaded = arena
        .load(loaded, 1, &image, digest, 4096, &control, |_| Ok(()))
        .expect("load");
    assert!(
        arena
            .read(&loaded, 1, layout(), pointer, &control, |_| Ok(()))
            .is_err()
    );
    let loaded_pointer = arena
        .resolve(&loaded, 1, layout(), pointer.offset(), &control)
        .expect("admitted relative offset");
    assert_eq!(
        arena
            .read(&loaded, 1, layout(), loaded_pointer, &control, |bytes| {
                Ok(i64::from_le_bytes(bytes.try_into().expect("i64")))
            })
            .expect("read"),
        42
    );
    arena
        .with_read(&loaded, 1, &control, |read| {
            assert_eq!(read.image, image);
            assert_eq!(read.image.as_ptr() as usize % 64, 0);
            Ok(())
        })
        .expect("exact image");
}

#[test]
fn refused_open_and_cancelled_mutations_leave_existing_custody_usable() {
    let arena = Arena::new().expect("arena");
    let control = ExecutionControl::uncancelled();
    assert!(
        arena
            .open(1, 256, &control, |_| Err(capacity_error()))
            .is_err()
    );
    assert!(arena.is_empty());
    let cap = arena.open(1, 256, &control, |_| Ok(())).expect("region");
    let pointer = arena
        .allocate(&cap, 1, layout(), &11_i64.to_le_bytes(), &control)
        .expect("allocation");
    let cancelled = ExecutionControl::uncancelled();
    cancelled.cancel();
    assert!(
        arena
            .write(
                &cap,
                1,
                layout(),
                pointer,
                &29_i64.to_le_bytes(),
                &cancelled
            )
            .is_err()
    );
    assert_eq!(
        arena
            .read(&cap, 1, layout(), pointer, &control, |bytes| {
                Ok(i64::from_le_bytes(bytes.try_into().expect("i64")))
            })
            .expect("unchanged"),
        11
    );
}

#[test]
fn region_arena_has_no_former_live_slot_ceiling() {
    let arena = Arena::new().unwrap();
    let control = ExecutionControl::uncancelled();
    let mut scopes = Vec::new();
    for _ in 0..65_537 {
        let cap = arena.open(1, 0, &control, |_| Ok(())).unwrap();
        scopes.push(cap.identity());
    }
    assert_eq!(arena.read_state().live, scopes.len());
    for identity in scopes {
        arena.close(identity).unwrap();
    }
    assert!(arena.is_empty());
}

#[test]
fn region_slot_growth_checks_host_layout_without_wrapping_or_clamping() {
    assert_eq!(slot_capacity::<u8>(0).unwrap(), 1);
    assert_eq!(slot_capacity::<u8>(65_536).unwrap(), 131_072);
    let maximum = isize::MAX as usize / std::mem::size_of::<Slot<u8>>();
    assert_eq!(slot_capacity::<u8>(maximum - 1).unwrap(), maximum);
    assert!(slot_capacity::<u8>(maximum).is_err());
    assert!(slot_capacity::<u8>(usize::MAX).is_err());
}
