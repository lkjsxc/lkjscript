//! Infer declaration parameters from structural types in lexical environments.
//! Alias parameters get their own environment; transparent expansion never
//! binds an unrelated same-spelled parameter in the caller.
use super::{Builder, Compiler, Result, a, err, k};
use std::collections::BTreeMap;

enum Work {
    Match {
        pattern: a::TypeId,
        actual: usize,
        environment: usize,
    },
    AliasArguments {
        arguments: Vec<a::TypeId>,
        parameters: Vec<String>,
        alias_environment: usize,
        caller_environment: usize,
    },
}

impl Compiler<'_> {
    pub(super) fn resolve_if_known(
        &mut self,
        id: a::TypeId,
        substitutions: &BTreeMap<String, usize>,
    ) -> Result<Option<usize>> {
        let mut pending = vec![id];
        while let Some(id) = pending.pop() {
            let node = &self.source.types[id];
            match &node.kind {
                a::Type::Infer => return Ok(None),
                a::Type::Path(path) => {
                    if self.names.generic_path(path, node.span)
                        && !substitutions.contains_key(&path.name)
                        && !substitutions.contains_key(path.name.split("::").next().unwrap())
                    {
                        return Ok(None);
                    }
                    pending.extend(path.args.iter().copied());
                }
                a::Type::Ref { inner, .. }
                | a::Type::Slice(inner)
                | a::Type::Array { inner, .. } => pending.push(*inner),
                a::Type::Tuple(elements) => pending.extend(elements.iter().copied()),
                a::Type::Function { params, result } => {
                    pending.push(*result);
                    pending.extend(params.iter().copied());
                }
                _ => {}
            }
        }
        self.resolve(id, substitutions).map(Some)
    }
}

impl Builder<'_, '_> {
    pub(super) fn infer_type(
        &mut self,
        pattern: a::TypeId,
        actual: usize,
        substitutions: &mut BTreeMap<String, usize>,
    ) -> Result<()> {
        if self.infer_type_matches(pattern, actual, substitutions)? {
            Ok(())
        } else {
            Err(err(
                self.compiler.source.types[pattern].span,
                "E_TYPE",
                "generic arguments disagree",
            ))
        }
    }

