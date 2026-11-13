//! Symbolic origin variables in signatures. A type parameter's variable is a
//! map of reference-leaf paths, not one concrete allocation root.
use super::{Compiler, Result, a, k};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Label {
    Lifetime(String),
    Parameter(String, Vec<k::Projection>),
    Associated(String, Vec<k::Projection>),
    Static,
    Elided,
    Unknown,
}
#[derive(Clone, Default)]
struct Environment {
    types: BTreeMap<String, (usize, usize)>,
    lifetimes: BTreeMap<String, String>,
}
type SignatureChild = (usize, usize, Vec<k::Projection>);
#[derive(Clone, Copy, PartialEq, Eq)]
enum LeafPurpose {
    Outputs,
    Inputs,
    Regions,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum BorrowAccess {
    Owned,
    Shared,
    Mutable,
}
pub(super) struct RegionLeaf {
    pub(super) path: Vec<k::Projection>,
    pub(super) label: Label,
    pub(super) preserve: bool,
    pub(super) mutable_storage: bool,
}
struct Leaf {
    borrowed: bool,
    mutable_storage: bool,
    path: Vec<k::Projection>,
    label: Label,
    transport: bool,
    mutable: bool,
}
struct Candidate {
    input: k::InputLeaf,
    transport: bool,
    mutable: bool,
}
fn substituted(
    source: &a::Program,
    environments: &[Environment],
    mut syntax: usize,
    mut environment: usize,
) -> (usize, usize) {
    while let a::Type::Path(path) = &source.types[syntax].kind {
        if path.declaration.is_some() || !path.args.is_empty() {
            break;
        }
        if let Some((next, parent)) = environments[environment].types.get(&path.name) {
            syntax = *next;
            environment = *parent;
        } else {
            break;
        }
    }
    (syntax, environment)
}
fn lifetime(environments: &[Environment], environment: usize, name: &str) -> String {
    environments[environment]
        .lifetimes
        .get(name)
        .cloned()
        .unwrap_or_else(|| name.to_string())
}
impl Compiler<'_> {
    fn signature_leaves(
        &self,
        source: &a::Function,
        source_index: usize,
        syntax: usize,
        concrete: usize,
        input: bool,
    ) -> Vec<Leaf> {
        self.signature_leaf_inventory(
            source,
            source_index,
            syntax,
            concrete,
            if input {
                LeafPurpose::Inputs
            } else {
                LeafPurpose::Outputs
            },
        )
    }

    pub(super) fn region_leaves(
        &self,
        source_index: usize,
        syntax: usize,
        concrete: usize,
    ) -> Vec<RegionLeaf> {
        let source = &self.source.functions[source_index];
        self.signature_leaf_inventory(source, source_index, syntax, concrete, LeafPurpose::Regions)
            .into_iter()
            .map(|leaf| RegionLeaf {
                path: leaf.path,
                label: leaf.label,
                preserve: leaf.borrowed,
                mutable_storage: leaf.mutable_storage,
            })
            .collect()
    }

