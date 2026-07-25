use crate::verify::fail;
use crate::{RuntimeOp, Signature, SsaType};

mod containers;
mod core;
mod host;

pub(crate) fn verify_runtime_signature(
    operation: RuntimeOp,
    signature: &Signature,
) -> crate::Result<()> {
    let parameters = &signature.parameters;
    let result = signature.result.as_ref();
    let valid = core::core_signature(operation, parameters, result)
        .or_else(|| host::host_signature(operation, parameters, result))
        .or_else(|| containers::container_signature(operation, parameters, result))
        .unwrap_or(false);
    if valid {
        Ok(())
    } else {
        fail(format!(
            "SSA runtime operation {operation:?} has an impossible signature"
        ))
    }
}

pub(crate) fn system_result(success: SsaType) -> SsaType {
    SsaType::Result(Box::new(success), Box::new(SsaType::Str))
}
