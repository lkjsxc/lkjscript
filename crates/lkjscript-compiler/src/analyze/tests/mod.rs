#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::path::PathBuf;

use lkjscript_core::{Op, Result};

use super::analyze_program;
use crate::codegen::compile_program;
use crate::hir::{
    BindingKind, BindingStorage, CoreTrait, EffectSet, ExprKind, Operation, Origin,
    TraitWitnessKind, Type,
};
use crate::source::{validate_source_set_for_analysis, ValidatedSourceTree};
use crate::ssa::lower_program;

fn parsed_program(files: &[(&str, &str)]) -> Result<ValidatedSourceTree> {
    let root = files.last().map_or("test.lkjscript", |(path, _)| *path);
    validate_source_set_for_analysis(files, root, &lkjscript_core::Limits::default())
}

fn analyze_one(source: &str) -> Result<crate::hir::Program> {
    analyze_program(&parsed_program(&[("test.lkjscript", source)])?)
}

fn compile_hir(program: &crate::hir::Program) -> Result<lkjscript_core::Chunk> {
    let ssa = lower_program(program)?;
    compile_program(&ssa).map(|(chunk, _links)| chunk)
}

fn analysis_error(source: &str) -> String {
    analyze_one(source)
        .expect_err("analysis must fail")
        .to_string()
}

fn main_source(return_type: &str, body: &str) -> String {
    format!("main/\nsig/\ninputs/\n/inputs\noutput/\n{return_type}\n/output\n/sig\n{body}\n/main\n")
}

fn function_source(
    name: &str,
    forall: &[&str],
    signature: &str,
    params: &str,
    body: &str,
) -> String {
    let forall = if forall.is_empty() {
        String::new()
    } else {
        format!("forall/\n{}\n/forall\n", forall.join("\n"))
    };
    format!(
        "def/\nname/\n{name}\n/name\nfn/\n{forall}sig/\n{signature}\n/sig\nparams/\n{params}\n/params\n{body}\n/fn\n/def\n"
    )
}

fn summary(program: &crate::hir::Program, name: &str) -> EffectSet {
    let binding = program
        .bindings
        .iter()
        .find(|binding| binding.name == name)
        .expect("named function binding")
        .id;
    program
        .functions
        .iter()
        .find(|function| function.binding == binding)
        .expect("named HIR function")
        .summary
}

const POINT_PRODUCT: &str = "product/\nname/\npoint\n/name\nfields/\nfield/\nname/\nx\n/name\ntype/\ni64\n/type\n/field\nfield/\nname/\ny\n/name\ntype/\ni64\n/type\n/field\n/fields\n/product\n";

fn marker_trait(name: &str) -> String {
    format!("trait/\nname/\n{name}\n/name\n/trait\n")
}

fn marker_impl(trait_name: &str, product_name: &str) -> String {
    format!("impl/\ntrait/\n{trait_name}\n/trait\nfor/\nproduct\n{product_name}\n/for\n/impl\n")
}

fn bounded_identity(name: &str, trait_name: &str) -> String {
    format!(
        "def/\nname/\n{name}\n/name\nfn/\nforall/\nt\n/forall\nbounds/\nbound/\nt\n{trait_name}\n/bound\n/bounds\nsig/\ninputs/\nt\n/inputs\noutput/\nt\n/output\n/sig\nparams/\nvalue\nt\n/params\nvalue\n/fn\n/def\n"
    )
}

mod effects_direct;
mod effects_recursive;
mod enum_values;
mod enums;
mod matches;
mod metadata;
mod products;
mod program_shape;
mod traits_auto;
mod traits_identity;
mod traits_validation;
