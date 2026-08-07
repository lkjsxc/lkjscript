use std::collections::{BTreeMap, HashMap};

use lkjscript_core::{Error, ResourceKind, Result};

use crate::hir::{
    self, BindingId, BindingStorage, BorrowKind, Expr, ExprKind, LocalDefinition, Operation, Type,
};

use super::*;

pub(super) fn derive(program: &hir::Program) -> Result<HirMemoryPlan> {
    Producer::new(program)?.run()
}

struct Producer<'a> {
    program: &'a hir::Program,
    type_planner: TypePlanner<'a>,
    function_ids: HashMap<BindingId, MemoryFunctionId>,
    signatures: Vec<FunctionMemorySignature>,
    functions: Vec<FunctionMemoryPlan>,
    entries: Vec<MemoryPlanEntry>,
    expression_entries: HashMap<MemoryExpressionId, MemoryEntryId>,
    child_entries:
        HashMap<MemoryExpressionId, Vec<(u64, MemoryExpressionId, Option<MemoryDropPathId>)>>,
    places_by_binding: HashMap<(MemoryFunctionId, u64), u64>,
    products_by_id: HashMap<hir::ProductId, usize>,
    enums_by_id: HashMap<hir::EnumId, usize>,
    uses: Vec<MemoryUse>,
    loans: Vec<MemoryLoanPlan>,
    constants: Vec<MemoryConstantPlan>,
    calls: Vec<MemoryCallPlan>,
    obligations: Vec<MemoryObligation>,
    destinations: Vec<MemoryDestinationPlan>,
    borrow_scopes: Vec<MemoryBorrowScopePlan>,
    current_function: MemoryFunctionId,
    next_expression: u64,
    next_place: u64,
    expression_parents: BTreeMap<MemoryExpressionId, Option<MemoryExpressionId>>,
    work: MemoryPlanWork,
}

include!("producer/type_graph.rs");
include!("producer/type_plan/mod.rs");
include!("producer/type_plan/demand.rs");
include!("producer/type_plan/transport.rs");
include!("producer/type_plan/transport_calls.rs");
include!("producer/type_helpers.rs");
#[path = "producer/placement/mod.rs"]
mod placement;
use placement::derive_value_placements;
include!("producer/recursive.rs");
mod lifecycle;
mod records;
mod walk;

fn observe(slot: &mut u64, amount: usize, label: &str) -> Result<()> {
    let amount = u64::try_from(amount)
        .map_err(|_| Error::msg(format!("HIR memory-plan {label} observation exceeds u64")))?;
    *slot = slot
        .checked_add(amount)
        .ok_or_else(|| Error::msg(format!("HIR memory-plan {label} observation overflow")))?;
    Ok(())
}

#[cfg(test)]
mod observational_accounting_tests {
    use super::*;