    fn signature_leaf_inventory(
        &self,
        source: &a::Function,
        source_index: usize,
        syntax: usize,
        concrete: usize,
        purpose: LeafPurpose,
    ) -> Vec<Leaf> {
        let mut parameters: BTreeSet<String> = source
            .generics
            .iter()
            .filter(|p| !p.lifetime)
            .map(|p| p.name.clone())
            .collect();
        let mut root = Environment::default();
        if let a::FunctionOwner::Implementation(owner) = self.source.function_owner(a::FunctionId {
            index: source_index,
        }) {
            let implementation = &self.source.impls[owner.index];
            for parameter in &implementation.generics {
                if !parameter.lifetime {
                    parameters.insert(parameter.name.clone());
                }
            }
            root.types
                .insert("Self".to_string(), (implementation.target, 0));
        }
        let mut environments = vec![root];
        let mut pending = vec![(syntax, concrete, 0usize, Vec::new(), BorrowAccess::Owned)];
        let mut result = Vec::new();
        while let Some((syntax, concrete, environment, path, access)) = pending.pop() {
            let borrowed = access != BorrowAccess::Owned;
            let (syntax, environment) =
                substituted(self.source, &environments, syntax, environment);
            if !self.has_view(concrete) {
                continue;
            }
            match (&self.source.types[syntax].kind, &self.types[concrete].kind) {
                (
                    a::Type::Ref {
                        lifetime: name,
                        inner: syntax_inner,
                        ..
                    },
                    k::TypeKind::Ref { inner, mutable },
                ) => {
                    let name = lifetime(&environments, environment, name);
                    let label = if name == "'static" {
                        Label::Static
                    } else if name.is_empty() || name == "'_" {
                        Label::Elided
                    } else {
                        Label::Lifetime(name)
                    };
                    if !borrowed || !mutable || purpose == LeafPurpose::Regions {
                        result.push(Leaf {
                            borrowed,
                            mutable_storage: access == BorrowAccess::Mutable,
                            path: path.clone(),
                            label,
                            transport: false,
                            mutable: *mutable,
                        });
                    }
                    if purpose != LeafPurpose::Outputs && self.has_view(*inner) {
                        let mut nested = path;
                        nested.push(k::Projection::Deref);
                        let access = if access == BorrowAccess::Shared || !mutable {
                            BorrowAccess::Shared
                        } else {
                            BorrowAccess::Mutable
                        };
                        pending.push((*syntax_inner, *inner, environment, nested, access));
                    }
                }
                (a::Type::Tuple(syntax_fields), k::TypeKind::Struct { fields, .. })
                    if syntax_fields.len() == fields.len() =>
                {
                    for (index, (syntax, field)) in syntax_fields.iter().zip(fields).enumerate() {
                        let mut nested = path.clone();
                        nested.push(k::Projection::Field(index));
                        pending.push((*syntax, field.ty, environment, nested, access));
                    }
                }
                (a::Type::Path(named), _)
                    if named.declaration.is_none() && parameters.contains(&named.name) =>
                {
                    for (relative, mutable) in crate::check::reference_paths(&self.types, concrete)
                    {
                        if borrowed && mutable && purpose != LeafPurpose::Regions {
                            continue;
                        }
                        let mut leaf_path = path.clone();
                        leaf_path.extend_from_slice(&relative);
                        result.push(Leaf {
                            borrowed,
                            mutable_storage: access == BorrowAccess::Mutable,
                            path: leaf_path,
                            label: Label::Parameter(named.name.clone(), relative),
                            transport: !borrowed,
                            mutable,
                        });
                    }
                }
                (a::Type::Path(named), _)
                    if named.declaration.is_none()
                        && named.name.split_once("::").is_some_and(|(owner, _)| {
                            owner == "Self" || parameters.contains(owner)
                        }) =>
                {
                    for (relative, mutable) in crate::check::reference_paths(&self.types, concrete)
                    {
                        if borrowed && mutable && purpose != LeafPurpose::Regions {
                            continue;
                        }
                        let mut leaf_path = path.clone();
                        leaf_path.extend_from_slice(&relative);
                        result.push(Leaf {
                            borrowed,
                            mutable_storage: access == BorrowAccess::Mutable,
                            path: leaf_path,
                            label: Label::Associated(named.name.clone(), relative),
                            transport: !borrowed,
                            mutable,
                        });
                    }
                }
                (a::Type::Path(named), _) if named.declaration.is_some() => {
                    let declaration = named.declaration.unwrap();
                    let (parameters, children): (&[a::GenericParam], Vec<SignatureChild>) =
                        match self.source.declarations[declaration].item {
                            a::Item::Struct(index) => {
                                let definition = &self.source.structs[index];
                                let k::TypeKind::Struct { fields, .. } = &self.types[concrete].kind
                                else {
                                    continue;
                                };
                                (
                                    &definition.generics,
                                    definition
                                        .fields
                                        .iter()
                                        .zip(fields)
                                        .enumerate()
                                        .map(|(index, (syntax, field))| {
                                            (syntax.ty, field.ty, vec![k::Projection::Field(index)])
                                        })
                                        .collect(),
                                )
                            }
                            a::Item::Enum(index) => {
                                let definition = &self.source.enums[index];
                                let k::TypeKind::Enum { variants, .. } = &self.types[concrete].kind
                                else {
                                    continue;
                                };
                                let mut children = Vec::new();
                                for (tag, (syntax, variant)) in
                                    definition.variants.iter().zip(variants).enumerate()
                                {
                                    for (index, (syntax, field)) in
                                        syntax.fields.iter().zip(&variant.fields).enumerate()
                                    {
                                        children.push((
                                            syntax.ty,
                                            field.ty,
                                            vec![k::Projection::VariantField(tag, index)],
                                        ));
                                    }
                                }
                                (&definition.generics, children)
                            }
                            a::Item::Alias(index) => {
                                let definition = &self.source.aliases[index];
                                (
                                    &definition.generics,
                                    vec![(definition.ty, concrete, Vec::new())],
                                )
                            }
                            _ => continue,
                        };
                    let mut nested = Environment::default();
                    let mut arguments = named.args.iter().peekable();
                    for parameter in parameters {
                        if parameter.lifetime {
                            let name = if let Some(argument) = arguments.peek() {
                                if let a::Type::Path(path) = &self.source.types[**argument].kind {
                                    if path.name.starts_with('\'') {
                                        arguments.next();
                                        lifetime(&environments, environment, &path.name)
                                    } else {
                                        String::new()
                                    }
                                } else {
                                    String::new()
                                }
                            } else {
                                String::new()
                            };
                            nested.lifetimes.insert(parameter.name.clone(), name);
                        } else if let Some(argument) = arguments.next() {
                            nested
                                .types
                                .insert(parameter.name.clone(), (*argument, environment));
                        }
                    }
                    let nested_id = environments.len();
                    environments.push(nested);
                    for (syntax, concrete, suffix) in children {
                        let mut nested_path = path.clone();
                        nested_path.extend(suffix);
                        pending.push((syntax, concrete, nested_id, nested_path, access));
                    }
                }
                _ => {
                    for (relative, mutable) in crate::check::reference_paths(&self.types, concrete)
                    {
                        let mut leaf_path = path.clone();
                        leaf_path.extend(relative);
                        result.push(Leaf {
                            borrowed,
                            mutable_storage: access == BorrowAccess::Mutable,
                            path: leaf_path,
                            label: Label::Unknown,
                            transport: false,
                            mutable,
                        });
                    }
                }
            }
        }
        result
    }

