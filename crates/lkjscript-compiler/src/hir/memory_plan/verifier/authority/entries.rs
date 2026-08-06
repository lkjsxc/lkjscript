use super::*;

pub(super) fn authority_entry_type<'a>(
    program: &'a hir::Program,
    facts: &'a Facts<'a>,
    entry: &MemoryPlanEntry,
) -> Result<&'a Type> {
    match entry.subject {
        MemorySubject::Expression { expression, .. }
        | MemorySubject::Loan { expression, .. }
        | MemorySubject::Constant { expression, .. }
        | MemorySubject::Call { expression, .. } => facts
            .expression(expression)
            .map(|item| &item.expression.ty)
            .ok_or_else(|| Error::msg("memory authority entry lost expression type")),
        MemorySubject::Parameter {
            function, index, ..
        } => authority_parameter_type(program, function, index),
        MemorySubject::Result { function } => authority_result_type(program, function),
        MemorySubject::Place { binding, .. } => program
            .binding(BindingId::new(binding))
            .map(|binding| &binding.ty)
            .ok_or_else(|| Error::msg("memory authority place lost type")),
    }
}

pub(super) fn verify_no_partial_or_affine_copy(
    facts: &Facts<'_>,
    types: &mut VerifiedTypes<'_>,
) -> Result<()> {
    for fact in &facts.expressions {
        let type_id = types.intern(&fact.expression.ty)?;
        let mode = types.expected(type_id)?.derived.mode;
        let projection_load = match &fact.expression.kind {
            hir::ExprKind::ProductField { value, .. }
            | hir::ExprKind::WithProductField { value, .. }
            | hir::ExprKind::EnumField { value, .. }
            | hir::ExprKind::EnumUnwrap { value, .. } => {
                matches!(value.kind, hir::ExprKind::Load(_))
            }
            _ => false,
        };
        if projection_load && mode == MemoryAggregateMode::Affine {
            return Err(Error::msg(format!(
                "LKJ-MEM-PARTIAL-MOVE type={:?}",
                verified_memory_type(&fact.expression.ty)
            )));
        }
        if matches!(fact.expression.kind, hir::ExprKind::Load(_))
            && matches!(fact.expression.ty, Type::Product(_) | Type::Enum { .. })
            && mode == MemoryAggregateMode::Affine
        {
            return Err(Error::msg(format!(
                "LKJ-MEM-AFFINE-AGGREGATE-COPY type={:?}",
                verified_memory_type(&fact.expression.ty)
            )));
        }
    }
    Ok(())
}

fn authority_parameter_type(
    program: &hir::Program,
    function: MemoryFunctionId,
    index: u64,
) -> Result<&Type> {
    let fi = function
        .index()
        .ok_or_else(|| Error::msg("memory function exceeds usize"))?;
    let pi = usize::try_from(index).map_err(|_| Error::msg("memory parameter exceeds usize"))?;
    if let Some(function) = program.functions.get(fi) {
        let binding = *function
            .params
            .get(pi)
            .ok_or_else(|| Error::msg("memory parameter is missing"))?;
        program
            .binding(binding)
            .map(|binding| &binding.ty)
            .ok_or_else(|| Error::msg("memory parameter type is missing"))
    } else if fi == program.functions.len() {
        program
            .main
            .param_types
            .get(pi)
            .ok_or_else(|| Error::msg("main parameter type is missing"))
    } else {
        Err(Error::msg("memory parameter function is missing"))
    }
}

fn authority_result_type(program: &hir::Program, function: MemoryFunctionId) -> Result<&Type> {
    let fi = function
        .index()
        .ok_or_else(|| Error::msg("memory function exceeds usize"))?;
    if let Some(function) = program.functions.get(fi) {
        let binding = program
            .binding(function.binding)
            .ok_or_else(|| Error::msg("memory result binding missing"))?;
        callable_result(&binding.ty)
    } else if fi == program.functions.len() {
        Ok(&program.main.return_type)
    } else {
        Err(Error::msg("memory result function is missing"))
    }
}