    #[test]
    fn observational_accounting_reports_real_u64_overflow() -> Result<()> {
        let mut observed = u64::MAX;
        let error = match observe(&mut observed, 1, "test") {
            Err(error) => error,
            Ok(()) => return Err(Error::msg("u64 telemetry overflow was not reported")),
        };
        assert_eq!(
            error.to_string(),
            "HIR memory-plan test observation overflow"
        );
        assert_eq!(observed, u64::MAX);
        Ok(())
    }
}
fn index_u64(index: usize) -> Result<u64> {
    u64::try_from(index).map_err(|_| Error::msg("HIR memory-plan child index exceeds u64"))
}
fn function_result_type(ty: &Type) -> Result<&Type> {
    let (_, result) = callable_type(ty)?;
    Ok(result)
}
fn callable_type(ty: &Type) -> Result<(&[Type], &Type)> {
    match ty {
        Type::Fn { params, ret } => Ok((params, ret)),
        Type::Forall { body, .. } => callable_type(body),
        _ => Err(Error::msg(
            "HIR memory-plan callable binding has non-function type",
        )),
    }
}
fn resource_parameter_consumed(expression: &Expr, binding: BindingId) -> bool {
    match &expression.kind {
        ExprKind::Move { binding: moved, .. } if moved.binding == binding => true,
        ExprKind::Operation {
            operation, args, ..
        } if consuming_operation(*operation)
            && args
                .iter()
                .any(|argument| expression_uses_binding(argument, binding)) =>
        {
            true
        }
        _ => children(expression)
            .into_iter()
            .any(|child| resource_parameter_consumed(child, binding)),
    }
}
fn expression_uses_binding(expression: &Expr, binding: BindingId) -> bool {
    match expression.kind {
        ExprKind::Load(reference)
        | ExprKind::Move {
            binding: reference, ..
        }
        | ExprKind::Borrow {
            binding: reference, ..
        } => reference.binding == binding,
        _ => children(expression)
            .into_iter()
            .any(|child| expression_uses_binding(child, binding)),
    }
}
fn children(expression: &Expr) -> Vec<&Expr> {
    match &expression.kind {
        ExprKind::Call { args, .. }
        | ExprKind::Operation { args, .. }
        | ExprKind::Do(args)
        | ExprKind::Loop { body: args, .. }
        | ExprKind::ProductValue { fields: args, .. }
        | ExprKind::EnumValue { fields: args, .. } => args.iter().collect(),
        ExprKind::While {
            condition, body, ..
        } => std::iter::once(condition.as_ref())
            .chain(body.iter())
            .collect(),
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
        | ExprKind::EnumUnwrap { value, .. } => vec![value],
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => vec![condition, then_branch, else_branch],
        ExprKind::Let { bindings, body } => bindings
            .iter()
            .map(|binding| &binding.value)
            .chain(std::iter::once(body.as_ref()))
            .collect(),
        ExprKind::MutableLocal { initial, body, .. } => vec![initial, body],
        ExprKind::WithProductField {
            value, replacement, ..
        } => vec![value, replacement],
        _ => Vec::new(),
    }
}
fn consuming_operation(operation: Operation) -> bool {
    matches!(
        operation,
        Operation::DropResource | Operation::SysSqliteClose | Operation::SysSqliteFinalize
    )
}
fn parameter_mode(ty: &Type, resource_consumed: bool) -> MemoryParameterMode {
    match ty {
        Type::Bytes | Type::ByteVector => MemoryParameterMode::Consume,
        Type::ByteSlice => MemoryParameterMode::BorrowShared,
        Type::ByteSliceMut => MemoryParameterMode::BorrowExclusive,
        Type::Str | Type::Path => MemoryParameterMode::BorrowShared,
        Type::Resource(_) if resource_consumed => MemoryParameterMode::Consume,
        Type::Resource(_) => MemoryParameterMode::BorrowExclusive,
        _ => MemoryParameterMode::Copy,
    }
}
fn operation_parameter_mode(operation: Operation, ty: &Type) -> MemoryParameterMode {
    match ty {
        Type::Resource(_) if consuming_operation(operation) => MemoryParameterMode::Consume,
        Type::Resource(_) => MemoryParameterMode::BorrowExclusive,
        _ => parameter_mode(ty, false),
    }
}
fn result_mode(ty: &Type) -> MemoryResultMode {
    match ty {
        Type::Bytes | Type::ByteVector => MemoryResultMode::Owned,
        Type::ByteSlice | Type::ByteSliceMut => MemoryResultMode::Trivial,
        Type::Str | Type::Path | Type::Product(_) | Type::Enum { .. } => MemoryResultMode::Owned,
        Type::Resource(_) => MemoryResultMode::External,
        _ => MemoryResultMode::Trivial,
    }
}
fn borrow_kind(kind: BorrowKind) -> MemoryBorrowKind {
    match kind {
        BorrowKind::Shared => MemoryBorrowKind::Shared,
        BorrowKind::Mutable => MemoryBorrowKind::Exclusive,
    }
}
fn constant_value(kind: &ExprKind) -> Option<MemoryConstantValue> {
    match kind {
        ExprKind::LitI64(value) => Some(MemoryConstantValue::I64(*value)),
        ExprKind::LitF64(value) => Some(MemoryConstantValue::F64(value.to_bits())),
        ExprKind::LitBool(value) => Some(MemoryConstantValue::Bool(*value)),
        ExprKind::LitUnit => Some(MemoryConstantValue::Unit),
        ExprKind::EmptyList => Some(MemoryConstantValue::EmptyList),
        ExprKind::LitStr(value) => Some(MemoryConstantValue::String(value.clone())),
        ExprKind::LitBytes(value) => Some(MemoryConstantValue::Bytes(value.clone())),
        ExprKind::QuoteSymbol(value) => Some(MemoryConstantValue::Symbol(value.clone())),
        _ => None,
    }
}

