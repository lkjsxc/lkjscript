use super::*;

#[test]
fn product11_product0_and_product19_unit_result_layouts_reject_cross_handles() {
    let mix = |state: u32, value: u32| state.wrapping_mul(16_777_619) ^ value;
    let old_product = |product: u32| mix(8, product + 1);
    assert_eq!(
        mix(old_product(11), old_product(0)),
        mix(
            old_product(19),
            lkjscript_native::ValueType::Unit.layout_identity().get(),
        ),
        "the superseded 32-bit Result heap tag collided"
    );

    let first = lkjscript_native::ReferenceType::Result(
        lkjscript_native::LayoutIdentity::new(70_000),
        lkjscript_native::LayoutIdentity::product(11),
        lkjscript_native::LayoutIdentity::product(0),
    );
    let second = lkjscript_native::ReferenceType::Result(
        lkjscript_native::LayoutIdentity::new(70_001),
        lkjscript_native::LayoutIdentity::product(19),
        lkjscript_native::ValueType::Unit.layout_identity(),
    );
    assert_ne!(
        super::reference_layout_key(first),
        super::reference_layout_key(second)
    );

    let mut heap = GcHeap::default();
    let value = heap
        .try_alloc_with_layout(
            HeapObj::ResultOk(Value::UNIT),
            super::reference_layout_key(first),
        )
        .expect("first Result layout allocation");
    let reference = lkjscript_native::NativeReference::new(
        second,
        u64::from(value.as_heap().expect("heap handle")) + 1,
    );
    assert_eq!(
        super::native_reference_value(&heap, reference),
        Err("native heap handle layout mismatch".into())
    );
}
