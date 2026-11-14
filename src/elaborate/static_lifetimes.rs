//! The current K return contract can require every live view to be static.
//! Stronger requirements on individual parameters, fields, or result leaves need
//! leaf contracts; reject those source annotations instead of erasing them.
use super::{Result, a, err};
use std::collections::{BTreeMap, BTreeSet};

fn explicit_static(kind: &a::Type) -> bool {
    match kind {
        a::Type::Ref { lifetime, .. } => lifetime == "'static",
        a::Type::Path(path) => path.name == "'static",
        _ => false,
    }
}

fn children(kind: &a::Type, pending: &mut Vec<usize>) {
    match kind {
        a::Type::Path(path) => pending.extend_from_slice(&path.args),
        a::Type::Ref { inner, .. } | a::Type::Slice(inner) | a::Type::Array { inner, .. } => {
            pending.push(*inner)
        }
        a::Type::Tuple(fields) => pending.extend_from_slice(fields),
        a::Type::Function { params, result } => {
            pending.extend_from_slice(params);
            pending.push(*result);
        }
        _ => {}
    }
}

/// Every syntactic annotation is visited, including unreachable bodies and unused
/// definitions. Alias and associated-type definitions have their own type nodes.
pub(super) fn annotations(source: &a::Program) -> Result<()> {
    let mut outputs = vec![false; source.types.len()];
    let mut pending = Vec::new();
    for function in &source.functions {
        pending.push(function.result);
    }
    while let Some(id) = pending.pop() {
        if outputs[id] {
            continue;
        }
        outputs[id] = true;
        children(&source.types[id].kind, &mut pending);
    }
    for (id, node) in source.types.iter().enumerate() {
        if explicit_static(&node.kind) && !outputs[id] {
            return Err(err(
                node.span,
                "E_UNSUPPORTED",
                "static lifetime constraints are implemented only for wholly static function outputs",
            ));
        }
    }
    Ok(())
}

pub(super) fn has_static(source: &a::Program, result: usize) -> bool {
    let mut pending = vec![result];
    while let Some(id) = pending.pop() {
        let kind = &source.types[id].kind;
        if explicit_static(kind) {
            return true;
        }
        children(kind, &mut pending);
    }
    false
}

#[derive(Default)]
struct Environment {
    lifetimes: BTreeMap<String, String>,
    types: BTreeMap<String, (usize, usize)>,
}

type EnvironmentKey = (Vec<(String, String)>, Vec<(String, (usize, usize))>);

impl Environment {
    fn key(&self) -> EnvironmentKey {
        let mut lifetimes = Vec::new();
        for (name, value) in &self.lifetimes {
            lifetimes.push((name.clone(), value.clone()));
        }
        let mut types = Vec::new();
        for (name, value) in &self.types {
            types.push((name.clone(), *value));
        }
        (lifetimes, types)
    }
}

fn substitution(
    source: &a::Program,
    environments: &[Environment],
    mut ty: usize,
    mut environment: usize,
) -> (usize, usize) {
    while let a::Type::Path(path) = &source.types[ty].kind {
        if path.declaration.is_some() || !path.args.is_empty() {
            break;
        }
        if let Some((next, next_environment)) = environments[environment].types.get(&path.name) {
            ty = *next;
            environment = *next_environment;
        } else {
            break;
        }
    }
    (ty, environment)
}

