//! Source annotations stay attached to the body that owns their K identities.
//! Probe builders discard these records; completed functions install them once.
use super::{Builder, Result, a, err, k};
use crate::check::scoped::{
    Annotation, Annotations, Construction, Environment, Region, ResultLeaf,
};
use std::collections::BTreeMap;

#[derive(Clone, Default)]
pub(super) struct Body {
    pub(super) annotations: Annotations,
    pub(super) constructions: BTreeMap<usize, Vec<Construction>>,
    pub(super) environment: Option<Environment>,
    pub(super) source_scope: Option<usize>,
    pub(super) parameters: Vec<Annotation>,
    pub(super) results: Vec<ResultLeaf>,
}
impl Body {
    pub(super) fn is_empty(&self) -> bool {
        self.annotations.bindings.is_empty()
            && self.annotations.checkpoints.is_empty()
            && self.constructions.is_empty()
            && self.environment.is_none()
            && self.parameters.is_empty()
            && self.results.is_empty()
    }
    pub(super) fn places_mut(&mut self) -> impl Iterator<Item = &mut k::Place> {
        self.annotations
            .bindings
            .iter_mut()
            .map(|entry| &mut entry.place)
            .chain(
                self.annotations
                    .checkpoints
                    .values_mut()
                    .flatten()
                    .map(|entry| &mut entry.place),
            )
            .chain(
                self.constructions
                    .values_mut()
                    .flatten()
                    .map(|entry| &mut entry.place),
            )
    }
}

