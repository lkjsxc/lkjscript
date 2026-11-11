//! Iterative elaboration of source types and nominal applications.
//!
//! Source edges, generic substitutions, nominal fields, and transparent aliases
//! use explicit work frames. In-progress nominal IDs are interned before their
//! fields, preserving recursive types through indirection without native recursion.
use super::{Compiler, Result, TypeKey, a, err, is_copy, k};
use std::collections::BTreeMap;

enum Work {
    Source {
        id: a::TypeId,
        environment: usize,
    },
    Remember {
        id: a::TypeId,
        environment: usize,
    },
    Named {
        path: a::Path,
        environment: usize,
        span: a::Span,
    },
    Apply {
        path: a::Path,
        environment: usize,
        span: a::Span,
        arguments: usize,
    },
    Reference {
        mutable: bool,
    },
    Slice,
    Array {
        len: String,
        span: a::Span,
    },
    Tuple {
        fields: usize,
    },
    Struct {
        id: usize,
        source: usize,
        args: Vec<usize>,
        span: a::Span,
    },
    Enum {
        id: usize,
        source: usize,
        args: Vec<usize>,
        span: a::Span,
    },
    Alias {
        key: TypeKey,
    },
}

struct Resolution {
    pending: Vec<Work>,
    values: Vec<usize>,
    environments: Vec<BTreeMap<String, usize>>,
    resolved: BTreeMap<(a::TypeId, usize), usize>,
}

impl Resolution {
    fn new(start: Work, substitutions: &BTreeMap<String, usize>) -> Self {
        Resolution {
            pending: vec![start],
            values: Vec::new(),
            environments: vec![substitutions.clone()],
            resolved: BTreeMap::new(),
        }
    }

    fn take_values(&mut self, count: usize) -> Vec<usize> {
        self.values.split_off(self.values.len() - count)
    }

    fn environment(&mut self, bindings: BTreeMap<String, usize>) -> usize {
        let id = self.environments.len();
        self.environments.push(bindings);
        id
    }
}

