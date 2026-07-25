//! Resolve and type-analyze parsed programs into owned HIR.

use std::collections::{HashMap, HashSet};

use lkjscript_core::{BudgetLedger, Error, ProductId, Result, MAX_PRODUCT_FIELDS};

use crate::hir::{
    self, Binding, BindingId, BindingKind, BindingRef, BindingStorage, BorrowKind, CoreTrait,
    EffectSet, Expr, ExprKind, Function, GenericInstantiation, ImplDefinition, ImplId, LoanId,
    LocalDefinition, Main, Operation, Origin, PlaceId, ProductDefinition, ProductField, Source,
    SourceId, TraitBound, TraitDefinition, TraitId, TraitWitness, TraitWitnessKind, Type,
    TypeSubstitution,
};
use crate::source::Expr as AstExpr;

pub const TRAIT_SOLVER_MAX_DEPTH: usize = 32;
pub const TRAIT_SOLVER_MAX_WORK: usize = 256;
use crate::source::ValidatedSourceTree;
use crate::types::parse_one;

#[cfg(test)]
pub(crate) fn analyze_program(program: &ValidatedSourceTree) -> Result<hir::Program> {
    let mut program = analyze_program_without_effects(program)?;
    crate::effects::infer(&mut program);
    Ok(program)
}

pub(crate) fn analyze_program_with_budget(
    source: &ValidatedSourceTree,
    ledger: &mut BudgetLedger,
) -> Result<hir::Program> {
    let mut program = analyze_program_without_effects_with_budget(source, ledger)?;
    crate::effects::infer(&mut program);
    Ok(program)
}

pub(crate) fn analyze_program_without_effects_with_budget(
    source: &ValidatedSourceTree,
    ledger: &mut BudgetLedger,
) -> Result<hir::Program> {
    let program = analyze_program_without_effects(source)?;
    crate::budget::charge_hir(&program, ledger)?;
    Ok(program)
}

pub(crate) fn analyze_program_without_effects(
    program: &ValidatedSourceTree,
) -> Result<hir::Program> {
    let mut analyzer = Analyzer::new(program)?;
    analyzer.install_operations()?;
    analyzer.install_core_traits()?;
    analyzer.collect_trait_names(program)?;
    analyzer.collect_product_names(program)?;
    analyzer.collect_products(program)?;
    analyzer.collect_implementations(program)?;
    let (pending_functions, pending_main) = analyzer.collect_headers(program)?;

    let mut functions = Vec::with_capacity(pending_functions.len());
    for function in pending_functions {
        functions.push(analyzer.resolve_function(
            function.binding,
            function.origin,
            function.parsed,
            function.bounds,
        )?);
    }
    let main = analyzer.resolve_main(pending_main)?;
    let global_layout = analyzer.build_global_layout(&functions)?;

    let program = hir::Program {
        sources: analyzer.sources,
        bindings: analyzer.bindings,
        products: analyzer.products,
        traits: analyzer.traits,
        implementations: analyzer.implementations,
        functions,
        main,
        global_layout,
    };
    crate::ownership::check(&program)?;
    Ok(program)
}

struct Analyzer {
    sources: Vec<Source>,
    bindings: Vec<Binding>,
    globals: HashMap<String, BindingId>,
    operations: HashMap<Operation, BindingId>,
    product_names: HashMap<String, ProductId>,
    products: Vec<ProductDefinition>,
    trait_names: HashMap<String, TraitId>,
    traits: Vec<TraitDefinition>,
    implementations: Vec<ImplDefinition>,
    implementation_index: HashMap<(TraitId, ProductId), ImplId>,
    function_bounds: HashMap<BindingId, Vec<TraitBound>>,
    next_loan: u32,
}

mod declarations;
mod resolution;

use declarations::*;
use resolution::*;

struct Resolver<'a> {
    analyzer: &'a mut Analyzer,
    origin: SourceId,
    scopes: Vec<HashMap<String, BindingId>>,
    local_slots: HashMap<BindingId, u8>,
    local_places: HashMap<BindingId, PlaceId>,
    type_variables: HashSet<String>,
    next_slot: usize,
    max_slots: usize,
    next_place: u32,
}

struct PendingFunction<'a> {
    binding: BindingId,
    origin: SourceId,
    parsed: ParsedFunction<'a>,
    bounds: Vec<TraitBound>,
}

struct PendingMain<'a> {
    origin: SourceId,
    return_type: Type,
    body: &'a AstExpr,
}

struct ParsedBound {
    parameter: String,
    trait_name: String,
}

struct ParsedFunction<'a> {
    signature_params: Vec<Type>,
    signature_return: Type,
    param_names: Vec<String>,
    param_types: Vec<Type>,
    body: &'a AstExpr,
    forall_vars: Vec<String>,
    bounds: Vec<ParsedBound>,
}

#[cfg(test)]
mod tests;
