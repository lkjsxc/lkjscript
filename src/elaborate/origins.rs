//! Symbolic origin variables in signatures. A type parameter's variable is a
//! map of reference-leaf paths, not one concrete allocation root.
use super::{Compiler, Result, a, k};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Label {
    Lifetime(String),
    Parameter(String, Vec<k::Projection>),
    Static,
    Unknown,
}
#[derive(Clone, Default)]
struct Environment {
    types: BTreeMap<String, (usize, usize)>,
    lifetimes: BTreeMap<String, String>,
}
type SignatureChild = (usize, usize, Vec<k::Projection>);
struct Leaf {
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
        let mut pending = vec![(syntax, concrete, 0usize, Vec::new(), false)];
        let mut result = Vec::new();
        while let Some((syntax, concrete, environment, path, borrowed)) = pending.pop() {
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
                        Label::Unknown
                    } else {
                        Label::Lifetime(name)
                    };
                    if !borrowed || !mutable {
                        result.push(Leaf {
                            path: path.clone(),
                            label,
                            transport: false,
                            mutable: *mutable,
                        });
                    }
                    if input && self.has_view(*inner) {
                        let mut nested = path;
                        nested.push(k::Projection::Deref);
                        pending.push((*syntax_inner, *inner, environment, nested, true));
                    }
                }
                (a::Type::Tuple(syntax_fields), k::TypeKind::Struct { fields, .. })
                    if syntax_fields.len() == fields.len() =>
                {
                    for (index, (syntax, field)) in syntax_fields.iter().zip(fields).enumerate() {
                        let mut nested = path.clone();
                        nested.push(k::Projection::Field(index));
                        pending.push((*syntax, field.ty, environment, nested, borrowed));
                    }
                }
                (a::Type::Path(named), _)
                    if named.declaration.is_none() && parameters.contains(&named.name) =>
                {
                    for (relative, mutable) in crate::check::reference_paths(&self.types, concrete)
                    {
                        if borrowed && mutable {
                            continue;
                        }
                        let mut leaf_path = path.clone();
                        leaf_path.extend_from_slice(&relative);
                        result.push(Leaf {
                            path: leaf_path,
                            label: Label::Parameter(named.name.clone(), relative),
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
                        pending.push((syntax, concrete, nested_id, nested_path, borrowed));
                    }
                }
                _ => {
                    for (relative, mutable) in crate::check::reference_paths(&self.types, concrete)
                    {
                        let mut leaf_path = path.clone();
                        leaf_path.extend(relative);
                        result.push(Leaf {
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
                .any(|leaf| matches!(leaf.label, Label::Unknown))
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
                Label::Unknown => return Ok(None),
                label => {
                    let Some(candidates) = inputs.get(label) else {
                        return Ok(None);
                    };
                    let exact = matches!(label, Label::Parameter(_, _));
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
}
