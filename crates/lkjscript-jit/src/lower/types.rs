use super::*;

pub(super) fn lower_signature(
    function: FunctionId,
    signature: &lkjscript_ir::Signature,
    layouts: &LayoutInterner,
) -> Result<Signature, LoweringError> {
    if !signature.type_parameters.is_empty() {
        return Err(LoweringError::new(
            LoweringFailureCode::UnsupportedSignature,
            Some(function),
            "polymorphic native signatures are unsupported",
        ));
    }
    let parameters = signature
        .parameters
        .iter()
        .map(|ty| lower_type(function, ty, layouts))
        .collect::<Result<Vec<_>, _>>()?;
    let result = lower_type(function, &signature.result, layouts)?;
    Signature::new(parameters, result).map_err(|error| {
        LoweringError::new(
            LoweringFailureCode::UnsupportedSignature,
            Some(function),
            error.to_string(),
        )
    })
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
        SsaType::ByteVector | SsaType::ByteSlice | SsaType::ByteSliceMut => {
            Err(LoweringError::new(
                LoweringFailureCode::UnsupportedType,
                Some(function),
                "collector-free byte-vector native lowering is not yet installed",
            ))
        }
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
