use crate::{EngineError, FailureCode, VerifiedProgram};

pub(super) fn reject_native_bytes(program: &VerifiedProgram) -> Result<(), EngineError> {
    let found = program.program().functions.iter().any(|function| {
        function
            .signature
            .parameters
            .iter()
            .chain(std::iter::once(function.signature.result.as_ref()))
            .any(contains_bytes)
            || function
                .places
                .iter()
                .any(|place| contains_bytes(&place.ty))
            || function.blocks.iter().any(|block| {
                block
                    .parameters
                    .iter()
                    .any(|parameter| contains_bytes(&parameter.ty))
                    || block
                        .instructions
                        .iter()
                        .any(|instruction| contains_bytes(&instruction.ty))
            })
    });
    if found {
        Err(EngineError::new(
            FailureCode::UnsupportedType,
            Some(program.program().main),
            "collector-free immutable bytes native lowering is not installed",
        ))
    } else {
        Ok(())
    }
}

fn contains_bytes(ty: &lkjscript_ir::SsaType) -> bool {
    match ty {
        lkjscript_ir::SsaType::Bytes => true,
        lkjscript_ir::SsaType::List(inner) => contains_bytes(inner),
        lkjscript_ir::SsaType::Enum { arguments, .. } => arguments.iter().any(contains_bytes),
        lkjscript_ir::SsaType::Function(signature) => signature
            .parameters
            .iter()
            .chain(std::iter::once(signature.result.as_ref()))
            .any(contains_bytes),
        _ => false,
    }
}
