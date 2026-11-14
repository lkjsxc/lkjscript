//! Infer declaration parameters from structural types in lexical environments.
//! Alias parameters get their own environment; transparent expansion never
//! binds an unrelated same-spelled parameter in the caller.
use super::{Builder, Compiler, Result, a, err, k};
use std::collections::BTreeMap;

#[derive(Clone, Copy, PartialEq, Eq)]
enum MatchMode {
    Exact,
    Argument,
    Referent,
}

enum Work {
    Match {
        pattern: a::TypeId,
        actual: usize,
        environment: usize,
        mode: MatchMode,
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
                    let first_end = path.name.find("::").unwrap_or(path.name.len());
                    let first = path.name[..first_end].to_string();
                    if self.names.generic_path(path, node.span)
                        && !substitutions.contains_key(&path.name)
                        && !substitutions.contains_key(&first)
                    {
                        return Ok(None);
                    }
                    pending.extend_from_slice(&path.args);
                }
                a::Type::Ref { inner, .. }
                | a::Type::Slice(inner)
                | a::Type::Array { inner, .. } => pending.push(*inner),
                a::Type::Tuple(elements) => pending.extend_from_slice(elements),
                a::Type::Function { params, result } => {
                    pending.push(*result);
                    pending.extend_from_slice(params);
                }
                _ => {}
            }
        }
        let resolved = self.resolve(id, substitutions)?;
        Ok(Some(resolved))
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

    pub(super) fn infer_type_matches(
        &mut self,
        pattern: a::TypeId,
        actual: usize,
        substitutions: &mut BTreeMap<String, usize>,
    ) -> Result<bool> {
        self.compiler
            .infer_type_bindings(pattern, actual, substitutions)
    }
}

