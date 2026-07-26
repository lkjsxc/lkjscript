use super::{reader::Reader, writer::Writer, ImageCodecError};
use crate::*;

pub(super) fn function(out: &mut Writer, value: FunctionId) -> Result<(), ImageCodecError> {
    out.u32(value.index)
}

pub(super) fn read_function(
    input: &mut Reader<'_>,
    plan: u64,
) -> Result<FunctionId, ImageCodecError> {
    Ok(FunctionId {
        plan,
        index: input.u32()?,
    })
}

pub(super) fn value_type(out: &mut Writer, value: ValueType) -> Result<(), ImageCodecError> {
    match value {
        ValueType::I64 => out.u8(0),
        ValueType::F64 => out.u8(1),
        ValueType::Bool => out.u8(2),
        ValueType::Unit => out.u8(3),
        ValueType::Reference(reference) => {
            out.u8(4)?;
            reference_type(out, reference)
        }
    }
}

pub(super) fn read_value_type(input: &mut Reader<'_>) -> Result<ValueType, ImageCodecError> {
    match input.u8()? {
        0 => Ok(ValueType::I64),
        1 => Ok(ValueType::F64),
        2 => Ok(ValueType::Bool),
        3 => Ok(ValueType::Unit),
        4 => Ok(ValueType::Reference(read_reference_type(input)?)),
        _ => Err(ImageCodecError::new("unknown image value type")),
    }
}

pub(super) fn signature(out: &mut Writer, value: &Signature) -> Result<(), ImageCodecError> {
    out.sequence(value.parameters(), |out, item| value_type(out, *item))?;
    value_type(out, value.result())
}

pub(super) fn read_signature(input: &mut Reader<'_>) -> Result<Signature, ImageCodecError> {
    let count = input.count()?;
    let mut parameters = Vec::with_capacity(count);
    for _ in 0..count {
        parameters.push(read_value_type(input)?);
    }
    Signature::new(parameters, read_value_type(input)?)
        .map_err(|_| ImageCodecError::new("invalid image signature"))
}

pub(super) fn runtime_call(
    out: &mut Writer,
    value: RuntimeCallSlot,
) -> Result<(), ImageCodecError> {
    out.u8(match value {
        RuntimeCallSlot::IdentityI64 => 0,
        RuntimeCallSlot::Poll => 1,
        RuntimeCallSlot::EnterFunction => 2,
        RuntimeCallSlot::CollectReference => 3,
        RuntimeCallSlot::HeapDispatch => 4,
        RuntimeCallSlot::ReserveFrame => 5,
        RuntimeCallSlot::RegisterFrame => 6,
        RuntimeCallSlot::PublishSafepoint => 7,
        RuntimeCallSlot::UnregisterFrame => 8,
    })
}

pub(super) fn read_runtime_call(
    input: &mut Reader<'_>,
) -> Result<RuntimeCallSlot, ImageCodecError> {
    match input.u8()? {
        0 => Ok(RuntimeCallSlot::IdentityI64),
        1 => Ok(RuntimeCallSlot::Poll),
        2 => Ok(RuntimeCallSlot::EnterFunction),
        3 => Ok(RuntimeCallSlot::CollectReference),
        4 => Ok(RuntimeCallSlot::HeapDispatch),
        5 => Ok(RuntimeCallSlot::ReserveFrame),
        6 => Ok(RuntimeCallSlot::RegisterFrame),
        7 => Ok(RuntimeCallSlot::PublishSafepoint),
        8 => Ok(RuntimeCallSlot::UnregisterFrame),
        _ => Err(ImageCodecError::new("unknown runtime-call slot")),
    }
}

pub(super) fn frame_home(out: &mut Writer, value: FrameHome) -> Result<(), ImageCodecError> {
    match value.kind() {
        FrameHomeKind::Local(index) => {
            out.u8(0)?;
            out.u32(index)?;
        }
        FrameHomeKind::Value(index) => {
            out.u8(1)?;
            out.u32(index)?;
        }
    }
    value_type(out, value.value_type())?;
    out.i32(value.rbp_displacement())
}

pub(super) fn read_frame_home(input: &mut Reader<'_>) -> Result<FrameHome, ImageCodecError> {
    let kind = match input.u8()? {
        0 => FrameHomeKind::Local(input.u32()?),
        1 => FrameHomeKind::Value(input.u32()?),
        _ => return Err(ImageCodecError::new("unknown frame-home kind")),
    };
    Ok(super::super::frame_home(
        kind,
        read_value_type(input)?,
        input.i32()?,
    ))
}

pub(super) fn source(out: &mut Writer, value: Option<SourceOrigin>) -> Result<(), ImageCodecError> {
    match value {
        Some(value) => {
            out.u8(1)?;
            out.u32(value.get())
        }
        None => out.u8(0),
    }
}

pub(super) fn read_source(input: &mut Reader<'_>) -> Result<Option<SourceOrigin>, ImageCodecError> {
    match input.u8()? {
        0 => Ok(None),
        1 => Ok(Some(SourceOrigin::new(input.u32()?))),
        _ => Err(ImageCodecError::new("noncanonical source option")),
    }
}

fn reference_type(out: &mut Writer, value: ReferenceType) -> Result<(), ImageCodecError> {
    match value {
        ReferenceType::Buf => out.u8(0),
        ReferenceType::Str => out.u8(1),
        ReferenceType::List(list, element) => {
            out.u8(2)?;
            out.u32(list.get())?;
            out.u32(element.get())
        }
        ReferenceType::Product(layout) => {
            out.u8(3)?;
            out.u32(layout.get())
        }
        ReferenceType::Enum(layout, identity) => {
            out.u8(4)?;
            out.u32(layout.get())?;
            out.fixed(&identity)
        }
    }
}

fn read_reference_type(input: &mut Reader<'_>) -> Result<ReferenceType, ImageCodecError> {
    match input.u8()? {
        0 => Ok(ReferenceType::Buf),
        1 => Ok(ReferenceType::Str),
        2 => Ok(ReferenceType::List(
            LayoutIdentity::new(input.u32()?),
            LayoutIdentity::new(input.u32()?),
        )),
        3 => Ok(ReferenceType::Product(LayoutIdentity::new(input.u32()?))),
        4 => Ok(ReferenceType::Enum(
            LayoutIdentity::new(input.u32()?),
            input.fixed()?,
        )),
        _ => Err(ImageCodecError::new("unknown reference type")),
    }
}
