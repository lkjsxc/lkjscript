use super::*;

#[derive(Debug, PartialEq)]
pub struct Expr {
    pub ty: Type,
    pub effects: EffectSet,
    pub origin: Origin,
    pub kind: ExprKind,
}

impl Clone for Expr {
    fn clone(&self) -> Self {
        enum Work<'a> {
            Visit(&'a Expr),
            Finish(&'a Expr, usize),
        }

        let mut work = vec![Work::Visit(self)];
        let mut completed = Vec::new();
        while let Some(item) = work.pop() {
            match item {
                Work::Visit(expression) => {
                    let children = expression_children(expression);
                    let child_count = children.len();
                    work.push(Work::Finish(expression, child_count));
                    work.extend(children.into_iter().rev().map(Work::Visit));
                }
                Work::Finish(expression, child_count) => {
                    let split = completed.len() - child_count;
                    let children = completed.split_off(split);
                    completed.push(Self {
                        ty: expression.ty.clone(),
                        effects: expression.effects,
                        origin: expression.origin,
                        kind: clone_kind(&expression.kind, children),
                    });
                }
            }
        }
        completed.pop().unwrap_or_else(|| Self {
            ty: Type::Unit,
            effects: EffectSet::PURE,
            origin: self.origin,
            kind: ExprKind::LitUnit,
        })
    }
}

fn expression_children(expression: &Expr) -> Vec<&Expr> {
    let mut children = Vec::new();
    for_each_expression_child(expression, &mut |child| children.push(child));
    children
}

pub(crate) fn for_each_expression_child<'a>(
    expression: &'a Expr,
    action: &mut impl FnMut(&'a Expr),
) {
    match &expression.kind {
        ExprKind::Call { args, .. }
        | ExprKind::Operation { args, .. }
        | ExprKind::Do(args)
        | ExprKind::Loop { body: args, .. }
        | ExprKind::ProductValue { fields: args, .. }
        | ExprKind::EnumValue { fields: args, .. } => {
            for child in args {
                action(child);
            }
        }
        ExprKind::While {
            condition, body, ..
        } => {
            action(condition);
            for child in body {
                action(child);
            }
        }
        ExprKind::Match {
            scrutinee, arms, ..
        } => {
            action(scrutinee);
            for arm in arms {
                action(arm);
            }
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
        | ExprKind::EnumUnwrap { value, .. } => action(value),
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            action(condition);
            action(then_branch);
            action(else_branch);
        }
        ExprKind::Let { bindings, body } => {
            for binding in bindings {
                action(&binding.value);
            }
            action(body);
        }
        ExprKind::MutableLocal { initial, body, .. }
        | ExprKind::WithProductField {
            value: initial,
            replacement: body,
            ..
        } => {
            action(initial);
            action(body);
        }
        ExprKind::Hole
        | ExprKind::LitI64(_)
        | ExprKind::LitF64(_)
        | ExprKind::LitBool(_)
        | ExprKind::LitUnit
        | ExprKind::EmptyList
        | ExprKind::LitStr(_)
        | ExprKind::LitBytes(_)
        | ExprKind::Load(_)
        | ExprKind::Move { .. }
        | ExprKind::Borrow { .. }
        | ExprKind::BorrowBytes { .. }
        | ExprKind::Continue { .. }
        | ExprKind::MatchUnreachable { .. }
        | ExprKind::QuoteSymbol(_) => {}
    }
}

fn clone_kind(kind: &ExprKind, mut children: Vec<Expr>) -> ExprKind {
    match kind {
        ExprKind::Hole => ExprKind::Hole,
        ExprKind::LitI64(value) => ExprKind::LitI64(*value),
        ExprKind::LitF64(value) => ExprKind::LitF64(*value),
        ExprKind::LitBool(value) => ExprKind::LitBool(*value),
        ExprKind::LitUnit => ExprKind::LitUnit,
        ExprKind::EmptyList => ExprKind::EmptyList,
        ExprKind::LitStr(value) => ExprKind::LitStr(value.clone()),
        ExprKind::LitBytes(value) => ExprKind::LitBytes(value.clone()),
        ExprKind::Load(value) => ExprKind::Load(*value),
        ExprKind::Move { place, binding } => ExprKind::Move {
            place: *place,
            binding: *binding,
        },
        ExprKind::Borrow {
            place,
            loan,
            kind,
            binding,
        } => ExprKind::Borrow {
            place: *place,
            loan: *loan,
            kind: *kind,
            binding: *binding,
        },
        ExprKind::BorrowBytes {
            place,
            loan,
            binding,
        } => ExprKind::BorrowBytes {
            place: *place,
            loan: *loan,
            binding: *binding,
        },
        ExprKind::Call {
            callee,
            instantiation,
            ..
        } => ExprKind::Call {
            callee: *callee,
            args: children,
            instantiation: instantiation.clone(),
        },
        ExprKind::Operation {
            operation,
            resolved_signature,
            ..
        } => ExprKind::Operation {
            operation: *operation,
            resolved_signature: resolved_signature.clone(),
            args: children,
        },
        ExprKind::F64FromI64Exact(_) => ExprKind::F64FromI64Exact(Box::new(children.remove(0))),
        ExprKind::F64FromI64Rounded(_) => ExprKind::F64FromI64Rounded(Box::new(children.remove(0))),
        ExprKind::I64FromF64Exact(_) => ExprKind::I64FromF64Exact(Box::new(children.remove(0))),
        ExprKind::I64FromF64Trunc(_) => ExprKind::I64FromF64Trunc(Box::new(children.remove(0))),
        ExprKind::Do(_) => ExprKind::Do(children),
        ExprKind::If { .. } => ExprKind::If {
            condition: Box::new(children.remove(0)),
            then_branch: Box::new(children.remove(0)),
            else_branch: Box::new(children.remove(0)),
        },
        ExprKind::While { loop_id, .. } => ExprKind::While {
            loop_id: *loop_id,
            condition: Box::new(children.remove(0)),
            body: children,
        },
        ExprKind::Loop {
            loop_id,
            result_type,
            ..
        } => ExprKind::Loop {
            loop_id: *loop_id,
            result_type: result_type.clone(),
            body: children,
        },
        ExprKind::Return { .. } => ExprKind::Return {
            value: Box::new(children.remove(0)),
        },
        ExprKind::Break { loop_id, .. } => ExprKind::Break {
            loop_id: *loop_id,
            value: Box::new(children.remove(0)),
        },
        ExprKind::Continue { loop_id } => ExprKind::Continue { loop_id: *loop_id },
        ExprKind::Trap { .. } => ExprKind::Trap {
            value: Box::new(children.remove(0)),
        },
        ExprKind::Exit { .. } => ExprKind::Exit {
            code: Box::new(children.remove(0)),
        },
        ExprKind::Let { bindings, .. } => {
            let body = Box::new(children.remove(bindings.len()));
            let bindings = bindings
                .iter()
                .zip(children)
                .map(|(binding, value)| LocalDefinition {
                    binding: binding.binding,
                    place: binding.place,
                    static_bytes: binding.static_bytes,
                    slot: binding.slot,
                    value,
                })
                .collect();
            ExprKind::Let { bindings, body }
        }
        ExprKind::MutableLocal {
            binding,
            place,
            slot,
            ..
        } => ExprKind::MutableLocal {
            binding: *binding,
            place: *place,
            slot: *slot,
            initial: Box::new(children.remove(0)),
            body: Box::new(children.remove(0)),
        },
        ExprKind::SetLocal { target, slot, .. } => ExprKind::SetLocal {
            target: *target,
            slot: *slot,
            value: Box::new(children.remove(0)),
        },
        ExprKind::ProductValue { product, .. } => ExprKind::ProductValue {
            product: *product,
            fields: children,
        },
        ExprKind::ProductField { product, field, .. } => ExprKind::ProductField {
            product: *product,
            field: *field,
            value: Box::new(children.remove(0)),
        },
        ExprKind::WithProductField { product, field, .. } => ExprKind::WithProductField {
            product: *product,
            field: *field,
            value: Box::new(children.remove(0)),
            replacement: Box::new(children.remove(0)),
        },
        ExprKind::EnumValue {
            enum_id,
            variant,
            layout,
            ..
        } => ExprKind::EnumValue {
            enum_id: *enum_id,
            variant: *variant,
            layout: *layout,
            fields: children,
        },
        ExprKind::EnumIsVariant {
            enum_id,
            variant,
            layout,
            ..
        } => ExprKind::EnumIsVariant {
            enum_id: *enum_id,
            variant: *variant,
            layout: *layout,
            value: Box::new(children.remove(0)),
        },
        ExprKind::EnumField {
            enum_id,
            variant,
            field,
            field_index,
            layout,
            ..
        } => ExprKind::EnumField {
            enum_id: *enum_id,
            variant: *variant,
            field: *field,
            field_index: *field_index,
            layout: *layout,
            value: Box::new(children.remove(0)),
        },
        ExprKind::EnumUnwrap {
            enum_id,
            variant,
            field,
            field_index,
            layout,
            trap,
            ..
        } => ExprKind::EnumUnwrap {
            enum_id: *enum_id,
            variant: *variant,
            field: *field,
            field_index: *field_index,
            layout: *layout,
            value: Box::new(children.remove(0)),
            trap: trap.clone(),
        },
        ExprKind::Match { plan, .. } => ExprKind::Match {
            plan: *plan,
            scrutinee: Box::new(children.remove(0)),
            arms: children,
        },
        ExprKind::MatchUnreachable { plan } => ExprKind::MatchUnreachable { plan: *plan },
        ExprKind::QuoteSymbol(value) => ExprKind::QuoteSymbol(value.clone()),
    }
}

impl Expr {
    pub(crate) fn try_at_preorder(&self, target: u64) -> lkjscript_core::Result<Option<&Self>> {
        let mut pending = Vec::new();
        pending
            .try_reserve(1)
            .map_err(|_| lkjscript_core::Error::host("HIR lookup work allocation failed"))?;
        pending.push(self);
        let mut ordinal = 0_u64;
        while let Some(expression) = pending.pop() {
            if ordinal == target {
                return Ok(Some(expression));
            }
            let Some(next) = ordinal.checked_add(1) else {
                return Ok(None);
            };
            ordinal = next;
            let children = expression_children(expression);
            pending
                .try_reserve(children.len())
                .map_err(|_| lkjscript_core::Error::host("HIR lookup work allocation failed"))?;
            pending.extend(children.into_iter().rev());
        }
        Ok(None)
    }