impl Compiler<'_> {
    pub(super) fn resolve(
        &mut self,
        id: a::TypeId,
        sub: &BTreeMap<String, usize>,
    ) -> Result<usize> {
        self.resolve_work(Work::Source { id, environment: 0 }, sub)
    }

    pub(super) fn named_type(
        &mut self,
        path: &a::Path,
        sub: &BTreeMap<String, usize>,
        span: a::Span,
    ) -> Result<usize> {
        self.resolve_work(
            Work::Named {
                path: path.clone(),
                environment: 0,
                span,
            },
            sub,
        )
    }

    fn resolve_work(&mut self, start: Work, sub: &BTreeMap<String, usize>) -> Result<usize> {
        let mut work = Resolution::new(start, sub);
        while let Some(frame) = work.pending.pop() {
            match frame {
                Work::Source { id, environment } => {
                    if let Some(ty) = work.resolved.get(&(id, environment)) {
                        work.values.push(*ty);
                        continue;
                    }
                    work.pending.push(Work::Remember { id, environment });
                    let node = &self.source.types[id];
                    match &node.kind {
                        a::Type::Infer => {
                            return Err(err(node.span, "E_TYPE", "type needs an annotation"));
                        }
                        a::Type::Path(path) => work.pending.push(Work::Named {
                            path: path.clone(),
                            environment,
                            span: node.span,
                        }),
                        a::Type::Ref { mutable, inner, .. } => {
                            work.pending.push(Work::Reference { mutable: *mutable });
                            work.pending.push(Work::Source {
                                id: *inner,
                                environment,
                            });
                        }
                        a::Type::Slice(inner) => {
                            work.pending.push(Work::Slice);
                            work.pending.push(Work::Source {
                                id: *inner,
                                environment,
                            });
                        }
                        a::Type::Array { inner, len } => {
                            work.pending.push(Work::Array {
                                len: len.clone(),
                                span: node.span,
                            });
                            work.pending.push(Work::Source {
                                id: *inner,
                                environment,
                            });
                        }
                        a::Type::Tuple(fields) => {
                            work.pending.push(Work::Tuple {
                                fields: fields.len(),
                            });
                            for field in fields.iter().rev() {
                                work.pending.push(Work::Source {
                                    id: *field,
                                    environment,
                                });
                            }
                        }
                        a::Type::Never => work.values.push(k::UNIT),
                        _ => {
                            return Err(err(
                                node.span,
                                "E_UNSUPPORTED",
                                "this type form is not implemented yet",
                            ));
                        }
                    }
                }
                Work::Remember { id, environment } => {
                    let value = *work.values.last().expect("completed source type");
                    work.resolved.insert((id, environment), value);
                }
                Work::Named {
                    path,
                    environment,
                    span,
                } => {
                    let bindings = &work.environments[environment];
                    if let Some(id) = bindings.get(&path.name) {
                        work.values.push(*id);
                        continue;
                    }
                    if let Some(id) = self.associated_contract(&path, bindings, span)? {
                        work.values.push(id);
                        continue;
                    }
                    if path.args.is_empty() && path.declaration.is_none() {
                        if let Some(id) = self.type_names.get(&path.name) {
                            work.values.push(*id);
                            continue;
                        }
                    }
                    let mut arguments = Vec::new();
                    for arg in &path.args {
                        if !matches!(&self.source.types[*arg].kind, a::Type::Path(p) if p.name.starts_with('\''))
                        {
                            arguments.push(*arg);
                        }
                    }
                    work.pending.push(Work::Apply {
                        path,
                        environment,
                        span,
                        arguments: arguments.len(),
                    });
                    for argument in arguments.into_iter().rev() {
                        work.pending.push(Work::Source {
                            id: argument,
                            environment,
                        });
                    }
                }
                Work::Apply {
                    path,
                    environment,
                    span,
                    arguments,
                } => {
                    let args = work.take_values(arguments);
                    self.apply_named_type(path, args, environment, span, &mut work)?;
                }
                Work::Reference { mutable } => {
                    let inner = work.values.pop().expect("reference child");
                    work.values.push(self.reference(inner, mutable));
                }
                Work::Slice => {
                    let inner = work.values.pop().expect("slice child");
                    let id = self.intern(
                        String::new(),
                        k::TypeKind::Slice {
                            inner,
                            mutable: false,
                        },
                    );
                    work.values.push(id);
                }
                Work::Array { len, span } => {
                    let inner = work.values.pop().expect("array child");
                    let len = len.replace('_', "").parse::<usize>().map_err(|_| {
                        err(span, "E_TYPE", "array length must be an integer literal")
                    })?;
                    let id = self.intern(String::new(), k::TypeKind::Array { inner, len });
                    work.values.push(id);
                }
                Work::Tuple { fields } => {
                    let values = work.take_values(fields);
                    if values.is_empty() {
                        work.values.push(k::UNIT);
                        continue;
                    }
                    let mut fields = Vec::new();
                    for (index, ty) in values.into_iter().enumerate() {
                        fields.push(k::Field {
                            name: index.to_string(),
                            ty,
                        });
                    }
                    let copy = fields.iter().all(|field| is_copy(&self.types, field.ty));
                    let id = self.intern(String::new(), k::TypeKind::Struct { fields, copy });
                    work.values.push(id);
                }
                Work::Struct {
                    id,
                    source,
                    args,
                    span,
                } => {
                    let definition = &self.source.structs[source];
                    let values = work.take_values(definition.fields.len());
                    let mut fields = Vec::new();
                    for (field, ty) in definition.fields.iter().zip(values) {
                        fields.push(k::Field {
                            name: field.name.clone(),
                            ty,
                        });
                    }
                    let requested_copy = definition.attrs.iter().any(|attr| {
                        attr.name == "derive" && attr.args.iter().any(|name| name == "Copy")
                    });
                    let fields_copy = fields.iter().all(|field| is_copy(&self.types, field.ty));
                    if requested_copy
                        && !fields_copy
                        && definition
                            .generics
                            .iter()
                            .all(|parameter| parameter.lifetime)
                    {
                        return Err(err(span, "E_TYPE", "Copy requires Copy fields"));
                    }
                    self.types[id].kind = k::TypeKind::Struct {
                        fields,
                        copy: requested_copy && fields_copy,
                    };
                    self.applications.insert(
                        id,
                        (super::qualified(&definition.module, &definition.name), args),
                    );
                    work.values.push(id);
                }
                Work::Enum {
                    id,
                    source,
                    args,
                    span,
                } => {
                    let definition = &self.source.enums[source];
                    let count = definition
                        .variants
                        .iter()
                        .map(|variant| variant.fields.len())
                        .sum();
                    let values = work.take_values(count);
                    let mut at = 0;
                    let mut variants = Vec::new();
                    for variant in &definition.variants {
                        let mut fields = Vec::new();
                        for field in &variant.fields {
                            fields.push(k::Field {
                                name: field.name.clone(),
                                ty: values[at],
                            });
                            at += 1;
                        }
                        variants.push(k::Variant {
                            name: variant.name.clone(),
                            fields,
                        });
                    }
                    let requested_copy = definition.attrs.iter().any(|attr| {
                        attr.name == "derive" && attr.args.iter().any(|name| name == "Copy")
                    });
                    let fields_copy = variants.iter().all(|variant| {
                        variant
                            .fields
                            .iter()
                            .all(|field| is_copy(&self.types, field.ty))
                    });
                    if requested_copy
                        && !fields_copy
                        && definition
                            .generics
                            .iter()
                            .all(|parameter| parameter.lifetime)
                    {
                        return Err(err(span, "E_TYPE", "Copy requires Copy fields"));
                    }
                    self.types[id].kind = k::TypeKind::Enum {
                        variants,
                        copy: requested_copy && fields_copy,
                    };
                    self.applications.insert(
                        id,
                        (super::qualified(&definition.module, &definition.name), args),
                    );
                    work.values.push(id);
                }
                Work::Alias { key } => {
                    let id = *work.values.last().expect("alias target");
                    self.type_keys.insert(key, id);
                }
            }
        }
        Ok(work.values.pop().expect("resolved type"))
    }

    fn apply_named_type(
        &mut self,
        path: a::Path,
        mut args: Vec<usize>,
        environment: usize,
        span: a::Span,
        work: &mut Resolution,
    ) -> Result<()> {
        // Only unresolved builtin names select intrinsic representations.
        // Source declarations (including user types named Box or Vec) retain
        // their identity; synthetic nominal paths use the canonical map below.
        if path.declaration.is_none() && (path.name == "Vec" || path.name == "Box") {
            if args.len() != 1 {
                return Err(err(span, "E_TYPE", "collection requires one type argument"));
            }
            let kind = if path.name == "Vec" {
                k::TypeKind::Vec { inner: args[0] }
            } else {
                k::TypeKind::Box { inner: args[0] }
            };
            work.values.push(self.intern(String::new(), kind));
            return Ok(());
        }
        if path.declaration.is_none() && path.name == "World" {
            work.values
                .push(self.intern(String::new(), k::TypeKind::World));
            return Ok(());
        }
        let declaration = path
            .declaration
            .or_else(|| self.declaration_names.get(&path.name).copied());
        let item = declaration.map(|id| self.source.declarations[id].item);
        if args.is_empty() {
            let parameters = match item {
                Some(a::Item::Struct(index)) => &self.source.structs[index].generics[..],
                Some(a::Item::Enum(index)) => &self.source.enums[index].generics[..],
                _ => &[],
            };
            for parameter in parameters {
                if !parameter.lifetime {
                    if let Some(ty) = work.environments[environment].get(&parameter.name) {
                        args.push(*ty);
                    }
                }
            }
        }
        let declaration = declaration
            .ok_or_else(|| err(span, "E_TYPE", &format!("unknown type {}", path.name)))?;
        let key = TypeKey::Nominal(declaration, args.clone());
        if let Some(id) = self.type_keys.get(&key) {
            work.values.push(*id);
            return Ok(());
        }
        let parameters = match item {
            Some(a::Item::Struct(index)) => &self.source.structs[index].generics,
            Some(a::Item::Enum(index)) => &self.source.enums[index].generics,
            Some(a::Item::Alias(index)) => &self.source.aliases[index].generics,
            _ => return Err(err(span, "E_TYPE", &format!("unknown type {}", path.name))),
        };
        let type_parameters: Vec<_> = parameters
            .iter()
            .filter(|parameter| !parameter.lifetime)
            .collect();
        if type_parameters.len() != args.len() {
            return Err(err(
                span,
                "E_TYPE",
                if matches!(item, Some(a::Item::Alias(_))) {
                    "wrong number of alias type arguments"
                } else {
                    "wrong number of type arguments"
                },
            ));
        }
        let mut bindings = work.environments[environment].clone();
        for (parameter, argument) in type_parameters.iter().zip(&args) {
            bindings.insert(parameter.name.clone(), *argument);
        }
        self.bounds(parameters, &bindings)?;
        let environment = work.environment(bindings);
        if let Some(a::Item::Alias(index)) = item {
            work.pending.push(Work::Alias { key });
            work.pending.push(Work::Source {
                id: self.source.aliases[index].ty,
                environment,
            });
            return Ok(());
        }
        let canonical = &self.source.declarations[declaration].name;
        let description = if args.is_empty() {
            canonical.clone()
        } else {
            format!(
                "{}<{}>",
                canonical,
                args.iter()
                    .map(|ty| format!("t{ty}"))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        let id = self.types.len();
        self.type_keys.insert(key, id);
        self.types.push(k::Type {
            name: description,
            kind: k::TypeKind::Unit,
        });
        match item {
            Some(a::Item::Struct(index)) => {
                work.pending.push(Work::Struct {
                    id,
                    source: index,
                    args,
                    span,
                });
                for field in self.source.structs[index].fields.iter().rev() {
                    work.pending.push(Work::Source {
                        id: field.ty,
                        environment,
                    });
                }
            }
            Some(a::Item::Enum(index)) => {
                work.pending.push(Work::Enum {
                    id,
                    source: index,
                    args,
                    span,
                });
                for variant in self.source.enums[index].variants.iter().rev() {
                    for field in variant.fields.iter().rev() {
                        work.pending.push(Work::Source {
                            id: field.ty,
                            environment,
                        });
                    }
                }
            }
            _ => unreachable!("nominal declaration"),
        }
        Ok(())
    }
}
