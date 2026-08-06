use super::*;

pub(super) fn verify_destinations(
    program: &hir::Program,
    plan: &HirMemoryPlan,
    facts: &Facts<'_>,
    types: &VerifiedTypes<'_>,
) -> Result<()> {
    let mut entries_by_expression = BTreeMap::new();
    let mut children_by_parent: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for entry in &plan.entries {
        if let MemorySubject::Expression {
            expression,
            parent,
            child_index,
            ..
        } = entry.subject
        {
            if entries_by_expression.insert(expression, entry).is_some() {
                return Err(Error::msg(
                    "memory destination expression entry is duplicated",
                ));
            }
            if let Some(parent) = parent {
                children_by_parent.entry(parent).or_default().push((
                    child_index,
                    expression,
                    entry.drop_path,
                ));
            }
        }
    }
    let product_indices: HashMap<_, _> = program
        .products
        .iter()
        .enumerate()
        .map(|(index, product)| (product.id, index))
        .collect();
    let enum_indices: HashMap<_, _> = program
        .enums
        .iter()
        .enumerate()
        .map(|(index, enumeration)| (enumeration.id, index))
        .collect();
    for (index, destination) in plan.destinations.iter().enumerate() {
        if destination.id.raw() != index_u64(index)? {
            return Err(Error::msg("HIR memory destinations are not dense"));
        }
        let fact = facts
            .expression(destination.expression)
            .ok_or_else(|| Error::msg("memory destination lost construction expression"))?;
        let entry = entries_by_expression
            .get(&destination.expression)
            .copied()
            .ok_or_else(|| Error::msg("memory authority lost expression entry"))?;
        let type_fact = types.expected(entry.type_fact)?;
        let mut children = children_by_parent
            .remove(&destination.expression)
            .unwrap_or_default();
        children.sort_by_key(|item| item.0);
        let (field_count, active_payload) =
            verified_destination_shape(program, &product_indices, &enum_indices, fact.expression)?;
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
    let construction_count = u64::try_from(constructions)
        .map_err(|_| Error::msg("memory destination count exceeds u64"))?;
    if plan.destinations.len() != constructions || plan.work.destinations != construction_count {
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

fn verified_destination_shape(
    program: &hir::Program,
    product_indices: &HashMap<hir::ProductId, usize>,
    enum_indices: &HashMap<hir::EnumId, usize>,
    expression: &hir::Expr,
) -> Result<(u32, Option<MemoryActivePayload>)> {
    match &expression.kind {
        hir::ExprKind::ProductValue { product, fields } => {
            let declared = product_indices
                .get(product)
                .and_then(|index| program.products.get(*index))
                .filter(|item| item.id == *product)
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
            let declared = enum_indices
                .get(enum_id)
                .and_then(|index| program.enums.get(*index))
                .filter(|item| item.id == *enum_id)
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