fn expression_kind(kind: &ExprKind) -> MemoryExpressionKind {
    match kind {
        ExprKind::LitI64(value) => MemoryExpressionKind::I64Literal(*value),
        ExprKind::LitF64(value) => MemoryExpressionKind::F64Literal(value.to_bits()),
        ExprKind::LitBool(value) => MemoryExpressionKind::BoolLiteral(*value),
        ExprKind::LitUnit => MemoryExpressionKind::UnitLiteral,
        ExprKind::EmptyList => MemoryExpressionKind::EmptyList,
        ExprKind::LitStr(_) => MemoryExpressionKind::StringLiteral,
        ExprKind::LitBytes(_) => MemoryExpressionKind::BytesLiteral,
        ExprKind::Load(binding) => MemoryExpressionKind::Load {
            binding: binding.binding.raw(),
            storage: binding_storage(binding.storage),
        },
        ExprKind::Move { place, binding } => MemoryExpressionKind::Move {
            place: place.raw(),
            binding: binding.binding.raw(),
        },
        ExprKind::BorrowBytes {
            place,
            loan,
            binding,
        } => MemoryExpressionKind::Borrow {
            place: place.raw(),
            loan: loan.raw(),
            kind: MemoryBorrowKind::Shared,
            binding: binding.binding.raw(),
        },
        ExprKind::Borrow {
            place,
            loan,
            kind,
            binding,
        } => MemoryExpressionKind::Borrow {
            place: place.raw(),
            loan: loan.raw(),
            kind: borrow_kind(*kind),
            binding: binding.binding.raw(),
        },
        ExprKind::Call { callee, .. } => match callee.storage {
            BindingStorage::Function => MemoryExpressionKind::DirectCall,
            BindingStorage::Local(_) => MemoryExpressionKind::IndirectCall,
        },
        ExprKind::Operation { operation, .. } => {
            MemoryExpressionKind::Operation(operation.identity().as_u16())
        }
        ExprKind::F64FromI64Exact(_) => MemoryExpressionKind::F64FromI64Exact,
        ExprKind::F64FromI64Rounded(_) => MemoryExpressionKind::F64FromI64Rounded,
        ExprKind::I64FromF64Exact(_) => MemoryExpressionKind::I64FromF64Exact,
        ExprKind::I64FromF64Trunc(_) => MemoryExpressionKind::I64FromF64Trunc,
        ExprKind::Do(_) => MemoryExpressionKind::Sequence,
        ExprKind::If { .. } => MemoryExpressionKind::If,
        ExprKind::While { .. } => MemoryExpressionKind::While,
        ExprKind::Loop { .. } => MemoryExpressionKind::Loop,
        ExprKind::Return { .. } => MemoryExpressionKind::Return,
        ExprKind::Break { .. } => MemoryExpressionKind::Break,
        ExprKind::Continue { .. } => MemoryExpressionKind::Continue,
        ExprKind::Trap { .. } => MemoryExpressionKind::Trap,
        ExprKind::Exit { .. } => MemoryExpressionKind::Exit,
        ExprKind::Let { .. } => MemoryExpressionKind::Let,
        ExprKind::MutableLocal { .. } => MemoryExpressionKind::MutableLocal,
        ExprKind::SetLocal { .. } => MemoryExpressionKind::SetLocal,
        ExprKind::ProductValue { .. } => MemoryExpressionKind::ProductValue,
        ExprKind::ProductField { .. } => MemoryExpressionKind::ProductField,
        ExprKind::WithProductField { .. } => MemoryExpressionKind::WithProductField,
        ExprKind::EnumValue { .. } => MemoryExpressionKind::EnumValue,
        ExprKind::EnumIsVariant { .. } => MemoryExpressionKind::EnumIsVariant,
        ExprKind::EnumField { .. } => MemoryExpressionKind::EnumField,
        ExprKind::EnumUnwrap { .. } => MemoryExpressionKind::EnumUnwrap,
        ExprKind::MatchUnreachable { .. } => MemoryExpressionKind::MatchUnreachable,
        ExprKind::QuoteSymbol(_) => MemoryExpressionKind::SymbolLiteral,
    }
}
fn binding_storage(storage: BindingStorage) -> MemoryBindingStorage {
    match storage {
        BindingStorage::Local(_) => MemoryBindingStorage::Local,
        BindingStorage::Function => MemoryBindingStorage::Function,
    }
}
pub(super) fn memory_type(ty: &Type) -> MemoryType {
    crate::stack::grow(|| memory_type_inner(ty))
}

fn memory_type_inner(ty: &Type) -> MemoryType {
    match ty {
        Type::Never => MemoryType::Never,
        Type::Unit => MemoryType::Unit,
        Type::Bool => MemoryType::Bool,
        Type::I64 => MemoryType::I64,
        Type::F64 => MemoryType::F64,
        Type::Str => MemoryType::String,
        Type::Bytes => MemoryType::Bytes,
        Type::Path => MemoryType::Path,
        Type::Capability(kind) => MemoryType::Capability(*kind),
        Type::ByteVector => MemoryType::ByteVector,
        Type::ByteSlice => MemoryType::ByteSlice,
        Type::ByteSliceMut => MemoryType::ByteSliceMut,
        Type::Symbol => MemoryType::Symbol,
        Type::Resource(kind) => MemoryType::Resource(*kind),
        Type::Product(name) => MemoryType::Product(name.clone()),
        Type::Enum {
            id,
            name,
            arguments,
        } => MemoryType::Enum {
            id: id.bytes(),
            name: name.clone(),
            arguments: arguments.iter().map(memory_type).collect(),
        },
        Type::Param(name) => MemoryType::TypeParameter(name.clone()),
        Type::List(inner) => MemoryType::List(Box::new(memory_type(inner))),
        Type::Fn { params, ret } => MemoryType::Function {
            parameters: params.iter().map(memory_type).collect(),
            result: Box::new(memory_type(ret)),
        },
        Type::Forall { vars, body } => MemoryType::ForAll {
            variables: vars.clone(),
            body: Box::new(memory_type(body)),
        },
    }
}

