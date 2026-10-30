//! Same-kind scalar updates within a live function's existing expression ownership tree.

use super::{AuthoredExpressionOperation, AuthoredLowerer, DeclarationSelector};
use crate::platform::binary64::Binary64;
use crate::platform::change::{BudgetCounter, BudgetDimension, CanonicalBaseRead, WitnessBaseRead};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DeclarationPayload, ExpressionOperation, OwnerKey, OwnerRecord, TextValue,
};
use crate::platform::semantic_id::ExpressionId;
use crate::platform::witness::aggregation_children_with_bindings;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AuthoredLiteralValue {
    Bool { value: bool },
    I64 { value: i64 },
    F64 { value: Binary64 },
    Text { value: String },
    StaticText { value: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredLiteralUpdate {
    pub expression: ExpressionId,
    pub value: AuthoredLiteralValue,
}

impl AuthoredLiteralValue {
    pub(crate) fn from_authored(operation: &AuthoredExpressionOperation) -> Option<Self> {
        use AuthoredExpressionOperation as A;
        Some(match operation {
            A::Bool { value } => Self::Bool { value: *value },
            A::I64 { value } => Self::I64 { value: *value },
            A::F64 { value } => Self::F64 { value: *value },
            A::Text { value } => Self::Text {
                value: value.clone(),
            },
            A::StaticText { value } => Self::StaticText {
                value: value.clone(),
            },
            _ => return None,
        })
    }

    pub(crate) fn authored(&self) -> AuthoredExpressionOperation {
        use AuthoredExpressionOperation as A;
        match self {
            Self::Bool { value } => A::Bool { value: *value },
            Self::I64 { value } => A::I64 { value: *value },
            Self::F64 { value } => A::F64 { value: *value },
            Self::Text { value } => A::Text {
                value: value.clone(),
            },
            Self::StaticText { value } => A::StaticText {
                value: value.clone(),
            },
        }
    }

    fn accepts(&self, operation: &ExpressionOperation) -> bool {
        matches!(
            (self, operation),
            (Self::Bool { .. }, ExpressionOperation::Bool { .. })
                | (Self::I64 { .. }, ExpressionOperation::I64 { .. })
                | (Self::F64 { .. }, ExpressionOperation::F64 { .. })
                | (Self::Text { .. }, ExpressionOperation::Text { .. })
                | (
                    Self::StaticText { .. },
                    ExpressionOperation::StaticText { .. }
                )
        )
    }

    fn operation(&self) -> ExpressionOperation {
        match self {
            Self::Bool { value } => ExpressionOperation::Bool { value: *value },
            Self::I64 { value } => ExpressionOperation::I64 { value: *value },
            Self::F64 { value } => ExpressionOperation::F64 { value: *value },
            Self::Text { value } => ExpressionOperation::Text {
                value: TextValue::Inline {
                    text: value.clone(),
                },
            },
            Self::StaticText { value } => ExpressionOperation::StaticText {
                value: TextValue::Inline {
                    text: value.clone(),
                },
            },
        }
    }
}

pub(super) fn lower<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    function: &DeclarationSelector,
    literals: &[AuthoredLiteralUpdate],
) -> Result<(), Diagnostic> {
    if literals.is_empty() {
        return Err(error(
            "change_literal_empty",
            "literal update requires a nonempty selection",
        ));
    }
    lowerer.budget.check_canonical_edit_counts(
        u64::try_from(literals.len()).unwrap_or(u64::MAX),
        0,
        0,
        0,
        "function literal selection admission",
    )?;
    let function = lowerer.resolve_declaration(function)?;
    let record = live(lowerer, OwnerKey::Declaration(function))?;
    let OwnerRecord::Declaration(declaration) = record else {
        return Err(error(
            "change_literal_function",
            "literal update requires a function",
        ));
    };
    let DeclarationPayload::Function(function) = declaration.payload else {
        return Err(error(
            "change_literal_function",
            "literal update requires a function",
        ));
    };
    let mut selected = BTreeMap::new();
    for literal in literals {
        if selected
            .insert(literal.expression, &literal.value)
            .is_some()
        {
            return Err(error(
                "change_literal_duplicate",
                "literal update repeats an expression",
            ));
        }
    }
    let mut pending = vec![OwnerKey::Expression(function.body)];
    let mut seen = BTreeSet::new();
    charge(lowerer, 1)?;
    // Read the actual candidate, not a stale ownership index or a request-supplied path.
    // Finish all membership/kind checks before changing any literal in this operation.
    while let Some(owner) = pending.pop() {
        if !seen.insert(owner) {
            return Err(error(
                "change_literal_structure",
                "function body repeats an owned identity",
            ));
        }
        let record = live(lowerer, owner)?;
        match &record {
            OwnerRecord::Expression(expression) => {
                if let Some(value) = selected.remove(&expression.id)
                    && !value.accepts(&expression.operation)
                {
                    return Err(error(
                        "change_literal_kind",
                        "literal update must preserve its scalar kind",
                    ));
                }
            }
            OwnerRecord::Binding(_) => {}
            _ => {
                return Err(error(
                    "change_literal_structure",
                    "function body contains a foreign owner kind",
                ));
            }
        }
        let children = aggregation_children_with_bindings(&record, |binding| {
            match live(lowerer, OwnerKey::Binding(binding))? {
                OwnerRecord::Binding(record) => Ok(Some(record)),
                _ => Err(error(
                    "change_literal_structure",
                    "function body binding identity selects a foreign owner kind",
                )),
            }
        })?;
        charge(lowerer, children.len())?;
        pending.extend(children.into_iter().map(|(_, child)| child));
    }
    if !selected.is_empty() {
        return Err(error(
            "change_literal_foreign",
            "selected literal is not live within this function",
        ));
    }
    for literal in literals {
        let record = lowerer.candidate_mut(OwnerKey::Expression(literal.expression))?;
        let OwnerRecord::Expression(expression) = record else {
            return Err(error(
                "change_literal_kind",
                "selected identity is not an expression",
            ));
        };
        expression.operation = literal.value.operation();
    }
    Ok(())
}

fn live<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    owner: OwnerKey,
) -> Result<OwnerRecord, Diagnostic> {
    lowerer.require_owner(owner)?;
    let working = lowerer.owners.get(&owner).ok_or_else(|| {
        error(
            "change_literal_owner",
            "literal traversal lost its candidate owner",
        )
    })?;
    if working.original.is_none() {
        return Err(error(
            "change_literal_created",
            "literal update requires existing accepted identities",
        ));
    }
    if working.deleted {
        return Err(error(
            "change_literal_deleted",
            "literal update requires live accepted identities",
        ));
    }
    lowerer.check_budget("function literal canonical reads")?;
    Ok(working.record.clone())
}

