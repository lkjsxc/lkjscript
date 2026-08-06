use crate::{CapabilityKind, ResourceKind};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticPrimitiveKind {
    Never,
    Unit,
    Bool,
    I64,
    F64,
    String,
    Bytes,
    Path,
    ByteVector,
    ByteSlice,
    ByteSliceMut,
    Symbol,
}

pub enum SemanticType {
    Primitive(SemanticPrimitiveKind),
    Capability(CapabilityKind),
    Resource(ResourceKind),
    Product([u8; 32]),
    Enum {
        identity: [u8; 32],
        arguments: Vec<Self>,
    },
    Parameter(String),
    List(Box<Self>),
    Function {
        parameters: Vec<Self>,
        result: Box<Self>,
    },
    ForAll {
        parameters: Vec<String>,
        body: Box<Self>,
    },
}

impl Clone for SemanticType {
    fn clone(&self) -> Self {
        enum Work<'a> {
            Visit(&'a SemanticType),
            Enum([u8; 32], usize),
            List,
            Function(usize),
            ForAll(&'a [String]),
        }
        let mut pending = vec![Work::Visit(self)];
        let mut completed = Vec::new();
        while let Some(item) = pending.pop() {
            match item {
                Work::Visit(ty) => match ty {
                    SemanticType::Primitive(kind) => completed.push(SemanticType::Primitive(*kind)),
                    SemanticType::Capability(kind) => {
                        completed.push(SemanticType::Capability(*kind))
                    }
                    SemanticType::Resource(kind) => completed.push(SemanticType::Resource(*kind)),
                    SemanticType::Product(id) => completed.push(SemanticType::Product(*id)),
                    SemanticType::Enum {
                        identity,
                        arguments,
                    } => {
                        pending.push(Work::Enum(*identity, arguments.len()));
                        pending.extend(arguments.iter().rev().map(Work::Visit));
                    }
                    SemanticType::Parameter(name) => {
                        completed.push(SemanticType::Parameter(name.clone()));
                    }
                    SemanticType::List(inner) => {
                        pending.push(Work::List);
                        pending.push(Work::Visit(inner));
                    }
                    SemanticType::Function { parameters, result } => {
                        pending.push(Work::Function(parameters.len()));
                        pending.push(Work::Visit(result));
                        pending.extend(parameters.iter().rev().map(Work::Visit));
                    }
                    SemanticType::ForAll { parameters, body } => {
                        pending.push(Work::ForAll(parameters));
                        pending.push(Work::Visit(body));
                    }
                },
                Work::Enum(identity, count) => {
                    let Some(split) = completed.len().checked_sub(count) else {
                        unreachable!("semantic type clone enum completion order")
                    };
                    let arguments = completed.split_off(split);
                    completed.push(SemanticType::Enum {
                        identity,
                        arguments,
                    });
                }
                Work::List => {
                    let Some(inner) = completed.pop() else {
                        unreachable!("semantic type clone list completion order")
                    };
                    completed.push(SemanticType::List(Box::new(inner)));
                }
                Work::Function(count) => {
                    let Some(result) = completed.pop() else {
                        unreachable!("semantic type clone function result completion order")
                    };
                    let Some(split) = completed.len().checked_sub(count) else {
                        unreachable!("semantic type clone function parameter completion order")
                    };
                    let parameters = completed.split_off(split);
                    completed.push(SemanticType::Function {
                        parameters,
                        result: Box::new(result),
                    });
                }
                Work::ForAll(parameters) => {
                    let Some(body) = completed.pop() else {
                        unreachable!("semantic type clone forall completion order")
                    };
                    completed.push(SemanticType::ForAll {
                        parameters: parameters.to_vec(),
                        body: Box::new(body),
                    });
                }
            }
        }
        match completed.pop() {
            Some(ty) => ty,
            None => unreachable!("semantic type clone omitted its root"),
        }
    }
}

impl Drop for SemanticType {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        take_semantic_children(self, &mut pending);
        while let Some(mut ty) = pending.pop() {
            take_semantic_children(&mut ty, &mut pending);
        }
    }
}

fn take_semantic_children(ty: &mut SemanticType, pending: &mut Vec<SemanticType>) {
    let unit = || SemanticType::Primitive(SemanticPrimitiveKind::Unit);
    match ty {
        SemanticType::Enum { arguments, .. } => pending.append(arguments),
        SemanticType::List(inner) => pending.push(std::mem::replace(inner.as_mut(), unit())),
        SemanticType::Function { parameters, result } => {
            pending.append(parameters);
            pending.push(std::mem::replace(result.as_mut(), unit()));
        }
        SemanticType::ForAll { body, .. } => {
            pending.push(std::mem::replace(body.as_mut(), unit()));
        }
        _ => {}
    }
}

