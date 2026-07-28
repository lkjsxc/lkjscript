use super::*;

pub(super) fn lower_signature(
    function: &Function,
    modes: &BytesModes,
    layouts: &LayoutInterner,
) -> Result<Signature, LoweringError> {
    if !function.signature.type_parameters.is_empty() {
        return Err(LoweringError::new(
            LoweringFailureCode::UnsupportedSignature,
            Some(function.id),
            "polymorphic native signatures are unsupported",
        ));
    }
    let entry = function
        .blocks
        .get(
            function
                .entry
                .index()
                .ok_or_else(|| mode_signature_error(function.id))?,
        )
        .ok_or_else(|| mode_signature_error(function.id))?;
    let parameters = function
        .signature
        .parameters
        .iter()
        .zip(&entry.parameters)
        .map(|(ty, parameter)| lower_value_type(function.id, parameter.id, ty, modes, layouts))
        .collect::<Result<Vec<_>, _>>()?;
    let result = if function.signature.result.as_ref() == &SsaType::Bytes {
        lower_bytes_mode(modes.result(function.id)?)
    } else {
        lower_type(function.id, &function.signature.result, layouts)?
    };
    Signature::new(parameters, result).map_err(|error| {
        LoweringError::new(
            LoweringFailureCode::UnsupportedSignature,
            Some(function.id),
            error.to_string(),
        )
    })
}

pub(super) fn lower_value_type(
    function: FunctionId,
    value: ValueId,
    ty: &SsaType,
    modes: &BytesModes,
    layouts: &LayoutInterner,
) -> Result<ValueType, LoweringError> {
    if ty == &SsaType::Bytes {
        Ok(lower_bytes_mode(modes.value(function, value)?))
    } else {
        lower_type(function, ty, layouts)
    }
}

fn lower_bytes_mode(mode: BytesMode) -> ValueType {
    match mode {
        BytesMode::Static => ValueType::StaticBytes,
        BytesMode::Owner => ValueType::Unique(UniqueType::Bytes),
        BytesMode::Loan => ValueType::Loan(LoanType::Bytes),
    }
}

fn mode_signature_error(function: FunctionId) -> LoweringError {
    LoweringError::new(
        LoweringFailureCode::InvalidFunction,
        Some(function),
        "native bytes signature has no exact entry mode",
    )
}

pub(super) fn lower_type(
    function: FunctionId,
    ty: &SsaType,
    layouts: &LayoutInterner,
) -> Result<ValueType, LoweringError> {
    match ty {
        SsaType::Unit => Ok(ValueType::Unit),
        SsaType::Bool => Ok(ValueType::Bool),
        SsaType::I64 => Ok(ValueType::I64),
        SsaType::F64 => Ok(ValueType::F64),
        SsaType::Capability(kind) => Ok(ValueType::Capability(*kind)),
        SsaType::Resource(kind) => Ok(ValueType::Resource(*kind)),
        SsaType::Str => Ok(ValueType::Reference(ReferenceType::Str)),
        SsaType::Buf => Ok(ValueType::Reference(ReferenceType::Buf)),
        SsaType::Product(product) => Ok(ValueType::Reference(ReferenceType::Product(
            LayoutIdentity::product(u32::from(product.raw())),
        ))),
        SsaType::List(element) => Ok(ValueType::Reference(ReferenceType::List(
            exact_layout_identity(function, layouts, ty)?,
            exact_layout_identity(function, layouts, element)?,
        ))),
        SsaType::Enum { arguments, .. } => {
            for argument in arguments {
                lower_type(function, argument, layouts)?;
            }
            Ok(ValueType::Reference(ReferenceType::Enum(
                exact_layout_identity(function, layouts, ty)?,
                layouts.enum_layout(ty).ok_or_else(|| {
                    LoweringError::new(
                        LoweringFailureCode::UnsupportedType,
                        Some(function),
                        "enum type is missing its runtime layout identity",
                    )
                })?,
            )))
        }
        SsaType::Bytes => Err(LoweringError::new(
            LoweringFailureCode::UnsupportedType,
            Some(function),
            "collector-free bytes native lowering is not yet installed",
        )),
        SsaType::ByteVector => Ok(ValueType::Unique(UniqueType::ByteVector)),
        SsaType::ByteSlice => Ok(ValueType::Loan(LoanType::ByteSlice)),
        SsaType::ByteSliceMut => Ok(ValueType::Loan(LoanType::ByteSliceMut)),
        _ => Err(LoweringError::new(
            LoweringFailureCode::UnsupportedType,
            Some(function),
            format!("type {ty:?} contains a reference or unsupported native representation"),
        )),
    }
}

pub(super) fn require_resource_island_type(
    function: FunctionId,
    ty: &SsaType,
) -> Result<(), LoweringError> {
    if matches!(
        ty,
        SsaType::Unit
            | SsaType::Bool
            | SsaType::I64
            | SsaType::F64
            | SsaType::Capability(_)
            | SsaType::Resource(_)
    ) {
        Ok(())
    } else {
        Err(LoweringError::new(
            LoweringFailureCode::UnsupportedType,
            Some(function),
            format!("type {ty:?} is reachable from a collector-free resource group"),
        ))
    }
}

pub(super) fn require_unique_island_type(
    function: FunctionId,
    ty: &SsaType,
) -> Result<(), LoweringError> {
    if matches!(
        ty,
        SsaType::Unit
            | SsaType::Bool
            | SsaType::I64
            | SsaType::F64
            | SsaType::Bytes
            | SsaType::ByteVector
            | SsaType::ByteSlice
            | SsaType::ByteSliceMut
    ) {
        Ok(())
    } else {
        Err(LoweringError::new(
            LoweringFailureCode::UnsupportedType,
            Some(function),
            format!("type {ty:?} is reachable from a collector-free unique group"),
        ))
    }
}

pub(super) fn exact_layout_identity(
    function: FunctionId,
    layouts: &LayoutInterner,
    ty: &SsaType,
) -> Result<LayoutIdentity, LoweringError> {
    layouts.identity(ty).ok_or_else(|| {
        LoweringError::new(
            LoweringFailureCode::UnsupportedType,
            Some(function),
            format!("type {ty:?} has no supported structural layout identity"),
        )
    })
}
