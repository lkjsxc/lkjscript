use std::sync::Arc;

use crate::hir::{
    self, Binding, Function, ImplDefinition, MatchPlan, ProductDefinition, TraitDefinition,
};
use lkjscript_core::{Error, Result};

/// The single mutable semantic program authority.
///
/// Unlike compiler HIR, it permits an absent entry point and real hole
/// expressions. Imported source records are retained separately as optional
/// diagnostic provenance; they are not required for semantic existence.
#[derive(Debug)]
pub(super) struct SemanticProgram {
    pub bindings: Vec<Binding>,
    pub products: Vec<ProductDefinition>,
    pub enums: Vec<hir::EnumDefinition>,
    pub traits: Vec<TraitDefinition>,
    pub implementations: Vec<ImplDefinition>,
    pub match_plans: Vec<MatchPlan>,
    pub functions: Vec<Function>,
    pub main: Option<hir::Main>,
    pub global_layout: Vec<hir::BindingId>,
}

impl SemanticProgram {
    pub(super) fn empty() -> Result<Self> {
        let mut traits = Vec::new();
        install_core_trait_definitions(&mut traits)?;
        Ok(Self {
            bindings: Vec::new(),
            products: Vec::new(),
            enums: Vec::new(),
            traits,
            implementations: Vec::new(),
            match_plans: Vec::new(),
            functions: Vec::new(),
            main: None,
            global_layout: Vec::new(),
        })
    }

    pub(super) fn from_hir(program: hir::Program) -> (Self, Arc<[hir::Source]>) {
        let hir::Program {
            sources,
            bindings,
            products,
            enums,
            traits,
            implementations,
            match_plans,
            functions,
            main,
            global_layout,
        } = program;
        (
            Self {
                bindings,
                products,
                enums,
                traits,
                implementations,
                match_plans,
                functions,
                main: Some(main),
                global_layout,
            },
            sources.into(),
        )
    }

    pub(super) fn binding(&self, id: hir::BindingId) -> Option<&Binding> {
        id.index().and_then(|index| self.bindings.get(index))
    }

    pub(super) fn try_complete(&self, sources: &[hir::Source]) -> Result<hir::Program> {
        let main = self
            .main
            .as_ref()
            .ok_or_else(|| Error::msg("semantic program has no entry point"))?;
        if contains_holes(&main.body)
            || self
                .functions
                .iter()
                .any(|function| contains_holes(&function.body))
        {
            return Err(Error::msg(
                "semantic program contains incomplete expressions",
            ));
        }
        let mut functions = Vec::new();
        functions
            .try_reserve(self.functions.len())
            .map_err(|_| Error::host("complete HIR function allocation failed"))?;
        for function in &self.functions {
            functions.push(hir::Function {
                binding: function.binding,
                origin: function.origin,
                params: clone_values(&function.params, "function parameters")?,
                param_places: clone_values(&function.param_places, "function places")?,
                bounds: clone_values(&function.bounds, "function bounds")?,
                arity: function.arity,
                local_count: function.local_count,
                summary: function.summary,
                body: function.body.try_clone()?,
            });
        }
        let mut complete = hir::Program {
            sources: clone_values(sources, "source provenance")?,
            bindings: clone_values(&self.bindings, "bindings")?,
            products: clone_values(&self.products, "products")?,
            enums: clone_values(&self.enums, "enums")?,
            traits: clone_values(&self.traits, "traits")?,
            implementations: clone_values(&self.implementations, "implementations")?,
            match_plans: clone_values(&self.match_plans, "match plans")?,
            functions,
            main: hir::Main {
                origin: main.origin,
                params: clone_values(&main.params, "main parameters")?,
                param_places: clone_values(&main.param_places, "main places")?,
                param_types: clone_values(&main.param_types, "main parameter types")?,
                return_type: main.return_type.clone(),
                arity: main.arity,
                local_count: main.local_count,
                body: main.body.try_clone()?,
            },
            global_layout: clone_values(&self.global_layout, "global layout")?,
        };
        crate::analyze::lower_semantic_matches(&mut complete)?;
        crate::effects::infer(&mut complete);
        Ok(complete)
    }
}

fn contains_holes(root: &hir::Expr) -> bool {
    let mut pending = vec![root];
    while let Some(expression) = pending.pop() {
        if matches!(expression.kind, hir::ExprKind::Hole) {
            return true;
        }
        hir::for_each_expression_child(expression, &mut |child| pending.push(child));
    }
    false
}

fn clone_values<T: Clone>(values: &[T], name: &str) -> Result<Vec<T>> {
    let mut result = Vec::new();
    result
        .try_reserve(values.len())
        .map_err(|_| Error::host(format!("complete HIR {name} allocation failed")))?;
    result.extend(values.iter().cloned());
    Ok(result)
}

pub(super) fn install_core_traits_if_absent(program: &mut hir::Program) -> Result<()> {
    if !program.traits.is_empty() {
        return Ok(());
    }
    install_core_trait_definitions(&mut program.traits)
}

fn install_core_trait_definitions(traits: &mut Vec<hir::TraitDefinition>) -> Result<()> {
    traits
        .try_reserve(hir::CoreTrait::ALL.len())
        .map_err(|_| Error::host("core trait metadata allocation failed"))?;
    for core in hir::CoreTrait::ALL {
        let raw = u64::try_from(traits.len())
            .map_err(|_| Error::host("core trait identity exceeds u64"))?;
        traits.push(hir::TraitDefinition {
            id: hir::TraitId::new(raw),
            name: core.name().to_owned(),
            origin: hir::Origin::Builtin,
            core: Some(core),
        });
    }
    Ok(())
}