impl Builder<'_, '_> {
    pub(super) fn explicit_type_arguments(
        &self,
        arguments: &[a::TypeId],
        span: a::Span,
    ) -> Result<()> {
        use std::collections::BTreeSet;
        use std::rc::Rc;
        if arguments.is_empty() {
            return Ok(());
        }
        let empty = Rc::new(BTreeSet::<String>::new());
        let mut pending: Vec<_> = arguments.iter().map(|ty| (*ty, empty.clone())).collect();
        let mut declarations = BTreeSet::new();
        while let Some((ty, bound)) = pending.pop() {
            let named = |name: &str| !name.is_empty() && name != "'_" && !bound.contains(name);
            let unsupported = || {
                err(
                    span,
                    "E_UNSUPPORTED",
                    "explicit named-region type arguments need a checked type-instantiation contract",
                )
            };
            match &self.compiler.source.types[ty].kind {
                a::Type::Ref {
                    lifetime, inner, ..
                } => {
                    if named(lifetime) {
                        return Err(unsupported());
                    }
                    pending.push((*inner, bound));
                }
                a::Type::Path(path) => {
                    if path.name.starts_with('\'') && named(&path.name) {
                        return Err(unsupported());
                    }
                    pending.extend(path.args.iter().map(|ty| (*ty, bound.clone())));
                    if let Some(id) = path.declaration.filter(|id| declarations.insert(*id)) {
                        let (parameters, fields): (&[a::GenericParam], Vec<a::TypeId>) =
                            match self.compiler.source.declarations[id].item {
                                a::Item::Alias(index) => {
                                    let definition = &self.compiler.source.aliases[index];
                                    (&definition.generics, vec![definition.ty])
                                }
                                a::Item::Struct(index) => {
                                    let definition = &self.compiler.source.structs[index];
                                    (
                                        &definition.generics,
                                        definition.fields.iter().map(|field| field.ty).collect(),
                                    )
                                }
                                a::Item::Enum(index) => {
                                    let definition = &self.compiler.source.enums[index];
                                    (
                                        &definition.generics,
                                        definition
                                            .variants
                                            .iter()
                                            .flat_map(|variant| {
                                                variant.fields.iter().map(|field| field.ty)
                                            })
                                            .collect(),
                                    )
                                }
                                _ => continue,
                            };
                        let bound: Rc<BTreeSet<String>> = Rc::new(
                            parameters
                                .iter()
                                .filter(|parameter| parameter.lifetime)
                                .map(|parameter| parameter.name.clone())
                                .collect(),
                        );
                        pending.extend(fields.into_iter().map(|ty| (ty, bound.clone())));
                    }
                }
                a::Type::Tuple(fields) => {
                    pending.extend(fields.iter().map(|ty| (*ty, bound.clone())))
                }
                a::Type::Array { inner, .. } | a::Type::Slice(inner) => {
                    pending.push((*inner, bound))
                }
                a::Type::Function { params, result } => {
                    pending.extend(params.iter().map(|ty| (*ty, bound.clone())));
                    pending.push((*result, bound));
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub(super) fn fixed_annotation(
        &self,
        syntax: a::TypeId,
        place: &k::Place,
    ) -> Result<Vec<Annotation>> {
        let ty =
            super::place_type(&self.compiler.types, self.function, place).ok_or_else(|| {
                err(
                    self.compiler.source.types[syntax].span,
                    "E_TYPE",
                    "invalid annotated place",
                )
            })?;
        Ok(self
            .fixed_regions(syntax, ty)?
            .into_iter()
            .map(|leaf| {
                let mut projected = place.clone();
                projected.projections.extend(leaf.projections);
                Annotation {
                    place: projected,
                    region: leaf.region,
                    created: leaf.created,
                    invariant: leaf.invariant,
                }
            })
            .collect())
    }
    pub(super) fn fixed_regions(&self, syntax: a::TypeId, ty: usize) -> Result<Vec<ResultLeaf>> {
        let mut pending = vec![syntax];
        let mut named = false;
        while let Some(id) = pending.pop() {
            let node = &self.compiler.source.types[id];
            let verify = |name: &str| -> Result<()> {
                if name.is_empty() || name == "'_" || name == "'static" {
                    return Ok(());
                }
                if !self
                    .compiler
                    .scoped_parameters(self.source_function)
                    .iter()
                    .any(|parameter| parameter.lifetime && parameter.name == name)
                {
                    return Err(err(
                        node.span,
                        "E_TYPE",
                        &format!("undeclared lifetime {name}"),
                    ));
                }
                Ok(())
            };
            match &node.kind {
                a::Type::Ref {
                    lifetime, inner, ..
                } => {
                    verify(lifetime)?;
                    named |= !lifetime.is_empty() && lifetime != "'_";
                    pending.push(*inner);
                }
                a::Type::Path(path) => {
                    if path.name.starts_with('\'') {
                        verify(&path.name)?;
                        named |= path.name != "'_";
                    }
                    if path.declaration.is_none() && path.name == "Self" {
                        if let a::FunctionOwner::Implementation(owner) =
                            self.compiler.source.function_owner(a::FunctionId {
                                index: self.source_function,
                            })
                        {
                            pending.push(self.compiler.source.impls[owner.index].target);
                        }
                    }
                    pending.extend_from_slice(&path.args);
                }
                a::Type::Tuple(fields) => pending.extend_from_slice(fields),
                a::Type::Slice(inner) | a::Type::Array { inner, .. } => pending.push(*inner),
                a::Type::Function { params, result } => {
                    pending.extend_from_slice(params);
                    pending.push(*result);
                }
                _ => {}
            }
        }
        let mut annotations = Vec::new();
        for leaf in self
            .compiler
            .region_leaves(self.source_function, syntax, ty)
        {
            let name = match leaf.label {
                super::origins::Label::Lifetime(name) => name,
                super::origins::Label::Static => "'static".to_string(),
                _ => continue,
            };
            if name != "'static"
                && !self
                    .compiler
                    .scoped_parameters(self.source_function)
                    .iter()
                    .any(|parameter| parameter.lifetime && parameter.name == name)
            {
                return Err(err(
                    self.compiler.source.types[syntax].span,
                    "E_TYPE",
                    &format!("undeclared lifetime {name}"),
                ));
            }
            let invariant =
                crate::check::scoped::invariant_path(&self.compiler.types, ty, &leaf.path);
            let span = self.compiler.source.types[syntax].span;
            annotations.push(ResultLeaf {
                projections: leaf.path,
                region: Region {
                    scope: if name == "'static" {
                        0
                    } else {
                        self.source_function
                    },
                    name,
                },
                created: (span.line, span.column),
                invariant,
            });
        }
        if named || !annotations.is_empty() {
            let mut pending = vec![(ty, false)];
            let mut visited = std::collections::BTreeSet::new();
            while let Some((ty, in_slice)) = pending.pop() {
                if !visited.insert((ty, in_slice)) {
                    continue;
                }
                match &self.compiler.types[ty].kind {
                    k::TypeKind::Ref { inner, .. } => {
                        if in_slice {
                            return Err(err(
                                self.compiler.source.types[syntax].span,
                                "E_UNSUPPORTED",
                                "named local regions in slice elements need a collection reference inventory",
                            ));
                        }
                        pending.push((*inner, false));
                    }
                    k::TypeKind::Slice { inner, .. } => pending.push((*inner, true)),
                    k::TypeKind::Box { inner }
                    | k::TypeKind::Vec { inner }
                    | k::TypeKind::Array { inner, .. } => pending.push((*inner, in_slice)),
                    k::TypeKind::Struct { fields, .. } => {
                        pending.extend(fields.iter().map(|field| (field.ty, in_slice)))
                    }
                    k::TypeKind::Enum { variants, .. } => {
                        pending.extend(variants.iter().flat_map(|variant| {
                            variant.fields.iter().map(|field| (field.ty, in_slice))
                        }))
                    }
                    _ => {}
                }
            }
        }
        Ok(annotations)
    }
    pub(super) fn annotation_initializer(
        &mut self,
        syntax: Option<a::TypeId>,
        place: k::Place,
    ) -> Result<()> {
        let annotations = match syntax {
            Some(syntax) => self.fixed_annotation(syntax, &place)?,
            None => Vec::new(),
        };
        if !annotations.is_empty() {
            self.scoped_body
                .annotations
                .checkpoints
                .entry(self.function.body.len())
                .or_default()
                .extend(annotations.clone());
        }
        self.typed_bindings = annotations;
        Ok(())
    }
    pub(super) fn annotation_uninitialized(
        &mut self,
        syntax: a::TypeId,
        local: usize,
    ) -> Result<()> {
        let annotations = self.fixed_annotation(syntax, &k::Place::local(local))?;
        self.scoped_body.annotations.bindings.extend(annotations);
        Ok(())
    }
    pub(super) fn annotation_binding(&mut self, from: &k::Place, local: usize, borrowed: bool) {
        for annotation in &self.typed_bindings {
            if annotation.place.local != from.local
                || !annotation.place.projections.starts_with(&from.projections)
            {
                continue;
            }
            let mut next = annotation.clone();
            next.place = k::Place::local(local);
            if borrowed {
                next.place.projections.push(k::Projection::Deref);
                next.invariant |= matches!(
                    self.compiler.types[self.ty(local)].kind,
                    k::TypeKind::Ref { mutable: true, .. }
                );
            }
            next.place
                .projections
                .extend_from_slice(&annotation.place.projections[from.projections.len()..]);
            self.scoped_body.annotations.bindings.push(next);
        }
    }
}