/// Follow nominal declarations and aliases with explicit substituted environments.
/// Repeated states are shared, so recursive nominal data does not recurse on the
/// native stack. Generic expansion has already passed the finite-data analysis.
fn wholly_static(source: &a::Program, result: usize) -> bool {
    let mut environments = vec![Environment::default()];
    let mut interned: BTreeMap<EnvironmentKey, usize> = BTreeMap::new();
    interned.insert(Environment::default().key(), 0);
    let mut pending = vec![(result, 0)];
    let mut seen = BTreeSet::new();
    while let Some((id, environment)) = pending.pop() {
        let (id, environment) = substitution(source, &environments, id, environment);
        if !seen.insert((id, environment)) {
            continue;
        }
        match &source.types[id].kind {
            a::Type::Ref {
                lifetime, mutable, ..
            } => {
                let lifetime = environments[environment]
                    .lifetimes
                    .get(lifetime)
                    .unwrap_or(lifetime);
                if *mutable || lifetime != "'static" {
                    return false;
                }
            }
            a::Type::Tuple(fields) => {
                for field in fields {
                    pending.push((*field, environment));
                }
            }
            a::Type::Slice(inner) | a::Type::Array { inner, .. } => {
                pending.push((*inner, environment))
            }
            a::Type::Path(path) => {
                let Some(declaration) = path.declaration else {
                    if matches!(
                        path.name.as_str(),
                        "bool"
                            | "i8"
                            | "i16"
                            | "i32"
                            | "i64"
                            | "isize"
                            | "u8"
                            | "u16"
                            | "u32"
                            | "u64"
                            | "usize"
                            | "f64"
                            | "str"
                            | "World"
                            | "Stdout"
                            | "Stdin"
                            | "Stderr"
                            | "Directory"
                            | "File"
                            | "Vec"
                            | "Box"
                    ) {
                        for arg in &path.args {
                            pending.push((*arg, environment));
                        }
                        continue;
                    }
                    // An unconstrained parameter or associated type could carry
                    // dynamic reference leaves even beside a static reference.
                    return false;
                };
                let (parameters, fields): (&Vec<a::GenericParam>, Vec<usize>) =
                    match source.declarations[declaration].item {
                        a::Item::Struct(index) => {
                            let definition = &source.structs[index];
                            let mut fields = Vec::new();
                            for field in &definition.fields {
                                fields.push(field.ty);
                            }
                            (&definition.generics, fields)
                        }
                        a::Item::Enum(index) => {
                            let definition = &source.enums[index];
                            let mut fields = Vec::new();
                            for variant in &definition.variants {
                                for field in &variant.fields {
                                    fields.push(field.ty);
                                }
                            }
                            (&definition.generics, fields)
                        }
                        a::Item::Alias(index) => {
                            let definition = &source.aliases[index];
                            (&definition.generics, vec![definition.ty])
                        }
                        _ => return false,
                    };
                let mut bindings = Environment::default();
                let mut argument_index = 0;
                for parameter in parameters {
                    if parameter.lifetime {
                        let value = if argument_index < path.args.len() {
                            if let a::Type::Path(path) =
                                &source.types[path.args[argument_index]].kind
                            {
                                if path.name.starts_with('\'') {
                                    argument_index += 1;
                                    environments[environment]
                                        .lifetimes
                                        .get(&path.name)
                                        .unwrap_or(&path.name)
                                        .clone()
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        };
                        bindings.lifetimes.insert(parameter.name.clone(), value);
                    } else if argument_index < path.args.len() {
                        let argument = path.args[argument_index];
                        argument_index += 1;
                        bindings.types.insert(
                            parameter.name.clone(),
                            substitution(source, &environments, argument, environment),
                        );
                    }
                }
                let key = bindings.key();
                let known = interned.get(&key).copied();
                let nested = if let Some(id) = known {
                    id
                } else {
                    let id = environments.len();
                    environments.push(bindings);
                    interned.insert(key, id);
                    id
                };
                for field in fields {
                    pending.push((field, nested));
                }
            }
            a::Type::Never => {}
            a::Type::Infer | a::Type::Function { .. } => return false,
        }
    }
    true
}

pub(super) fn outputs(source: &a::Program) -> Result<()> {
    for function in &source.functions {
        if has_static(source, function.result) && !wholly_static(source, function.result) {
            return Err(err(
                source.types[function.result].span,
                "E_UNSUPPORTED",
                "mixed static and dynamic result constraints need reference-leaf contracts",
            ));
        }
    }
    for implementation in &source.impls {
        let Some(trait_type) = implementation.trait_ty else {
            continue;
        };
        let a::Type::Path(path) = &source.types[trait_type].kind else {
            continue;
        };
        let Some(declaration) = path.declaration else {
            continue;
        };
        let a::Item::Trait(index) = source.declarations[declaration].item else {
            continue;
        };
        for method in &implementation.methods {
            let actual = &source.functions[*method];
            let expected = source.traits[index]
                .methods
                .iter()
                .map(|index| &source.functions[*index])
                .find(|function| function.name == actual.name);
            if let Some(expected) = expected {
                if has_static(source, expected.result) && !has_static(source, actual.result) {
                    return Err(err(
                        actual.span,
                        "E_TRAIT",
                        "implementation method weakens the trait's static result contract",
                    ));
                }
            }
        }
    }
    Ok(())
}
