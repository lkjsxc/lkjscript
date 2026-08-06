pub enum MemoryType {
    Never,
    Unit,
    Bool,
    I64,
    F64,
    String,
    Bytes,
    Path,
    Capability(CapabilityKind),
    ByteVector,
    ByteSlice,
    ByteSliceMut,
    Symbol,
    Resource(ResourceKind),
    Product(String),
    Enum {
        id: [u8; 32],
        name: String,
        arguments: Vec<Self>,
    },
    TypeParameter(String),
    List(Box<Self>),
    Function {
        parameters: Vec<Self>,
        result: Box<Self>,
    },
    ForAll {
        variables: Vec<String>,
        body: Box<Self>,
    },
}

impl Clone for MemoryType {
    fn clone(&self) -> Self {
        enum Work<'a> {
            Visit(&'a MemoryType),
            Enum([u8; 32], &'a str, usize),
            List,
            Function(usize),
            ForAll(&'a [String]),
        }

        let mut pending = vec![Work::Visit(self)];
        let mut completed = Vec::new();
        while let Some(item) = pending.pop() {
            match item {
                Work::Visit(ty) => match ty {
                    Self::Never => completed.push(Self::Never),
                    Self::Unit => completed.push(Self::Unit),
                    Self::Bool => completed.push(Self::Bool),
                    Self::I64 => completed.push(Self::I64),
                    Self::F64 => completed.push(Self::F64),
                    Self::String => completed.push(Self::String),
                    Self::Bytes => completed.push(Self::Bytes),
                    Self::Path => completed.push(Self::Path),
                    Self::Capability(kind) => completed.push(Self::Capability(*kind)),
                    Self::ByteVector => completed.push(Self::ByteVector),
                    Self::ByteSlice => completed.push(Self::ByteSlice),
                    Self::ByteSliceMut => completed.push(Self::ByteSliceMut),
                    Self::Symbol => completed.push(Self::Symbol),
                    Self::Resource(kind) => completed.push(Self::Resource(*kind)),
                    Self::Product(name) => completed.push(Self::Product(name.clone())),
                    Self::Enum {
                        id,
                        name,
                        arguments,
                    } => {
                        pending.push(Work::Enum(*id, name, arguments.len()));
                        pending.extend(arguments.iter().rev().map(Work::Visit));
                    }
                    Self::TypeParameter(name) => {
                        completed.push(Self::TypeParameter(name.clone()));
                    }
                    Self::List(inner) => {
                        pending.push(Work::List);
                        pending.push(Work::Visit(inner));
                    }
                    Self::Function { parameters, result } => {
                        pending.push(Work::Function(parameters.len()));
                        pending.push(Work::Visit(result));
                        pending.extend(parameters.iter().rev().map(Work::Visit));
                    }
                    Self::ForAll { variables, body } => {
                        pending.push(Work::ForAll(variables));
                        pending.push(Work::Visit(body));
                    }
                },
                Work::Enum(id, name, count) => {
                    let Some(split) = completed.len().checked_sub(count) else {
                        unreachable!("memory type clone enum completion order")
                    };
                    let arguments = completed.split_off(split);
                    completed.push(Self::Enum {
                        id,
                        name: name.to_owned(),
                        arguments,
                    });
                }
                Work::List => {
                    let Some(inner) = completed.pop() else {
                        unreachable!("memory type clone list completion order")
                    };
                    completed.push(Self::List(Box::new(inner)));
                }
                Work::Function(parameter_count) => {
                    let Some(result) = completed.pop() else {
                        unreachable!("memory type clone function result completion order")
                    };
                    let Some(split) = completed.len().checked_sub(parameter_count) else {
                        unreachable!("memory type clone function parameter completion order")
                    };
                    let parameters = completed.split_off(split);
                    completed.push(Self::Function {
                        parameters,
                        result: Box::new(result),
                    });
                }
                Work::ForAll(variables) => {
                    let Some(body) = completed.pop() else {
                        unreachable!("memory type clone forall completion order")
                    };
                    completed.push(Self::ForAll {
                        variables: variables.to_vec(),
                        body: Box::new(body),
                    });
                }
            }
        }
        match completed.pop() {
            Some(ty) => ty,
            None => unreachable!("memory type clone omitted its root"),
        }
    }
}

impl Drop for MemoryType {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        take_memory_type_children(self, &mut pending);
        while let Some(mut ty) = pending.pop() {
            take_memory_type_children(&mut ty, &mut pending);
        }
    }
}

fn take_memory_type_children(ty: &mut MemoryType, pending: &mut Vec<MemoryType>) {
    match ty {
        MemoryType::Enum { arguments, .. } => pending.append(arguments),
        MemoryType::List(inner) => {
            pending.push(std::mem::replace(inner.as_mut(), MemoryType::Unit));
        }
        MemoryType::Function { parameters, result } => {
            pending.append(parameters);
            pending.push(std::mem::replace(result.as_mut(), MemoryType::Unit));
        }
        MemoryType::ForAll { body, .. } => {
            pending.push(std::mem::replace(body.as_mut(), MemoryType::Unit));
        }
        _ => {}
    }
}

impl PartialEq for MemoryType {
    fn eq(&self, other: &Self) -> bool {
        let mut pending = vec![(self, other)];
        while let Some((left, right)) = pending.pop() {
            match (left, right) {
                (Self::Never, Self::Never)
                | (Self::Unit, Self::Unit)
                | (Self::Bool, Self::Bool)
                | (Self::I64, Self::I64)
                | (Self::F64, Self::F64)
                | (Self::String, Self::String)
                | (Self::Bytes, Self::Bytes)
                | (Self::Path, Self::Path)
                | (Self::ByteVector, Self::ByteVector)
                | (Self::ByteSlice, Self::ByteSlice)
                | (Self::ByteSliceMut, Self::ByteSliceMut)
                | (Self::Symbol, Self::Symbol) => {}
                (Self::Capability(left), Self::Capability(right)) if left == right => {}
                (Self::Resource(left), Self::Resource(right)) if left == right => {}
                (Self::Product(left), Self::Product(right))
                | (Self::TypeParameter(left), Self::TypeParameter(right)) if left == right => {}
                (
                    Self::Enum {
                        id: left_id,
                        name: left_name,
                        arguments: left_arguments,
                    },
                    Self::Enum {
                        id: right_id,
                        name: right_name,
                        arguments: right_arguments,
                    },
                ) if left_id == right_id
                    && left_name == right_name
                    && left_arguments.len() == right_arguments.len() =>
                {
                    pending.extend(left_arguments.iter().zip(right_arguments));
                }
                (Self::List(left), Self::List(right)) => pending.push((left, right)),
                (
                    Self::Function {
                        parameters: left_parameters,
                        result: left_result,
                    },
                    Self::Function {
                        parameters: right_parameters,
                        result: right_result,
                    },
                ) if left_parameters.len() == right_parameters.len() => {
                    pending.push((left_result, right_result));
                    pending.extend(left_parameters.iter().zip(right_parameters));
                }
                (
                    Self::ForAll {
                        variables: left_variables,
                        body: left_body,
                    },
                    Self::ForAll {
                        variables: right_variables,
                        body: right_body,
                    },
                ) if left_variables == right_variables => pending.push((left_body, right_body)),
                _ => return false,
            }
        }
        true
    }
}

impl Eq for MemoryType {}

impl std::fmt::Debug for MemoryType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        crate::stack::grow(|| match self {
            Self::Never => formatter.write_str("Never"),
            Self::Unit => formatter.write_str("Unit"),
            Self::Bool => formatter.write_str("Bool"),
            Self::I64 => formatter.write_str("I64"),
            Self::F64 => formatter.write_str("F64"),
            Self::String => formatter.write_str("String"),
            Self::Bytes => formatter.write_str("Bytes"),
            Self::Path => formatter.write_str("Path"),
            Self::Capability(kind) => formatter.debug_tuple("Capability").field(kind).finish(),
            Self::ByteVector => formatter.write_str("ByteVector"),
            Self::ByteSlice => formatter.write_str("ByteSlice"),
            Self::ByteSliceMut => formatter.write_str("ByteSliceMut"),
            Self::Symbol => formatter.write_str("Symbol"),
            Self::Resource(kind) => formatter.debug_tuple("Resource").field(kind).finish(),
            Self::Product(name) => formatter.debug_tuple("Product").field(name).finish(),
            Self::Enum { id, name, arguments } => formatter
                .debug_struct("Enum")
                .field("id", id)
                .field("name", name)
                .field("arguments", arguments)
                .finish(),
            Self::TypeParameter(name) => {
                formatter.debug_tuple("TypeParameter").field(name).finish()
            }
            Self::List(inner) => formatter.debug_tuple("List").field(inner).finish(),
            Self::Function { parameters, result } => formatter
                .debug_struct("Function")
                .field("parameters", parameters)
                .field("result", result)
                .finish(),
            Self::ForAll { variables, body } => formatter
                .debug_struct("ForAll")
                .field("variables", variables)
                .field("body", body)
                .finish(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryParameterMode {
    Copy,
    BorrowShared,
    BorrowExclusive,
    Consume,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryResultMode {
    Trivial,
    Owned,
    SealedShared,
    External,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryBorrowKind {
    Shared,
    Exclusive,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MemoryBindingStorage {
    Local,
    Function,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MemoryExpressionKind {
    I64Literal(i64),
    F64Literal(u64),
    BoolLiteral(bool),
    UnitLiteral,
    EmptyList,
    StringLiteral,
    BytesLiteral,
    Load {
        binding: u32,
        storage: MemoryBindingStorage,
    },
    Move {
        place: u32,
        binding: u32,
    },
    Borrow {
        place: u32,
        loan: u32,
        kind: MemoryBorrowKind,
        binding: u32,
    },
    DirectCall,
    IndirectCall,
    Operation(u16),
    F64FromI64Exact,
    F64FromI64Rounded,
    I64FromF64Exact,
    I64FromF64Trunc,
    Sequence,
    If,
    While,
    Loop,
    Return,
    Break,
    Continue,
    Trap,
    Exit,
    Let,
    MutableLocal,
    SetLocal,
    ProductValue,
    ProductField,
    WithProductField,
    EnumValue,
    EnumIsVariant,
    EnumField,
    EnumUnwrap,
    MatchUnreachable,
    SymbolLiteral,
}