fn charge<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    count: usize,
) -> Result<(), Diagnostic> {
    let amount = match u64::try_from(count) {
        Ok(count) => BudgetCounter::exact(count),
        Err(_) => BudgetCounter {
            consumed: u64::MAX,
            saturated: true,
        },
    };
    lowerer.work.saturation.charge(
        &mut lowerer.work.ownership_steps,
        BudgetDimension::ImpactOwnershipSteps,
        amount,
        lowerer.budget.impact.maximum_ownership_steps,
        "function literal ownership traversal",
    )?;
    lowerer.check_budget("function literal ownership traversal")
}

fn error(code: &'static str, message: &'static str) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}

#[cfg(test)]
mod tests {
    use super::super::AuthoredLoweringInputs;
    use super::*;
    use crate::platform::change::ChangeBudget;
    use crate::platform::publication::GraphRepository;

    #[test]
    fn literal_ownership_charge_preserves_exact_policy_and_saturation() {
        let memory = crate::platform::MemProjectStore::new(None).unwrap();
        let repository = GraphRepository::create_memory(
            &memory,
            &crate::platform::kernel::tests::witness_snapshot(),
            None,
        )
        .unwrap()
        .repository;
        let view = repository.view_current().unwrap();
        let before_head = repository.current().unwrap().head;
        let before_root = crate::platform::kernel::encode_root(view.semantic_root())
            .unwrap()
            .0;
        let make = |maximum| {
            let mut budget = ChangeBudget::default();
            budget.impact.maximum_ownership_steps = maximum;
            AuthoredLowerer::new(
                &view,
                &view,
                AuthoredLoweringInputs {
                    intent_base: view.revision(),
                    allocation_seed: [3; 32],
                    deletion_change: crate::platform::kernel::ChangeDigest::of(
                        b"literal ownership charge policy",
                    ),
                    allocated: BTreeMap::new(),
                    definitions: BTreeMap::new(),
                    allocations: Vec::new(),
                    budget,
                },
            )
            .unwrap()
        };
        let dimension = BudgetDimension::ImpactOwnershipSteps;
        let expected_code = "change_budget_impact_ownership_steps";
        for maximum in [None, Some(0), Some(1), Some(2), Some(u64::MAX)] {
            let mut lowerer = make(maximum);
            let before = lowerer.work;
            let result = charge(&mut lowerer, 2);
            if matches!(maximum, Some(0 | 1)) {
                let error = result.unwrap_err();
                assert_eq!(error.class, DiagnosticClass::Resource);
                assert_eq!(error.code, expected_code);
                assert_eq!(lowerer.work, before);
            } else {
                result.unwrap();
                let mut expected = before;
                expected.ownership_steps = 2;
                assert_eq!(lowerer.work, expected);
            }
            assert!(
                lowerer.owner_edits.is_empty()
                    && lowerer.dependency_edits.is_empty()
                    && lowerer.retirement_edits.is_empty()
                    && lowerer.type_additions.is_empty()
            );
        }

        let mut zero = make(Some(0));
        let before_zero = zero.work;
        charge(&mut zero, 0).unwrap();
        assert_eq!(zero.work, before_zero);

        let mut finite = make(Some(u64::MAX));
        finite.work.ownership_steps = u64::MAX - 1;
        charge(&mut finite, 1).unwrap();
        assert_eq!(
            finite
                .work
                .saturation
                .counter(dimension, finite.work.ownership_steps),
            BudgetCounter::exact(u64::MAX)
        );
        let exact_maximum = finite.work;
        charge(&mut finite, 0).unwrap();
        assert_eq!(finite.work, exact_maximum);
        let error = charge(&mut finite, 1).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Resource);
        assert_eq!(error.code, expected_code);
        assert_eq!(finite.work, exact_maximum);

