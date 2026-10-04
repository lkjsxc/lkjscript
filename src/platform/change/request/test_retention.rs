//! Retain accepted test trees only after a complete typed literal-edit proof.
use super::*;
use crate::platform::kernel::{ExpressionOperation as E, LocalValueReference as L};
use crate::platform::witness::aggregation_children;

pub(super) fn retain<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    before: [ExpressionId; 2],
    after: [ExpressionId; 2],
) -> Result<bool, Diagnostic> {
    let mut pending = before
        .into_iter()
        .zip(after)
        .map(|(old, new)| (OwnerKey::Expression(old), OwnerKey::Expression(new)))
        .collect::<Vec<_>>();
    let mut identities = BTreeMap::new();
    let mut accepted = BTreeSet::new();
    let mut pairs = Vec::new();
    while let Some((old, new)) = pending.pop() {
        charge(lowerer, 1)?;
        if identities.insert(new, old).is_some() || !accepted.insert(old) {
            return Ok(false);
        }
        lowerer.require_owner(old)?;
        let previous = lowerer.owners.get(&old).ok_or_else(missing)?;
        if previous.original.is_none() || previous.deleted {
            return Ok(false);
        }
        let previous = previous.record.clone();
        let proposed = lowerer.owners.get(&new).ok_or_else(missing)?;
        if proposed.original.is_some() || proposed.deleted {
            return Ok(false);
        }
        let proposed = proposed.record.clone();
        if !matches!(
            (&previous, &proposed),
            (OwnerRecord::Expression(_), OwnerRecord::Expression(_))
                | (OwnerRecord::Binding(_), OwnerRecord::Binding(_))
        ) {
            return Ok(false);
        }
        let old_children = aggregation_children(&previous)?;
        let new_children = aggregation_children(&proposed)?;
        if old_children.len() != new_children.len() {
            return Ok(false);
        }
        charge(lowerer, old_children.len())?;
        for ((old_role, old), (new_role, new)) in old_children.into_iter().zip(new_children) {
            if old_role != new_role {
                return Ok(false);
            }
            pending.push((old, new));
        }
        pairs.push((old, previous, proposed));
    }

    let mut updates = Vec::new();
    for (old, previous, mut proposed) in pairs {
        charge(lowerer, 1)?;
        match (&previous, &mut proposed) {
            (OwnerRecord::Expression(previous), OwnerRecord::Expression(proposed)) => {
                proposed.id = previous.id;
                proposed.contract_version = previous.contract_version;
                remap(&mut proposed.operation, &identities);
                if same_scalar(&previous.operation, &proposed.operation) {
                    if previous.operation != proposed.operation {
                        updates.push((old, proposed.operation.clone()));
                    }
                    proposed.operation = previous.operation.clone();
                }
            }
            (OwnerRecord::Binding(previous), OwnerRecord::Binding(proposed)) => {
                proposed.header = previous.header;
                if let Some(value) = &mut proposed.value {
                    map_expression(value, &identities);
                }
            }
            _ => return Ok(false),
        }
        if proposed != previous {
            return Ok(false);
        }
    }

    // Earlier operations may already reference a request-local identity. Do not
    // discard such an owner; ordinary replacement preserves those exact references.
    let owner_count = lowerer.owners.len();
    charge(lowerer, owner_count)?;
    let owners = lowerer.owners.keys().copied().collect::<Vec<_>>();
    for owner in owners {
        if identities.contains_key(&owner) {
            continue;
        }
        let record = &lowerer.owners.get(&owner).ok_or_else(missing)?.record;
        if aggregation_children(record)?
            .iter()
            .any(|(_, child)| identities.contains_key(child))
        {
            return Ok(false);
        }
        let referenced = match record {
            OwnerRecord::Expression(expression) => match expression.operation {
                E::Local {
                    value: L::LexicalBinding(id) | L::MatchPayload(id) | L::TransactionBinding(id),
                } => Some(OwnerKey::Binding(id)),
                _ => None,
            },
            OwnerRecord::Documentation(record) => Some(record.owner),
            OwnerRecord::Annotation(record) => Some(record.owner),
            _ => None,
        };
        if referenced.is_some_and(|owner| identities.contains_key(&owner)) {
            return Ok(false);
        }
    }
    // All comparisons precede edits. Only accepted scalar owners change; labels
    // supplied by a proposal played no role in establishing this correspondence.
    for (owner, operation) in updates {
        let OwnerRecord::Expression(expression) = lowerer.candidate_mut(owner)? else {
            return Err(missing());
        };
        expression.operation = operation;
    }
    for owner in identities.keys() {
        lowerer.owners.remove(owner);
        lowerer.owner_edits.remove(owner);
    }
    lowerer
        .allocations
        .retain(|allocation| !identities.contains_key(&allocation.owner));
    // Later operations using the same request symbol must select the retained
    // accepted identity, never a discarded speculative allocation.
    for owner in lowerer.allocated.values_mut() {
        if let Some(retained) = identities.get(owner) {
            *owner = *retained;
        }
    }
    Ok(true)
}

