use super::{values, writer::Writer, ImageCodecError};
use crate::*;

pub(super) fn descriptor(
    out: &mut Writer,
    value: &HeapCallDescriptor,
) -> Result<(), ImageCodecError> {
    operation(out, value.operation())?;
    out.sequence(value.input_types(), |out, item| {
        values::value_type(out, *item)
    })?;
    values::value_type(out, value.result_type())?;
    out.u8(match value.allocation() {
        AllocationClass::None => 0,
        AllocationClass::Bounded => 1,
    })?;
    out.u8(match value.store() {
        StoreClass::None => 0,
        StoreClass::Initialization => 1,
        StoreClass::Scalar => 2,
        StoreClass::Reference => 3,
        StoreClass::ReferenceClearing => 4,
    })
}

fn operation(out: &mut Writer, value: &HeapOperation) -> Result<(), ImageCodecError> {
    match value {
        HeapOperation::ConstantStr(text) => {
            out.u8(0)?;
            out.string(text)
        }
        HeapOperation::EmptyStr => out.u8(1),
        HeapOperation::EmptyList => out.u8(2),
        HeapOperation::ProductValue { product, fields } => {
            product_fields(out, 3, *product, *fields)
        }
        HeapOperation::ProductField {
            product,
            field,
            field_type,
        } => typed_product(out, 4, *product, *field, *field_type),
        HeapOperation::WithProductField {
            product,
            field,
            field_type,
        } => typed_product(out, 5, *product, *field, *field_type),
        HeapOperation::EnumValue {
            enum_id,
            variant,
            layout,
            physical_tag,
            substitutions,
            fields,
        } => {
            enum_prefix(out, 6, enum_id, variant, layout, *physical_tag)?;
            out.sequence(substitutions, |out, item| out.u32(item.get()))?;
            out.u8(*fields)
        }
        HeapOperation::EnumIsVariant {
            enum_id,
            variant,
            layout,
            physical_tag,
        } => enum_prefix(out, 7, enum_id, variant, layout, *physical_tag),
        HeapOperation::EnumField {
            enum_id,
            variant,
            field,
            layout,
            physical_tag,
            field_index,
            field_type,
        } => {
            enum_prefix(out, 8, enum_id, variant, layout, *physical_tag)?;
            out.fixed(field)?;
            out.u8(*field_index)?;
            values::value_type(out, *field_type)
        }
        HeapOperation::BufToStr { error_type } => typed_reference(out, 21, *error_type),
        HeapOperation::BufSlice {
            error_type,
            code_option_type,
            detail_option_type,
        } => {
            typed_reference(out, 22, *error_type)?;
            values::value_type(out, ValueType::Reference(*code_option_type))?;
            values::value_type(out, ValueType::Reference(*detail_option_type))
        }
        HeapOperation::F64FromI64Exact { error_type } => typed_value(out, 36, *error_type),
        HeapOperation::I64FromF64Exact { error_type } => typed_value(out, 38, *error_type),
        HeapOperation::I64FromF64Trunc { error_type } => typed_value(out, 39, *error_type),
        other => out.u8(simple_tag(other)?),
    }
}

fn simple_tag(value: &HeapOperation) -> Result<u8, ImageCodecError> {
    let tag = match value {
        HeapOperation::Cons => 9,
        HeapOperation::Car => 10,
        HeapOperation::Cdr => 11,
        HeapOperation::IsEmptyList => 12,
        HeapOperation::BufNew => 13,
        HeapOperation::BufLen => 14,
        HeapOperation::BufRef => 15,
        HeapOperation::BufSet => 16,
        HeapOperation::BufClone => 17,
        HeapOperation::BufFromStr => 18,
        HeapOperation::BufGetU32 => 23,
        HeapOperation::BufSetU32 => 24,
        HeapOperation::StrLen => 25,
        HeapOperation::StrRef => 26,
        HeapOperation::StrAppend => 27,
        HeapOperation::StrSlice => 28,
        HeapOperation::StrFromByte => 29,
        HeapOperation::StrFromI64 => 30,
        HeapOperation::StrFromF64 => 31,
        HeapOperation::EqualValue => 32,
        HeapOperation::SameObject => 33,
        HeapOperation::ListEqual => 34,
        HeapOperation::F64FromI64Rounded => 37,
        _ => return Err(ImageCodecError::new("unsupported heap operation")),
    };
    Ok(tag)
}

fn product_fields(
    out: &mut Writer,
    tag: u8,
    product: u32,
    fields: u8,
) -> Result<(), ImageCodecError> {
    out.u8(tag)?;
    out.u32(product)?;
    out.u8(fields)
}

fn typed_product(
    out: &mut Writer,
    tag: u8,
    product: u32,
    field: u8,
    value_type: ValueType,
) -> Result<(), ImageCodecError> {
    product_fields(out, tag, product, field)?;
    values::value_type(out, value_type)
}

fn enum_prefix(
    out: &mut Writer,
    tag: u8,
    enum_id: &[u8; 32],
    variant: &[u8; 32],
    layout: &[u8; 32],
    physical_tag: u16,
) -> Result<(), ImageCodecError> {
    out.u8(tag)?;
    out.fixed(enum_id)?;
    out.fixed(variant)?;
    out.fixed(layout)?;
    out.u16(physical_tag)
}

fn typed_reference(out: &mut Writer, tag: u8, value: ReferenceType) -> Result<(), ImageCodecError> {
    typed_value(out, tag, ValueType::Reference(value))
}

fn typed_value(out: &mut Writer, tag: u8, value: ValueType) -> Result<(), ImageCodecError> {
    out.u8(tag)?;
    values::value_type(out, value)
}
