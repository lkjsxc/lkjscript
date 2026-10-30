//! Complete value-parameter ordering after explicit additions and deletions.
//!
//! The order is a bijection of the final candidate inventory. It changes only the
//! parent payload; retained parameter identities and records remain untouched.

use super::{AuthoredLowerer, OwnerSelector, ParameterParentSelector, request_error};
use crate::platform::change::{BudgetCounter, BudgetDimension, CanonicalBaseRead, WitnessBaseRead};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{DeclarationPayload, OwnerKey, OwnerRecord, ParameterParent};
use crate::platform::semantic_id::ParameterId;
use std::collections::BTreeSet;

pub(super) fn lower<'request, B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    orders: impl IntoIterator<Item = (&'request ParameterParentSelector, &'request [OwnerSelector])>,
) -> Result<(), Diagnostic> {
    let mut ordered_parents = BTreeSet::new();
    for (selector, parameters) in orders {
        lowerer.base.validation_checkpoint()?;
        charge(lowerer, BudgetCounter::exact(1))?;
        let owner = match selector {
            ParameterParentSelector::Declaration { declaration } => {
                OwnerKey::Declaration(lowerer.resolve_declaration(declaration)?)
            }
            ParameterParentSelector::Operation { operation } => lowerer.resolve_owner(operation)?,
        };
        let parent = match (selector, owner) {
            (ParameterParentSelector::Declaration { .. }, OwnerKey::Declaration(declaration)) => {
                ParameterParent::Function(declaration)
            }
            (ParameterParentSelector::Operation { .. }, OwnerKey::Operation(operation)) => {
                ParameterParent::Operation(operation)
            }
            _ => return Err(parent_error(owner)),
        };
        if !ordered_parents.insert(owner) {
            return Err(request_error(
                DiagnosticClass::Semantic,
                "change_parameter_order_duplicate_parent",
                format!("value-parameter order is specified more than once for {owner}"),
            ));
        }

        let count = parameter_ids(record(lowerer, owner)?)?.len();
        if parameters.len() != count {
            return Err(request_error(
                DiagnosticClass::Semantic,
                "change_parameter_order_membership",
                format!(
                    "complete value-parameter order for {owner} supplies {} members, but its surviving candidate inventory contains {count}",
                    parameters.len(),
                ),
            ));
        }
        let count = u64::try_from(count).map_err(|_| {
            request_error(
                DiagnosticClass::Resource,
                "change_parameter_order_count",
                "value-parameter inventory length exceeds the canonical u64 count domain",
            )
        })?;
        // Charge one step per current member and requested selector before either
        // membership set or the replacement inventory allocates storage.
        charge(lowerer, BudgetCounter::product(count, 2))?;
        let expected = parameter_ids(record(lowerer, owner)?)?
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if expected.len() != parameters.len() {
            return Err(request_error(
                DiagnosticClass::Semantic,
                "change_parameter_order_membership",
                format!(
                    "candidate value-parameter inventory for {owner} contains duplicate identities"
                ),
            ));
        }
        let mut selected = BTreeSet::new();
        let mut ordered = Vec::new();
        ordered.try_reserve_exact(parameters.len()).map_err(|_| {
            request_error(
                DiagnosticClass::Resource,
                "change_parameter_order_allocation",
                "complete value-parameter order allocation failed",
            )
        })?;
        for selector in parameters {
            lowerer.base.validation_checkpoint()?;
            let parameter_owner = lowerer.resolve_owner(selector)?;
            let OwnerKey::Parameter(parameter) = parameter_owner else {
                return Err(request_error(
                    DiagnosticClass::Semantic,
                    "change_parameter_order_kind",
                    format!("ordered member {parameter_owner} is not a value parameter"),
                ));
            };
            if !selected.insert(parameter) {
                return Err(request_error(
                    DiagnosticClass::Semantic,
                    "change_parameter_order_duplicate",
                    format!("complete value-parameter order repeats {parameter_owner}"),
                ));
            }
            let OwnerRecord::Parameter(value) = record(lowerer, parameter_owner)? else {
                return Err(request_error(
                    DiagnosticClass::Corrupt,
                    "change_parameter_order_cache",
                    "selected parameter identity has a foreign candidate record",
                ));
            };
            if value.parent != parent {
                return Err(request_error(
                    DiagnosticClass::Semantic,
                    "change_parameter_order_foreign_parent",
                    format!("ordered parameter {parameter_owner} does not belong to {owner}"),
                ));
            }
            if !expected.contains(&parameter) {
                return Err(request_error(
                    DiagnosticClass::Semantic,
                    "change_parameter_order_membership",
                    format!(
                        "ordered parameter {parameter_owner} is absent from the surviving inventory of {owner}"
                    ),
                ));
            }
            ordered.push(parameter);
        }
        // Equal cardinalities, uniqueness and membership prove a complete bijection.
        // No child is created, detached, reparented or retired by this operation.
        match lowerer.candidate_mut(owner)? {
            OwnerRecord::Declaration(declaration) => match &mut declaration.payload {
                DeclarationPayload::Function(function) => function.parameters = ordered,
                DeclarationPayload::External(external) => external.parameters = ordered,
                _ => return Err(parent_error(owner)),
            },
            OwnerRecord::Operation(operation) => operation.parameters = ordered,
            _ => return Err(parent_error(owner)),
        }
    }
    Ok(())
}

fn record<'a, B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &'a AuthoredLowerer<'_, B, W>,
    owner: OwnerKey,
) -> Result<&'a OwnerRecord, Diagnostic> {
    lowerer
        .owners
        .get(&owner)
        .map(|working| &working.record)
        .ok_or_else(|| {
            request_error(
                DiagnosticClass::Corrupt,
                "change_parameter_order_cache",
                "resolved value-parameter order owner is absent from the candidate cache",
            )
        })
}

fn parameter_ids(record: &OwnerRecord) -> Result<&[ParameterId], Diagnostic> {
    match record {
        OwnerRecord::Declaration(declaration) => match &declaration.payload {
            DeclarationPayload::Function(function) => Ok(&function.parameters),
            DeclarationPayload::External(external) => Ok(&external.parameters),
            _ => Err(parent_error(record.owner())),
        },
        OwnerRecord::Operation(operation) => Ok(&operation.parameters),
        _ => Err(parent_error(record.owner())),
    }
}

fn parent_error(owner: OwnerKey) -> Diagnostic {
    request_error(
        DiagnosticClass::Semantic,
        "change_parameter_order_parent",
        format!("selected owner {owner} is not a callable declaration or interface operation"),
    )
}

fn charge<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    steps: BudgetCounter,
) -> Result<(), Diagnostic> {
    lowerer.work.saturation.charge(
        &mut lowerer.work.ownership_steps,
        BudgetDimension::ImpactOwnershipSteps,
        steps,
        lowerer.budget.impact.maximum_ownership_steps,
        "complete value-parameter order admission",
    )
}

#[cfg(test)]
#[path = "parameter_order_tests.rs"]
mod tests;
