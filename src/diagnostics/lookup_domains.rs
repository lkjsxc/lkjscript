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
    let mut seen = BTreeSet::new();
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
        return matches!(
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
        )
        .then(|| path.name.clone());
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
        let failed: BTreeMap<_, _> = failed.iter().map(|(id, problem)| (*id, problem)).collect();
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
            if implementations[id] && !failed.contains_key(&id) {
                continue;
            }
            let reason = types
                .get(implementation.target)
                .cloned()
                .or_else(|| failed.get(&id).map(|problem| (*problem).clone()))
                .unwrap_or_else(|| {
                    blocked(implementation.span, "implementation target is unavailable")
                });
            for method in &implementation.methods {
                result
                    .methods
                    .entry(source.functions[*method].name.clone())
                    .or_default()
                    .push(MethodDomain {
                        head: head(source, types, implementation.target),
                        inherent: implementation.trait_ty.is_none(),
                        problem: reason.clone(),
                    });
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
                                result.methods.entry(name.clone()).or_default().push(entry);
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
                            result.methods.entry(name.clone()).or_default().push(entry);
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
    pub fn propagate_globs(mut self, resolver: &Resolver<'_>) -> Self {
        let mut edges: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (source, destination) in resolver.recovery_globs() {
            edges.entry(source).or_default().push(destination);
        }
        let mut pending = std::collections::VecDeque::new();
        let mut seen = BTreeSet::new();
        for (id, domain) in self.names.iter().enumerate() {
            pending.push_back((id, id));
            seen.insert((id, domain.owner));
        }
        while let Some((root, id)) = pending.pop_front() {
            let domain = self.names[id].clone();
            for destination in edges.get(&domain.owner).into_iter().flatten() {
                if seen.insert((root, *destination)) {
                    let mut imported = domain.clone();
                    imported.owner = *destination;
                    imported.glob = true;
                    let next = self.names.len();
                    self.names.push(imported);
                    pending.push_back((root, next));
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
        self.all_methods
            .iter()
            .chain(self.methods.get(name).into_iter().flatten())
            .find(|domain| {
                (!inherent_only || domain.inherent)
                    && (domain.head.is_none()
                        || receiver.is_none()
                        || domain.head.as_deref() == receiver)
            })
            .map(|domain| &domain.problem)
    }
    pub fn name_problem(
        &self,
        names: &Resolver<'_>,
        scope: a::ModuleId,
        name: &str,
        namespace: Namespace,
        span: a::Span,
    ) -> Option<&a::Diagnostic> {
        self.names
            .iter()
            .find(|domain| {
                (if namespace == Namespace::Type {
                    domain.types
                } else {
                    domain.values
                }) && names.recovery_name_visible(
                    (scope, name, namespace, span),
                    domain.owner,
                    domain.name.as_deref(),
                    domain.glob,
                )
            })
            .map(|domain| &domain.problem)
    }
}
