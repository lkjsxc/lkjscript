//! Missing declarations restrict diagnostic conclusions; they never contribute
//! candidates, types, signatures, or trait evidence.
use super::{BlockedRegion, RecoveryOwner};
use crate::ast as a;
use crate::diagnostics::readiness::UnavailableTypes;
use crate::resolve::{Namespace, Resolver};
use std::collections::{BTreeMap, BTreeSet};

struct MethodDomain {
    head: Option<String>,
    inherent: bool,
    problem: a::Diagnostic,
}
#[derive(Clone)]
struct NameDomain {
    glob: bool,
    owner: a::ModuleId,
    name: Option<String>,
    types: bool,
    values: bool,
    problem: a::Diagnostic,
}
pub(crate) struct LookupDomains {
    methods: BTreeMap<String, Vec<MethodDomain>>,
    all_methods: Vec<MethodDomain>,
    names: Vec<NameDomain>,
}
fn blocked(span: a::Span, reason: &str) -> a::Diagnostic {
    a::Diagnostic::new(span, "E_BLOCKED", reason)
}
fn head(source: &a::Program, types: &UnavailableTypes, mut ty: a::TypeId) -> Option<String> {
    let mut seen: BTreeSet<usize> = BTreeSet::new();
    while seen.insert(ty) && types.get(ty).is_none() {
        let a::Type::Path(path) = &source.types[ty].kind else {
            return None;
        };
        if let Some(declaration) = path.declaration {
            return match source.declarations[declaration].item {
                a::Item::Struct(_) | a::Item::Enum(_) => {
                    Some(source.declarations[declaration].name.clone())
                }
                a::Item::Alias(alias) => {
                    ty = source.aliases[alias].ty;
                    continue;
                }
                _ => None,
            };
        }
        let builtin = matches!(
            path.name.as_str(),
            "bool"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "usize"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "isize"
                | "f64"
                | "char"
                | "str"
                | "Vec"
                | "Box"
                | "World"
                | "Stdout"
        );
        return if builtin {
            Some(path.name.clone())
        } else {
            None
        };
    }
    None
}
impl LookupDomains {
    pub fn new(
        source: &a::Program,
        types: &UnavailableTypes,
        implementations: &[bool],
        failed: &[(usize, a::Diagnostic)],
        blocked_regions: &[BlockedRegion],
    ) -> Self {
        let mut result = Self {
            methods: BTreeMap::new(),
            all_methods: Vec::new(),
            names: Vec::new(),
        };
        let mut failed_indices: BTreeMap<usize, usize> = BTreeMap::new();
        for (index, (id, _)) in failed.iter().enumerate() {
            failed_indices.insert(*id, index);
        }
        for module in &source.modules {
            if module.external {
                result.names.push(NameDomain {
                    glob: false,
                    owner: module.scope,
                    name: None,
                    types: true,
                    values: true,
                    problem: blocked(module.span, "module contents were not loaded"),
                });
                result.all_methods.push(MethodDomain {
                    head: None,
                    inherent: true,
                    problem: blocked(
                        module.span,
                        "implementation inventory includes an unloaded module",
                    ),
                });
            }
        }
        for (id, implementation) in source.impls.iter().enumerate() {
            if implementations[id] && !failed_indices.contains_key(&id) {
                continue;
            }
            let reason = if let Some(problem) = types.get(implementation.target) {
                problem.clone()
            } else if let Some(index) = failed_indices.get(&id) {
                failed[*index].1.clone()
            } else {
                blocked(implementation.span, "implementation target is unavailable")
            };
            for method in &implementation.methods {
                result.add_method(
                    source.functions[*method].name.clone(),
                    MethodDomain {
                        head: head(source, types, implementation.target),
                        inherent: implementation.trait_ty.is_none(),
                        problem: reason.clone(),
                    },
                );
            }
        }
        for region in blocked_regions {
            let problem = blocked(
                region.span,
                &format!("unavailable declaration: {}", region.reason),
            );
            if let Some(domain) = &region.domain {
                match &domain.owner {
                    Some(RecoveryOwner::Implementation { target, trait_ty }) => {
                        let entry = MethodDomain {
                            head: head(source, types, *target),
                            inherent: trait_ty.is_none(),
                            problem,
                        };
                        if domain.kind == "fn" {
                            if let Some(name) = &domain.name {
                                result.add_method(name.clone(), entry);
                            } else {
                                result.all_methods.push(entry);
                            }
                        } else {
                            result.all_methods.push(entry);
                        }
                    }
                    Some(RecoveryOwner::Trait { .. }) => {
                        let entry = MethodDomain {
                            head: None,
                            inherent: false,
                            problem,
                        };
                        if let Some(name) = &domain.name {
                            result.add_method(name.clone(), entry);
                        } else {
                            result.all_methods.push(entry);
                        }
                    }
                    None if matches!(domain.kind.as_str(), "impl" | "mod" | "use") => {
                        result.all_methods.push(MethodDomain {
                            head: None,
                            inherent: true,
                            problem: problem.clone(),
                        });
                        if domain.kind == "mod" {
                            result.names.push(NameDomain {
                                glob: false,
                                owner: domain.scope,
                                name: domain.name.clone(),
                                types: true,
                                values: false,
                                problem,
                            });
                        }
                    }
                    None => {
                        result.names.push(NameDomain {
                            glob: false,
                            owner: domain.scope,
                            name: domain.name.clone(),
                            types: matches!(
                                domain.kind.as_str(),
                                "struct" | "enum" | "trait" | "type"
                            ),
                            values: matches!(
                                domain.kind.as_str(),
                                "fn" | "const" | "struct" | "enum"
                            ),
                            problem,
                        });
                    }
                }
            } else if matches!(region.kind.as_str(), "file" | "module") {
                result.all_methods.push(MethodDomain {
                    head: None,
                    inherent: true,
                    problem,
                });
            }
        }
        result
    }
    fn add_method(&mut self, name: String, domain: MethodDomain) {
        if self.methods.contains_key(&name) {
            self.methods.get_mut(&name).unwrap().push(domain);
        } else {
            self.methods.insert(name, vec![domain]);
        }
    }
    pub fn propagate_globs(mut self, resolver: &Resolver<'_>) -> Self {
        let mut edges: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (source, destination) in resolver.recovery_globs() {
            if edges.contains_key(&source) {
                edges.get_mut(&source).unwrap().push(destination);
            } else {
                edges.insert(source, vec![destination]);
            }
        }
        let mut pending = Vec::new();
        let mut seen: BTreeSet<(usize, usize)> = BTreeSet::new();
        for (id, domain) in self.names.iter().enumerate() {
            pending.push((id, id));
            seen.insert((id, domain.owner));
        }
        let mut head = 0;
        while head < pending.len() {
            let (root, id) = pending[head];
            head += 1;
            let domain = self.names[id].clone();
            if let Some(destinations) = edges.get(&domain.owner) {
                for destination in destinations {
                    if seen.insert((root, *destination)) {
                        let mut imported = domain.clone();
                        imported.owner = *destination;
                        imported.glob = true;
                        let next = self.names.len();
                        self.names.push(imported);
                        pending.push((root, next));
                    }
                }
            }
        }
        self
    }
    pub fn method_problem(
        &self,
        receiver: Option<&str>,
        name: &str,
        inherent_only: bool,
    ) -> Option<&a::Diagnostic> {
        let mut index = 0;
        while index < self.all_methods.len() {
            if method_matches(&self.all_methods[index], receiver, inherent_only) {
                return Some(&self.all_methods[index].problem);
            }
            index += 1;
        }
        if let Some(domains) = self.methods.get(&name.to_string()) {
            let mut index = 0;
            while index < domains.len() {
                if method_matches(&domains[index], receiver, inherent_only) {
                    return Some(&domains[index].problem);
                }
                index += 1;
            }
        }
        None
    }
    pub fn name_problem(
        &self,
        names: &Resolver<'_>,
        scope: a::ModuleId,
        name: &str,
        namespace: Namespace,
        span: a::Span,
    ) -> Option<&a::Diagnostic> {
        let mut index = 0;
        while index < self.names.len() {
            let domain = &self.names[index];
            let namespace_matches = if namespace == Namespace::Type {
                domain.types
            } else {
                domain.values
            };
            if namespace_matches
                && names.recovery_name_visible(
                    (scope, name, namespace, span),
                    domain.owner,
                    domain.name.as_ref().map(|name| name.as_str()),
                    domain.glob,
                )
            {
                return Some(&self.names[index].problem);
            }
            index += 1;
        }
        None
    }
}

fn method_matches(domain: &MethodDomain, receiver: Option<&str>, inherent_only: bool) -> bool {
    (!inherent_only || domain.inherent)
        && (domain.head.is_none()
            || receiver.is_none()
            || domain.head.as_ref().map(|name| name.as_str()) == receiver)
}