    /// A candidate trial can fail to match without making the source invalid.
    /// Unification writes its inferred bindings only when the whole trial succeeds.
    pub(super) fn infer_type_matches(
        &mut self,
        pattern: a::TypeId,
        actual: usize,
        substitutions: &mut BTreeMap<String, usize>,
    ) -> Result<bool> {
        let mut environments = vec![substitutions.clone()];
        let mut pending = vec![Work::Match {
            pattern,
            actual,
            environment: 0,
        }];
        while let Some(work) = pending.pop() {
            match work {
                Work::AliasArguments {
                    arguments,
                    parameters,
                    alias_environment,
                    caller_environment,
                } => {
                    for (argument, parameter) in arguments.into_iter().zip(parameters).rev() {
                        if let Some(actual) =
                            environments[alias_environment].get(&parameter).copied()
                        {
                            pending.push(Work::Match {
                                pattern: argument,
                                actual,
                                environment: caller_environment,
                            });
                        }
                    }
                }
                Work::Match {
                    pattern,
                    actual,
                    environment,
                } => {
                    let node = self.compiler.source.types[pattern].clone();
                    match node.kind {
                        a::Type::Path(path) => {
                            if path.args.is_empty()
                                && self.compiler.names.is_type_parameter(&path, node.span)
                            {
                                if let Some(existing) = environments[environment].get(&path.name) {
                                    if *existing != actual {
                                        return Ok(false);
                                    }
                                } else {
                                    environments[environment].insert(path.name, actual);
                                }
                                continue;
                            }
                            let arguments: Vec<_> = path
                                .args
                                .iter()
                                .copied()
                                .filter(|id| {
                                    !matches!(&self.compiler.source.types[*id].kind,
                                    a::Type::Path(path) if path.name.starts_with('\''))
                                })
                                .collect();
                            let declaration = path
                                .declaration
                                .map(|id| self.compiler.source.declarations[id].item);
                            if let Some(a::Item::Alias(index)) = declaration {
                                let alias = self.compiler.source.aliases[index].clone();
                                let parameters: Vec<_> = alias
                                    .generics
                                    .iter()
                                    .filter(|parameter| !parameter.lifetime)
                                    .map(|parameter| parameter.name.clone())
                                    .collect();
                                if arguments.len() != parameters.len() {
                                    return Err(err(
                                        node.span,
                                        "E_TYPE",
                                        "wrong number of alias type arguments",
                                    ));
                                }
                                let caller = environments[environment].clone();
                                let mut bindings = BTreeMap::new();
                                for (parameter, argument) in parameters.iter().zip(&arguments) {
                                    if let Some(ty) =
                                        self.compiler.resolve_if_known(*argument, &caller)?
                                    {
                                        bindings.insert(parameter.clone(), ty);
                                    }
                                }
                                let alias_environment = environments.len();
                                environments.push(bindings);
                                pending.push(Work::AliasArguments {
                                    arguments,
                                    parameters,
                                    alias_environment,
                                    caller_environment: environment,
                                });
                                pending.push(Work::Match {
                                    pattern: alias.ty,
                                    actual,
                                    environment: alias_environment,
                                });
                                continue;
                            }
                            if let Some((name, actual_arguments)) =
                                self.compiler.applications.get(&actual)
                            {
                                let actual_declaration =
                                    self.compiler.declaration_names.get(name).copied();
                                if path.declaration.is_some()
                                    && path.declaration == actual_declaration
                                    && arguments.len() == actual_arguments.len()
                                {
                                    for (pattern, actual) in arguments
                                        .into_iter()
                                        .zip(actual_arguments.iter().copied())
                                        .rev()
                                    {
                                        pending.push(Work::Match {
                                            pattern,
                                            actual,
                                            environment,
                                        });
                                    }
                                }
                            } else if path.declaration.is_none() && arguments.len() == 1 {
                                let inner =
                                    match (&self.compiler.types[actual].kind, path.name.as_str()) {
                                        (k::TypeKind::Vec { inner }, "Vec")
                                        | (k::TypeKind::Box { inner }, "Box") => Some(*inner),
                                        _ => None,
                                    };
                                if let Some(actual) = inner {
                                    pending.push(Work::Match {
                                        pattern: arguments[0],
                                        actual,
                                        environment,
                                    });
                                }
                            }
                        }
                        a::Type::Ref { inner, .. } => {
                            if let k::TypeKind::Ref { inner: actual, .. } =
                                self.compiler.types[actual].kind
                            {
                                pending.push(Work::Match {
                                    pattern: inner,
                                    actual,
                                    environment,
                                });
                            }
                        }
                        a::Type::Slice(inner) => {
                            if let k::TypeKind::Slice { inner: actual, .. } =
                                self.compiler.types[actual].kind
                            {
                                pending.push(Work::Match {
                                    pattern: inner,
                                    actual,
                                    environment,
                                });
                            }
                        }
                        a::Type::Array { inner, .. } => {
                            if let k::TypeKind::Array { inner: actual, .. } =
                                self.compiler.types[actual].kind
                            {
                                pending.push(Work::Match {
                                    pattern: inner,
                                    actual,
                                    environment,
                                });
                            }
                        }
                        a::Type::Tuple(elements) => {
                            if !self.compiler.applications.contains_key(&actual) {
                                if let k::TypeKind::Struct { fields, .. } =
                                    &self.compiler.types[actual].kind
                                {
                                    if elements.len() == fields.len() {
                                        for (pattern, field) in
                                            elements.into_iter().zip(fields).rev()
                                        {
                                            pending.push(Work::Match {
                                                pattern,
                                                actual: field.ty,
                                                environment,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        *substitutions = environments.remove(0);
        Ok(true)
    }
}