    pub(super) fn return_leaf_contract(
        &self,
        source_index: usize,
        source: &a::Function,
        locals: &[k::Local],
        result: usize,
    ) -> Result<Option<Vec<k::ReturnLeaf>>> {
        let outputs = self.signature_leaves(source, source_index, source.result, result, false);
        // Elided lifetimes retain the established elision rule. A rigid type
        // variable transports its relative leaf paths even at equal concrete
        // instantiations of otherwise unrelated type parameters.
        if outputs.is_empty()
            || outputs
                .iter()
                .all(|leaf| matches!(leaf.label, Label::Static))
            || outputs
                .iter()
                .any(|leaf| matches!(leaf.label, Label::Unknown | Label::Elided))
        {
            return Ok(None);
        }
        let mut inputs: BTreeMap<Label, Vec<Candidate>> = BTreeMap::new();
        for (index, parameter) in source.params.iter().enumerate() {
            for leaf in
                self.signature_leaves(source, source_index, parameter.ty, locals[index].ty, true)
            {
                inputs.entry(leaf.label).or_default().push(Candidate {
                    input: k::InputLeaf {
                        index,
                        projections: leaf.path,
                    },
                    transport: leaf.transport,
                    mutable: leaf.mutable,
                });
            }
        }
        let mut mappings = Vec::new();
        for leaf in outputs {
            let origin = match &leaf.label {
                Label::Static => k::LeafOrigin::Static,
                Label::Unknown | Label::Elided => return Ok(None),
                label => {
                    let Some(candidates) = inputs.get(label) else {
                        return Ok(None);
                    };
                    let exact = matches!(label, Label::Parameter(_, _) | Label::Associated(_, _));
                    let candidates: Vec<_> = candidates
                        .iter()
                        .filter(|candidate| {
                            (!exact || candidate.transport) && (!leaf.mutable || candidate.mutable)
                        })
                        .map(|candidate| candidate.input.clone())
                        .collect();
                    if candidates.is_empty() {
                        return Ok(None);
                    }
                    k::LeafOrigin::Inputs {
                        inputs: candidates,
                        exact,
                    }
                }
            };
            mappings.push(k::ReturnLeaf {
                projections: leaf.path,
                origin,
            });
        }
        Ok(Some(mappings))
    }

