use lkjscript_core::{Error, Result};

use super::analysis::ModuleAnalysis;
use super::model::{
    LockedMemoryConstraint, LockedMemoryParameterMode, LockedMemoryRequirement,
    LockedMemoryResultMode, LockedTraitParameter,
};

pub(super) struct FunctionFields {
    pub(super) types: Vec<String>,
    pub(super) traits: Vec<LockedTraitParameter>,
    pub(super) requirements: Vec<LockedMemoryRequirement>,
    pub(super) parameters: Vec<LockedMemoryParameterMode>,
    pub(super) result: LockedMemoryResultMode,
    pub(super) equality: Vec<LockedMemoryConstraint>,
    pub(super) codec: Vec<LockedMemoryConstraint>,
}

pub(super) fn fields(
    function: &crate::hir::Function,
    analysis: &ModuleAnalysis,
) -> Result<FunctionFields> {
    let binding = analysis
        .hir
        .binding(function.binding)
        .ok_or_else(|| Error::msg("public HIR function binding is absent"))?;
    let types = match &binding.ty {
        crate::hir::Type::Forall { vars, .. } => vars.clone(),
        crate::hir::Type::Fn { .. } => Vec::new(),
        _ => return Err(Error::msg("public function binding is not callable")),
    };
    let planned = analysis
        .memory_plan
        .functions
        .iter()
        .find(|item| item.binding == Some(function.binding.raw()))
        .ok_or_else(|| Error::msg("verified memory plan omits public function"))?;
    let requirements = planned
        .signature
        .witness_parameters
        .iter()
        .map(super::interface_values::requirement)
        .collect();
    let parameters = planned
        .signature
        .parameters
        .iter()
        .copied()
        .map(super::interface_values::parameter_mode)
        .collect();
    let result = super::interface_values::result_mode(planned.signature.result);
    let traits = super::interface_values::trait_parameters(function, analysis)?;
    let (equality, codec) = super::interface_values::constraints(&types, analysis)?;
    Ok(FunctionFields {
        types,
        traits,
        requirements,
        parameters,
        result,
        equality,
        codec,
    })
}

pub(super) fn not_applicable() -> FunctionFields {
    FunctionFields {
        types: Vec::new(),
        traits: Vec::new(),
        requirements: Vec::new(),
        parameters: Vec::new(),
        result: LockedMemoryResultMode::NotApplicable,
        equality: Vec::new(),
        codec: Vec::new(),
    }
}