    pub(crate) fn try_clone(&self) -> lkjscript_core::Result<Self> {
        enum Work<'a> {
            Visit(&'a Expr),
            Finish(&'a Expr, usize),
        }

        let mut work = Vec::new();
        work.try_reserve(1)
            .map_err(|_| lkjscript_core::Error::host("HIR clone work allocation failed"))?;
        work.push(Work::Visit(self));
        let mut completed = Vec::new();
        while let Some(item) = work.pop() {
            match item {
                Work::Visit(expression) => {
                    let children = expression_children(expression);
                    let additional = children.len().checked_add(1).ok_or_else(|| {
                        lkjscript_core::Error::host("HIR clone child count overflow")
                    })?;
                    work.try_reserve(additional).map_err(|_| {
                        lkjscript_core::Error::host("HIR clone work allocation failed")
                    })?;
                    work.push(Work::Finish(expression, children.len()));
                    work.extend(children.into_iter().rev().map(Work::Visit));
                }
                Work::Finish(expression, child_count) => {
                    let split = completed.len().checked_sub(child_count).ok_or_else(|| {
                        lkjscript_core::Error::msg("HIR clone completion order is invalid")
                    })?;
                    let children = completed.split_off(split);
                    completed.try_reserve(1).map_err(|_| {
                        lkjscript_core::Error::host("HIR clone result allocation failed")
                    })?;
                    completed.push(Self {
                        ty: expression.ty.clone(),
                        effects: expression.effects,
                        origin: expression.origin,
                        kind: clone_kind(&expression.kind, children),
                    });
                }
            }
        }
        completed
            .pop()
            .ok_or_else(|| lkjscript_core::Error::msg("HIR clone omitted its root"))
    }

    pub(crate) fn try_lower_semantic_matches(
        &self,
        lower: &mut impl FnMut(MatchPlanId, Expr, Vec<Expr>) -> lkjscript_core::Result<Expr>,
    ) -> lkjscript_core::Result<Self> {
        enum Work<'a> {
            Visit(&'a Expr),
            Finish(&'a Expr, usize),
        }

        let mut work = Vec::new();
        work.try_reserve(1)
            .map_err(|_| lkjscript_core::Error::host("match derivation work allocation failed"))?;
        work.push(Work::Visit(self));
        let mut completed = Vec::new();
        while let Some(item) = work.pop() {
            match item {
                Work::Visit(expression) => {
                    let children = expression_children(expression);
                    let additional = children.len().checked_add(1).ok_or_else(|| {
                        lkjscript_core::Error::host("match derivation child count overflow")
                    })?;
                    work.try_reserve(additional).map_err(|_| {
                        lkjscript_core::Error::host("match derivation work allocation failed")
                    })?;
                    work.push(Work::Finish(expression, children.len()));
                    work.extend(children.into_iter().rev().map(Work::Visit));
                }
                Work::Finish(expression, child_count) => {
                    let split = completed.len().checked_sub(child_count).ok_or_else(|| {
                        lkjscript_core::Error::msg("match derivation completion order is invalid")
                    })?;
                    let mut children = completed.split_off(split);
                    let transformed = match &expression.kind {
                        ExprKind::Match { plan, .. } => {
                            let scrutinee = if children.is_empty() {
                                return Err(lkjscript_core::Error::msg(
                                    "semantic match omitted its scrutinee",
                                ));
                            } else {
                                children.remove(0)
                            };
                            lower(*plan, scrutinee, children)?
                        }
                        _ => Self {
                            ty: expression.ty.clone(),
                            effects: expression.effects,
                            origin: expression.origin,
                            kind: clone_kind(&expression.kind, children),
                        },
                    };
                    completed.try_reserve(1).map_err(|_| {
                        lkjscript_core::Error::host("match derivation result allocation failed")
                    })?;
                    completed.push(transformed);
                }
            }
        }
        let result = completed
            .pop()
            .ok_or_else(|| lkjscript_core::Error::msg("match derivation omitted its root"))?;
        if completed.is_empty() {
            Ok(result)
        } else {
            Err(lkjscript_core::Error::msg(
                "match derivation left disconnected results",
            ))
        }
    }

    pub(crate) fn try_replaced_preorder(
        &self,
        target: u64,
        replacement: &Self,
    ) -> lkjscript_core::Result<Option<Self>> {
        enum Work<'a> {
            Visit(&'a Expr),
            Finish(&'a Expr, usize),
        }

        let mut ordinal = 0_u64;
        let mut found = false;
        let mut work = Vec::new();
        work.try_reserve(1)
            .map_err(|_| lkjscript_core::Error::host("HIR replacement work allocation failed"))?;
        work.push(Work::Visit(self));
        let mut completed = Vec::new();
        while let Some(item) = work.pop() {
            match item {
                Work::Visit(expression) => {
                    let current = ordinal;
                    let Some(next) = ordinal.checked_add(1) else {
                        return Ok(None);
                    };
                    ordinal = next;
                    if current == target {
                        completed.try_reserve(1).map_err(|_| {
                            lkjscript_core::Error::host("HIR replacement result allocation failed")
                        })?;
                        completed.push(replacement.try_clone()?);
                        found = true;
                        continue;
                    }
                    let children = expression_children(expression);
                    let additional = children.len().checked_add(1).ok_or_else(|| {
                        lkjscript_core::Error::host("HIR replacement child count overflow")
                    })?;
                    work.try_reserve(additional).map_err(|_| {
                        lkjscript_core::Error::host("HIR replacement work allocation failed")
                    })?;
                    work.push(Work::Finish(expression, children.len()));
                    work.extend(children.into_iter().rev().map(Work::Visit));
                }
                Work::Finish(expression, child_count) => {
                    let Some(split) = completed.len().checked_sub(child_count) else {
                        return Ok(None);
                    };
                    let children = completed.split_off(split);
                    completed.try_reserve(1).map_err(|_| {
                        lkjscript_core::Error::host("HIR replacement result allocation failed")
                    })?;
                    completed.push(Self {
                        ty: expression.ty.clone(),
                        effects: expression.effects,
                        origin: expression.origin,
                        kind: clone_kind(&expression.kind, children),
                    });
                }
            }
        }
        Ok(found.then(|| completed.pop()).flatten())
    }
}

impl Drop for Expr {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        take_children(self, &mut pending);
        while let Some(mut expression) = pending.pop() {
            take_children(&mut expression, &mut pending);
        }
    }
}

fn take_children(expression: &mut Expr, pending: &mut Vec<Expr>) {
    let kind = std::mem::replace(&mut expression.kind, ExprKind::LitUnit);
    match kind {
        ExprKind::Call { mut args, .. }
        | ExprKind::Operation { mut args, .. }
        | ExprKind::Do(mut args)
        | ExprKind::While { body: mut args, .. }
        | ExprKind::Loop { body: mut args, .. }
        | ExprKind::ProductValue {
            fields: mut args, ..
        }
        | ExprKind::EnumValue {
            fields: mut args, ..
        } => pending.append(&mut args),
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
        | ExprKind::EnumUnwrap { value, .. } => pending.push(*value),
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            pending.push(*condition);
            pending.push(*then_branch);
            pending.push(*else_branch);
        }
        ExprKind::Let { bindings, body } => {
            pending.extend(bindings.into_iter().map(|binding| binding.value));
            pending.push(*body);
        }
        ExprKind::MutableLocal { initial, body, .. } => {
            pending.push(*initial);
            pending.push(*body);
        }
        ExprKind::WithProductField {
            value, replacement, ..
        } => {
            pending.push(*value);
            pending.push(*replacement);
        }
        ExprKind::Match {
            scrutinee,
            mut arms,
            ..
        } => {
            pending.push(*scrutinee);
            pending.append(&mut arms);
        }
        ExprKind::Hole
        | ExprKind::LitI64(_)
        | ExprKind::LitF64(_)
        | ExprKind::LitBool(_)
        | ExprKind::LitUnit
        | ExprKind::EmptyList
        | ExprKind::LitStr(_)
        | ExprKind::LitBytes(_)
        | ExprKind::Load(_)
        | ExprKind::Move { .. }
        | ExprKind::Borrow { .. }
        | ExprKind::BorrowBytes { .. }
        | ExprKind::Continue { .. }
        | ExprKind::MatchUnreachable { .. }
        | ExprKind::QuoteSymbol(_) => {}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BorrowKind {
    Shared,
    Mutable,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// A real incomplete expression. No executable expression exists beneath it.
    Hole,
    LitI64(i64),
    LitF64(f64),
    LitBool(bool),
    LitUnit,
    EmptyList,
    LitStr(String),
    LitBytes(Vec<u8>),
    Load(BindingRef),
    Move {
        place: PlaceId,
        binding: BindingRef,
    },
    Borrow {
        place: PlaceId,
        loan: LoanId,
        kind: BorrowKind,
        binding: BindingRef,
    },
    BorrowBytes {
        place: PlaceId,
        loan: LoanId,
        binding: BindingRef,
    },
    Call {
        callee: BindingRef,
        args: Vec<Expr>,
        instantiation: Option<GenericInstantiation>,
    },
    Operation {
        operation: Operation,
        resolved_signature: Type,
        args: Vec<Expr>,
    },
    F64FromI64Exact(Box<Expr>),
    F64FromI64Rounded(Box<Expr>),
    I64FromF64Exact(Box<Expr>),
    I64FromF64Trunc(Box<Expr>),
    Do(Vec<Expr>),
    If {
        condition: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Box<Expr>,
    },
    While {
        loop_id: LoopId,
        condition: Box<Expr>,
        body: Vec<Expr>,
    },
    Loop {
        loop_id: LoopId,
        result_type: Type,
        body: Vec<Expr>,
    },
    Return {
        value: Box<Expr>,
    },
    Break {
        loop_id: LoopId,
        value: Box<Expr>,
    },
    Continue {
        loop_id: LoopId,
    },
    Trap {
        value: Box<Expr>,
    },
    Exit {
        code: Box<Expr>,
    },
    Let {
        bindings: Vec<LocalDefinition>,
        body: Box<Expr>,
    },
    MutableLocal {
        binding: BindingId,
        place: PlaceId,
        slot: usize,
        initial: Box<Expr>,
        body: Box<Expr>,
    },
    SetLocal {
        target: BindingId,
        slot: usize,
        value: Box<Expr>,
    },
    ProductValue {
        product: ProductId,
        fields: Vec<Expr>,
    },
    ProductField {
        product: ProductId,
        field: u64,
        value: Box<Expr>,
    },
    WithProductField {
        product: ProductId,
        field: u64,
        value: Box<Expr>,
        replacement: Box<Expr>,
    },
    EnumValue {
        enum_id: EnumId,
        variant: VariantId,
        layout: RuntimeLayoutId,
        fields: Vec<Expr>,
    },
    EnumIsVariant {
        enum_id: EnumId,
        variant: VariantId,
        layout: RuntimeLayoutId,
        value: Box<Expr>,
    },
    EnumField {
        enum_id: EnumId,
        variant: VariantId,
        field: VariantFieldId,
        field_index: u64,
        layout: RuntimeLayoutId,
        value: Box<Expr>,
    },
    EnumUnwrap {
        enum_id: EnumId,
        variant: VariantId,
        field: VariantFieldId,
        field_index: u64,
        layout: RuntimeLayoutId,
        value: Box<Expr>,
        trap: String,
    },
    /// Syntax-independent semantic match authority. Complete compiler HIR
    /// derives and removes this node before ownership and lowering.
    Match {
        plan: MatchPlanId,
        scrutinee: Box<Expr>,
        arms: Vec<Expr>,
    },
    MatchUnreachable {
        plan: MatchPlanId,
    },
    QuoteSymbol(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct LocalDefinition {
    pub binding: BindingId,
    pub place: PlaceId,
    pub static_bytes: bool,
    pub slot: usize,
    pub value: Expr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BindingRef {
    pub binding: BindingId,
    pub storage: BindingStorage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingStorage {
    Local(usize),
    Function,
}
