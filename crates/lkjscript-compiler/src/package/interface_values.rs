use lkjscript_core::{Error, Result};

use super::analysis::ModuleAnalysis;
use super::model::{
    LockedConstraintSupport, LockedMemoryConstraint, LockedMemoryParameterMode,
    LockedMemoryRequirement, LockedMemoryResultMode, LockedTraitParameter,
};

pub(super) fn declaration(
    module: &str,
    export: &str,
    analysis: &ModuleAnalysis,
) -> Result<(String, &'static str)> {
    if let Some(item) = analysis
        .source
        .declarations()
        .iter()
        .find(|item| item.origin().logical_path() == module && item.name() == export)
    {
        return Ok((item.key().to_hex(), item.kind().as_str()));
    }
    let variant = crate::source::module_names::internal_name(module, export);
    for enumeration in &analysis.hir.enums {
        if enumeration.variants.iter().any(|item| item.name == variant) {
            let parent = analysis.source.declarations().iter().find(|item| {
                item.origin().logical_path() == module
                    && crate::source::module_names::internal_name(module, item.name())
                        == enumeration.name
            });
            if let Some(parent) = parent {
                return Ok((parent.key().to_hex(), "enum-variant"));
            }
        }
    }
    Err(Error::msg(format!(
        "public export has no resolved declaration: {module}:{export}"
    )))
}

pub(super) fn requirement(
    value: &crate::memory_plan::MemoryWitnessParameter,
) -> LockedMemoryRequirement {
    let operations = value
        .operations
        .iter()
        .map(|operation| match operation {
            crate::memory_plan::MemoryWitnessOperation::Transport => "transport".into(),
            crate::memory_plan::MemoryWitnessOperation::Compare => "compare".into(),
            crate::memory_plan::MemoryWitnessOperation::IndependentOwner => {
                "independent-owner".into()
            }
            crate::memory_plan::MemoryWitnessOperation::Dispose => "dispose".into(),
        })
        .collect();
    LockedMemoryRequirement {
        parameter: value.parameter.clone(),
        operations,
    }
}

pub(super) const fn parameter_mode(
    value: crate::memory_plan::MemoryParameterMode,
) -> LockedMemoryParameterMode {
    match value {
        crate::memory_plan::MemoryParameterMode::Copy => LockedMemoryParameterMode::Copy,
        crate::memory_plan::MemoryParameterMode::BorrowShared => {
            LockedMemoryParameterMode::BorrowShared
        }
        crate::memory_plan::MemoryParameterMode::BorrowExclusive => {
            LockedMemoryParameterMode::BorrowExclusive
        }
        crate::memory_plan::MemoryParameterMode::Consume => LockedMemoryParameterMode::Consume,
    }
}

pub(super) const fn result_mode(
    value: crate::memory_plan::MemoryResultMode,
) -> LockedMemoryResultMode {
    match value {
        crate::memory_plan::MemoryResultMode::Trivial => LockedMemoryResultMode::Trivial,
        crate::memory_plan::MemoryResultMode::Owned => LockedMemoryResultMode::Owned,
        crate::memory_plan::MemoryResultMode::SealedShared => LockedMemoryResultMode::SealedShared,
        crate::memory_plan::MemoryResultMode::External => LockedMemoryResultMode::External,
    }
}

pub(super) fn trait_parameters(
    function: &crate::hir::Function,
    analysis: &ModuleAnalysis,
) -> Result<Vec<LockedTraitParameter>> {
    function
        .bounds
        .iter()
        .map(|bound| {
            let definition = analysis
                .hir
                .traits
                .get(bound.trait_id.index().unwrap_or(usize::MAX))
                .ok_or_else(|| Error::msg("public function bound has unknown TraitId"))?;
            let contract = super::contracts::expected(lkjscript_contracts::MODULE_INTERFACE)?;
            let identity = super::graph::framed_hash(
                b"lkjscript.public-trait-parameter",
                &[&contract.as_bytes(), definition.name.as_bytes()],
            )?;
            Ok(LockedTraitParameter {
                parameter: bound.parameter.clone(),
                trait_identity: identity,
                trait_name: definition.name.clone(),
            })
        })
        .collect()
}

pub(super) fn constraints(
    parameters: &[String],
    analysis: &ModuleAnalysis,
) -> Result<(Vec<LockedMemoryConstraint>, Vec<LockedMemoryConstraint>)> {
    let mut equality = Vec::with_capacity(parameters.len());
    let mut snapshot = Vec::with_capacity(parameters.len());
    for parameter in parameters {
        let fact = analysis
            .memory_plan
            .type_facts
            .iter()
            .find(|fact| {
                fact.ty == crate::memory_plan::MemoryType::TypeParameter(parameter.clone())
            })
            .ok_or_else(|| Error::msg("verified memory plan omits public type parameter fact"))?;
        let witness = analysis.memory_plan.witness(fact.witness).ok_or_else(|| {
            Error::msg("verified memory plan omits public type parameter witness")
        })?;
        equality.push(LockedMemoryConstraint {
            parameter: parameter.clone(),
            support: equality_support(witness.facts.equality),
        });
        snapshot.push(LockedMemoryConstraint {
            parameter: parameter.clone(),
            support: snapshot_support(witness.facts.semantic_snapshot),
        });
    }
    Ok((equality, snapshot))
}

const fn equality_support(
    value: crate::memory_plan::MemoryEqualitySupport,
) -> LockedConstraintSupport {
    match value {
        crate::memory_plan::MemoryEqualitySupport::Unsupported => {
            LockedConstraintSupport::Unsupported
        }
        crate::memory_plan::MemoryEqualitySupport::EqualValue => LockedConstraintSupport::Value,
        crate::memory_plan::MemoryEqualitySupport::EqualList => LockedConstraintSupport::List,
        crate::memory_plan::MemoryEqualitySupport::CallerWitnessRequired => {
            LockedConstraintSupport::CallerWitnessRequired
        }
    }
}

const fn snapshot_support(
    value: crate::memory_plan::MemorySemanticSnapshotEligibility,
) -> LockedConstraintSupport {
    match value {
        crate::memory_plan::MemorySemanticSnapshotEligibility::Eligible => {
            LockedConstraintSupport::Eligible
        }
        crate::memory_plan::MemorySemanticSnapshotEligibility::Ineligible => {
            LockedConstraintSupport::Ineligible
        }
        crate::memory_plan::MemorySemanticSnapshotEligibility::CallerWitnessRequired => {
            LockedConstraintSupport::CallerWitnessRequired
        }
    }
}
