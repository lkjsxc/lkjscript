use crate::*;

pub(crate) fn reference_layout_key(reference_type: ReferenceType) -> u64 {
    match reference_type {
        ReferenceType::List(layout, _) => (3_u64 << 56) | u64::from(layout.get()),
        ReferenceType::Product(layout) => (4_u64 << 56) | u64::from(layout.get()),
        ReferenceType::Enum(layout, _) => (5_u64 << 56) | u64::from(layout.get()),
    }
}

pub(crate) fn native_reference_value(
    heap: &GcHeap,
    reference: lkjscript_native::NativeReference,
) -> Result<Value, String> {
    let reference_type = reference.reference_type();
    let word = reference.opaque_word();
    if word == 0 {
        return match reference_type {
            ReferenceType::List(_, _) => Ok(Value::EMPTY_LIST),
            _ => Err("zero native reference is invalid for this category".into()),
        };
    }
    let index = u32::try_from(word - 1).map_err(|_| "native heap handle out of range")?;
    let value = Value::from_legacy_traced(index);
    if heap.layout_of(value) != Some(reference_layout_key(reference_type)) {
        return Err("native heap handle layout mismatch".into());
    }
    let category_matches = match (reference_type, heap.get(value)) {
        (ReferenceType::List(_, _), Ok(HeapObj::Pair { .. })) => true,
        (ReferenceType::Product(layout), Ok(HeapObj::Product { product, .. })) => {
            layout == lkjscript_native::LayoutIdentity::product(u32::from(product.raw()))
        }
        (ReferenceType::Enum(_, semantic_layout), Ok(HeapObj::Enum { layout, .. })) => {
            layout.bytes() == semantic_layout
        }
        _ => false,
    };
    if !category_matches {
        return Err("native heap handle category mismatch".into());
    }
    Ok(value)
}

pub(crate) fn reference_native_value(
    heap: &GcHeap,
    value: Value,
    reference_type: ReferenceType,
) -> Result<NativeValue, String> {
    if value.is_empty_list() && matches!(reference_type, ReferenceType::List(_, _)) {
        return Ok(NativeValue::Reference(
            lkjscript_native::NativeReference::new(reference_type, 0),
        ));
    }
    let index = value
        .as_legacy_traced()
        .ok_or("expected heap reference result")?;
    let reference = lkjscript_native::NativeReference::new(reference_type, u64::from(index) + 1);
    native_reference_value(heap, reference)?;
    Ok(NativeValue::Reference(reference))
}

pub(crate) fn list_values_equal(
    heap: &GcHeap,
    mut left: Value,
    mut right: Value,
    limit: usize,
) -> Result<bool, String> {
    for _ in 0..limit {
        if left.is_empty_list() || right.is_empty_list() {
            return Ok(left.is_empty_list() && right.is_empty_list());
        }
        let (left_car, left_cdr) = match heap.get(left) {
            Ok(HeapObj::Pair { car, cdr }) => (*car, *cdr),
            _ => return Err("list-equal expects proper List values".into()),
        };
        let (right_car, right_cdr) = match heap.get(right) {
            Ok(HeapObj::Pair { car, cdr }) => (*car, *cdr),
            _ => return Err("list-equal expects proper List values".into()),
        };
        if !value_equal(heap, left_car, right_car)? {
            return Ok(false);
        }
        left = left_cdr;
        right = right_cdr;
    }
    if left.is_empty_list() || right.is_empty_list() {
        Ok(left.is_empty_list() && right.is_empty_list())
    } else {
        Err("list-equal step limit exceeded".into())
    }
}

pub(crate) fn value_equal(heap: &GcHeap, left: Value, right: Value) -> Result<bool, String> {
    if left.is_unit() || right.is_unit() {
        return if left.is_unit() && right.is_unit() {
            Ok(true)
        } else {
            Err("equal-value category mismatch".into())
        };
    }
    if left.as_bool().is_some() || right.as_bool().is_some() {
        return match (left.as_bool(), right.as_bool()) {
            (Some(left), Some(right)) => Ok(left == right),
            _ => Err("equal-value category mismatch".into()),
        };
    }
    if left.as_i64().is_some() || right.as_i64().is_some() {
        return match (left.as_i64(), right.as_i64()) {
            (Some(left), Some(right)) => Ok(left == right),
            _ => Err("equal-value category mismatch".into()),
        };
    }
    if left.as_f64().is_some() || right.as_f64().is_some() {
        return match (left.as_f64(), right.as_f64()) {
            (Some(left), Some(right)) => Ok(left == right),
            _ => Err("equal-value category mismatch".into()),
        };
    }
    if left == right {
        return Ok(true);
    }
    match (heap.get(left), heap.get(right)) {
        (
            Ok(HeapObj::Enum {
                layout: left_layout,
                physical_tag: left_tag,
                ..
            }),
            Ok(HeapObj::Enum {
                layout: right_layout,
                physical_tag: right_tag,
                ..
            }),
        ) if left_layout == right_layout && left_tag != right_tag => Ok(false),
        (
            Ok(HeapObj::Enum {
                layout: left_layout,
                physical_tag: left_tag,
                active_payload: left_payload,
            }),
            Ok(HeapObj::Enum {
                layout: right_layout,
                physical_tag: right_tag,
                active_payload: right_payload,
            }),
        ) if left_layout == right_layout && left_tag == right_tag => {
            if left_payload.len() != right_payload.len() {
                return Err("enum payload shape mismatch".into());
            }
            for (left, right) in left_payload.iter().zip(right_payload) {
                if !value_equal(heap, *left, *right)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        _ => Err("equal-value category mismatch".into()),
    }
}

pub(crate) fn install_error(function: FunctionId, error: InstallError) -> EngineError {
    let code = match error {
        InstallError::LimitExceeded(_) => FailureCode::InstallLimit,
        _ => FailureCode::InstallFailure,
    };
    EngineError::new(code, Some(function), error.to_string())
}

pub(crate) fn invocation_error(function: FunctionId, error: InvocationError) -> EngineError {
    EngineError::new(
        FailureCode::InvocationFailure,
        Some(function),
        error.to_string(),
    )
}
