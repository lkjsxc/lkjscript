use super::*;
use crate::hir::{Expr, ExprKind};

pub(super) struct ExprFact<'a> {
    pub id: super::super::MemoryExpressionId,
    pub function: MemoryFunctionId,
    pub expression: &'a Expr,
    pub parent: Option<super::super::MemoryExpressionId>,
    pub child_index: u64,
}

pub(super) struct Facts<'a> {
    pub expressions: Vec<ExprFact<'a>>,
    pub children: HashMap<(MemoryExpressionId, u64), usize>,
    pub uses_by_binding: HashMap<(MemoryFunctionId, u64), Vec<usize>>,
    pub loads_by_binding: HashMap<(MemoryFunctionId, u64), Vec<usize>>,
    pub bodies: Vec<super::super::MemoryExpressionId>,
    pub parameters: u64,
    pub places: u64,
    pub uses: u64,
    pub loans: u64,
    pub constants: u64,
    pub calls: u64,
    pub obligations: u64,
    pub steps: u64,
}

impl<'a> Facts<'a> {
    pub(super) fn expression(&self, id: MemoryExpressionId) -> Option<&ExprFact<'a>> {
        id.index()
            .and_then(|index| self.expressions.get(index))
            .filter(|fact| fact.id == id)
    }

    pub(super) fn child(
        &self,
        parent: MemoryExpressionId,
        child_index: u64,
    ) -> Option<&ExprFact<'a>> {
        self.children
            .get(&(parent, child_index))
            .and_then(|index| self.expressions.get(*index))
    }

    pub(super) fn binding_use_indices(&self, function: MemoryFunctionId, binding: u64) -> &[usize] {
        self.uses_by_binding
            .get(&(function, binding))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub(super) fn binding_loads(&self, function: MemoryFunctionId, binding: u64) -> &[usize] {
        self.loads_by_binding
            .get(&(function, binding))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

pub(super) fn collect(program: &hir::Program) -> Result<Facts<'_>> {
    let mut facts = Facts {
        expressions: Vec::new(),
        children: HashMap::new(),
        uses_by_binding: HashMap::new(),
        loads_by_binding: HashMap::new(),
        bodies: Vec::new(),
        parameters: 0,
        places: 0,
        uses: 0,
        loans: 0,
        constants: 0,
        calls: 0,
        obligations: 0,
        steps: 0,
    };
    for (index, function) in program.functions.iter().enumerate() {
        let id = MemoryFunctionId::new(index_u64(index)?);
        add(&mut facts.parameters, function.params.len())?;
        add(&mut facts.places, function.param_places.len())?;
        for parameter in &function.params {
            let ty = &program
                .binding(*parameter)
                .ok_or_else(|| Error::msg("memory verifier parameter binding is missing"))?
                .ty;
            if resource_consumed(&function.body, *parameter)
                || matches!(ty, Type::Bytes | Type::ByteVector)
            {
                add(&mut facts.obligations, 1)?;
            }
        }
        let body = walk(&function.body, id, None, 0, &mut facts)?;
        facts.bodies.push(body);
    }
    let main_id = MemoryFunctionId::new(index_u64(program.functions.len())?);
    add(&mut facts.parameters, program.main.params.len())?;
    add(&mut facts.places, program.main.param_places.len())?;
    let body = walk(&program.main.body, main_id, None, 0, &mut facts)?;
    facts.bodies.push(body);
    Ok(facts)
}

fn walk<'a>(
    expression: &'a Expr,
    function: MemoryFunctionId,
    parent: Option<super::super::MemoryExpressionId>,
    child_index: u64,
    facts: &mut Facts<'a>,
) -> Result<super::super::MemoryExpressionId> {
    crate::stack::grow(|| walk_inner(expression, function, parent, child_index, facts))
}