    fn callable_produced_labels(
        &self,
        source_index: usize,
        source: &a::Function,
        substitutions: &BTreeMap<String, usize>,
    ) -> BTreeSet<Label> {
        let mut parameters = source.generics.clone();
        if let a::FunctionOwner::Implementation(owner) = self.source.function_owner(a::FunctionId {
            index: source_index,
        }) {
            parameters.extend(self.source.impls[owner.index].generics.clone());
        }
        let mut produced = BTreeSet::new();
        for parameter in parameters {
            let Some(ty) = substitutions.get(&parameter.name) else {
                continue;
            };
            let Some(signature) = self.callable_signature(*ty) else {
                continue;
            };
            for bound in parameter.bounds {
                if !self.is_callable_bound(bound) {
                    continue;
                }
                let a::Type::Path(path) = &self.source.types[bound].kind else {
                    unreachable!()
                };
                let Some(output) = path.args.get(1) else {
                    continue;
                };
                for leaf in
                    self.signature_leaves(source, source_index, *output, signature.result, false)
                {
                    if matches!(leaf.label, Label::Parameter(_, _) | Label::Associated(_, _)) {
                        produced.insert(leaf.label);
                    }
                }
            }
        }
        produced
    }

    /// A produced type/associated family has no positional relationship to an
    /// unrelated input family. Enumerate admissible concrete input leaves as an
    /// upper bound; ordinary body checking will establish the actual selectors.
    pub(super) fn produced_return_leaf_contract(
        &self,
        source_index: usize,
        source: &a::Function,
        locals: &[k::Local],
        result: usize,
        substitutions: &BTreeMap<String, usize>,
    ) -> Option<Vec<k::ReturnLeaf>> {
        let outputs = self.signature_leaves(source, source_index, source.result, result, false);
        if !outputs
            .iter()
            .any(|leaf| matches!(leaf.label, Label::Parameter(_, _) | Label::Associated(_, _)))
            || outputs
                .iter()
                .any(|leaf| matches!(leaf.label, Label::Unknown | Label::Elided))
        {
            return None;
        }
        let mut inputs = BTreeMap::<Label, Vec<Candidate>>::new();
        for (index, parameter) in source.params.iter().enumerate() {
            for leaf in
                self.signature_leaves(source, source_index, parameter.ty, locals[index].ty, true)
            {
                inputs.entry(leaf.label).or_default().push(Candidate {
                    input: k::InputLeaf {
                        index,
                        projections: leaf.path,
                    },
                    transport: leaf.transport,
                    mutable: leaf.mutable,
                });
            }
        }
        let produced = self.callable_produced_labels(source_index, source, substitutions);
        let mut widened = false;
        let mut mappings = Vec::new();
        for leaf in outputs {
            let origin = if matches!(leaf.label, Label::Static) {
                k::LeafOrigin::Static
            } else {
                let family = matches!(leaf.label, Label::Parameter(_, _) | Label::Associated(_, _));
                let matching: Vec<_> = inputs
                    .get(&leaf.label)
                    .into_iter()
                    .flatten()
                    .filter(|candidate| {
                        (!family || candidate.transport) && (!leaf.mutable || candidate.mutable)
                    })
                    .map(|candidate| candidate.input.clone())
                    .collect();
                if !matching.is_empty() && !produced.contains(&leaf.label) {
                    k::LeafOrigin::Inputs {
                        inputs: matching,
                        exact: family,
                    }
                } else if family {
                    widened = true;
                    let candidates: BTreeSet<_> = inputs
                        .values()
                        .flatten()
                        .filter(|candidate| !leaf.mutable || candidate.mutable)
                        .map(|candidate| candidate.input.clone())
                        .collect();
                    if candidates.is_empty() {
                        if leaf.mutable {
                            return None;
                        }
                        k::LeafOrigin::Static
                    } else {
                        k::LeafOrigin::Inputs {
                            inputs: candidates.into_iter().collect(),
                            exact: false,
                        }
                    }
                } else {
                    return None;
                }
            };
            mappings.push(k::ReturnLeaf {
                projections: leaf.path,
                origin,
            });
        }
        widened.then_some(mappings)
    }
}
