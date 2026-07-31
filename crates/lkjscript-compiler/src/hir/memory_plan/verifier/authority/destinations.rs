use super::*;

pub(super) fn verify_destinations(
    program: &hir::Program,
    plan: &HirMemoryPlan,
    facts: &Facts<'_>,
    types: &VerifiedTypes<'_>,
) -> Result<()> {
    for (index, destination) in plan.destinations.iter().enumerate() {
        if destination.id.raw() != index_u32(index)? {
            return Err(Error::msg("HIR memory destinations are not dense"));
        }
        let fact = facts
            .expressions
            .iter()
            .find(|item| item.id == destination.expression)
            .ok_or_else(|| Error::msg("memory destination lost construction expression"))?;
        let entry = expression_entry(plan, destination.expression)?;
        let type_fact = types.expected(entry.type_fact)?;
        let mut children: Vec<_> = plan
            .entries
            .iter()
            .filter_map(|entry| match entry.subject {
                MemorySubject::Expression {
                    expression,
                    parent: Some(parent),
                    child_index,
                    ..
                } if parent == destination.expression => {
                    Some((child_index, expression, entry.drop_path))
                }
                _ => None,
            })
            .collect();
        children.sort_by_key(|item| item.0);
        let (field_count, active_payload) = verified_destination_shape(program, fact.expression)?;
        let fields = children
            .into_iter()
            .map(|(index, expression, drop_path)| MemoryDestinationField {
                index,
                expression,
                drop_path,
            })
            .collect::<Vec<_>>();
        let initialized_order: Vec<u32> = (0..field_count).collect();
        let (kind, execution, execution_cutover) = match type_fact.derived.closure.class {
            MemoryClosureClass::Deterministic => (
                MemoryDestinationKind::CutoverRequired,
                MemoryExecution::CutoverRequired,
                verified_execution_cutover(&fact.expression.ty),
            ),
            MemoryClosureClass::RegionClosed => (
                MemoryDestinationKind::OrdinaryRegion,
                MemoryExecution::Current,
                None,
            ),
            MemoryClosureClass::Unresolved | MemoryClosureClass::IllegalDomainBridge => (
                MemoryDestinationKind::UnsupportedRuntime,
                MemoryExecution::CutoverRequired,
                None,
            ),
        };
        let expected = MemoryDestinationPlan {
            id: destination.id,
            function: fact.function,
            expression: fact.id,
            kind,
            execution,
            execution_cutover,
            type_fact: entry.type_fact,
            field_count,
            fields,
            active_payload,
            initialized_order: initialized_order.clone(),
            reverse_abort_cleanup: initialized_order.into_iter().rev().collect(),
        };
        if destination != &expected || entry.destination != Some(destination.id) {
            return Err(Error::msg(
                "LKJ-MEM-INCOMPLETE-DESTINATION verifier mismatch",
            ));
        }
    }
    let constructions = facts
        .expressions
        .iter()
        .filter(|fact| {
            matches!(
                fact.expression.kind,
                hir::ExprKind::ProductValue { .. } | hir::ExprKind::EnumValue { .. }
            )
        })
        .count();
    if plan.destinations.len() != constructions
        || plan.work.destinations != u64::try_from(constructions).unwrap_or(u64::MAX)
    {
        return Err(Error::msg("memory destination coverage/work mismatch"));
    }
    for entry in &plan.entries {
        let construction = matches!(
            entry.subject,
            MemorySubject::Expression {
                kind: MemoryExpressionKind::ProductValue | MemoryExpressionKind::EnumValue,
                ..
            }
        );
        if construction != entry.destination.is_some() {
            return Err(Error::msg("memory destination eligibility mismatch"));
        }
    }
    Ok(())
}

fn expression_entry(
    plan: &HirMemoryPlan,
    expression: MemoryExpressionId,
) -> Result<&MemoryPlanEntry> {
    plan.entries
        .iter()
        .find(|entry| {
            matches!(entry.subject,
        MemorySubject::Expression { expression: item, .. } if item == expression)
        })
        .ok_or_else(|| Error::msg("memory authority lost expression entry"))
}

fn verified_destination_shape(
    program: &hir::Program,
    expression: &hir::Expr,
) -> Result<(u32, Option<MemoryActivePayload>)> {
    match &expression.kind {
        hir::ExprKind::ProductValue { product, fields } => {
            let declared = program
                .products
                .iter()
                .find(|item| item.id == *product)
                .ok_or_else(|| Error::msg("memory verifier lost destination product"))?;
            if declared.fields.len() != fields.len() {
                return Err(Error::msg("LKJ-MEM-INCOMPLETE-DESTINATION product fields"));
            }
            Ok((index_u32(fields.len())?, None))
        }
        hir::ExprKind::EnumValue {
            enum_id,
            variant,
            fields,
            ..
        } => {
            let declared = program
                .enums
                .iter()
                .find(|item| item.id == *enum_id)
                .and_then(|item| item.variants.iter().find(|item| item.id == *variant))
                .ok_or_else(|| Error::msg("memory verifier lost active enum payload"))?;
            if declared.fields.len() != fields.len() {
                return Err(Error::msg("LKJ-MEM-INCOMPLETE-DESTINATION enum fields"));
            }
            Ok((
                index_u32(fields.len())?,
                Some(MemoryActivePayload {
                    variant: variant.bytes(),
                    source_order: declared.source_order,
                }),
            ))
        }
        _ => Err(Error::msg("memory destination references non-construction")),
    }
}
