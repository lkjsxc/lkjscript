use super::*;
use crate::hir::Type;

pub(super) fn charge_type(root: &Type, ledger: &mut BudgetLedger) -> Result<()> {
    let mut stack = Vec::new();
    stack
        .try_reserve(1)
        .map_err(|_| Error::msg("cannot reserve type accounting stack"))?;
    stack.push(root);
    while let Some(ty) = stack.pop() {
        charge(ledger, ResourceCategory::TypeWork, 1)?;
        let growth = match ty {
            Type::Never => 0,
            Type::Owned(_)
            | Type::Ref(_)
            | Type::RefMut(_)
            | Type::List(_)
            | Type::Forall { .. } => 1,
            Type::Enum { arguments, .. } => arguments.len(),
            Type::Fn { params, .. } => params
                .len()
                .checked_add(1)
                .ok_or_else(|| Error::msg("type accounting stack growth overflow"))?,
            _ => 0,
        };
        stack
            .try_reserve(growth)
            .map_err(|_| Error::msg("cannot reserve type accounting stack"))?;
        charge_usize(ledger, ResourceCategory::TypeNesting, growth)?;
        match ty {
            Type::Owned(child)
            | Type::Ref(child)
            | Type::RefMut(child)
            | Type::List(child)
            | Type::Forall { body: child, .. } => stack.push(child),
            Type::Enum { arguments, .. } => stack.extend(arguments),
            Type::Fn { params, ret } => {
                stack.extend(params);
                stack.push(ret);
            }
            _ => {}
        }
    }
    Ok(())
}