        let mut unbounded = make(None);
        unbounded.work.ownership_steps = u64::MAX - 1;
        charge(&mut unbounded, 1).unwrap();
        assert_eq!(
            unbounded
                .work
                .saturation
                .counter(dimension, unbounded.work.ownership_steps),
            BudgetCounter::exact(u64::MAX)
        );
        charge(&mut unbounded, 1).unwrap();
        assert_eq!(unbounded.work.ownership_steps, u64::MAX);
        assert!(unbounded.work.saturation.contains(dimension));
        let saturated = unbounded.work;
        for count in [0, 1] {
            charge(&mut unbounded, count).unwrap();
            assert_eq!(unbounded.work, saturated);
        }
        let projected = unbounded.work.budget_work();
        assert_eq!(projected.impact_ownership_steps, u64::MAX);
        assert!(projected.saturation.contains(dimension));
        unbounded.budget.impact.maximum_ownership_steps = Some(u64::MAX);
        for count in [0, 1] {
            let error = charge(&mut unbounded, count).unwrap_err();
            assert_eq!(error.class, DiagnosticClass::Resource);
            assert_eq!(error.code, expected_code);
            assert_eq!(unbounded.work, saturated);
        }

        assert_eq!(repository.current().unwrap().head, before_head);
        let after = repository.view_current().unwrap();
        assert_eq!(
            crate::platform::kernel::encode_root(after.semantic_root())
                .unwrap()
                .0,
            before_root
        );
    }
}
