use super::{reader::Reader, values, ImageCodecError};
use crate::*;

type EnumPrefix = ([u8; 32], [u8; 32], [u8; 32], u16);

pub(super) fn descriptor(input: &mut Reader<'_>) -> Result<HeapCallDescriptor, ImageCodecError> {
    let operation = operation(input)?;
    let count = input.count()?;
    let mut input_types = Vec::with_capacity(count);
    for _ in 0..count {
        input_types.push(values::read_value_type(input)?);
    }
    let result = values::read_value_type(input)?;
    let allocation = match input.u8()? {
        0 => AllocationClass::None,
        1 => AllocationClass::Bounded,
        _ => return Err(ImageCodecError::new("unknown allocation class")),
    };
    let store = match input.u8()? {
        0 => StoreClass::None,
        1 => StoreClass::Initialization,
        2 => StoreClass::Scalar,
        3 => StoreClass::Reference,
        4 => StoreClass::ReferenceClearing,
        _ => return Err(ImageCodecError::new("unknown store class")),
    };
    HeapCallDescriptor::new(operation, input_types, result, allocation, store)
        .map_err(|_| ImageCodecError::new("invalid heap descriptor"))
}

fn operation(input: &mut Reader<'_>) -> Result<HeapOperation, ImageCodecError> {
    match input.u8()? {
        0 => Ok(HeapOperation::ConstantStr(input.string()?)),
        1 => Ok(HeapOperation::EmptyStr),
        2 => Ok(HeapOperation::EmptyList),
        3 => Ok(HeapOperation::ProductValue {
            product: input.u32()?,
            fields: input.u8()?,
        }),
        4 => product_field(input, false),
        5 => product_field(input, true),
        6 => enum_value(input),
        7 => {
            let (enum_id, variant, layout, physical_tag) = enum_prefix(input)?;
            Ok(HeapOperation::EnumIsVariant {
                enum_id,
                variant,
                layout,
                physical_tag,
            })
        }
        8 => enum_field(input),
        9 => Ok(HeapOperation::Cons),
        10 => Ok(HeapOperation::Car),
        11 => Ok(HeapOperation::Cdr),
        12 => Ok(HeapOperation::IsEmptyList),
        13 => Ok(HeapOperation::BufNew),
        14 => Ok(HeapOperation::BufLen),
        15 => Ok(HeapOperation::BufRef),
        16 => Ok(HeapOperation::BufSet),
        17 => Ok(HeapOperation::BufClone),
        18 => Ok(HeapOperation::BufFromStr),
        21 => Ok(HeapOperation::BufToStr {
            error_type: reference(input)?,
        }),
        22 => Ok(HeapOperation::BufSlice {
            error_type: reference(input)?,
            code_option_type: reference(input)?,
            detail_option_type: reference(input)?,
        }),
        23 => Ok(HeapOperation::BufGetU32),
        24 => Ok(HeapOperation::BufSetU32),
        25 => Ok(HeapOperation::StrLen),
        26 => Ok(HeapOperation::StrRef),
        27 => Ok(HeapOperation::StrAppend),
        28 => Ok(HeapOperation::StrSlice),
        29 => Ok(HeapOperation::StrFromByte),
        30 => Ok(HeapOperation::StrFromI64),
        31 => Ok(HeapOperation::StrFromF64),
        32 => Ok(HeapOperation::EqualValue),
        33 => Ok(HeapOperation::SameObject),
        34 => Ok(HeapOperation::ListEqual),
        36 => Ok(HeapOperation::F64FromI64Exact {
            error_type: values::read_value_type(input)?,
        }),
        37 => Ok(HeapOperation::F64FromI64Rounded),
        38 => Ok(HeapOperation::I64FromF64Exact {
            error_type: values::read_value_type(input)?,
        }),
        39 => Ok(HeapOperation::I64FromF64Trunc {
            error_type: values::read_value_type(input)?,
        }),
        _ => Err(ImageCodecError::new("unknown heap operation")),
    }
}

fn product_field(input: &mut Reader<'_>, replace: bool) -> Result<HeapOperation, ImageCodecError> {
    let product = input.u32()?;
    let field = input.u8()?;
    let field_type = values::read_value_type(input)?;
    if replace {
        Ok(HeapOperation::WithProductField {
            product,
            field,
            field_type,
        })
    } else {
        Ok(HeapOperation::ProductField {
            product,
            field,
            field_type,
        })
    }
}

fn enum_value(input: &mut Reader<'_>) -> Result<HeapOperation, ImageCodecError> {
    let (enum_id, variant, layout, physical_tag) = enum_prefix(input)?;
    let count = input.count()?;
    let mut substitutions = Vec::with_capacity(count);
    for _ in 0..count {
        substitutions.push(LayoutIdentity::new(input.u32()?));
    }
    Ok(HeapOperation::EnumValue {
        enum_id,
        variant,
        layout,
        physical_tag,
        substitutions,
        fields: input.u8()?,
    })
}

fn enum_field(input: &mut Reader<'_>) -> Result<HeapOperation, ImageCodecError> {
    let (enum_id, variant, layout, physical_tag) = enum_prefix(input)?;
    Ok(HeapOperation::EnumField {
        enum_id,
        variant,
        field: input.fixed()?,
        layout,
        physical_tag,
        field_index: input.u8()?,
        field_type: values::read_value_type(input)?,
    })
}

fn enum_prefix(input: &mut Reader<'_>) -> Result<EnumPrefix, ImageCodecError> {
    Ok((input.fixed()?, input.fixed()?, input.fixed()?, input.u16()?))
}

fn reference(input: &mut Reader<'_>) -> Result<ReferenceType, ImageCodecError> {
    match values::read_value_type(input)? {
        ValueType::Reference(value) => Ok(value),
        _ => Err(ImageCodecError::new(
            "heap operation requires reference type",
        )),
    }
}
