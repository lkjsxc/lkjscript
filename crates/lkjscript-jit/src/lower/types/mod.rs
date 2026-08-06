include!("modules.rs");

pub(super) fn lower_signature(
    function: &Function,
    modes: &BytesModes,
    layouts: &LayoutInterner,
) -> Result<Signature, LoweringError> {
    let authenticated = function.signature.type_parameters.iter().all(|parameter| {
        function
            .signature
            .memory_witness_parameters
            .iter()
            .any(|requirement| requirement.parameter == *parameter)
    });
    let residual_witness = !function.signature.type_parameters.is_empty()
        && authenticated
        && function
            .signature
            .memory_witness_parameters
            .iter()
            .any(|requirement| {
                requirement
                    .operations
                    .contains(&lkjscript_core::MemoryWitnessOperation::Compare)
                    || (requirement
                        .operations
                        .contains(&lkjscript_core::MemoryWitnessOperation::IndependentOwner)
                        && requirement
                            .operations
                            .contains(&lkjscript_core::MemoryWitnessOperation::Dispose))
            });
    if !function.signature.type_parameters.is_empty() && !residual_witness {
        return Err(LoweringError::new(
            LoweringFailureCode::UnsupportedSignature,
            Some(function.id),
            "polymorphic native signature lacks authenticated residual witnesses",
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
    let mut parameters = function
        .signature
        .parameters
        .iter()
        .zip(&entry.parameters)
        .map(|(ty, parameter)| lower_value_type(function.id, parameter.id, ty, modes, layouts))
        .collect::<Result<Vec<_>, _>>()?;
    parameters.extend(
        function
            .signature
            .memory_witness_parameters
            .iter()
            .map(|_| ValueType::MemoryWitnessLocator),
    );
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
    if let Some(value_type) = layouts.structural().owner_type(ty) {
        return Ok(value_type);
    }
    match ty {
        SsaType::Unit => Ok(ValueType::Unit),
        SsaType::Bool => Ok(ValueType::Bool),
        SsaType::I64 => Ok(ValueType::I64),
        SsaType::F64 => Ok(ValueType::F64),
        SsaType::Capability(kind) => Ok(ValueType::Capability(*kind)),
        SsaType::Resource(kind) => Ok(ValueType::Resource(*kind)),
        SsaType::Product(product) => {
            let identity = layouts.region_product_identity(*product).ok_or_else(|| {
                LoweringError::new(
                    LoweringFailureCode::UnsupportedType,
                    Some(function),
                    "product lacks structural or invocation-region storage metadata",
                )
            })?;
            Ok(ValueType::Reference(ReferenceType::RegionProduct(
                LayoutIdentity::product(product.raw()),
                identity,
            )))
        }
        SsaType::List(element) => Ok(ValueType::Reference(ReferenceType::List(
            exact_layout_identity(function, layouts, ty)?,
            layouts.semantic(ty).ok_or_else(|| {
                LoweringError::new(
                    LoweringFailureCode::UnsupportedType,
                    Some(function),
                    format!("type {ty:?} has no exact semantic identity"),
                )
            })?,
            exact_layout_identity(function, layouts, element)?,
            layouts.semantic(element).ok_or_else(|| {
                LoweringError::new(
                    LoweringFailureCode::UnsupportedType,
                    Some(function),
                    format!("type {element:?} has no exact semantic identity"),
                )
            })?,
        ))),
        SsaType::Enum { .. } => Err(LoweringError::new(
            LoweringFailureCode::UnsupportedType,
            Some(function),
            "enum lacks deterministic structural metadata",
        )),
        SsaType::Bytes => Err(LoweringError::new(
            LoweringFailureCode::UnsupportedType,
            Some(function),
            "collector-free bytes native lowering is not yet installed",
        )),
        SsaType::ByteVector => Ok(ValueType::Unique(UniqueType::ByteVector)),
        SsaType::ByteSlice => Ok(ValueType::Loan(LoanType::ByteSlice)),
        SsaType::ByteSliceMut => Ok(ValueType::Loan(LoanType::ByteSliceMut)),
        SsaType::StructuralDestination(_) => Err(LoweringError::new(
            LoweringFailureCode::UnsupportedType,
            Some(function),
            "structural destination native lowering is unavailable",
        )),
        SsaType::Str | SsaType::Path => Err(LoweringError::new(
            LoweringFailureCode::UnsupportedType,
            Some(function),
            format!(
                concat!(
                    "source structural type {:?} has no compiler-produced native structural ",
                    "owner; forced native fails closed",
                ),
                ty,
            ),
        )),
        SsaType::TypeParameter(_) => Ok(ValueType::StructuralKey),
        SsaType::Symbol | SsaType::Function(_) => Err(LoweringError::new(
            LoweringFailureCode::UnsupportedType,
            Some(function),
            format!("type {ty:?} contains a reference or unsupported native representation"),
        )),
    }
}
