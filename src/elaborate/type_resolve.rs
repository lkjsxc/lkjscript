//! Iterative elaboration of source types and nominal applications.
//!
//! Source edges, generic substitutions, nominal fields, and transparent aliases
//! use explicit work frames. In-progress nominal IDs are interned before their
//! fields, preserving recursive types through indirection without native recursion.
use super::{Compiler, Result, TypeKey, a, err, ir, is_copy};
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
        span: a::Span,
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
        let start = self.values.len() - count;
        let values = self.values[start..].to_vec();
        self.values.truncate(start);
        values
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
                            work.pending.push(Work::Reference {
                                mutable: *mutable,
                                span: node.span,
                            });
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
                            let mut index = fields.len();
                            while index != 0 {
                                index -= 1;
                                work.pending.push(Work::Source {
                                    id: fields[index],
                                    environment,
                                });
                            }
                        }
                        a::Type::Function { .. } => {
                            return Err(err(
                                node.span,
                                "E_UNSUPPORTED",
                                "function pointer types are not implemented",
                            ));
                        }
                        a::Type::Never => work.values.push(ir::UNIT),
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
                    if self.diagnostic_functions.is_some()
                        && path.declaration.is_some_and(|id| {
                            matches!(
                                self.source.declarations[id].item,
                                a::Item::Struct(_) | a::Item::Enum(_)
                            )
                        })
                    {
                        return Err(err(
                            span,
                            "E_BLOCKED",
                            "nominal layout requires its complete declaration contracts",
                        ));
                    }
                    let bindings = &work.environments[environment];
                    if self.names.generic_path(&path, span) {
                        if !path.args.is_empty() {
                            return Err(err(
                                span,
                                "E_TYPE",
                                "generic parameter or projection cannot take type arguments",
                            ));
                        }
                        if let Some(id) = bindings.get(&path.name) {
                            work.values.push(*id);
                            continue;
                        }
                        if self.names.is_type_parameter(&path, span) {
                            return Err(err(
                                span,
                                "E_TYPE",
                                &format!("generic parameter {} needs a type argument", path.name),
                            ));
                        }
                        if let Some(id) = self.associated_contract(&path, bindings, span)? {
                            work.values.push(id);
                            continue;
                        }
                    }
                    if path.args.is_empty() && path.declaration.is_none() {
                        if let Some(id) = self.type_names.get(&path.name) {
                            work.values.push(*id);
                            continue;
                        }
                    }
                    let mut arguments: Vec<usize> = Vec::new();
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
                    while let Some(argument) = arguments.pop() {
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
                Work::Reference { mutable, span } => {
                    let inner = work.values.pop().expect("reference child");
                    if mutable && matches!(self.types[inner].kind, ir::TypeKind::Str) {
                        return Err(err(
                            span,
                            "E_UNSUPPORTED",
                            "mutable str views are not implemented",
                        ));
                    }
                    work.values.push(self.reference(inner, mutable));
                }
                Work::Slice => {
                    let inner = work.values.pop().expect("slice child");
                    let id = self.intern(
                        String::new(),
                        ir::TypeKind::Slice {
                            inner,
                            mutable: false,
                        },
                    );
                    work.values.push(id);
                }
                Work::Array { len, span } => {
                    let inner = work.values.pop().expect("array child");
                    let digits = len.replace('_', "");
                    let magnitude: u64 = match super::integers::magnitude(&digits, 10) {
                        Some(value) => value,
                        None => {
                            return Err(err(
                                span,
                                "E_TYPE",
                                "array length must be an integer literal",
                            ));
                        }
                    };
                    let len = magnitude as usize;
                    if len as u64 != magnitude {
                        return Err(err(
                            span,
                            "E_TYPE",
                            "array length must be an integer literal",
                        ));
                    }
                    self.check_owned_element(inner, span)?;
                    let id = self.intern(String::new(), ir::TypeKind::Array { inner, len });
                    work.values.push(id);
                }
                Work::Tuple { fields } => {
                    let values = work.take_values(fields);
                    if values.is_empty() {
                        work.values.push(ir::UNIT);
                        continue;
                    }
                    let mut fields = Vec::new();
                    for (index, ty) in values.into_iter().enumerate() {
                        fields.push(ir::Field {
                            name: format!("{index}"),
                            ty,
                        });
                    }
                    let copy = fields.iter().all(|field| is_copy(&self.types, field.ty));
                    let id = self.intern(String::new(), ir::TypeKind::Struct { fields, copy });
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
                        fields.push(ir::Field {
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
                    self.types[id].kind = ir::TypeKind::Struct {
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
                    let mut count = 0;
                    for variant in &definition.variants {
                        count += variant.fields.len();
                    }
                    let values = work.take_values(count);
                    let mut at = 0;
                    let mut variants = Vec::new();
                    for variant in &definition.variants {
                        let mut fields = Vec::new();
                        for field in &variant.fields {
                            fields.push(ir::Field {
                                name: field.name.clone(),
                                ty: values[at],
                            });
                            at += 1;
                        }
                        variants.push(ir::Variant {
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
                    self.types[id].kind = ir::TypeKind::Enum {
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
            self.check_owned_element(args[0], span)?;
            let kind = if path.name == "Vec" {
                ir::TypeKind::Vec { inner: args[0] }
            } else {
                ir::TypeKind::Box { inner: args[0] }
            };
            work.values.push(self.intern(String::new(), kind));
            return Ok(());
        }
        if path.declaration.is_none() && path.name == "str" {
            if !args.is_empty() {
                return Err(err(span, "E_TYPE", "str takes no type arguments"));
            }
            work.values
                .push(self.intern(String::new(), ir::TypeKind::Str));
            return Ok(());
        }
        if path.declaration.is_none()
            && matches!(
                path.name.as_str(),
                "World" | "Stdout" | "Stdin" | "Stderr" | "Directory" | "File"
            )
        {
            if !args.is_empty() {
                return Err(err(
                    span,
                    "E_TYPE",
                    "capability types take no type arguments",
                ));
            }
            let kind = match path.name.as_str() {
                "World" => ir::TypeKind::World,
                "Stdout" => ir::TypeKind::Stdout,
                "Stdin" => ir::TypeKind::Stdin,
                "Stderr" => ir::TypeKind::Stderr,
                "Directory" => ir::TypeKind::Directory,
                "File" => ir::TypeKind::File,
                _ => unreachable!(),
            };
            work.values.push(self.intern(String::new(), kind));
            return Ok(());
        }
        let declaration = path
            .declaration
            .filter(|id| self.source.declarations[*id].name == path.name)
            .or_else(|| self.declaration_names.get(&path.name).copied());
        let item = declaration.map(|id| self.source.declarations[id].item);
        if args.is_empty() {
            let parameters: Vec<a::GenericParam> = match item {
                Some(a::Item::Struct(index)) => self.source.structs[index].generics.clone(),
                Some(a::Item::Enum(index)) => self.source.enums[index].generics.clone(),
                _ => Vec::new(),
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
        let parameters: Vec<a::GenericParam> = match item {
            Some(a::Item::Struct(index)) => self.source.structs[index].generics.clone(),
            Some(a::Item::Enum(index)) => self.source.enums[index].generics.clone(),
            Some(a::Item::Alias(index)) => self.source.aliases[index].generics.clone(),
            _ => return Err(err(span, "E_TYPE", &format!("unknown type {}", path.name))),
        };
        let mut type_parameters: Vec<String> = Vec::new();
        for parameter in &parameters {
            if !parameter.lifetime {
                type_parameters.push(parameter.name.clone());
            }
        }
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
            bindings.insert(parameter.clone(), *argument);
        }
        self.bounds(&parameters, &bindings)
            .map_err(|problem| super::at_use_site(problem, span))?;
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
            let mut description = canonical.clone();
            description.push('<');
            let mut index = 0;
            while index < args.len() {
                if index != 0 {
                    description.push(',');
                }
                description.push_str(&format!("t{}", args[index]));
                index += 1;
            }
            description.push('>');
            description
        };
        let id = self.types.len();
        self.type_keys.insert(key, id);
        self.types.push(ir::Type {
            name: description,
            kind: ir::TypeKind::Unit,
        });
        match item {
            Some(a::Item::Struct(index)) => {
                work.pending.push(Work::Struct {
                    id,
                    source: index,
                    args,
                    span,
                });
                let fields = &self.source.structs[index].fields;
                let mut field = fields.len();
                while field != 0 {
                    field -= 1;
                    work.pending.push(Work::Source {
                        id: fields[field].ty,
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
                let variants = &self.source.enums[index].variants;
                let mut variant = variants.len();
                while variant != 0 {
                    variant -= 1;
                    let fields = &variants[variant].fields;
                    let mut field = fields.len();
                    while field != 0 {
                        field -= 1;
                        work.pending.push(Work::Source {
                            id: fields[field].ty,
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
