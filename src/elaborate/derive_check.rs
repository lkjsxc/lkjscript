//! Validate derive declarations independently of instantiation and method use.
//!
//! This checks the structural contracts already used by trait selection. It does
//! not substitute for implementing the executable Clone/comparison operations.
use super::{Compiler, Result, a, err};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn validate(compiler: &mut Compiler<'_>) -> Result<()> {
    implementation_contracts(compiler)?;
    let source = compiler.source;
    for (declaration, item) in source.declarations.iter().enumerate() {
        let (parameters, attrs, fields) = match item.item {
            a::Item::Struct(index) => {
                let definition = &source.structs[index];
                (
                    &definition.generics,
                    &definition.attrs,
                    definition.fields.iter().collect::<Vec<_>>(),
                )
            }
            a::Item::Enum(index) => {
                let definition = &source.enums[index];
                (
                    &definition.generics,
                    &definition.attrs,
                    definition
                        .variants
                        .iter()
                        .flat_map(|variant| &variant.fields)
                        .collect(),
                )
            }
            _ => continue,
        };
        let mut seen = BTreeSet::new();
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
                    let generated = source.impls.iter().find(|implementation| {
                        implementation.span == item.span && matches!(&source.types[implementation.target].kind, a::Type::Path(path) if path.declaration == Some(declaration))
                    });
                    if generated.and_then(|implementation| implementation.trait_ty).is_none_or(|ty| !matches!(&source.types[ty].kind, a::Type::Path(path) if path.declaration == standard)) {
                        return Err(err(attr.span, "E_TRAIT", "derive Default requires the standard Default trait; help: avoid shadowing std at this derive"));
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
                if !seen.insert(bound) {
                    return Err(err(
                        attr.span,
                        "E_TRAIT",
                        &format!("duplicate derive {bound}"),
                    ));
                }
                crate::generics::derived_coherence(source, declaration, bound)?;
                let mut substitutions = BTreeMap::new();
                compiler.declare_parameters(parameters, &mut substitutions)?;
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
                    if !compiler.satisfies_bound(ty, parent) {
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
    let source = compiler.source;
    for implementation in &source.impls {
        let Some(trait_ty) = implementation.trait_ty else {
            continue;
        };
        let a::Type::Path(path) = &source.types[trait_ty].kind else {
            continue;
        };
        let crate::generics::TraitIdentity::Builtin(bound) =
            crate::generics::bound_identity(source, path)
        else {
            continue;
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
        if !path.args.is_empty() || !implementation.associated.is_empty() {
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
        let mut methods = BTreeSet::new();
        for method in &implementation.methods {
            let function = &source.functions[*method];
            if !methods.insert(function.name.as_str()) {
                return Err(err(
                    function.span,
                    "E_TRAIT",
                    "duplicate builtin implementation method",
                ));
            }
            if function
                .params
                .first()
                .is_none_or(|parameter| parameter.name != "self")
            {
                return Err(err(
                    function.span,
                    "E_TRAIT",
                    "builtin method requires a self receiver",
                ));
            }
            let (parameters, result) = match (bound.as_str(), function.name.as_str()) {
                ("Clone", "clone") => (vec![shared], receiver),
                ("Clone", "clone_from") => (vec![exclusive, shared], super::k::UNIT),
                ("PartialEq", "eq" | "ne") => (vec![shared, shared], super::k::BOOL),
                _ => {
                    return Err(err(
                        function.span,
                        "E_TRAIT",
                        "method is not declared by builtin trait",
                    ));
                }
            };
            if function.body.is_none()
                || function
                    .generics
                    .iter()
                    .any(|parameter| !parameter.lifetime)
                || function.params.len() != parameters.len()
            {
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
        if required.is_some_and(|name| !methods.contains(name)) {
            return Err(err(
                implementation.span,
                "E_TRAIT",
                "builtin implementation is missing a required method",
            ));
        }
        for parent in crate::generics::builtin_parents(&bound) {
            if !compiler.satisfies_bound(receiver, parent) {
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
