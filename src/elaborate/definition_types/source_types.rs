//! Source-type inference uses heap work frames, including transparent aliases.
//! Repeated syntax still creates fresh inference variables in its environment.
use super::{Infer, Result, Shape, Variable, a, integer_name, parameters};
use std::borrow::Cow;
use std::collections::BTreeMap;

enum Work {
    Source {
        id: a::TypeId,
        environment: usize,
    },
    Path {
        id: a::TypeId,
        path: a::Path,
        environment: usize,
        arguments: usize,
    },
    Reference(bool),
    Slice,
    Array(String),
    Tuple(usize),
}

impl Infer<'_, '_> {
    pub(super) fn source_type(
        &mut self,
        id: a::TypeId,
        environment: &BTreeMap<String, Variable>,
    ) -> Result<Variable> {
        let mut pending = vec![Work::Source { id, environment: 0 }];
        let mut environments = vec![Cow::Borrowed(environment)];
        let mut values = Vec::new();
        while let Some(work) = pending.pop() {
            match work {
                Work::Source { id, environment } => match self.source.types[id].kind.clone() {
                    a::Type::Infer => values.push(self.unknown()),
                    a::Type::Never => values.push(self.shaped(Shape::Never)),
                    a::Type::Ref { mutable, inner, .. } => {
                        pending.push(Work::Reference(mutable));
                        pending.push(Work::Source {
                            id: inner,
                            environment,
                        });
                    }
                    a::Type::Slice(inner) => {
                        pending.push(Work::Slice);
                        pending.push(Work::Source {
                            id: inner,
                            environment,
                        });
                    }
                    a::Type::Array { inner, len } => {
                        pending.push(Work::Array(len));
                        pending.push(Work::Source {
                            id: inner,
                            environment,
                        });
                    }
                    a::Type::Tuple(fields) => {
                        pending.push(Work::Tuple(fields.len()));
                        for id in fields.into_iter().rev() {
                            pending.push(Work::Source { id, environment });
                        }
                    }
                    a::Type::Path(path) => {
                        if path.declaration.is_none() && path.args.is_empty() {
                            if let Some(value) = environments[environment].get(&path.name) {
                                values.push(*value);
                                continue;
                            }
                            if integer_name(&path.name)
                                || matches!(
                                    path.name.as_str(),
                                    "bool" | "char" | "f64" | "str" | "World" | "Stdout"
                                )
                            {
                                values.push(self.atom(&path.name));
                                continue;
                            }
                            if path.name.contains("::") {
                                values.push(self.blocked());
                                continue;
                            }
                        }
                        let arguments: Vec<_> = path
                            .args
                            .iter()
                            .copied()
                            .filter(|arg| {
                                !matches!(&self.source.types[*arg].kind,
                                    a::Type::Path(path) if path.name.starts_with('\''))
                            })
                            .collect();
                        pending.push(Work::Path {
                            id,
                            path,
                            environment,
                            arguments: arguments.len(),
                        });
                        for id in arguments.into_iter().rev() {
                            pending.push(Work::Source { id, environment });
                        }
                    }
                    a::Type::Function { .. } => values.push(self.blocked()),
                },
                Work::Reference(mutable) => {
                    let inner = values.pop().expect("reference inference child");
                    values.push(self.shaped(Shape::Ref(mutable, inner)));
                }
                Work::Slice => {
                    let inner = values.pop().expect("slice inference child");
                    values.push(self.shaped(Shape::Slice(inner)));
                }
                Work::Array(len) => {
                    let inner = values.pop().expect("array inference child");
                    values.push(match len.replace('_', "").parse::<usize>() {
                        Ok(len) => self.shaped(Shape::Array(inner, len)),
                        Err(_) => self.blocked(),
                    });
                }
                Work::Tuple(count) => {
                    let fields = values.split_off(values.len() - count);
                    values.push(if fields.is_empty() {
                        self.atom("()")
                    } else {
                        self.shaped(Shape::Tuple(fields))
                    });
                }
                Work::Path {
                    id,
                    path,
                    environment,
                    arguments,
                } => {
                    let arguments = values.split_off(values.len() - arguments);
                    if path.declaration.is_none() && arguments.len() == 1 {
                        if path.name == "Vec" {
                            values.push(self.shaped(Shape::Vec(arguments[0])));
                            continue;
                        }
                        if path.name == "Box" {
                            values.push(self.shaped(Shape::Box(arguments[0])));
                            continue;
                        }
                    }
                    if let Some(declaration) = path.declaration {
                        if path.name != self.source.declarations[declaration].name {
                            values.push(self.shaped(Shape::Source(id)));
                            continue;
                        }
                        let item = self.source.declarations[declaration].item;
                        if let a::Item::Alias(index) = item {
                            let mut aliases = environments[environment].as_ref().clone();
                            for (parameter, value) in parameters(self.source, declaration)
                                .into_iter()
                                .zip(arguments)
                            {
                                aliases.insert(parameter, value);
                            }
                            let environment = environments.len();
                            environments.push(Cow::Owned(aliases));
                            pending.push(Work::Source {
                                id: self.source.aliases[index].ty,
                                environment,
                            });
                            continue;
                        }
                        if matches!(item, a::Item::Struct(_) | a::Item::Enum(_)) {
                            values.push(self.shaped(Shape::Nominal(declaration, arguments)));
                            continue;
                        }
                    }
                    values.push(self.shaped(Shape::Source(id)));
                }
            }
        }
        Ok(values.pop().expect("source inference type"))
    }
}
