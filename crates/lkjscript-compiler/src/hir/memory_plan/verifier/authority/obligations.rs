use super::*;

pub(super) fn verify_obligations(program: &hir::Program, plan: &HirMemoryPlan) -> Result<()> {
    let mut expected = BTreeMap::new();
    for entry in &plan.entries {
        match entry.subject {
            MemorySubject::Place { function, .. }
                if place_owns(program, plan, entry, function)? =>
            {
                let fact = plan
                    .type_fact(entry.type_fact)
                    .ok_or_else(|| Error::msg("drop obligation lost type fact"))?;
                if fact.mode != MemoryAggregateMode::Copy {
                    if let Some(glue) = fact.drop_glue {
                        let kind = match entry.ty {
                            MemoryType::Resource(kind) => MemoryObligationKind::DropResource(kind),
                            _ => MemoryObligationKind::DropWholeValue,
                        };
                        expected.insert(entry.id, (kind, Some(glue), fact.drop_path, true));
                    }
                }
            }
            MemorySubject::Loan { .. } => {
                expected.insert(
                    entry.id,
                    (MemoryObligationKind::EndBorrow, None, None, false),
                );
            }
            _ => {}
        }
    }
    let expected_count = u64::try_from(expected.len())
        .map_err(|_| Error::msg("memory obligation count exceeds u64"))?;
    if expected.len() != plan.obligations.len() || plan.work.obligations != expected_count {
        return Err(Error::msg("whole-value obligation coverage/work mismatch"));
    }
    let mut seen = BTreeSet::new();
    for (index, obligation) in plan.obligations.iter().enumerate() {
        if obligation.id.raw() != index_u32(index)? || !seen.insert(obligation.entry) {
            return Err(Error::msg("memory obligations are not dense and unique"));
        }
        let (kind, glue, path, has_class) = expected
            .get(&obligation.entry)
            .ok_or_else(|| Error::msg("memory obligation has no HIR basis"))?;
        if obligation.kind != *kind
            || obligation.drop_glue != *glue
            || obligation.drop_path != *path
            || obligation.drop_class.is_some() != *has_class
        {
            return Err(Error::msg(
                "independent verifier rejected whole-value obligation",
            ));
        }
    }
    Ok(())
}

fn place_owns(
    program: &hir::Program,
    plan: &HirMemoryPlan,
    entry: &MemoryPlanEntry,
    function: MemoryFunctionId,
) -> Result<bool> {
    let MemorySubject::Place { binding, .. } = entry.subject else {
        return Ok(false);
    };
    let binding_id = BindingId::new(binding);
    let fi = function
        .index()
        .ok_or_else(|| Error::msg("place function exceeds usize"))?;
    if let Some(item) = program.functions.get(fi) {
        if let Some(index) = item
            .params
            .iter()
            .position(|parameter| *parameter == binding_id)
        {
            return Ok(plan
                .function(function)
                .and_then(|function| function.signature.parameters.get(index))
                == Some(&MemoryParameterMode::Consume));
        }
    } else if fi == program.functions.len() {
        if let Some(index) = program
            .main
            .params
            .iter()
            .position(|parameter| *parameter == binding_id)
        {
            return Ok(plan
                .function(function)
                .and_then(|function| function.signature.parameters.get(index))
                == Some(&MemoryParameterMode::Consume));
        }
    }
    let binding = program
        .binding(binding_id)
        .ok_or_else(|| Error::msg("place binding is missing"))?;
    Ok(!matches!(binding.kind, hir::BindingKind::StaticBytesLocal))
}