fn destination_shape(
    program: &hir::Program,
    products_by_id: &HashMap<hir::ProductId, usize>,
    enums_by_id: &HashMap<hir::EnumId, usize>,
    expression: &Expr,
) -> Result<(u64, Option<MemoryActivePayload>)> {
    match &expression.kind {
        ExprKind::ProductValue { product, fields } => {
            let declared = products_by_id
                .get(product)
                .and_then(|index| program.products.get(*index))
                .filter(|item| item.id == *product)
                .ok_or_else(|| Error::msg("aggregate destination lost product declaration"))?;
            if declared.fields.len() != fields.len() {
                return Err(Error::msg("LKJ-MEM-INCOMPLETE-DESTINATION product fields"));
            }
            Ok((index_u64(fields.len())?, None))
        }
        ExprKind::EnumValue {
            enum_id,
            variant,
            fields,
            ..
        } => {
            let declared = enums_by_id
                .get(enum_id)
                .and_then(|index| program.enums.get(*index))
                .filter(|item| item.id == *enum_id)
                .and_then(|item| item.variants.iter().find(|item| item.id == *variant))
                .ok_or_else(|| Error::msg("aggregate destination lost active enum payload"))?;
            if declared.fields.len() != fields.len() {
                return Err(Error::msg("LKJ-MEM-INCOMPLETE-DESTINATION enum fields"));
            }
            Ok((
                index_u64(fields.len())?,
                Some(MemoryActivePayload {
                    variant: variant.bytes(),
                    source_order: declared.source_order,
                }),
            ))
        }
        _ => Err(Error::msg(
            "destination requested for non-construction expression",
        )),
    }
}

type DomainAxes = (
    MemoryAliasing,
    MemoryDomain,
    MemoryDestruction,
    MemoryIdentity,
    MemoryPortability,
    MemoryContention,
);

fn memory_mode(
    ty: &Type,
    fact: &MemoryTypeFact,
    effects: u16,
    escape: MemoryEscape,
) -> Result<(MemoryMode, MemoryExecution, Option<MemoryExecutionCutover>)> {
    let multiplicity = if matches!(ty, Type::ByteSlice | Type::ByteSliceMut) {
        MemoryMultiplicity::Borrowed
    } else {
        match fact.mode {
            MemoryAggregateMode::Copy => MemoryMultiplicity::Copy,
            MemoryAggregateMode::ImmutableValue => MemoryMultiplicity::ImmutableValue,
            MemoryAggregateMode::Affine => MemoryMultiplicity::Affine,
        }
    };
    let (aliasing, domain, destruction, identity, portability, contention) = match ty {
        Type::Never | Type::Unit | Type::Bool | Type::I64 | Type::F64 | Type::Capability(_) => (
            MemoryAliasing::Unique,
            MemoryDomain::Inline,
            MemoryDestruction::Trivial,
            MemoryIdentity::Value,
            MemoryPortability::Portable,
            MemoryContention::None,
        ),
        Type::Str => structural_domain(MemoryPortability::WorkerLocal),
        Type::Path => structural_domain(MemoryPortability::LinuxHost),
        Type::Bytes | Type::ByteVector => (
            MemoryAliasing::Unique,
            MemoryDomain::UniqueStructural,
            MemoryDestruction::DropGlue,
            MemoryIdentity::Value,
            MemoryPortability::WorkerLocal,
            MemoryContention::SingleOwner,
        ),
        Type::ByteSlice => borrowed_domain(false),
        Type::ByteSliceMut => borrowed_domain(true),
        Type::Symbol | Type::Fn { .. } | Type::Forall { .. } => static_domain(),
        Type::Resource(_) => (
            MemoryAliasing::External,
            MemoryDomain::ExternalResource,
            MemoryDestruction::ExternalClose,
            MemoryIdentity::ExternalResource,
            MemoryPortability::ProcessLocal,
            MemoryContention::ProviderSerialized,
        ),
        Type::Product(_) => product_domain(fact)?,
        Type::Enum { .. } => aggregate_domain(fact, MemoryPortability::WorkerLocal),
        Type::List(_) if fact.closure.class == MemoryClosureClass::RegionClosed => {
            region_list_domain()
        }
        Type::List(_) => unsupported_domain(MemoryPortability::WorkerLocal),
        Type::Param(_) => (
            MemoryAliasing::StaticShared,
            MemoryDomain::CallerDestination,
            MemoryDestruction::Trivial,
            MemoryIdentity::Value,
            MemoryPortability::WorkerLocal,
            MemoryContention::ImmutableShared,
        ),
    };
    let execution_cutover = if fact.closure.class == MemoryClosureClass::Deterministic {
        execution_cutover(ty)
    } else {
        None
    };
    let execution = if execution_cutover.is_some() || domain == MemoryDomain::UnsupportedRuntime {
        MemoryExecution::CutoverRequired
    } else {
        MemoryExecution::Current
    };
    Ok((
        MemoryMode {
            multiplicity,
            aliasing,
            escape,
            domain,
            destruction,
            identity,
            portability,
            contention,
            allocation_failure: allocation_failure(effects),
        },
        execution,
        execution_cutover,
    ))
}

