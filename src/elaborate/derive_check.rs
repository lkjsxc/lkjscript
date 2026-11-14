//! Validate derive declarations independently of instantiation and method use.
//!
//! This checks the structural contracts already used by trait selection. It does
//! not substitute for implementing the executable Clone/comparison operations.
use super::{Compiler, Result, a, err};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate(compiler: &mut Compiler<'_>) -> Result<()> {
    implementation_contracts(compiler)?;
    for declaration in 0..compiler.source.declarations.len() {
        let item = compiler.source.declarations[declaration].clone();
        let (parameters, attrs, fields) = match item.item {
            a::Item::Struct(index) => {
                let definition = &compiler.source.structs[index];
                (
                    definition.generics.clone(),
                    definition.attrs.clone(),
                    definition.fields.clone(),
                )
            }
            a::Item::Enum(index) => {
                let definition = &compiler.source.enums[index];
                let mut fields = Vec::new();
                for variant in &definition.variants {
                    fields.extend_from_slice(&variant.fields);
                }
                (
                    definition.generics.clone(),
                    definition.attrs.clone(),
                    fields,
                )
            }
            _ => continue,
        };
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for attr in attrs {
            if attr.name != "derive" {
                continue;
            }
            for bound in &attr.args {
                if bound == "Default" {
                    if !matches!(item.item, a::Item::Struct(_)) {
                        return Err(err(
                            attr.span,
                            "E_UNSUPPORTED",
                            "derive Default on enums needs an explicit default variant; help: write an explicit Default implementation",
                        ));
                    }
                    let standard = compiler
                        .names
                        .library_type("prelude", "Default", attr.span)?
                        .declaration;
                    let mut generated = None;
                    for implementation in &compiler.source.impls {
                        if implementation.span == item.span
                            && matches!(&compiler.source.types[implementation.target].kind, a::Type::Path(path) if path.declaration == Some(declaration))
                        {
                            generated = implementation.trait_ty;
                            break;
                        }
                    }
                    let valid = match generated {
                        Some(ty) => {
                            matches!(&compiler.source.types[ty].kind, a::Type::Path(path) if path.declaration == standard)
                        }
                        None => false,
                    };
                    if !valid {
                        return Err(err(
                            attr.span,
                            "E_TRAIT",
                            "derive Default requires the standard Default trait; help: avoid shadowing std at this derive",
                        ));
                    }
                    continue;
                }
                if !matches!(
                    bound.as_str(),
                    "Copy" | "Clone" | "PartialEq" | "Eq" | "PartialOrd" | "Ord"
                ) {
                    return Err(err(
                        attr.span,
                        "E_UNSUPPORTED",
                        &format!("derive {bound} is not implemented"),
                    ));
                }
                if !seen.insert(bound.clone()) {
                    return Err(err(
                        attr.span,
                        "E_TRAIT",
                        &format!("duplicate derive {bound}"),
                    ));
                }
                crate::generics::derived_coherence(compiler.source, declaration, bound)?;
                let mut substitutions = BTreeMap::new();
                compiler.declare_parameters(&parameters, &mut substitutions)?;
                for ty in substitutions.values() {
                    compiler.assume_builtin(*ty, bound);
                }
                for field in &fields {
                    let ty = compiler.resolve(field.ty, &substitutions)?;
                    if !compiler.satisfies_bound(ty, bound) {
                        return Err(err(
                            field.span,
                            if bound == "Copy" { "E_TYPE" } else { "E_TRAIT" },
                            &format!(
                                "derive {bound} requires field {} to implement {bound}",
                                field.name
                            ),
                        ));
                    }
                }
                let ty = compiler.named_type(
                    &a::Path {
                        declaration: Some(declaration),
                        name: item.name.clone(),
                        args: Vec::new(),
                    },
                    &substitutions,
                    attr.span,
                )?;
                for parent in crate::generics::builtin_parents(bound) {
                    if !compiler.satisfies_bound(ty, &parent) {
                        return Err(err(
                            attr.span,
                            "E_TRAIT",
                            &format!("derive {bound} requires {parent} for {}", item.name),
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

fn implementation_contracts(compiler: &mut Compiler<'_>) -> Result<()> {
    for implementation_index in 0..compiler.source.impls.len() {
        let implementation = compiler.source.impls[implementation_index].clone();
        let Some(trait_ty) = implementation.trait_ty else {
            continue;
        };
        let (bound, has_type_arguments) = {
            let a::Type::Path(path) = &compiler.source.types[trait_ty].kind else {
                continue;
            };
            let crate::generics::TraitIdentity::Builtin(bound) =
                crate::generics::bound_identity(compiler.source, path)
            else {
                continue;
            };
            (bound, !path.args.is_empty())
        };
        if !matches!(bound.as_str(), "Clone" | "PartialEq" | "Eq") {
            if matches!(
                bound.as_str(),
                "Copy" | "Ord" | "PartialOrd" | "Data" | "Send" | "Sync"
            ) {
                return Err(err(
                    implementation.span,
                    "E_UNSUPPORTED",
                    &format!("explicit builtin {bound} implementations are not implemented"),
                ));
            }
            continue;
        }
        if has_type_arguments || !implementation.associated.is_empty() {
            return Err(err(
                implementation.span,
                "E_UNSUPPORTED",
                "builtin implementation type arguments and associated types are not implemented",
            ));
        }
        let mut environment = BTreeMap::new();
        compiler.declare_parameters(&implementation.generics, &mut environment)?;
        let receiver = compiler.resolve(implementation.target, &environment)?;
        if !compiler.applications.contains_key(&receiver) {
            return Err(err(
                implementation.span,
                "E_UNSUPPORTED",
                &format!("explicit builtin {bound} requires a source nominal struct or enum"),
            ));
        }
        environment.insert("Self".to_string(), receiver);
        let shared = compiler.reference(receiver, false);
        let exclusive = compiler.reference(receiver, true);
        let mut methods: BTreeSet<String> = BTreeSet::new();
        for method in &implementation.methods {
            let function = compiler.source.functions[*method].clone();
            if !methods.insert(function.name.clone()) {
                return Err(err(
                    function.span,
                    "E_TRAIT",
                    "duplicate builtin implementation method",
                ));
            }
            if function.params.is_empty() || function.params[0].name != "self" {
                return Err(err(
                    function.span,
                    "E_TRAIT",
                    "builtin method requires a self receiver",
                ));
            }
            let (parameters, result) = match (bound.as_str(), function.name.as_str()) {
                ("Clone", "clone") => (vec![shared], receiver),
                ("Clone", "clone_from") => (vec![exclusive, shared], super::ir::UNIT),
                ("PartialEq", "eq" | "ne") => (vec![shared, shared], super::ir::BOOL),
                _ => {
                    return Err(err(
                        function.span,
                        "E_TRAIT",
                        "method is not declared by builtin trait",
                    ));
                }
            };
            let mut generic = false;
            for parameter in &function.generics {
                if !parameter.lifetime {
                    generic = true;
                }
            }
            if function.body.is_none() || generic || function.params.len() != parameters.len() {
                return Err(err(
                    function.span,
                    "E_TRAIT",
                    "builtin method signature differs from its contract",
                ));
            }
            for (parameter, expected) in function.params.iter().zip(parameters) {
                if compiler.resolve(parameter.ty, &environment)? != expected {
                    return Err(err(
                        parameter.span,
                        "E_TRAIT",
                        "builtin method parameter type differs from its contract",
                    ));
                }
            }
            if compiler.resolve(function.result, &environment)? != result {
                return Err(err(
                    function.span,
                    "E_TRAIT",
                    "builtin method result type differs from its contract",
                ));
            }
        }
        let required = match bound.as_str() {
            "Clone" => Some("clone"),
            "PartialEq" => Some("eq"),
            _ => None,
        };
        if required.is_some_and(|name| !methods.contains(&name.to_string())) {
            return Err(err(
                implementation.span,
                "E_TRAIT",
                "builtin implementation is missing a required method",
            ));
        }
        for parent in crate::generics::builtin_parents(&bound) {
            if !compiler.satisfies_bound(receiver, &parent) {
                return Err(err(
                    implementation.span,
                    "E_TRAIT",
                    &format!("builtin {bound} implementation requires {parent}"),
                ));
            }
        }
    }
    Ok(())
}
