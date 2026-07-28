use super::*;

#[test]
fn concrete_generic_enum_layouts_reject_cross_handles() {
    let first = lkjscript_native::ReferenceType::Enum(
        lkjscript_native::LayoutIdentity::new(70_000),
        lkjscript_core::RESULT_LAYOUT,
    );
    let second = lkjscript_native::ReferenceType::Enum(
        lkjscript_native::LayoutIdentity::new(70_001),
        lkjscript_core::RESULT_LAYOUT,
    );
    assert_ne!(
        super::reference_layout_key(first),
        super::reference_layout_key(second)
    );

    let mut heap = GcHeap::default();
    let value = heap
        .try_alloc_with_layout(
            HeapObj::Enum {
                layout: lkjscript_core::RuntimeLayoutId::new(lkjscript_core::RESULT_LAYOUT),
                physical_tag: 0,
                active_payload: vec![Value::UNIT],
            },
            super::reference_layout_key(first),
        )
        .expect("first generic enum layout allocation");
    let reference = lkjscript_native::NativeReference::new(
        second,
        u64::from(value.as_legacy_traced().expect("heap handle")) + 1,
    );
    assert_eq!(
        super::native_reference_value(&heap, reference),
        Err("native heap handle layout mismatch".into())
    );
}