fn product_domain(fact: &MemoryTypeFact) -> Result<DomainAxes> {
    match fact.closure.class {
        MemoryClosureClass::Deterministic => Ok(structural_domain(MemoryPortability::WorkerLocal)),
        MemoryClosureClass::RegionClosed => Ok(region_list_domain()),
        MemoryClosureClass::Unresolved | MemoryClosureClass::IllegalDomainBridge => Err(
            Error::msg("unresolved product reached memory mode derivation"),
        ),
    }
}

fn aggregate_domain(fact: &MemoryTypeFact, portability: MemoryPortability) -> DomainAxes {
    match fact.closure.class {
        MemoryClosureClass::Deterministic => structural_domain(portability),
        MemoryClosureClass::RegionClosed => region_list_domain(),
        MemoryClosureClass::Unresolved | MemoryClosureClass::IllegalDomainBridge => {
            unsupported_domain(portability)
        }
    }
}

fn structural_domain(portability: MemoryPortability) -> DomainAxes {
    (
        MemoryAliasing::Unique,
        MemoryDomain::UniqueStructural,
        MemoryDestruction::DropGlue,
        MemoryIdentity::Value,
        portability,
        MemoryContention::SingleOwner,
    )
}

fn region_list_domain() -> DomainAxes {
    (
        MemoryAliasing::RegionShared,
        MemoryDomain::OrdinaryRegion,
        MemoryDestruction::RegionReset,
        MemoryIdentity::Value,
        MemoryPortability::WorkerLocal,
        MemoryContention::ImmutableShared,
    )
}

fn unsupported_domain(portability: MemoryPortability) -> DomainAxes {
    (
        MemoryAliasing::UnresolvedShared,
        MemoryDomain::UnsupportedRuntime,
        MemoryDestruction::Unsupported,
        MemoryIdentity::UnsupportedValue,
        portability,
        MemoryContention::UnresolvedShared,
    )
}

fn borrowed_domain(exclusive: bool) -> DomainAxes {
    (
        if exclusive {
            MemoryAliasing::BorrowedExclusive
        } else {
            MemoryAliasing::BorrowedShared
        },
        MemoryDomain::BorrowedView,
        MemoryDestruction::EndBorrow,
        MemoryIdentity::Value,
        MemoryPortability::WorkerLocal,
        MemoryContention::SingleOwner,
    )
}

fn static_domain() -> DomainAxes {
    (
        MemoryAliasing::StaticShared,
        MemoryDomain::Static,
        MemoryDestruction::Trivial,
        MemoryIdentity::Value,
        MemoryPortability::WorkerLocal,
        MemoryContention::ImmutableShared,
    )
}

fn allocation_failure(effects: u16) -> MemoryAllocationFailure {
    let allocates = effects & crate::hir::EffectSet::ALLOCATES.bits() != 0;
    let trap = effects & crate::hir::EffectSet::MAY_TRAP.bits() != 0;
    let outcome = effects & crate::hir::EffectSet::MAY_EXIT.bits() != 0 || allocates;
    match (trap, outcome) {
        (false, false) => MemoryAllocationFailure::Impossible,
        (true, false) => MemoryAllocationFailure::Trap,
        (false, true) => MemoryAllocationFailure::StructuredOutcome,
        (true, true) => MemoryAllocationFailure::TrapOrOutcome,
    }
}

pub(crate) const fn resource_glue(kind: ResourceKind) -> MemoryDropGlueId {
    MemoryDropGlueId::new(1 + kind as u64)
}

