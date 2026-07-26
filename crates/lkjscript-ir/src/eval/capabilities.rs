use super::{EvalConfig, EvalValue};
use crate::{SsaType, VerifiedProgram};

pub(super) fn main_arguments(
    program: &VerifiedProgram,
    config: &EvalConfig,
) -> Result<Vec<EvalValue>, String> {
    let Some(main) = program
        .program()
        .functions
        .get(program.program().main.index().unwrap_or(usize::MAX))
    else {
        return Err("verified main is absent".into());
    };
    let required = main
        .signature
        .parameters
        .iter()
        .filter_map(|ty| match ty {
            SsaType::Capability(kind) => Some(*kind),
            _ => None,
        })
        .collect::<Vec<_>>();
    if required != config.capabilities {
        return Err(format!(
            "evaluation capability mismatch: required {required:?}, received {:?}",
            config.capabilities
        ));
    }
    Ok(config
        .capabilities
        .iter()
        .copied()
        .map(EvalValue::Capability)
        .collect())
}
