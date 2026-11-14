//! Source annotations stay attached to the body that owns their IR identities.
//! Probe builders discard these records; completed functions install them once.
use super::{Builder, Result, a, err, ir};
use crate::check::scoped::{
    Annotation, Annotations, Construction, Environment, Region, ResultLeaf,
};
use std::collections::{BTreeMap, BTreeSet};

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
    pub(super) fn remap_places(&mut self, remap: &super::closures::CaptureRemap<'_>) -> Result<()> {
        let mut index = 0;
        while index < self.annotations.bindings.len() {
            remap.apply(&mut self.annotations.bindings[index].place)?;
            index += 1;
        }
        let mut checkpoints: Vec<usize> = Vec::new();
        for key in self.annotations.checkpoints.keys() {
            checkpoints.push(*key);
        }
        for key in checkpoints {
            let entries = self.annotations.checkpoints.get_mut(&key).unwrap();
            let mut index = 0;
            while index < entries.len() {
                remap.apply(&mut entries[index].place)?;
                index += 1;
            }
        }
        let mut constructions: Vec<usize> = Vec::new();
        for key in self.constructions.keys() {
            constructions.push(*key);
        }
        for key in constructions {
            let entries = self.constructions.get_mut(&key).unwrap();
            let mut index = 0;
            while index < entries.len() {
                remap.apply(&mut entries[index].place)?;
                index += 1;
            }
        }
        Ok(())
    }
}

fn named_region_argument(name: &str, bound: &BTreeSet<String>) -> bool {
    !name.is_empty() && name != "'_" && !bound.contains(&name.to_string())
}

