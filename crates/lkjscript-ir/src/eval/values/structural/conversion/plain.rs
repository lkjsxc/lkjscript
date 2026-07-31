use crate::eval::{EvalValue, Flow};

pub(crate) fn clone_plain_eval_value(value: &EvalValue) -> Result<EvalValue, Flow> {
    match value {
        EvalValue::Unit => Ok(EvalValue::Unit),
        EvalValue::Bool(value) => Ok(EvalValue::Bool(*value)),
        EvalValue::I64(value) => Ok(EvalValue::I64(*value)),
        EvalValue::F64(value) => Ok(EvalValue::F64(*value)),
        EvalValue::Str(value) => Ok(EvalValue::Str(value.clone())),
        EvalValue::StaticString(identity) => Ok(EvalValue::StaticString(*identity)),
        EvalValue::StaticSymbol(identity) => Ok(EvalValue::StaticSymbol(*identity)),
        EvalValue::Symbol(symbol) => Ok(EvalValue::Symbol(symbol.clone())),
        EvalValue::StaticBytes(identity) => Ok(EvalValue::StaticBytes(*identity)),
        EvalValue::Capability(capability) => Ok(EvalValue::Capability(*capability)),
        EvalValue::Product(id, fields) => {
            clone_plain_values(fields).map(|fields| EvalValue::Product(*id, fields))
        }
        EvalValue::Enum {
            enum_id,
            variant,
            layout,
            physical_tag,
            payload,
        } => clone_plain_values(payload).map(|payload| EvalValue::Enum {
            enum_id: *enum_id,
            variant: *variant,
            layout: *layout,
            physical_tag: *physical_tag,
            payload,
        }),
        EvalValue::List(values) => clone_plain_values(values).map(EvalValue::List),
        EvalValue::Function(function) => Ok(EvalValue::Function(*function)),
        EvalValue::StructuralOwner(_)
        | EvalValue::StructuralView(_)
        | EvalValue::StructuralUtf8View(_)
        | EvalValue::StructuralDestination(_)
        | EvalValue::Bytes(_)
        | EvalValue::BytesBorrow(_)
        | EvalValue::ByteVector(_)
        | EvalValue::Path(_)
        | EvalValue::ByteSlice(_)
        | EvalValue::ByteSliceMut(_)
        | EvalValue::Resource(_) => Err(Flow::Trap(
            "evaluator owner or loan cannot be implicitly cloned".into(),
        )),
        EvalValue::ReturnedOwned(_)
        | EvalValue::ReturnedByteVector(_)
        | EvalValue::ReturnedBytes(_) => Err(Flow::Trap(
            "returned value cannot re-enter evaluator cloning".into(),
        )),
    }
}

pub(crate) fn copy_bytes(bytes: &[u8]) -> Result<Vec<u8>, Flow> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(bytes.len())
        .map_err(|_| Flow::Resource("structural field bytes".into()))?;
    copy.extend_from_slice(bytes);
    Ok(copy)
}

fn clone_plain_values(values: &[EvalValue]) -> Result<Vec<EvalValue>, Flow> {
    let mut output = Vec::new();
    output
        .try_reserve_exact(values.len())
        .map_err(|_| Flow::Resource("evaluator value clone".into()))?;
    for value in values {
        output.push(clone_plain_eval_value(value)?);
    }
    Ok(output)
}
