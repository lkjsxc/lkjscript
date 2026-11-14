//! Source-type inference uses heap work frames, including transparent aliases.
//! Repeated syntax still creates fresh inference variables in its environment.
use super::{Infer, Result, Shape, Variable, a, integer_name, parameters};
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
        base_environment: &BTreeMap<String, Variable>,
    ) -> Result<Variable> {
        let mut pending = vec![Work::Source { id, environment: 0 }];
        // Zero names the borrowed input; only alias substitutions need owned maps.
        let mut environments: Vec<BTreeMap<String, Variable>> = Vec::new();
        let mut values: Vec<Variable> = Vec::new();
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
                        let mut fields = fields;
                        while let Some(id) = fields.pop() {
                            pending.push(Work::Source { id, environment });
                        }
                    }
                    a::Type::Path(path) => {
                        if path.declaration.is_none() && path.args.is_empty() {
                            let bindings = if environment == 0 {
                                base_environment
                            } else {
                                &environments[environment - 1]
                            };
                            if let Some(value) = bindings.get(&path.name) {
                                values.push(*value);
                                continue;
                            }
                            if integer_name(&path.name)
                                || matches!(
                                    path.name.as_str(),
                                    "bool"
                                        | "char"
                                        | "f64"
                                        | "str"
                                        | "World"
                                        | "Stdout"
                                        | "Stdin"
                                        | "Stderr"
                                        | "Directory"
                                        | "File"
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
                        let mut arguments: Vec<a::TypeId> = Vec::new();
                        for arg in &path.args {
                            if !matches!(&self.source.types[*arg].kind,
                                    a::Type::Path(path) if path.name.starts_with('\''))
                            {
                                arguments.push(*arg);
                            }
                        }
                        pending.push(Work::Path {
                            id,
                            path,
                            environment,
                            arguments: arguments.len(),
                        });
                        while let Some(id) = arguments.pop() {
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
                    let len = len.replace('_', "");
                    values.push(match len.as_str().parse::<usize>().ok() {
                        Some(len) => self.shaped(Shape::Array(inner, len)),
                        None => self.blocked(),
                    });
                }
                Work::Tuple(count) => {
                    let fields = take_values(&mut values, count);
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
                    let arguments = take_values(&mut values, arguments);
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
                            let mut aliases = if environment == 0 {
                                base_environment.clone()
                            } else {
                                environments[environment - 1].clone()
                            };
                            let mut argument_index = 0;
                            for parameter in parameters(self.source, declaration) {
                                if argument_index >= arguments.len() {
                                    break;
                                }
                                let prefix = format!("{parameter}::");
                                let mut removed: Vec<String> = Vec::new();
                                for name in aliases.keys() {
                                    if name.starts_with(&prefix) {
                                        removed.push(name.clone());
                                    }
                                }
                                for name in removed {
                                    aliases.remove(&name);
                                }
                                aliases.insert(parameter, arguments[argument_index]);
                                argument_index += 1;
                            }
                            let generics = self.source.aliases[index].generics.clone();
                            self.parameter_equations(&generics, &mut aliases, false)?;
                            let environment = environments.len() + 1;
                            environments.push(aliases);
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

fn take_values(values: &mut Vec<Variable>, count: usize) -> Vec<Variable> {
    let start = values.len() - count;
    let mut result = Vec::new();
    for index in start..values.len() {
        result.push(values[index]);
    }
    values.truncate(start);
    result
}
