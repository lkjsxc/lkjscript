//! Resolve and type-analyze parsed programs into owned HIR.

use std::collections::{HashMap, HashSet};

use lkjscript_core::{Error, ProductId, Result};

use crate::hir::{
    self, Binding, BindingId, BindingKind, BindingRef, BindingStorage, BorrowKind, CoreTrait,
    EffectSet, EnumDefinition, EnumId, EnumLayoutFacts, EnumVariant, EnumVariantField, Expr,
    ExprKind, Function, GenericInstantiation, ImplDefinition, ImplId, LoanId, LocalDefinition,
    LoopId, Main, MatchBindingAssignment, MatchEdgeTarget, MatchFieldPattern, MatchLocal,
    MatchPattern, MatchPlan, MatchPlanCharges, MatchPlanId, MatchProjection, MatchTest,
    MatchTestKind, Operation, Origin, PlaceId, PlannedMatchArm, ProductDefinition, ProductField,
    RuntimeLayoutId, Source, SourceId, TraitBound, TraitDefinition, TraitId, TraitWitness,
    TraitWitnessKind, Type, TypeSubstitution, VariantFieldId, VariantId,
};
use crate::source::Expr as AstExpr;

use crate::source::ValidatedSourceTree;
use crate::types::parse_one;

pub(crate) fn analyze_program(program: &ValidatedSourceTree) -> Result<hir::Program> {
    let mut program = analyze_program_without_effects(program)?;
    crate::effects::infer(&mut program);
    Ok(program)
}

pub(crate) fn analyze_module_program(program: &ValidatedSourceTree) -> Result<hir::Program> {
    let projection = program
        .module_scoped_projection()
        .map_err(crate::source::SourceDiagnostic::into_core)?;
    analyze_program(&projection)
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
    enum_headers: HashMap<String, (EnumId, Vec<String>)>,
    enums: Vec<EnumDefinition>,
    trait_names: HashMap<String, TraitId>,
    traits: Vec<TraitDefinition>,
    implementations: Vec<ImplDefinition>,
    implementation_index: HashMap<(TraitId, ProductId), ImplId>,
    function_bounds: HashMap<BindingId, Vec<TraitBound>>,
    match_plans: Vec<MatchPlan>,
    next_loan: u32,
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

struct Resolver<'a> {
    analyzer: &'a mut Analyzer,
    origin: SourceId,
    scopes: Vec<HashMap<String, BindingId>>,
    local_slots: HashMap<BindingId, usize>,
    local_places: HashMap<BindingId, PlaceId>,
    type_variables: HashSet<String>,
    next_slot: usize,
    max_slots: usize,
    next_place: u32,
    return_type: Type,
    loops: Vec<LoopContext>,
    next_loop: u32,
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
