use crate::*;

pub(crate) fn scalar_box_layout(discriminator: u32) -> ReferenceType {
    ReferenceType::Product(lkjscript_native::LayoutIdentity::new(
        u32::MAX - discriminator,
    ))
}

pub(crate) fn reference_layout_key(reference_type: ReferenceType) -> u64 {
    match reference_type {
        ReferenceType::Buf => 1_u64 << 56,
        ReferenceType::Str => 2_u64 << 56,
        ReferenceType::List(layout, _) => (3_u64 << 56) | u64::from(layout.get()),
        ReferenceType::Option(layout, _) => (4_u64 << 56) | u64::from(layout.get()),
        ReferenceType::Result(layout, _, _) => (5_u64 << 56) | u64::from(layout.get()),
        ReferenceType::Product(layout) => (6_u64 << 56) | u64::from(layout.get()),
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
            ReferenceType::Option(_, _) => Ok(Value::NONE),
            _ => Err("zero native reference is invalid for this category".into()),
        };
    }
    let index = u32::try_from(word - 1).map_err(|_| "native heap handle out of range")?;
    let value = Value::from_heap(index);
    if heap.layout_of(value) != Some(reference_layout_key(reference_type)) {
        return Err("native heap handle layout mismatch".into());
    }
    let category_matches = match (reference_type, heap.get(value)) {
        (ReferenceType::Buf, Ok(HeapObj::Buf(_)))
        | (ReferenceType::Str, Ok(HeapObj::Str(_)))
        | (ReferenceType::List(_, _), Ok(HeapObj::Pair { .. }))
        | (ReferenceType::Option(_, _), Ok(HeapObj::OptionSome(_)))
        | (ReferenceType::Result(_, _, _), Ok(HeapObj::ResultOk(_) | HeapObj::ResultErr(_))) => {
            true
        }
        (ReferenceType::Product(layout), Ok(HeapObj::Product { product, .. })) => {
            layout == lkjscript_native::LayoutIdentity::product(u32::from(product.raw()))
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
    if value.is_empty_list() && matches!(reference_type, ReferenceType::List(_, _))
        || value.is_none() && matches!(reference_type, ReferenceType::Option(_, _))
    {
        return Ok(NativeValue::Reference(
            lkjscript_native::NativeReference::new(reference_type, 0),
        ));
    }
    let index = value.as_heap().ok_or("expected heap reference result")?;
    let reference = lkjscript_native::NativeReference::new(reference_type, u64::from(index) + 1);
    native_reference_value(heap, reference)?;
    Ok(NativeValue::Reference(reference))
}

pub(crate) fn index(value: i64, operation: &str) -> Result<usize, String> {
    usize::try_from(value).map_err(|_| format!("{operation} index out of range"))
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
    if left == right {
        return Ok(true);
    }
    if let (Some(left), Some(right)) = (left.as_bool(), right.as_bool()) {
        return Ok(left == right);
    }
    if let (Some(left), Some(right)) = (left.as_small_i64(), right.as_small_i64()) {
        return Ok(left == right);
    }
    match (heap.get(left), heap.get(right)) {
        (Ok(HeapObj::Int(left)), Ok(HeapObj::Int(right))) => Ok(left == right),
        (Ok(HeapObj::Float(left)), Ok(HeapObj::Float(right))) => Ok(left == right),
        (Ok(HeapObj::Str(left)), Ok(HeapObj::Str(right))) => Ok(left == right),
        (Ok(HeapObj::OptionSome(left)), Ok(HeapObj::OptionSome(right)))
        | (Ok(HeapObj::ResultOk(left)), Ok(HeapObj::ResultOk(right)))
        | (Ok(HeapObj::ResultErr(left)), Ok(HeapObj::ResultErr(right))) => {
            value_equal(heap, *left, *right)
        }
        (Ok(HeapObj::ResultOk(_)), Ok(HeapObj::ResultErr(_)))
        | (Ok(HeapObj::ResultErr(_)), Ok(HeapObj::ResultOk(_))) => Ok(false),
        _ if left.is_none() || right.is_none() => Ok(left.is_none() && right.is_none()),
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