fn same_scalar(left: &E, right: &E) -> bool {
    matches!(
        (left, right),
        (E::Bool { .. }, E::Bool { .. })
            | (E::I64 { .. }, E::I64 { .. })
            | (E::F64 { .. }, E::F64 { .. })
            | (E::Text { .. }, E::Text { .. })
            | (E::StaticText { .. }, E::StaticText { .. })
    )
}

fn map_expression(value: &mut ExpressionId, identities: &BTreeMap<OwnerKey, OwnerKey>) {
    if let Some(OwnerKey::Expression(retained)) = identities.get(&OwnerKey::Expression(*value)) {
        *value = *retained;
    }
}
fn map_binding(value: &mut BindingId, identities: &BTreeMap<OwnerKey, OwnerKey>) {
    if let Some(OwnerKey::Binding(retained)) = identities.get(&OwnerKey::Binding(*value)) {
        *value = *retained;
    }
}

fn remap(operation: &mut E, identities: &BTreeMap<OwnerKey, OwnerKey>) {
    let expression = |value: &mut ExpressionId| map_expression(value, identities);
    let binding = |value: &mut BindingId| map_binding(value, identities);
    match operation {
        E::Local { value } => match value {
            L::LexicalBinding(id) | L::MatchPayload(id) | L::TransactionBinding(id) => binding(id),
            L::FunctionParameter(_) | L::OperationParameter(_) => {}
        },
        E::If {
            condition,
            when_true,
            when_false,
        } => {
            expression(condition);
            expression(when_true);
            expression(when_false);
        }
        E::Let { bindings, body } => {
            bindings.iter_mut().for_each(binding);
            expression(body);
        }
        E::Parallel { left, right } => {
            expression(left);
            expression(right);
        }
        E::Sequence { items }
        | E::List { items, .. }
        | E::Call {
            arguments: items, ..
        }
        | E::CapabilityCall {
            arguments: items, ..
        }
        | E::ImplementationCall {
            arguments: items, ..
        }
        | E::MethodCall {
            arguments: items, ..
        } => {
            items.iter_mut().for_each(expression);
        }
        E::Invoke { callee, arguments } | E::Bind { callee, arguments } => {
            expression(callee);
            arguments.iter_mut().for_each(expression);
        }
        E::Record { fields, .. } => fields
            .iter_mut()
            .for_each(|field| expression(&mut field.value)),
        E::PackOwned { fields, .. } => fields
            .iter_mut()
            .for_each(|field| expression(&mut field.value)),
        E::Variant { payload, .. } => {
            if let Some(payload) = payload {
                expression(payload);
            }
        }
        E::Field { value, .. } | E::ChooseOwned { value, .. } => expression(value),
        E::Map { entries, .. } => entries.iter_mut().for_each(|entry| {
            expression(&mut entry.key);
            expression(&mut entry.value);
        }),
        E::Match { value, arms } => {
            expression(value);
            for arm in arms {
                expression(&mut arm.body);
                if let Some(payload) = &mut arm.payload_binding {
                    binding(payload);
                }
            }
        }
        E::Transaction {
            binding: id, body, ..
        }
        | E::TransactionOutcome {
            binding: id, body, ..
        } => {
            binding(id);
            expression(body);
        }
        E::UnpackOwned {
            source,
            fields,
            body,
            ..
        } => {
            expression(source);
            expression(body);
            for field in fields {
                binding(&mut field.binding);
            }
        }
        E::MatchOwned { source, arms, .. } | E::MatchBorrowedOwned { source, arms, .. } => {
            expression(source);
            for arm in arms {
                binding(&mut arm.binding);
                expression(&mut arm.body);
            }
        }
        E::BorrowOwnedField {
            source,
            binding: id,
            body,
            ..
        } => {
            expression(source);
            binding(id);
            expression(body);
        }
        E::BorrowOwnedItem {
            source,
            index,
            binding: id,
            body,
            ..
        } => {
            expression(source);
            expression(index);
            binding(id);
            expression(body);
        }
        E::SequenceLength { source, .. } | E::SequencePop { source, .. } => expression(source),
        E::SequencePush { value, source, .. } => {
            expression(value);
            expression(source);
        }
        E::Unit {}
        | E::SequenceEmpty { .. }
        | E::Bool { .. }
        | E::I64 { .. }
        | E::F64 { .. }
        | E::Text { .. }
        | E::StaticText { .. }
        | E::Constant { .. }
        | E::FunctionValue { .. } => {}
    }
}

