//! Resolve and type-analyze parsed programs into owned HIR.

use std::collections::{HashMap, HashSet};

use lkjscript_core::{Error, ProductId, Result};

use crate::hir::{
    self, Binding, BindingId, BindingKind, BindingRef, BindingStorage, BorrowKind, CoreTrait,
    EffectSet, EnumDefinition, EnumId, EnumLayoutFacts, EnumVariant, EnumVariantField, Expr,
    ExprKind, Function, GenericInstantiation, ImplDefinition, ImplId, LoanId, LocalDefinition,
    LoopId, Main, MatchBindingAssignment, MatchEdgeTarget, MatchFieldPattern, MatchLocal,
    MatchPattern, MatchPlan, MatchPlanId, MatchProjection, MatchTest, MatchTestKind, Operation,
    Origin, PlaceId, PlannedMatchArm, ProductDefinition, ProductField, RuntimeLayoutId, Source,
    SourceId, TraitBound, TraitDefinition, TraitId, Type, TypeSubstitution, VariantFieldId,
    VariantId,
};
use crate::source::Expr as AstExpr;

use crate::source::ValidatedSourceTree;
use crate::types::parse_one;

#[cfg(test)]
pub(crate) fn analyze_program(program: &ValidatedSourceTree) -> Result<hir::Program> {
    let mut program = analyze_program_without_effects(program)?;
    crate::effects::infer(&mut program);
    Ok(program)
}

pub(crate) fn analyze_program_without_effects(
    program: &ValidatedSourceTree,
) -> Result<hir::Program> {
    let mut analyzer = Analyzer::new(program)?;
    analyzer.install_operations()?;
    analyzer.install_prelude_enums()?;
    analyzer.install_core_traits()?;
    analyzer.collect_trait_names(program)?;
    analyzer.collect_product_names(program)?;
    analyzer.collect_enum_names(program)?;
    analyzer.collect_enums(program)?;
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
        enums: analyzer.enums,
        traits: analyzer.traits,
        implementations: analyzer.implementations,
        match_plans: analyzer.match_plans,
        functions,
        main,
        global_layout,
    };
    let mut complete = program.clone();
    resolution::matching::lower_semantic_matches(&mut complete)?;
    crate::ownership::check(&complete)?;
    Ok(program)
}

struct Analyzer {
    sources: Vec<Source>,
    bindings: Vec<Binding>,
    globals: HashMap<String, BindingId>,
    operations: HashMap<Operation, BindingId>,
    product_names: HashMap<String, ProductId>,
    products: Vec<ProductDefinition>,
    enum_headers: HashMap<String, (EnumId, Vec<String>)>,
    enums: Vec<EnumDefinition>,
    trait_names: HashMap<String, TraitId>,
    traits: Vec<TraitDefinition>,
    implementations: Vec<ImplDefinition>,
    implementation_index: HashMap<(TraitId, ProductId), ImplId>,
    function_bounds: HashMap<BindingId, Vec<TraitBound>>,
    match_plans: Vec<MatchPlan>,
    next_loan: u64,
}

mod declarations;
mod diagnostics;
mod interface;
mod resolution;

pub(crate) use interface::analyze_interface_program;

use declarations::*;
use diagnostics::{AnalysisDiagnostic, NameUse};
use resolution::*;

pub(crate) fn verify_match_plans(program: &hir::Program) -> Result<()> {
    resolution::matching::verify_match_plans(program)
}

pub(crate) fn build_match_plan(
    id: MatchPlanId,
    origin: Origin,
    scrutinee: MatchLocal,
    arms: Vec<PlannedMatchArm>,
    enums: &[EnumDefinition],
    products: &[ProductDefinition],
) -> Result<MatchPlan> {
    resolution::matching::build_match_plan(id, origin, scrutinee, arms, enums, products)
}

pub(crate) fn lower_semantic_matches(program: &mut hir::Program) -> Result<()> {
    resolution::matching::lower_semantic_matches(program)
}

pub(crate) fn is_reserved_semantic_name(name: &str) -> bool {
    declarations::is_builtin_type_name(name) || resolution::is_contextual_name(name)
}

struct Resolver<'a> {
    analyzer: &'a mut Analyzer,
    origin: SourceId,
    scopes: Vec<HashMap<String, BindingId>>,
    local_slots: HashMap<BindingId, usize>,
    local_places: HashMap<BindingId, PlaceId>,
    type_variables: HashSet<String>,
    next_slot: usize,
    max_slots: usize,
    next_place: u64,
    return_type: Type,
    loops: Vec<LoopContext>,
    next_loop: u64,
}

#[derive(Clone)]
struct LoopContext {
    id: LoopId,
    result_type: Type,
    is_while: bool,
}

struct PendingFunction<'a> {
    binding: BindingId,
    origin: SourceId,
    parsed: ParsedFunction<'a>,
    bounds: Vec<TraitBound>,
}

struct PendingMain<'a> {
    origin: SourceId,
    param_names: Vec<String>,
    param_types: Vec<Type>,
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