fn walk_inner<'a>(
    expression: &'a Expr,
    function: MemoryFunctionId,
    parent: Option<super::super::MemoryExpressionId>,
    child_index: u64,
    facts: &mut Facts<'a>,
) -> Result<super::super::MemoryExpressionId> {
    let id = super::super::MemoryExpressionId::new(index_u64(facts.expressions.len())?);
    let expression_index = facts.expressions.len();
    facts.expressions.push(ExprFact {
        id,
        function,
        expression,
        parent,
        child_index,
    });
    if let Some(parent) = parent {
        if facts
            .children
            .insert((parent, child_index), expression_index)
            .is_some()
        {
            return Err(Error::msg(
                "memory verifier expression child index is duplicated",
            ));
        }
    }
    let binding = match &expression.kind {
        ExprKind::Load(reference)
        | ExprKind::Move {
            binding: reference, ..
        }
        | ExprKind::Borrow {
            binding: reference, ..
        }
        | ExprKind::BorrowBytes {
            binding: reference, ..
        } => Some(reference.binding.raw()),
        _ => None,
    };
    if let Some(binding) = binding {
        let uses = facts
            .uses_by_binding
            .entry((function, binding))
            .or_default();
        uses.try_reserve(1)
            .map_err(|_| Error::host("memory verifier binding-use index allocation failed"))?;
        uses.push(expression_index);
        if matches!(expression.kind, ExprKind::Load(_)) {
            let loads = facts
                .loads_by_binding
                .entry((function, binding))
                .or_default();
            loads
                .try_reserve(1)
                .map_err(|_| Error::host("memory verifier binding-load index allocation failed"))?;
            loads.push(expression_index);
        }
    }
    add(&mut facts.steps, 1)?;
    match &expression.kind {
        ExprKind::Hole => unreachable!("complete HIR cannot contain a hole"),
        ExprKind::UnresolvedValueReference { .. } => {
            unreachable!("complete HIR cannot contain an unresolved value reference")
        }
        ExprKind::Match { .. } => {
            unreachable!("semantic matches must be lowered before memory verification")
        }
        ExprKind::LitI64(_)
        | ExprKind::LitF64(_)
        | ExprKind::LitBool(_)
        | ExprKind::LitUnit
        | ExprKind::EmptyList
        | ExprKind::LitStr(_)
        | ExprKind::LitBytes(_)
        | ExprKind::QuoteSymbol(_) => add(&mut facts.constants, 1)?,
        ExprKind::Load(_) | ExprKind::Move { .. } => add(&mut facts.uses, 1)?,
        ExprKind::Borrow { .. } | ExprKind::BorrowBytes { .. } => {
            add(&mut facts.uses, 1)?;
            add(&mut facts.loans, 1)?;
            add(&mut facts.obligations, 1)?;
        }
        ExprKind::Call { args, .. } => {
            add(&mut facts.uses, 1)?;
            add(&mut facts.calls, 1)?;
            walk_children(args, function, id, facts)?;
        }
        ExprKind::Operation { args, .. } => {
            add(&mut facts.calls, 1)?;
            walk_children(args, function, id, facts)?;
        }
        ExprKind::F64FromI64Exact(value)
        | ExprKind::F64FromI64Rounded(value)
        | ExprKind::I64FromF64Exact(value)
        | ExprKind::I64FromF64Trunc(value)
        | ExprKind::Return { value }
        | ExprKind::Break { value, .. }
        | ExprKind::Trap { value }
        | ExprKind::Exit { code: value }
        | ExprKind::SetLocal { value, .. }
        | ExprKind::ProductField { value, .. }
        | ExprKind::EnumIsVariant { value, .. }
        | ExprKind::EnumField { value, .. }
        | ExprKind::EnumUnwrap { value, .. } => {
            walk(value, function, Some(id), 0, facts)?;
        }
        ExprKind::Do(values)
        | ExprKind::Loop { body: values, .. }
        | ExprKind::ProductValue { fields: values, .. }
        | ExprKind::EnumValue { fields: values, .. } => {
            walk_children(values, function, id, facts)?;
        }
        ExprKind::While {
            condition, body, ..
        } => {
            walk(condition, function, Some(id), 0, facts)?;
            for (index, child) in body.iter().enumerate() {
                walk(
                    child,
                    function,
                    Some(id),
                    index_u64(index.saturating_add(1))?,
                    facts,
                )?;
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            walk(condition, function, Some(id), 0, facts)?;
            walk(then_branch, function, Some(id), 1, facts)?;
            walk(else_branch, function, Some(id), 2, facts)?;
        }
        ExprKind::Let { bindings, body } => {
            for (index, binding) in bindings.iter().enumerate() {
                add(&mut facts.places, 1)?;
                if affine(&binding.value.ty) && !binding.static_bytes {
                    add(&mut facts.obligations, 1)?;
                }
                walk(&binding.value, function, Some(id), index_u64(index)?, facts)?;
            }
            walk(body, function, Some(id), index_u64(bindings.len())?, facts)?;
        }
        ExprKind::MutableLocal { initial, body, .. } => {
            add(&mut facts.places, 1)?;
            if affine(&initial.ty) {
                add(&mut facts.obligations, 1)?;
            }
            walk(initial, function, Some(id), 0, facts)?;
            walk(body, function, Some(id), 1, facts)?;
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            walk(value, function, Some(id), 0, facts)?;
            walk(replacement, function, Some(id), 1, facts)?;
        }
        ExprKind::Continue { .. } | ExprKind::MatchUnreachable { .. } => {}
    }
    Ok(id)
}

fn walk_children<'a>(
    children: &'a [Expr],
    function: MemoryFunctionId,
    parent: super::super::MemoryExpressionId,
    facts: &mut Facts<'a>,
) -> Result<()> {
    for (index, child) in children.iter().enumerate() {
        walk(child, function, Some(parent), index_u64(index)?, facts)?;
    }
    Ok(())
}

fn add(slot: &mut u64, amount: usize) -> Result<()> {
    *slot = slot
        .checked_add(u64::try_from(amount).map_err(|_| Error::msg("memory work exceeds u64"))?)
        .ok_or_else(|| Error::msg("independent memory verifier work overflow"))?;
    Ok(())
}
