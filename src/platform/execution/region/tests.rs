use super::*;

fn scalar() -> RegionLayout<i64> {
    RegionLayout::new([1; 32], 8, 8).unwrap()
}

fn number(bytes: &[u8]) -> i64 {
    i64::from_le_bytes(bytes.try_into().unwrap())
}

#[test]
fn aligned_storage_has_four_byte_pointers_one_charge_and_stable_freeze() {
    assert_eq!(std::mem::size_of::<RelPtr<'_, i64>>(), 4);
    assert_eq!(std::mem::size_of::<RegionSlice<'_, i64>>(), 8);
    let control = ExecutionControl::uncancelled();
    let mut charges = Vec::new();
    let result = with_region(
        512,
        &control,
        |bytes| {
            charges.push(bytes);
            Ok(())
        },
        |mut region| {
            let pointer = region.allocate(scalar(), &42_i64.to_le_bytes(), &control)?;
            let address = region.slab.image().as_ptr() as usize;
            assert_eq!(address % 64, 0);
            assert_eq!(region.read_cap().observe().cells, 1);
            let frozen = region.freeze(&control)?;
            assert_eq!(frozen.image().as_ptr() as usize, address);
            Ok(number(frozen.read_cap().read(
                scalar(),
                pointer,
                &control,
            )?))
        },
    )
    .unwrap();
    assert_eq!(result, 42);
    assert_eq!(
        charges,
        [u64::from(payload_start(512).unwrap() + 512 + 63) + std::mem::size_of::<Slab>() as u64]
    );
}

#[test]
fn cyclic_cells_use_relative_offsets_and_no_per_cell_custody() {
    let control = ExecutionControl::uncancelled();
    let layout = RegionLayout::<()>::new([2; 32], 16, 8).unwrap();
    with_region(
        512,
        &control,
        |_| Ok(()),
        |mut region| {
            let a = region.allocate(layout, &[0; 16], &control)?;
            let b = region.allocate(layout, &[0; 16], &control)?;
            let before = region.read_cap().observe();
            let mut left = [0; 16];
            left[..8].copy_from_slice(&11_i64.to_le_bytes());
            left[8..12].copy_from_slice(&b.offset().to_le_bytes());
            let mut right = [0; 16];
            right[..8].copy_from_slice(&29_i64.to_le_bytes());
            right[8..12].copy_from_slice(&a.offset().to_le_bytes());
            region.write(layout, a, &left, &control)?;
            region.write(layout, b, &right, &control)?;
            assert_eq!(region.read_cap().observe(), before);
            let frozen = region.freeze(&control)?;
            let read = frozen.read_cap();
            let mut current = a;
            let mut sum = 0;
            for _ in 0..100 {
                let bytes = read.read(layout, current, &control)?;
                sum += number(&bytes[..8]);
                current = read.resolve(layout, read_u32(bytes, 8)?, &control)?;
            }
            assert_eq!(sum, 2000);
            assert_eq!(current.offset(), a.offset());
            assert_eq!(read.observe().cells, 2);
            Ok(())
        },
    )
    .unwrap();
}

#[test]
fn failed_capacity_layout_pointer_and_cancelled_writes_preserve_the_image() {
    let control = ExecutionControl::uncancelled();
    with_region(
        64,
        &control,
        |_| Ok(()),
        |mut region| {
            let pointer = region.allocate(scalar(), &7_i64.to_le_bytes(), &control)?;
            let before = region.slab.image().to_vec();
            assert_eq!(
                region
                    .allocate(scalar(), &[0; 8], &control)
                    .err()
                    .unwrap()
                    .code,
                "region_capacity"
            );
            assert_eq!(
                region
                    .write(scalar(), pointer, &[0; 7], &control)
                    .unwrap_err()
                    .code,
                "region_layout"
            );
            let wrong = RegionLayout::<i64>::new([3; 32], 8, 8).unwrap();
            assert_eq!(
                region
                    .write(wrong, pointer, &[0; 8], &control)
                    .unwrap_err()
                    .code,
                "region_layout"
            );
            for offset in [
                0,
                pointer.offset() - 8,
                pointer.offset() + 1,
                pointer.offset() + 8,
                u32::MAX,
            ] {
                assert_eq!(
                    region
                        .read_cap()
                        .resolve(scalar(), offset, &control)
                        .err()
                        .unwrap()
                        .code,
                    "region_pointer"
                );
            }
            control.cancel();
            assert_eq!(
                region
                    .write(scalar(), pointer, &[0; 8], &control)
                    .unwrap_err()
                    .class,
                ExecutionFailureClass::Cancelled
            );
            assert_eq!(region.slab.image(), before);
            Ok(())
        },
    )
    .unwrap();
}

#[test]
fn aligned_arrays_empty_arrays_and_zero_size_elements_keep_exact_counts() {
    let control = ExecutionControl::uncancelled();
    with_region(
        2048,
        &control,
        |_| Ok(()),
        |mut region| {
            for alignment in [1, 2, 4, 8, 16, 32, 64] {
                let layout = RegionLayout::<()>::new([alignment as u8; 32], alignment, alignment)?;
                let bytes = vec![alignment as u8; alignment as usize * 3];
                let slice = region.allocate_slice(layout, 3, &bytes, &control)?;
                let read = region.read_cap();
                assert_eq!(slice.len(), 3);
                assert!(!slice.is_empty());
                assert_eq!(
                    (read.image.as_ptr() as usize + slice.offset() as usize) % alignment as usize,
                    0
                );
                assert_eq!(read.read_slice(layout, slice, &control)?, bytes);
                assert_eq!(
                    read.read_item(layout, slice, 2, &control)?,
                    &bytes[..alignment as usize]
                );
                assert!(read.read_item(layout, slice, 3, &control).is_err());
                assert!(
                    read.resolve_slice(layout, slice.offset(), 2, &control)
                        .is_err()
                );
                assert!(read.resolve(layout, slice.offset(), &control).is_err());
            }
            let empty = region.allocate_slice(scalar(), 0, &[], &control)?;
            assert!(empty.is_empty());
            assert_eq!(
                region.read_cap().read_slice(scalar(), empty, &control)?,
                &[] as &[u8]
            );
            assert!(
                region
                    .read_cap()
                    .read_item(scalar(), empty, 0, &control)
                    .is_err()
            );
            let unit = RegionLayout::<()>::new([8; 32], 0, 8)?;
            let units = region.allocate_slice(unit, u32::MAX, &[], &control)?;
            assert_eq!(
                region
                    .read_cap()
                    .read_item(unit, units, u32::MAX - 1, &control)?,
                &[] as &[u8]
            );
            Ok(())
        },
    )
    .unwrap();
}

#[test]
fn immutable_image_admission_and_scoped_parallel_reads_copy_no_payload() {
    let control = ExecutionControl::uncancelled();
    with_region(
        512,
        &control,
        |_| Ok(()),
        |mut region| {
            let pointer = region.allocate(scalar(), &73_i64.to_le_bytes(), &control)?;
            let frozen = region.freeze(&control)?;
            let address = frozen.image().as_ptr() as usize;
            let frozen = std::thread::scope(|scope| scope.spawn(move || frozen).join().unwrap());
            assert_eq!(frozen.image().as_ptr() as usize, address);
            let read = frozen.read_cap();
            let values = std::thread::scope(|scope| {
                let first = scope.spawn(|| number(read.read(scalar(), pointer, &control).unwrap()));
                let second =
                    scope.spawn(|| number(read.read(scalar(), pointer, &control).unwrap()));
                (first.join().unwrap(), second.join().unwrap())
            });
            assert_eq!(values, (73, 73));
            with_frozen_image(frozen.image(), frozen.image().len(), &control, |mapped| {
                assert_eq!(mapped.image.as_ptr() as usize, address);
                let rebound = mapped.resolve(scalar(), pointer.offset(), &control)?;
                assert_eq!(number(mapped.read(scalar(), rebound, &control)?), 73);
                Ok(())
            })
        },
    )
    .unwrap();
}

fn aligned_copy(bytes: &[u8]) -> (Vec<u8>, usize) {
    let mut copy = vec![0; bytes.len() + 63];
    let start = (copy.as_ptr() as usize).wrapping_neg() & 63;
    copy[start..start + bytes.len()].copy_from_slice(bytes);
    (copy, start)
}

#[test]
fn complete_image_admission_rejects_corrupt_headers_bitmap_padding_and_truncation() {
    let control = ExecutionControl::uncancelled();
    with_region(
        256,
        &control,
        |_| Ok(()),
        |mut region| {
            let cell = region.allocate(scalar(), &42_i64.to_le_bytes(), &control)?;
            let frozen = region.freeze(&control)?;
            let image = frozen.image();
            let cell_header = cell.offset() as usize - CELL_BYTES as usize;
            let changes = [
                (0, 0),
                (8, 2),
                (10, 1),
                (28, 1),
                (16, 0),
                (20, 0),
                (24, 0),
                (64, 0),
                (65, 1),
                (68, 1),
                (cell_header + 32, 16),
                (cell_header + 36, 2),
                (cell_header + 40, 3),
                (cell_header + 44, 1),
            ];
            for (offset, value) in changes {
                let (mut damaged, start) = aligned_copy(image);
                damaged[start + offset] = value;
                let mut entered = false;
                let rejected = with_frozen_image(
                    &damaged[start..start + image.len()],
                    image.len(),
                    &control,
                    |_| {
                        entered = true;
                        Ok(())
                    },
                );
                assert!(rejected.is_err(), "accepted damaged offset {offset}");
                assert!(!entered);
            }
            assert!(
                with_frozen_image(&image[..image.len() - 1], image.len(), &control, |_| Ok(()))
                    .is_err()
            );
            assert!(with_frozen_image(&image[1..], image.len(), &control, |_| Ok(())).is_err());
            assert_eq!(
                with_frozen_image(image, image.len() - 1, &control, |_| Ok(()))
                    .unwrap_err()
                    .code,
                "region_capacity"
            );
            Ok(())
        },
    )
    .unwrap();
}

#[test]
fn images_are_canonical_across_allocations_and_all_growth_requires_prior_admission() {
    let control = ExecutionControl::uncancelled();
    let make = || {
        with_region(
            512,
            &control,
            |_| Ok(()),
            |mut region| {
                region.allocate(scalar(), &42_i64.to_le_bytes(), &control)?;
                Ok(region.freeze(&control)?.image().to_vec())
            },
        )
        .unwrap()
    };
    assert_eq!(make(), make());
    let mut entered = false;
    let error = with_region(
        512,
        &control,
        |_| Err(ExecutionError::resource("quota", "test quota")),
        |_| {
            entered = true;
            Ok(())
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "quota");
    assert!(!entered);
    let mut charged = false;
    assert!(
        with_region(
            u32::MAX,
            &control,
            |_| {
                charged = true;
                Ok(())
            },
            |_| Ok(())
        )
        .is_err()
    );
    assert!(!charged);
    control.cancel();
    assert_eq!(
        with_region(
            512,
            &control,
            |_| {
                charged = true;
                Ok(())
            },
            |_| Ok(())
        )
        .unwrap_err()
        .class,
        ExecutionFailureClass::Cancelled
    );
    assert!(!charged);
    assert!(RegionLayout::<()>::new([0; 32], 3, 2).is_err());
    assert!(RegionLayout::<()>::new([0; 32], 8, 0).is_err());
    assert!(RegionLayout::<()>::new([0; 32], 128, 128).is_err());
}
