use crate::ownership::*;

pub(in crate::ownership) fn check_arguments(
    program: &Program,
    args: &[Expr],
    places: &BTreeMap<BindingId, PlaceId>,
    state: &mut State,
    future: &BTreeSet<BindingId>,
) -> Result<()> {
    let mut temporary = Vec::new();
    for (index, argument) in args.iter().enumerate() {
        let mut later = future.clone();
        for item in &args[index.saturating_add(1)..] {
            later.extend(uses(item));
        }
        let context = if is_ref(&argument.ty) || is_ref_mut(&argument.ty) {
            UseContext::ExactReferenceArgument
        } else {
            UseContext::Ordinary
        };
        check_expr(program, argument, places, state, &later, context)?;
        if let ExprKind::Borrow { place, loan, .. } = argument.kind {
            temporary.push((place, loan));
        }
    }
    for (place, loan) in temporary {
        end_loan(state, place, loan);
    }
    Ok(())
}

pub(in crate::ownership) fn check_sequence(
    program: &Program,
    expressions: &[Expr],
    places: &BTreeMap<BindingId, PlaceId>,
    state: &mut State,
    future: &BTreeSet<BindingId>,
) -> Result<()> {
    for (index, expression) in expressions.iter().enumerate() {
        let mut later = future.clone();
        for item in &expressions[index.saturating_add(1)..] {
            later.extend(uses(item));
        }
        check_expr(
            program,
            expression,
            places,
            state,
            &later,
            UseContext::Ordinary,
        )?;
    }
    Ok(())
}