impl Compiler<'_> {
    /// A candidate trial can fail to match without making the source invalid.
    /// Unification writes its inferred bindings only when the whole trial succeeds.
    pub(super) fn infer_type_bindings(
        &mut self,
        pattern: a::TypeId,
        actual: usize,
        substitutions: &mut BTreeMap<String, usize>,
    ) -> Result<bool> {
        self.infer_bindings(pattern, actual, substitutions, MatchMode::Exact)
    }

    /// Infer through the reference coercions admitted at an argument site.
    /// Implementation/callable matching keeps exact structural equations.
    pub(super) fn infer_argument_bindings(
        &mut self,
        pattern: a::TypeId,
        actual: usize,
        substitutions: &mut BTreeMap<String, usize>,
    ) -> Result<bool> {
        self.infer_bindings(pattern, actual, substitutions, MatchMode::Argument)
    }

    fn infer_bindings(
        &mut self,
        pattern: a::TypeId,
        actual: usize,
        substitutions: &mut BTreeMap<String, usize>,
        mode: MatchMode,
    ) -> Result<bool> {
        let mut environments = vec![substitutions.clone()];
        let mut pending = vec![Work::Match {
            pattern,
            actual,
            environment: 0,
            mode,
        }];
        while let Some(work) = pending.pop() {
            match work {
                Work::AliasArguments {
                    arguments,
                    parameters,
                    alias_environment,
                    caller_environment,
                } => {
                    let mut index = if arguments.len() < parameters.len() {
                        arguments.len()
                    } else {
                        parameters.len()
                    };
                    while index != 0 {
                        index -= 1;
                        let argument = arguments[index];
                        if let Some(actual) = environments[alias_environment]
                            .get(&parameters[index])
                            .copied()
                        {
                            pending.push(Work::Match {
                                pattern: argument,
                                actual,
                                environment: caller_environment,
                                mode: MatchMode::Exact,
                            });
                        }
                    }
                }
                Work::Match {
                    pattern,
                    actual,
                    environment,
                    mode,
                } => {
                    let node = self.source.types[pattern].clone();
                    match node.kind {
                        a::Type::Path(path) => {
                            if path.args.is_empty()
                                && self.names.is_type_parameter(&path, node.span)
                            {
                                let known = environments[environment].get(&path.name).copied();
                                if let Some(existing) = known {
                                    if existing != actual {
                                        return Ok(false);
                                    }
                                } else {
                                    environments[environment].insert(path.name, actual);
                                }
                                continue;
                            }
                            let mut arguments: Vec<usize> = Vec::new();
                            for id in &path.args {
                                if !matches!(&self.source.types[*id].kind,
                                    a::Type::Path(path) if path.name.starts_with('\''))
                                {
                                    arguments.push(*id);
                                }
                            }
                            let declaration =
                                path.declaration.map(|id| self.source.declarations[id].item);
                            if let Some(a::Item::Alias(index)) = declaration {
                                let alias = self.source.aliases[index].clone();
                                let mut parameters: Vec<String> = Vec::new();
                                for parameter in &alias.generics {
                                    if !parameter.lifetime {
                                        parameters.push(parameter.name.clone());
                                    }
                                }
                                if arguments.len() != parameters.len() {
                                    return Err(err(
                                        node.span,
                                        "E_TYPE",
                                        "wrong number of alias type arguments",
                                    ));
                                }
                                let caller = environments[environment].clone();
                                let mut bindings: BTreeMap<String, usize> = BTreeMap::new();
                                for (parameter, argument) in parameters.iter().zip(&arguments) {
                                    if let Some(ty) = self.resolve_if_known(*argument, &caller)? {
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
                                    mode,
                                });
                                continue;
                            }
                            if let Some((name, actual_arguments)) = self.applications.get(&actual) {
                                let actual_declaration = self.declaration_names.get(name).copied();
                                if path.declaration.is_some()
                                    && path.declaration == actual_declaration
                                    && arguments.len() == actual_arguments.len()
                                {
                                    let mut index = arguments.len();
                                    while index != 0 {
                                        index -= 1;
                                        pending.push(Work::Match {
                                            pattern: arguments[index],
                                            actual: actual_arguments[index],
                                            environment,
                                            mode: MatchMode::Exact,
                                        });
                                    }
                                }
                            } else if path.declaration.is_none() && arguments.len() == 1 {
                                let inner = match (&self.types[actual].kind, path.name.as_str()) {
                                    (k::TypeKind::Vec { inner }, "Vec")
                                    | (k::TypeKind::Box { inner }, "Box") => Some(*inner),
                                    _ => None,
                                };
                                if let Some(actual) = inner {
                                    pending.push(Work::Match {
                                        pattern: arguments[0],
                                        actual,
                                        environment,
                                        mode: MatchMode::Exact,
                                    });
                                }
                            }
                        }
                        a::Type::Ref { inner, .. } => {
                            if let k::TypeKind::Ref { inner: actual, .. } = self.types[actual].kind
                            {
                                pending.push(Work::Match {
                                    pattern: inner,
                                    actual,
                                    environment,
                                    mode: if mode == MatchMode::Argument {
                                        MatchMode::Referent
                                    } else {
                                        MatchMode::Exact
                                    },
                                });
                            }
                        }
                        a::Type::Slice(inner) => {
                            let element = match (&self.types[actual].kind, mode) {
                                (k::TypeKind::Slice { inner, .. }, _) => Some(*inner),
                                (
                                    k::TypeKind::Vec { inner } | k::TypeKind::Array { inner, .. },
                                    MatchMode::Referent,
                                ) => Some(*inner),
                                _ => None,
                            };
                            if let Some(actual) = element {
                                pending.push(Work::Match {
                                    pattern: inner,
                                    actual,
                                    environment,
                                    mode: MatchMode::Exact,
                                });
                            }
                        }
                        a::Type::Array { inner, .. } => {
                            if let k::TypeKind::Array { inner: actual, .. } =
                                self.types[actual].kind
                            {
                                pending.push(Work::Match {
                                    pattern: inner,
                                    actual,
                                    environment,
                                    mode: MatchMode::Exact,
                                });
                            }
                        }
                        a::Type::Tuple(elements) => {
                            if !self.applications.contains_key(&actual) {
                                if let k::TypeKind::Struct { fields, .. } = &self.types[actual].kind
                                {
                                    if elements.len() == fields.len() {
                                        let mut index = elements.len();
                                        while index != 0 {
                                            index -= 1;
                                            pending.push(Work::Match {
                                                pattern: elements[index],
                                                actual: fields[index].ty,
                                                environment,
                                                mode: MatchMode::Exact,
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
        *substitutions = std::mem::replace(&mut environments[0], BTreeMap::new());
        Ok(true)
    }
}