impl PartialEq for SemanticType {
    fn eq(&self, other: &Self) -> bool {
        let mut pending = vec![(self, other)];
        while let Some((left, right)) = pending.pop() {
            match (left, right) {
                (SemanticType::Primitive(left), SemanticType::Primitive(right))
                    if left == right => {}
                (SemanticType::Capability(left), SemanticType::Capability(right))
                    if left == right => {}
                (SemanticType::Resource(left), SemanticType::Resource(right)) if left == right => {}
                (SemanticType::Product(left), SemanticType::Product(right)) if left == right => {}
                (
                    SemanticType::Enum {
                        identity: left_id,
                        arguments: left_arguments,
                    },
                    SemanticType::Enum {
                        identity: right_id,
                        arguments: right_arguments,
                    },
                ) if left_id == right_id && left_arguments.len() == right_arguments.len() => {
                    pending.extend(left_arguments.iter().zip(right_arguments));
                }
                (SemanticType::Parameter(left), SemanticType::Parameter(right))
                    if left == right => {}
                (SemanticType::List(left), SemanticType::List(right)) => {
                    pending.push((left, right))
                }
                (
                    SemanticType::Function {
                        parameters: left_parameters,
                        result: left_result,
                    },
                    SemanticType::Function {
                        parameters: right_parameters,
                        result: right_result,
                    },
                ) if left_parameters.len() == right_parameters.len() => {
                    pending.push((left_result, right_result));
                    pending.extend(left_parameters.iter().zip(right_parameters));
                }
                (
                    SemanticType::ForAll {
                        parameters: left_parameters,
                        body: left_body,
                    },
                    SemanticType::ForAll {
                        parameters: right_parameters,
                        body: right_body,
                    },
                ) if left_parameters == right_parameters => pending.push((left_body, right_body)),
                _ => return false,
            }
        }
        true
    }
}

impl Eq for SemanticType {}

impl std::fmt::Debug for SemanticType {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        enum Work<'a> {
            Type(&'a SemanticType),
            Text(&'static str),
        }

        let mut pending = vec![Work::Type(self)];
        while let Some(item) = pending.pop() {
            match item {
                Work::Text(text) => output.write_str(text)?,
                Work::Type(ty) => match ty {
                    SemanticType::Primitive(kind) => write!(output, "Primitive({kind:?})")?,
                    SemanticType::Capability(kind) => write!(output, "Capability({kind:?})")?,
                    SemanticType::Resource(kind) => write!(output, "Resource({kind:?})")?,
                    SemanticType::Product(identity) => write!(output, "Product({identity:?})")?,
                    SemanticType::Enum {
                        identity,
                        arguments,
                    } => {
                        write!(output, "Enum {{ identity: {identity:?}, arguments: [")?;
                        pending.push(Work::Text("] }"));
                        for (index, argument) in arguments.iter().enumerate().rev() {
                            pending.push(Work::Type(argument));
                            if index != 0 {
                                pending.push(Work::Text(", "));
                            }
                        }
                    }
                    SemanticType::Parameter(name) => write!(output, "Parameter({name:?})")?,
                    SemanticType::List(inner) => {
                        output.write_str("List(")?;
                        pending.push(Work::Text(")"));
                        pending.push(Work::Type(inner));
                    }
                    SemanticType::Function { parameters, result } => {
                        output.write_str("Function { parameters: [")?;
                        pending.push(Work::Text(" }"));
                        pending.push(Work::Type(result));
                        pending.push(Work::Text("], result: "));
                        for (index, parameter) in parameters.iter().enumerate().rev() {
                            pending.push(Work::Type(parameter));
                            if index != 0 {
                                pending.push(Work::Text(", "));
                            }
                        }
                    }
                    SemanticType::ForAll { parameters, body } => {
                        write!(output, "ForAll {{ parameters: {parameters:?}, body: ")?;
                        pending.push(Work::Text(" }"));
                        pending.push(Work::Type(body));
                    }
                },
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticProductField {
    pub identity: [u8; 32],
    pub source_order: u64,
    pub ty: SemanticType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticProductDeclaration {
    pub identity: [u8; 32],
    pub fields: Vec<SemanticProductField>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticEnumVariantField {
    pub identity: [u8; 32],
    pub source_order: u64,
    pub ty: SemanticType,
    pub indirect: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticEnumVariant {
    pub identity: [u8; 32],
    pub source_order: u64,
    pub fields: Vec<SemanticEnumVariantField>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticEnumDeclaration {
    pub identity: [u8; 32],
    pub type_parameters: Vec<String>,
    pub variants: Vec<SemanticEnumVariant>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticDeclaration {
    Product(SemanticProductDeclaration),
    Enum(SemanticEnumDeclaration),
}

impl SemanticDeclaration {
    pub fn identity(&self) -> [u8; 32] {
        match self {
            Self::Product(value) => value.identity,
            Self::Enum(value) => value.identity,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticDescriptor {
    pub root: SemanticType,
    /// Exact reachable closure, sorted by stable declaration identity.
    pub declarations: Vec<SemanticDeclaration>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticContractError(pub &'static str);

impl std::fmt::Display for SemanticContractError {
    fn fmt(&self, output: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        output.write_str(self.0)
    }
}

impl std::error::Error for SemanticContractError {}