const fn bytes_glue() -> MemoryDropGlueId {
    MemoryDropGlueId::new(1 + ResourceKind::ALL.len() as u64)
}

fn execution_cutover(ty: &Type) -> Option<MemoryExecutionCutover> {
    match ty {
        Type::Str => Some(MemoryExecutionCutover::StructuralString),
        Type::Path => Some(MemoryExecutionCutover::StructuralPath),
        Type::Product(name) => Some(MemoryExecutionCutover::Product(name.clone())),
        Type::Enum { id, arguments, .. } => Some(MemoryExecutionCutover::Enum {
            id: id.bytes(),
            arguments: arguments.iter().map(memory_type).collect(),
        }),
        _ => None,
    }
}

impl Producer<'_> {
    fn planned_parameter_mode(&mut self, ty: &Type, consumed: bool) -> Result<MemoryParameterMode> {
        if matches!(ty, Type::ByteSlice) {
            return Ok(MemoryParameterMode::BorrowShared);
        }
        if matches!(ty, Type::ByteSliceMut) {
            return Ok(MemoryParameterMode::BorrowExclusive);
        }
        if matches!(ty, Type::Resource(_)) {
            return Ok(if consumed {
                MemoryParameterMode::Consume
            } else {
                MemoryParameterMode::BorrowExclusive
            });
        }
        let id = self.type_planner.intern(ty)?;
        let fact = self.type_planner.fact(id)?;
        Ok(
            if fact.closure.class != MemoryClosureClass::Deterministic
                || matches!(ty, Type::List(_))
            {
                MemoryParameterMode::Copy
            } else {
                match fact.mode {
                    MemoryAggregateMode::Copy => MemoryParameterMode::Copy,
                    MemoryAggregateMode::ImmutableValue => MemoryParameterMode::BorrowShared,
                    MemoryAggregateMode::Affine => MemoryParameterMode::Consume,
                }
            },
        )
    }

    fn planned_result_mode(&mut self, ty: &Type) -> Result<MemoryResultMode> {
        let id = self.type_planner.intern(ty)?;
        let fact = self.type_planner.fact(id)?;
        if fact.contains_borrow {
            return Err(Error::msg(format!(
                "LKJ-MEM-BORROWED-RESULT type={:?} reason=borrowed result/escape",
                memory_type(ty),
            )));
        }
        if matches!(ty, Type::Resource(_)) {
            return Ok(MemoryResultMode::External);
        }
        Ok(
            if fact.closure.class != MemoryClosureClass::Deterministic
                || fact.mode == MemoryAggregateMode::Copy
                || matches!(ty, Type::List(_))
            {
                MemoryResultMode::Trivial
            } else {
                MemoryResultMode::Owned
            },
        )
    }

    fn finish_type_work(&mut self) -> Result<()> {
        self.work.type_nodes = u64::try_from(self.type_planner.facts.len())
            .map_err(|_| Error::msg("HIR memory-plan type facts exceed u64"))?;
        self.work.witnesses = u64::try_from(self.type_planner.witnesses.len())
            .map_err(|_| Error::msg("HIR memory-plan witnesses exceed u64"))?;
        if self.work.witnesses != self.work.type_nodes {
            return Err(Error::msg("HIR memory-plan witness table is not exact"));
        }
        self.work.type_edges = self.type_planner.graph.edges;
        self.work.scc_work = self.type_planner.graph.scc_work;
        self.work.aggregate_fields = self.type_planner.fields;
        self.work.aggregate_variants = self.type_planner.variants;
        self.work.destinations = u64::try_from(self.destinations.len())
            .map_err(|_| Error::msg("HIR memory-plan destinations exceed u64"))?;
        self.work.borrow_scopes = u64::try_from(self.borrow_scopes.len())
            .map_err(|_| Error::msg("HIR memory-plan borrow scopes exceed u64"))?;
        self.work.drop_paths = u64::try_from(self.type_planner.drop_paths.len())
            .map_err(|_| Error::msg("HIR memory-plan drop paths exceed u64"))?;
        Ok(())
    }

    fn reject_partial_projection(&mut self, expression: &Expr) -> Result<()> {
        let source = match &expression.kind {
            ExprKind::ProductField { value, .. }
            | ExprKind::WithProductField { value, .. }
            | ExprKind::EnumField { value, .. }
            | ExprKind::EnumUnwrap { value, .. } => value,
            _ => return Ok(()),
        };
        if !matches!(source.kind, ExprKind::Load(_)) {
            return Ok(());
        }
        let id = self.type_planner.intern(&expression.ty)?;
        if self.type_planner.fact(id)?.mode == MemoryAggregateMode::Affine {
            return Err(Error::msg(format!(
                "LKJ-MEM-PARTIAL-MOVE type={:?} path={:?} reason=affine aggregate field projection",
                memory_type(&expression.ty),
                expression_kind(&expression.kind),
            )));
        }
        Ok(())
    }

    fn add_destination(
        &mut self,
        expression: &Expr,
        expression_id: MemoryExpressionId,
    ) -> Result<()> {
        let entry_id = *self
            .expression_entries
            .get(&expression_id)
            .ok_or_else(|| Error::msg("aggregate destination lost expression entry"))?;
        let entry_index = entry_id
            .index()
            .ok_or_else(|| Error::msg("aggregate entry exceeds usize"))?;
        let type_fact = self.entries[entry_index].type_fact;
        let fact = self.type_planner.fact(type_fact)?.clone();
        let mut children = self
            .child_entries
            .remove(&expression_id)
            .unwrap_or_default();
        children.sort_by_key(|item| item.0);
        let (field_count, active_payload) = destination_shape(
            self.program,
            &self.products_by_id,
            &self.enums_by_id,
            expression,
        )?;
        let field_count_index = usize::try_from(field_count)
            .map_err(|_| Error::msg("destination field count exceeds host usize"))?;
        if children.len() != field_count_index {
            return Err(Error::msg(
                "LKJ-MEM-INCOMPLETE-DESTINATION field count mismatch",
            ));
        }
        let id = MemoryDestinationId::new(
            u64::try_from(self.destinations.len())
                .map_err(|_| Error::msg("HIR memory-plan destination identity exceeds u64"))?,
        );
        let initialized_order: Vec<u64> = (0..field_count).collect();
        let fields = children
            .into_iter()
            .map(|(index, expression, drop_path)| MemoryDestinationField {
                index,
                expression,
                drop_path,
            })
            .collect();
        let (kind, execution, execution_cutover) = match fact.closure.class {
            MemoryClosureClass::Deterministic => (
                MemoryDestinationKind::CutoverRequired,
                MemoryExecution::CutoverRequired,
                execution_cutover(&expression.ty),
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
        self.destinations.push(MemoryDestinationPlan {
            id,
            function: self.current_function,
            expression: expression_id,
            kind,
            execution,
            execution_cutover,
            type_fact,
            field_count,
            fields,
            active_payload,
            initialized_order: initialized_order.clone(),
            reverse_abort_cleanup: initialized_order.into_iter().rev().collect(),
        });
        self.entries[entry_index].destination = Some(id);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DropFlow {
    initialized: bool,
    conditional: bool,
}

impl Producer<'_> {
    fn finish_drop_classes(&mut self) -> Result<()> {
        let classes = self
            .obligations
            .iter()
            .map(|obligation| self.drop_class(obligation))
            .collect::<Result<Vec<_>>>()?;
        for (obligation, class) in self.obligations.iter_mut().zip(classes) {
            obligation.drop_class = class;
        }
        Ok(())
    }

    fn drop_class(&self, obligation: &MemoryObligation) -> Result<Option<MemoryDropClass>> {
        if matches!(obligation.kind, MemoryObligationKind::EndBorrow) {
            return Ok(None);
        }
        let entry = obligation
            .entry
            .index()
            .and_then(|index| self.entries.get(index))
            .ok_or_else(|| Error::msg("HIR drop obligation entry is missing"))?;
        let MemorySubject::Place { binding, .. } = entry.subject else {
            return Err(Error::msg(
                "HIR drop obligation does not name a whole place",
            ));
        };
        let body = function_body(self.program, obligation.function)?;
        let flow = producer_drop_flow(
            body,
            BindingId::new(binding),
            DropFlow {
                initialized: true,
                conditional: false,
            },
        )?;
        Ok(Some(if flow.conditional {
            MemoryDropClass::Conditional
        } else if flow.initialized {
            MemoryDropClass::Static
        } else {
            MemoryDropClass::Dead
        }))
    }
}

fn function_body(program: &hir::Program, function: MemoryFunctionId) -> Result<&Expr> {
    let index = function
        .index()
        .ok_or_else(|| Error::msg("HIR drop class function identity exceeds usize"))?;
    if let Some(function) = program.functions.get(index) {
        Ok(&function.body)
    } else if index == program.functions.len() {
        Ok(&program.main.body)
    } else {
        Err(Error::msg("HIR drop class function identity is missing"))
    }
}

fn producer_drop_flow(
    expression: &Expr,
    binding: BindingId,
    mut flow: DropFlow,
) -> Result<DropFlow> {
    if directly_consumes(expression, binding) {
        if !flow.initialized {
            return Err(open_drop_error());
        }
        flow.initialized = false;
        return Ok(flow);
    }
    match &expression.kind {
        ExprKind::SetLocal { target, value, .. } if *target == binding => {
            flow = producer_drop_flow(value, binding, flow)?;
            if flow.initialized {
                return Err(open_drop_error());
            }
            flow.initialized = true;
            Ok(flow)
        }
        ExprKind::If {
            condition,
            then_branch,
            else_branch,
        } => {
            let entry = producer_drop_flow(condition, binding, flow)?;
            let left = producer_drop_flow(then_branch, binding, entry)?;
            let right = producer_drop_flow(else_branch, binding, entry)?;
            match (then_branch.ty == Type::Never, else_branch.ty == Type::Never) {
                (true, false) => Ok(right),
                (false, true) => Ok(left),
                (true, true) => Ok(entry),
                (false, false) if left.initialized == right.initialized => Ok(DropFlow {
                    initialized: left.initialized,
                    conditional: left.conditional || right.conditional,
                }),
                (false, false) => Ok(DropFlow {
                    initialized: false,
                    conditional: true,
                }),
            }
        }
        ExprKind::While { .. } | ExprKind::Loop { .. } => {
            let after = producer_drop_children(expression, binding, flow)?;
            if after == flow {
                Ok(flow)
            } else {
                Err(open_drop_error())
            }
        }
        _ => producer_drop_children(expression, binding, flow),
    }
}

fn producer_drop_children(
    expression: &Expr,
    binding: BindingId,
    mut flow: DropFlow,
) -> Result<DropFlow> {
    for child in children(expression) {
        flow = producer_drop_flow(child, binding, flow)?;
    }
    Ok(flow)
}

fn directly_consumes(expression: &Expr, binding: BindingId) -> bool {
    match &expression.kind {
        ExprKind::Move { binding: moved, .. } => moved.binding == binding,
        ExprKind::Operation {
            operation, args, ..
        } if consuming_operation(*operation) => args
            .iter()
            .any(|argument| expression_uses_binding(argument, binding)),
        _ => false,
    }
}

fn open_drop_error() -> Error {
    Error::msg("HIR memory plan rejects an open or multiply consumed whole place")
}

impl Producer<'_> {
    fn add_inferred_borrow_scope(
        &mut self,
        call: MemoryCallId,
        argument_index: usize,
        argument: &Expr,
        expression: MemoryExpressionId,
        end_after: MemoryExpressionId,
    ) -> Result<()> {
        let call_index = call
            .index()
            .ok_or_else(|| Error::msg("call identity exceeds usize"))?;
        let mode = *self
            .calls
            .get(call_index)
            .and_then(|item| item.parameters.get(argument_index))
            .ok_or_else(|| Error::msg("call borrow parameter is missing"))?;
        let kind = match mode {
            MemoryParameterMode::BorrowShared => MemoryBorrowKind::Shared,
            MemoryParameterMode::BorrowExclusive => MemoryBorrowKind::Exclusive,
            _ => return Ok(()),
        };
        let ExprKind::Load(reference) = argument.kind else {
            return Ok(());
        };
        let type_id = self.type_planner.intern(&argument.ty)?;
        let fact = self.type_planner.fact(type_id)?;
        if kind == MemoryBorrowKind::Shared
            && (fact.closure.class != MemoryClosureClass::Deterministic
                || fact.mode != MemoryAggregateMode::ImmutableValue)
        {
            return Ok(());
        }
        let place = *self
            .places_by_binding
            .get(&(self.current_function, reference.binding.raw()))
            .ok_or_else(|| Error::msg("inferred direct-call borrow lost source place"))?;
        let id = MemoryBorrowScopeId::new(
            u64::try_from(self.borrow_scopes.len())
                .map_err(|_| Error::msg("HIR memory-plan borrow scope identity exceeds u64"))?,
        );
        let entry_id = *self
            .expression_entries
            .get(&expression)
            .ok_or_else(|| Error::msg("inferred direct-call borrow lost argument entry"))?;
        let entry = entry_id
            .index()
            .and_then(|index| self.entries.get_mut(index))
            .filter(|entry| entry.id == entry_id)
            .ok_or_else(|| Error::msg("inferred direct-call borrow argument entry is stale"))?;
        entry.borrow_scope = Some(id);
        entry.copy_share = if kind == MemoryBorrowKind::Shared {
            MemoryCopySharePlan::BorrowShared
        } else {
            MemoryCopySharePlan::BorrowExclusive
        };
        self.calls[call_index].borrow_scopes[argument_index] = Some(id);
        self.borrow_scopes.push(MemoryBorrowScopePlan {
            id,
            function: self.current_function,
            call,
            argument_index: index_u64(argument_index)?,
            source_expression: expression,
            binding: reference.binding.raw(),
            place,
            kind,
            semantic_uses: 1,
            end_after,
        });
        Ok(())
    }
}