impl Builder<'_, '_> {
    pub(super) fn explicit_type_arguments(
        &self,
        arguments: &[a::TypeId],
        span: a::Span,
    ) -> Result<()> {
        if arguments.is_empty() {
            return Ok(());
        }
        // Bound-name tables are immutable. Pending types share their owned
        // table by index rather than storing reference-counted sets.
        let mut environments = vec![BTreeSet::<String>::new()];
        let mut pending: Vec<(a::TypeId, usize)> = Vec::new();
        for ty in arguments {
            pending.push((*ty, 0));
        }
        let mut declarations = BTreeSet::new();
        while let Some((ty, bound)) = pending.pop() {
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
                    if named_region_argument(lifetime, &environments[bound]) {
                        return Err(unsupported());
                    }
                    pending.push((*inner, bound));
                }
                a::Type::Path(path) => {
                    if path.name.starts_with('\'')
                        && named_region_argument(&path.name, &environments[bound])
                    {
                        return Err(unsupported());
                    }
                    for ty in &path.args {
                        pending.push((*ty, bound));
                    }
                    if let Some(id) = path.declaration {
                        if !declarations.insert(id) {
                            continue;
                        }
                        let (parameters, fields): (&Vec<a::GenericParam>, Vec<a::TypeId>) =
                            match self.compiler.source.declarations[id].item {
                                a::Item::Alias(index) => {
                                    let definition = &self.compiler.source.aliases[index];
                                    (&definition.generics, vec![definition.ty])
                                }
                                a::Item::Struct(index) => {
                                    let definition = &self.compiler.source.structs[index];
                                    let mut fields = Vec::new();
                                    for field in &definition.fields {
                                        fields.push(field.ty);
                                    }
                                    (&definition.generics, fields)
                                }
                                a::Item::Enum(index) => {
                                    let definition = &self.compiler.source.enums[index];
                                    let mut fields = Vec::new();
                                    for variant in &definition.variants {
                                        for field in &variant.fields {
                                            fields.push(field.ty);
                                        }
                                    }
                                    (&definition.generics, fields)
                                }
                                _ => continue,
                            };
                        let mut names = BTreeSet::new();
                        for parameter in parameters {
                            if parameter.lifetime {
                                names.insert(parameter.name.clone());
                            }
                        }
                        let environment = environments.len();
                        environments.push(names);
                        for ty in fields {
                            pending.push((ty, environment));
                        }
                    }
                }
                a::Type::Tuple(fields) => {
                    for ty in fields {
                        pending.push((*ty, bound));
                    }
                }
                a::Type::Array { inner, .. } | a::Type::Slice(inner) => {
                    pending.push((*inner, bound))
                }
                a::Type::Function { params, result } => {
                    for ty in params {
                        pending.push((*ty, bound));
                    }
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
        place: &ir::Place,
    ) -> Result<Vec<Annotation>> {
        let ty =
            super::place_type(&self.compiler.types, self.function, place).ok_or_else(|| {
                err(
                    self.compiler.source.types[syntax].span,
                    "E_TYPE",
                    "invalid annotated place",
                )
            })?;
        let mut annotations = Vec::new();
        for leaf in self.fixed_regions(syntax, ty)? {
            let mut projected = place.clone();
            for projection in leaf.projections {
                projected.projections.push(projection);
            }
            annotations.push(Annotation {
                place: projected,
                region: leaf.region,
                created: leaf.created,
                invariant: leaf.invariant,
            });
        }
        Ok(annotations)
    }
    fn declared_region(&self, name: &str) -> bool {
        for parameter in self.compiler.scoped_parameters(self.source_function) {
            if parameter.lifetime && parameter.name == name {
                return true;
            }
        }
        false
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
                if !self.declared_region(name) {
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
            if name != "'static" && !self.declared_region(&name) {
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
                    ir::TypeKind::Ref { inner, .. } => {
                        if in_slice {
                            return Err(err(
                                self.compiler.source.types[syntax].span,
                                "E_UNSUPPORTED",
                                "named local regions in slice elements need a collection reference inventory",
                            ));
                        }
                        pending.push((*inner, false));
                    }
                    ir::TypeKind::Slice { inner, .. } => pending.push((*inner, true)),
                    ir::TypeKind::Box { inner }
                    | ir::TypeKind::Vec { inner }
                    | ir::TypeKind::Array { inner, .. } => pending.push((*inner, in_slice)),
                    ir::TypeKind::Struct { fields, .. } => {
                        for field in fields {
                            pending.push((field.ty, in_slice));
                        }
                    }
                    ir::TypeKind::Enum { variants, .. } => {
                        for variant in variants {
                            for field in &variant.fields {
                                pending.push((field.ty, in_slice));
                            }
                        }
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
        place: ir::Place,
    ) -> Result<()> {
        let annotations = match syntax {
            Some(syntax) => self.fixed_annotation(syntax, &place)?,
            None => Vec::new(),
        };
        if !annotations.is_empty() {
            let position = self.function.body.len();
            if self
                .scoped_body
                .annotations
                .checkpoints
                .contains_key(&position)
            {
                let entries = self
                    .scoped_body
                    .annotations
                    .checkpoints
                    .get_mut(&position)
                    .unwrap();
                entries.extend_from_slice(&annotations);
            } else {
                self.scoped_body
                    .annotations
                    .checkpoints
                    .insert(position, annotations.clone());
            }
        }
        self.typed_bindings = annotations;
        Ok(())
    }
    pub(super) fn annotation_uninitialized(
        &mut self,
        syntax: a::TypeId,
        local: usize,
    ) -> Result<()> {
        let annotations = self.fixed_annotation(syntax, &ir::Place::local(local))?;
        for annotation in annotations {
            self.scoped_body.annotations.bindings.push(annotation);
        }
        Ok(())
    }
    pub(super) fn annotation_binding(&mut self, from: &ir::Place, local: usize, borrowed: bool) {
        for annotation in &self.typed_bindings {
            if annotation.place.local != from.local
                || !annotation.place.projections.starts_with(&from.projections)
            {
                continue;
            }
            let mut next = annotation.clone();
            next.place = ir::Place::local(local);
            if borrowed {
                next.place.projections.push(ir::Projection::Deref);
                next.invariant |= matches!(
                    self.compiler.types[self.ty(local)].kind,
                    ir::TypeKind::Ref { mutable: true, .. }
                );
            }
            next.place
                .projections
                .extend_from_slice(&annotation.place.projections[from.projections.len()..]);
            self.scoped_body.annotations.bindings.push(next);
        }
    }
}