fn charge<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    lowerer: &mut AuthoredLowerer<'_, B, W>,
    count: usize,
) -> Result<(), Diagnostic> {
    lowerer.work.ownership_steps = lowerer
        .work
        .ownership_steps
        .saturating_add(u64::try_from(count).unwrap_or(u64::MAX));
    lowerer.check_budget("test literal ownership correspondence")
}
fn missing() -> Diagnostic {
    request_error(
        DiagnosticClass::Corrupt,
        "change_literal_owner",
        "test literal correspondence lost its candidate owner",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::control::{
        decode_compact_change, decode_compact_change_in_repository, render_native_draft,
    };
    use crate::platform::execution::ExecutionControl;
    use crate::platform::publication::GraphRepository;

    #[test]
    fn test_literal_retention_preserves_generated_binders_and_later_symbol_selection() {
        let temporary = tempfile::tempdir().unwrap();
        let created = GraphRepository::create(
            &temporary.path().join("meaning"),
            &crate::platform::kernel::tests::witness_snapshot(),
            None,
        )
        .unwrap();
        let repository = created.repository;
        let source = format!(
            r#"request base={}
declarations.begin
(units (module create test-retention
  (test create nested (as $test) (visibility private)
    (actual (record structural (field z (i64 7)) (field a (i64 1))))
    (expected (record structural (field z (i64 7)) (field a (i64 1)))))))
declarations.end
"#,
            repository.view_current().unwrap().revision(),
        );
        let request = decode_compact_change("retained-test", source.as_bytes()).unwrap();
        let initial = repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap();
        let test = initial.allocated["$test"];
        repository.publish(&initial.publication).unwrap();
        let draft = |repository: &GraphRepository| {
            String::from_utf8(
                render_native_draft(
                    &repository.view_current().unwrap(),
                    &[test.into()],
                    1_048_576,
                    ExecutionControl::uncancelled(),
                )
                .unwrap(),
            )
            .unwrap()
        };
        let original = draft(&repository);
        assert!(original.contains("(binding record-field"));
        let edited = original.replace("(i64 7)", "(i64 8)");
        let request = decode_compact_change_in_repository(
            "retained-test-edit",
            edited.as_bytes(),
            &repository,
        )
        .unwrap();
        let binding_symbol = request
            .semantic
            .changes
            .iter()
            .find_map(|change| {
                let AuthoredChange::SetTest { actual, .. } = change else {
                    return None;
                };
                let AuthoredExpressionOperation::Let { bindings, .. } = &actual.operation else {
                    return None;
                };
                Some(bindings[0].symbol.clone())
            })
            .unwrap();
        let prepared = repository
            .prepare_authored_change(&request.semantic, request.options.clone())
            .unwrap();
        assert!(prepared.logical_plan.allocations.is_empty());
        assert!(prepared.logical_plan.retirements.is_empty());
        let retained = prepared.allocated[&binding_symbol];
        let before = repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        assert!(matches!(
            before.owners.get(&retained),
            Some(OwnerRecord::Binding(_))
        ));

        let mut composed = request.semantic;
        composed.changes.push(AuthoredChange::RenameOwner {
            owner: OwnerSelector::Symbol {
                symbol: binding_symbol.clone(),
            },
            name: Name::new("retained-field").unwrap(),
        });
        let composed = repository
            .prepare_authored_change(&composed, request.options)
            .unwrap();
        assert_eq!(composed.allocated[&binding_symbol], retained);
        repository.publish(&composed.publication).unwrap();
        let after = repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        assert_eq!(
            before.owners.keys().collect::<Vec<_>>(),
            after.owners.keys().collect::<Vec<_>>()
        );
        assert_eq!(
            after.owners[&retained].name().unwrap().as_str(),
            "retained-field"
        );
        assert!(draft(&repository).contains("(i64 8)"));

        let changed_shape =
            draft(&repository).replace("(i64 8)", "(if (bool true) (i64 8) (i64 9))");
        let request = decode_compact_change_in_repository(
            "replaced-test-shape",
            changed_shape.as_bytes(),
            &repository,
        )
        .unwrap();
        let replacement = repository
            .prepare_authored_change(&request.semantic, request.options)
            .unwrap();
        assert!(!replacement.logical_plan.allocations.is_empty());
        assert!(!replacement.logical_plan.retirements.is_empty());
        repository.publish(&replacement.publication).unwrap();
        let after = repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        crate::platform::kernel::validate_full(&after).unwrap();
    }
}
