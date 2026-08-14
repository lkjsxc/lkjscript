use std::collections::HashMap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use lkjscript_contracts::{CapabilityKind, ResourceKind};

use super::model::{EntityAddress, SnapshotIndexes};
use super::program::SemanticProgram;
use super::{EntityId, EntityKind, WorkspaceError, WorkspaceSnapshot};

/// Stable identity for a compiler-owned prelude enum constructor.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum BuiltinEnum {
    Option,
    Result,
    NumericError,
    Utf8Error,
    SystemError,
}

/// Exact nominal enum constructor identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SemanticEnum {
    Entity(EntityId),
    Builtin(BuiltinEnum),
}

/// Stable identity for a compiler-owned core trait.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum BuiltinTrait {
    Copy,
    Clone,
    Drop,
    Send,
    Sync,
}

/// Exact trait identity at the workspace boundary.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SemanticTrait {
    Entity(EntityId),
    Builtin(BuiltinTrait),
}

/// One exact source-independent semantic type.
///
/// Nominal and binder references are stable workspace identities. Recursive
/// operations are implemented iteratively so valid type depth does not consume
/// unbounded native stack.
#[non_exhaustive]
pub enum SemanticType {
    Never,
    Unit,
    Bool,
    I64,
    F64,
    String,
    Bytes,
    ByteVector,
    ByteSlice,
    ByteSliceMut,
    Path,
    Capability(CapabilityKind),
    Symbol,
    Resource(ResourceKind),
    Product(EntityId),
    Enum {
        constructor: SemanticEnum,
        arguments: Vec<SemanticType>,
    },
    TypeParameter(EntityId),
    List(Box<SemanticType>),
    Function {
        parameters: Vec<SemanticType>,
        result: Box<SemanticType>,
    },
    Forall {
        parameters: Vec<EntityId>,
        body: Box<SemanticType>,
    },
}

impl Clone for SemanticType {
    fn clone(&self) -> Self {
        enum Work<'a> {
            Visit(&'a SemanticType),
            Enum(SemanticEnum, usize),
            List,
            Function(usize),
            Forall(&'a [EntityId]),
        }
        let mut work = vec![Work::Visit(self)];
        let mut completed = Vec::new();
        while let Some(item) = work.pop() {
            match item {
                Work::Visit(ty) => match ty {
                    SemanticType::Never => completed.push(SemanticType::Never),
                    SemanticType::Unit => completed.push(SemanticType::Unit),
                    SemanticType::Bool => completed.push(SemanticType::Bool),
                    SemanticType::I64 => completed.push(SemanticType::I64),
                    SemanticType::F64 => completed.push(SemanticType::F64),
                    SemanticType::String => completed.push(SemanticType::String),
                    SemanticType::Bytes => completed.push(SemanticType::Bytes),
                    SemanticType::ByteVector => completed.push(SemanticType::ByteVector),
                    SemanticType::ByteSlice => completed.push(SemanticType::ByteSlice),
                    SemanticType::ByteSliceMut => completed.push(SemanticType::ByteSliceMut),
                    SemanticType::Path => completed.push(SemanticType::Path),
                    SemanticType::Capability(kind) => {
                        completed.push(SemanticType::Capability(*kind));
                    }
                    SemanticType::Symbol => completed.push(SemanticType::Symbol),
                    SemanticType::Resource(kind) => {
                        completed.push(SemanticType::Resource(*kind));
                    }
                    SemanticType::Product(entity) => {
                        completed.push(SemanticType::Product(*entity));
                    }
                    SemanticType::Enum {
                        constructor,
                        arguments,
                    } => {
                        work.push(Work::Enum(*constructor, arguments.len()));
                        work.extend(arguments.iter().rev().map(Work::Visit));
                    }
                    SemanticType::TypeParameter(entity) => {
                        completed.push(SemanticType::TypeParameter(*entity));
                    }
                    SemanticType::List(inner) => {
                        work.push(Work::List);
                        work.push(Work::Visit(inner));
                    }
                    SemanticType::Function { parameters, result } => {
                        work.push(Work::Function(parameters.len()));
                        work.push(Work::Visit(result));
                        work.extend(parameters.iter().rev().map(Work::Visit));
                    }
                    SemanticType::Forall { parameters, body } => {
                        work.push(Work::Forall(parameters));
                        work.push(Work::Visit(body));
                    }
                },
                Work::Enum(constructor, count) => {
                    let Some(split) = completed.len().checked_sub(count) else {
                        unreachable!("semantic type clone completion order")
                    };
                    let arguments = completed.split_off(split);
                    completed.push(SemanticType::Enum {
                        constructor,
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
                        unreachable!("semantic type clone function completion order")
                    };
                    let Some(split) = completed.len().checked_sub(count) else {
                        unreachable!("semantic type clone parameter completion order")
                    };
                    let parameters = completed.split_off(split);
                    completed.push(SemanticType::Function {
                        parameters,
                        result: Box::new(result),
                    });
                }
                Work::Forall(parameters) => {
                    let Some(body) = completed.pop() else {
                        unreachable!("semantic type clone forall completion order")
                    };
                    completed.push(SemanticType::Forall {
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
        take_children(self, &mut pending);
        while let Some(mut ty) = pending.pop() {
            take_children(&mut ty, &mut pending);
        }
    }
}

fn take_children(ty: &mut SemanticType, pending: &mut Vec<SemanticType>) {
    match ty {
        SemanticType::Enum { arguments, .. } => pending.append(arguments),
        SemanticType::List(inner) => {
            pending.push(std::mem::replace(inner.as_mut(), SemanticType::Unit));
        }
        SemanticType::Function { parameters, result } => {
            pending.append(parameters);
            pending.push(std::mem::replace(result.as_mut(), SemanticType::Unit));
        }
        SemanticType::Forall { body, .. } => {
            pending.push(std::mem::replace(body.as_mut(), SemanticType::Unit));
        }
        _ => {}
    }
}

impl PartialEq for SemanticType {
    fn eq(&self, other: &Self) -> bool {
        let mut pending = vec![(self, other)];
        while let Some((left, right)) = pending.pop() {
            match (left, right) {
                (SemanticType::Never, SemanticType::Never)
                | (SemanticType::Unit, SemanticType::Unit)
                | (SemanticType::Bool, SemanticType::Bool)
                | (SemanticType::I64, SemanticType::I64)
                | (SemanticType::F64, SemanticType::F64)
                | (SemanticType::String, SemanticType::String)
                | (SemanticType::Bytes, SemanticType::Bytes)
                | (SemanticType::ByteVector, SemanticType::ByteVector)
                | (SemanticType::ByteSlice, SemanticType::ByteSlice)
                | (SemanticType::ByteSliceMut, SemanticType::ByteSliceMut)
                | (SemanticType::Path, SemanticType::Path)
                | (SemanticType::Symbol, SemanticType::Symbol) => {}
                (SemanticType::Capability(left), SemanticType::Capability(right))
                    if left == right => {}
                (SemanticType::Resource(left), SemanticType::Resource(right)) if left == right => {}
                (SemanticType::Product(left), SemanticType::Product(right)) if left == right => {}
                (SemanticType::TypeParameter(left), SemanticType::TypeParameter(right))
                    if left == right => {}
                (
                    SemanticType::Enum {
                        constructor: left_constructor,
                        arguments: left_arguments,
                    },
                    SemanticType::Enum {
                        constructor: right_constructor,
                        arguments: right_arguments,
                    },
                ) if left_constructor == right_constructor
                    && left_arguments.len() == right_arguments.len() =>
                {
                    pending.extend(left_arguments.iter().zip(right_arguments));
                }
                (SemanticType::List(left), SemanticType::List(right)) => {
                    pending.push((left, right));
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
                    SemanticType::Forall {
                        parameters: left_parameters,
                        body: left_body,
                    },
                    SemanticType::Forall {
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

impl Hash for SemanticType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut pending = vec![self];
        while let Some(ty) = pending.pop() {
            semantic_type_tag(ty).hash(state);
            match ty {
                SemanticType::Capability(kind) => kind.hash(state),
                SemanticType::Resource(kind) => kind.hash(state),
                SemanticType::Product(entity) | SemanticType::TypeParameter(entity) => {
                    entity.hash(state);
                }
                SemanticType::Enum {
                    constructor,
                    arguments,
                } => {
                    constructor.hash(state);
                    arguments.len().hash(state);
                    pending.extend(arguments.iter().rev());
                }
                SemanticType::List(inner) => pending.push(inner),
                SemanticType::Function { parameters, result } => {
                    parameters.len().hash(state);
                    pending.push(result);
                    pending.extend(parameters.iter().rev());
                }
                SemanticType::Forall { parameters, body } => {
                    parameters.hash(state);
                    pending.push(body);
                }
                _ => {}
            }
        }
    }
}

const fn semantic_type_tag(ty: &SemanticType) -> u8 {
    match ty {
        SemanticType::Never => 0,
        SemanticType::Unit => 1,
        SemanticType::Bool => 2,
        SemanticType::I64 => 3,
        SemanticType::F64 => 4,
        SemanticType::String => 5,
        SemanticType::Bytes => 6,
        SemanticType::ByteVector => 7,
        SemanticType::ByteSlice => 8,
        SemanticType::ByteSliceMut => 9,
        SemanticType::Path => 10,
        SemanticType::Capability(_) => 11,
        SemanticType::Symbol => 12,
        SemanticType::Resource(_) => 13,
        SemanticType::Product(_) => 14,
        SemanticType::Enum { .. } => 15,
        SemanticType::TypeParameter(_) => 16,
        SemanticType::List(_) => 17,
        SemanticType::Function { .. } => 18,
        SemanticType::Forall { .. } => 19,
    }
}

impl fmt::Display for SemanticType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        enum Work<'a> {
            Type(&'a SemanticType),
            Text(&'static str),
            Entity(EntityId),
        }
        let mut pending = vec![Work::Type(self)];
        while let Some(item) = pending.pop() {
            match item {
                Work::Text(text) => formatter.write_str(text)?,
                Work::Entity(entity) => {
                    write!(formatter, "{}:{}", entity.slot(), entity.generation())?
                }
                Work::Type(ty) => match ty {
                    SemanticType::Never => formatter.write_str("never")?,
                    SemanticType::Unit => formatter.write_str("unit")?,
                    SemanticType::Bool => formatter.write_str("bool")?,
                    SemanticType::I64 => formatter.write_str("i64")?,
                    SemanticType::F64 => formatter.write_str("f64")?,
                    SemanticType::String => formatter.write_str("string")?,
                    SemanticType::Bytes => formatter.write_str("bytes")?,
                    SemanticType::ByteVector => formatter.write_str("byte-vector")?,
                    SemanticType::ByteSlice => formatter.write_str("byte-slice")?,
                    SemanticType::ByteSliceMut => formatter.write_str("byte-slice-mut")?,
                    SemanticType::Path => formatter.write_str("path")?,
                    SemanticType::Capability(kind) => {
                        write!(formatter, "capability {}", kind.as_str())?;
                    }
                    SemanticType::Symbol => formatter.write_str("symbol")?,
                    SemanticType::Resource(kind) => formatter.write_str(kind.as_str())?,
                    SemanticType::Product(entity) => {
                        formatter.write_str("product(")?;
                        pending.push(Work::Text(")"));
                        pending.push(Work::Entity(*entity));
                    }
                    SemanticType::Enum {
                        constructor,
                        arguments,
                    } => {
                        match constructor {
                            SemanticEnum::Entity(entity) => {
                                formatter.write_str("enum(")?;
                                write!(formatter, "{}:{}", entity.slot(), entity.generation())?;
                                formatter.write_str(")")?;
                            }
                            SemanticEnum::Builtin(kind) => formatter.write_str(match kind {
                                BuiltinEnum::Option => "option",
                                BuiltinEnum::Result => "result",
                                BuiltinEnum::NumericError => "numeric-error",
                                BuiltinEnum::Utf8Error => "utf8-error",
                                BuiltinEnum::SystemError => "system-error",
                            })?,
                        }
                        for argument in arguments.iter().rev() {
                            pending.push(Work::Type(argument));
                            pending.push(Work::Text(" "));
                        }
                    }
                    SemanticType::TypeParameter(entity) => {
                        formatter.write_str("type-parameter(")?;
                        pending.push(Work::Text(")"));
                        pending.push(Work::Entity(*entity));
                    }
                    SemanticType::List(inner) => {
                        formatter.write_str("list ")?;
                        pending.push(Work::Type(inner));
                    }
                    SemanticType::Function { parameters, result } => {
                        formatter.write_str("fn inputs")?;
                        pending.push(Work::Type(result));
                        pending.push(Work::Text(" output "));
                        for parameter in parameters.iter().rev() {
                            pending.push(Work::Type(parameter));
                            pending.push(Work::Text(" "));
                        }
                    }
                    SemanticType::Forall { parameters, body } => {
                        formatter.write_str("forall")?;
                        pending.push(Work::Type(body));
                        pending.push(Work::Text(" "));
                        for parameter in parameters.iter().rev() {
                            pending.push(Work::Entity(*parameter));
                            pending.push(Work::Text(" "));
                        }
                    }
                },
            }
        }
        Ok(())
    }
}

impl fmt::Debug for SemanticType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

pub(super) fn view(
    program: &SemanticProgram,
    indexes: &SnapshotIndexes,
    ty: &crate::Type,
    context: Option<EntityId>,
) -> Result<SemanticType, WorkspaceError> {
    enum Work<'a> {
        Visit(&'a crate::Type),
        Enum(SemanticEnum, usize),
        List,
        Function(usize),
        Forall(Vec<EntityId>),
    }
    let node_count = internal_type_node_count(ty)?;
    let mut work = Vec::new();
    reserve(&mut work, 1, "semantic type conversion work")?;
    work.push(Work::Visit(ty));
    let mut completed = Vec::new();
    reserve(
        &mut completed,
        node_count,
        "semantic type conversion results",
    )?;
    while let Some(item) = work.pop() {
        match item {
            Work::Visit(ty) => match ty {
                crate::Type::Never => completed.push(SemanticType::Never),
                crate::Type::Unit => completed.push(SemanticType::Unit),
                crate::Type::Bool => completed.push(SemanticType::Bool),
                crate::Type::I64 => completed.push(SemanticType::I64),
                crate::Type::F64 => completed.push(SemanticType::F64),
                crate::Type::Str => completed.push(SemanticType::String),
                crate::Type::Bytes => completed.push(SemanticType::Bytes),
                crate::Type::ByteVector => completed.push(SemanticType::ByteVector),
                crate::Type::ByteSlice => completed.push(SemanticType::ByteSlice),
                crate::Type::ByteSliceMut => completed.push(SemanticType::ByteSliceMut),
                crate::Type::Path => completed.push(SemanticType::Path),
                crate::Type::Capability(kind) => completed.push(SemanticType::Capability(*kind)),
                crate::Type::Symbol => completed.push(SemanticType::Symbol),
                crate::Type::Resource(kind) => completed.push(SemanticType::Resource(*kind)),
                crate::Type::Product(id) => {
                    let definition = id
                        .index()
                        .and_then(|index| program.products.get(index))
                        .filter(|definition| definition.id == *id)
                        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("product type")))?;
                    let entity = indexes
                        .address_entities
                        .get(&EntityAddress::Product(definition.id.raw()))
                        .copied()
                        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("product type")))?;
                    completed.push(SemanticType::Product(entity));
                }
                crate::Type::Enum { id, arguments, .. } => {
                    let constructor = builtin_enum(id.bytes()).map_or_else(
                        || {
                            indexes
                                .enum_identity_indices
                                .get(id)
                                .copied()
                                .ok_or_else(|| {
                                    WorkspaceError::StaleIdentity(Arc::from("enum type"))
                                })
                                .and_then(|index| {
                                    u64::try_from(index).map_err(|_| {
                                        WorkspaceError::Host(Arc::from(
                                            "enum type index exceeds u64",
                                        ))
                                    })
                                })
                                .and_then(|raw| {
                                    indexes
                                        .address_entities
                                        .get(&EntityAddress::Enum(raw))
                                        .copied()
                                        .map(SemanticEnum::Entity)
                                        .ok_or_else(|| {
                                            WorkspaceError::StaleIdentity(Arc::from("enum type"))
                                        })
                                })
                        },
                        |builtin| Ok(SemanticEnum::Builtin(builtin)),
                    )?;
                    let additional = arguments.len().checked_add(1).ok_or_else(|| {
                        WorkspaceError::Host(Arc::from("semantic enum child count overflow"))
                    })?;
                    reserve(&mut work, additional, "semantic enum conversion work")?;
                    work.push(Work::Enum(constructor, arguments.len()));
                    work.extend(arguments.iter().rev().map(Work::Visit));
                }
                crate::Type::Param(name) => {
                    let owner = context.ok_or_else(|| {
                        WorkspaceError::Validation(Arc::from(
                            "type parameter has no semantic declaration context",
                        ))
                    })?;
                    let entity = indexes
                        .type_parameter_entities
                        .get(&owner)
                        .and_then(|parameters| parameters.get(name.as_str()))
                        .copied()
                        .ok_or_else(|| {
                            WorkspaceError::StaleIdentity(Arc::from("type parameter"))
                        })?;
                    completed.push(SemanticType::TypeParameter(entity));
                }
                crate::Type::List(inner) => {
                    reserve(&mut work, 2, "semantic list conversion work")?;
                    work.push(Work::List);
                    work.push(Work::Visit(inner));
                }
                crate::Type::Fn { params, ret } => {
                    let additional = params.len().checked_add(2).ok_or_else(|| {
                        WorkspaceError::Host(Arc::from("semantic function type size overflow"))
                    })?;
                    reserve(&mut work, additional, "semantic function conversion work")?;
                    work.push(Work::Function(params.len()));
                    work.push(Work::Visit(ret));
                    work.extend(params.iter().rev().map(Work::Visit));
                }
                crate::Type::Forall { vars, body } => {
                    let owner = context.ok_or_else(|| {
                        WorkspaceError::Validation(Arc::from(
                            "universal type has no semantic declaration context",
                        ))
                    })?;
                    let mut parameters = Vec::new();
                    reserve(
                        &mut parameters,
                        vars.len(),
                        "semantic forall parameter conversion",
                    )?;
                    for name in vars {
                        parameters.push(
                            indexes
                                .type_parameter_entities
                                .get(&owner)
                                .and_then(|parameters| parameters.get(name.as_str()))
                                .copied()
                                .ok_or_else(|| {
                                    WorkspaceError::StaleIdentity(Arc::from("type parameter"))
                                })?,
                        );
                    }
                    reserve(&mut work, 2, "semantic forall conversion work")?;
                    work.push(Work::Forall(parameters));
                    work.push(Work::Visit(body));
                }
            },
            Work::Enum(constructor, count) => {
                let split = completed.len().checked_sub(count).ok_or_else(|| {
                    WorkspaceError::Validation(Arc::from(
                        "semantic enum conversion order is invalid",
                    ))
                })?;
                let arguments = completed.split_off(split);
                completed.push(SemanticType::Enum {
                    constructor,
                    arguments,
                });
            }
            Work::List => {
                let inner = completed.pop().ok_or_else(|| {
                    WorkspaceError::Validation(Arc::from(
                        "semantic list conversion omitted its child",
                    ))
                })?;
                completed.push(SemanticType::List(Box::new(inner)));
            }
            Work::Function(count) => {
                let result = completed.pop().ok_or_else(|| {
                    WorkspaceError::Validation(Arc::from(
                        "semantic function conversion omitted its result",
                    ))
                })?;
                let split = completed.len().checked_sub(count).ok_or_else(|| {
                    WorkspaceError::Validation(Arc::from(
                        "semantic function conversion omitted parameters",
                    ))
                })?;
                let parameters = completed.split_off(split);
                completed.push(SemanticType::Function {
                    parameters,
                    result: Box::new(result),
                });
            }
            Work::Forall(parameters) => {
                let body = completed.pop().ok_or_else(|| {
                    WorkspaceError::Validation(Arc::from(
                        "semantic forall conversion omitted its body",
                    ))
                })?;
                completed.push(SemanticType::Forall {
                    parameters,
                    body: Box::new(body),
                });
            }
        }
    }
    let result = completed.pop().ok_or_else(|| {
        WorkspaceError::Validation(Arc::from("semantic type conversion omitted its root"))
    })?;
    if completed.is_empty() {
        Ok(result)
    } else {
        Err(WorkspaceError::Validation(Arc::from(
            "semantic type conversion left disconnected results",
        )))
    }
}

#[derive(Clone, Copy)]
pub(super) struct StagedEnumType {
    entity: EntityId,
    id: crate::hir::EnumId,
    arity: usize,
}

impl StagedEnumType {
    pub(super) const fn new(entity: EntityId, id: crate::hir::EnumId, arity: usize) -> Self {
        Self { entity, id, arity }
    }
}

pub(super) fn resolve(
    snapshot: &WorkspaceSnapshot,
    program: &SemanticProgram,
    ty: &SemanticType,
    context: Option<EntityId>,
    allow_never: bool,
    allow_forall: bool,
    subject: &str,
) -> Result<crate::Type, WorkspaceError> {
    resolve_with_staged_type_parameters(
        snapshot,
        program,
        ty,
        context,
        &HashMap::new(),
        None,
        allow_never,
        allow_forall,
        subject,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn resolve_with_staged_type_parameters(
    snapshot: &WorkspaceSnapshot,
    program: &SemanticProgram,
    ty: &SemanticType,
    context: Option<EntityId>,
    staged_type_parameters: &HashMap<EntityId, String>,
    staged_enum: Option<StagedEnumType>,
    allow_never: bool,
    allow_forall: bool,
    subject: &str,
) -> Result<crate::Type, WorkspaceError> {
    enum Work<'a> {
        Visit(&'a SemanticType),
        Enum(crate::hir::EnumId, usize),
        List,
        Function(usize),
        Forall(Vec<String>),
    }
    let node_count = semantic_type_node_count(ty)?;
    let mut work = Vec::new();
    reserve(&mut work, 1, "semantic type validation work")?;
    work.push(Work::Visit(ty));
    let mut completed = Vec::new();
    reserve(
        &mut completed,
        node_count,
        "semantic type validation results",
    )?;
    while let Some(item) = work.pop() {
        match item {
            Work::Visit(ty) => match ty {
                SemanticType::Never if !allow_never => {
                    return Err(WorkspaceError::InvalidSemanticType {
                        position: Arc::from(subject.to_owned()),
                        reason: Arc::from("never is not valid in this type position"),
                    });
                }
                SemanticType::Forall { .. } if !allow_forall => {
                    return Err(WorkspaceError::InvalidSemanticType {
                        position: Arc::from(subject.to_owned()),
                        reason: Arc::from("universal types are not valid in this type position"),
                    });
                }
                SemanticType::Never => completed.push(crate::Type::Never),
                SemanticType::Unit => completed.push(crate::Type::Unit),
                SemanticType::Bool => completed.push(crate::Type::Bool),
                SemanticType::I64 => completed.push(crate::Type::I64),
                SemanticType::F64 => completed.push(crate::Type::F64),
                SemanticType::String => completed.push(crate::Type::Str),
                SemanticType::Bytes => completed.push(crate::Type::Bytes),
                SemanticType::ByteVector => completed.push(crate::Type::ByteVector),
                SemanticType::ByteSlice => completed.push(crate::Type::ByteSlice),
                SemanticType::ByteSliceMut => completed.push(crate::Type::ByteSliceMut),
                SemanticType::Path => completed.push(crate::Type::Path),
                SemanticType::Capability(kind) => completed.push(crate::Type::Capability(*kind)),
                SemanticType::Symbol => completed.push(crate::Type::Symbol),
                SemanticType::Resource(kind) => completed.push(crate::Type::Resource(*kind)),
                SemanticType::Product(entity) => {
                    let header = semantic_entity(snapshot, *entity, "product type")?;
                    if header.kind != EntityKind::Product {
                        return Err(wrong_kind(subject, "product declaration", header.kind));
                    }
                    let EntityAddress::Product(raw) = entity_address(snapshot, *entity)? else {
                        return Err(WorkspaceError::StaleIdentity(Arc::from("product type")));
                    };
                    let definition = program
                        .products
                        .get(host_index(raw, "product type")?)
                        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("product type")))?;
                    if definition.id.raw() != raw {
                        return Err(WorkspaceError::StaleIdentity(Arc::from("product type")));
                    }
                    completed.push(crate::Type::Product(definition.id));
                }
                SemanticType::Enum {
                    constructor,
                    arguments,
                } => {
                    let (id, arity) = match constructor {
                        SemanticEnum::Builtin(kind) => builtin_enum_facts(*kind),
                        SemanticEnum::Entity(entity) => {
                            if let Some(staged) =
                                staged_enum.filter(|staged| staged.entity == *entity)
                            {
                                (staged.id, staged.arity)
                            } else {
                                let header = semantic_entity(snapshot, *entity, "enum type")?;
                                if header.kind != EntityKind::Enum {
                                    return Err(wrong_kind(
                                        subject,
                                        "enum declaration",
                                        header.kind,
                                    ));
                                }
                                let EntityAddress::Enum(raw) = entity_address(snapshot, *entity)?
                                else {
                                    return Err(WorkspaceError::StaleIdentity(Arc::from(
                                        "enum type",
                                    )));
                                };
                                let definition = program
                                    .enums
                                    .get(host_index(raw, "enum type")?)
                                    .ok_or_else(|| {
                                    WorkspaceError::StaleIdentity(Arc::from("enum type"))
                                })?;
                                (definition.id, definition.type_parameters.len())
                            }
                        }
                    };
                    if arguments.len() != arity {
                        return Err(WorkspaceError::InvalidSemanticType {
                            position: Arc::from(subject.to_owned()),
                            reason: Arc::from(format!(
                                "enum type requires {arity} argument(s), got {}",
                                arguments.len()
                            )),
                        });
                    }
                    let additional = arguments.len().checked_add(1).ok_or_else(|| {
                        WorkspaceError::Host(Arc::from("semantic enum child count overflow"))
                    })?;
                    reserve(&mut work, additional, "semantic enum validation work")?;
                    work.push(Work::Enum(id, arguments.len()));
                    work.extend(arguments.iter().rev().map(Work::Visit));
                }
                SemanticType::TypeParameter(entity) => {
                    if let Some(name) = staged_type_parameters.get(entity) {
                        completed.push(crate::Type::Param(clone_type_name(
                            name,
                            "staged type-parameter name",
                        )?));
                        continue;
                    }
                    let header = semantic_entity(snapshot, *entity, "type parameter")?;
                    if header.kind != EntityKind::TypeParameter {
                        return Err(wrong_kind(subject, "type parameter", header.kind));
                    }
                    let expected_owner = context
                        .ok_or(WorkspaceError::InvisibleTypeParameter { parameter: *entity })?;
                    if header.owner != Some(expected_owner) {
                        return Err(WorkspaceError::WrongTypeParameterOwner {
                            parameter: Box::new(*entity),
                            expected: Box::new(expected_owner),
                            actual: header.owner.map(Box::new),
                        });
                    }
                    completed.push(crate::Type::Param(clone_type_name(
                        &header.name,
                        "type-parameter name",
                    )?));
                }
                SemanticType::List(inner) => {
                    reserve(&mut work, 2, "semantic list validation work")?;
                    work.push(Work::List);
                    work.push(Work::Visit(inner));
                }
                SemanticType::Function { parameters, result } => {
                    let additional = parameters.len().checked_add(2).ok_or_else(|| {
                        WorkspaceError::Host(Arc::from("semantic function type size overflow"))
                    })?;
                    reserve(&mut work, additional, "semantic function validation work")?;
                    work.push(Work::Function(parameters.len()));
                    work.push(Work::Visit(result));
                    work.extend(parameters.iter().rev().map(Work::Visit));
                }
                SemanticType::Forall { parameters, body } => {
                    let expected_owner =
                        context.ok_or_else(|| WorkspaceError::InvalidSemanticType {
                            position: Arc::from(subject.to_owned()),
                            reason: Arc::from("universal type has no declaration owner"),
                        })?;
                    let mut names = Vec::new();
                    reserve(&mut names, parameters.len(), "semantic forall validation")?;
                    for parameter in parameters {
                        let header = semantic_entity(snapshot, *parameter, "type parameter")?;
                        if header.kind != EntityKind::TypeParameter {
                            return Err(wrong_kind(subject, "type parameter", header.kind));
                        }
                        if header.owner != Some(expected_owner) {
                            return Err(WorkspaceError::WrongTypeParameterOwner {
                                parameter: Box::new(*parameter),
                                expected: Box::new(expected_owner),
                                actual: header.owner.map(Box::new),
                            });
                        }
                        names.push(clone_type_name(
                            &header.name,
                            "universal type-parameter name",
                        )?);
                    }
                    reserve(&mut work, 2, "semantic forall validation work")?;
                    work.push(Work::Forall(names));
                    work.push(Work::Visit(body));
                }
            },
            Work::Enum(id, count) => {
                let split = completed.len().checked_sub(count).ok_or_else(|| {
                    WorkspaceError::InvalidSemanticType {
                        position: Arc::from(subject.to_owned()),
                        reason: Arc::from("enum type children are incomplete"),
                    }
                })?;
                let arguments = completed.split_off(split);
                completed.push(crate::Type::Enum { id, arguments });
            }
            Work::List => {
                let inner = completed
                    .pop()
                    .ok_or_else(|| WorkspaceError::InvalidSemanticType {
                        position: Arc::from(subject.to_owned()),
                        reason: Arc::from("list type child is incomplete"),
                    })?;
                completed.push(crate::Type::List(Box::new(inner)));
            }
            Work::Function(count) => {
                let result =
                    completed
                        .pop()
                        .ok_or_else(|| WorkspaceError::InvalidSemanticType {
                            position: Arc::from(subject.to_owned()),
                            reason: Arc::from("function result type is incomplete"),
                        })?;
                let split = completed.len().checked_sub(count).ok_or_else(|| {
                    WorkspaceError::InvalidSemanticType {
                        position: Arc::from(subject.to_owned()),
                        reason: Arc::from("function parameter types are incomplete"),
                    }
                })?;
                let params = completed.split_off(split);
                completed.push(crate::Type::Fn {
                    params,
                    ret: Box::new(result),
                });
            }
            Work::Forall(vars) => {
                let body = completed
                    .pop()
                    .ok_or_else(|| WorkspaceError::InvalidSemanticType {
                        position: Arc::from(subject.to_owned()),
                        reason: Arc::from("universal type body is incomplete"),
                    })?;
                completed.push(crate::Type::Forall {
                    vars,
                    body: Box::new(body),
                });
            }
        }
    }
    let result = completed
        .pop()
        .ok_or_else(|| WorkspaceError::InvalidSemanticType {
            position: Arc::from(subject.to_owned()),
            reason: Arc::from("semantic type omitted its root"),
        })?;
    if completed.is_empty() {
        Ok(result)
    } else {
        Err(WorkspaceError::InvalidSemanticType {
            position: Arc::from(subject.to_owned()),
            reason: Arc::from("semantic type contains disconnected nodes"),
        })
    }
}

fn internal_type_node_count(root: &crate::Type) -> Result<usize, WorkspaceError> {
    let mut pending = Vec::new();
    reserve(&mut pending, 1, "internal type counting work")?;
    pending.push(root);
    let mut count = 0_usize;
    while let Some(ty) = pending.pop() {
        count = count
            .checked_add(1)
            .ok_or_else(|| WorkspaceError::Host(Arc::from("semantic type node count overflow")))?;
        match ty {
            crate::Type::Enum { arguments, .. } => {
                reserve(&mut pending, arguments.len(), "internal type counting work")?;
                pending.extend(arguments);
            }
            crate::Type::List(inner) => {
                reserve(&mut pending, 1, "internal type counting work")?;
                pending.push(inner);
            }
            crate::Type::Fn { params, ret } => {
                let additional = params.len().checked_add(1).ok_or_else(|| {
                    WorkspaceError::Host(Arc::from("semantic type child count overflow"))
                })?;
                reserve(&mut pending, additional, "internal type counting work")?;
                pending.push(ret);
                pending.extend(params);
            }
            crate::Type::Forall { body, .. } => {
                reserve(&mut pending, 1, "internal type counting work")?;
                pending.push(body);
            }
            _ => {}
        }
    }
    Ok(count)
}

fn semantic_type_node_count(root: &SemanticType) -> Result<usize, WorkspaceError> {
    let mut pending = Vec::new();
    reserve(&mut pending, 1, "semantic type counting work")?;
    pending.push(root);
    let mut count = 0_usize;
    while let Some(ty) = pending.pop() {
        count = count
            .checked_add(1)
            .ok_or_else(|| WorkspaceError::Host(Arc::from("semantic type node count overflow")))?;
        match ty {
            SemanticType::Enum { arguments, .. } => {
                reserve(&mut pending, arguments.len(), "semantic type counting work")?;
                pending.extend(arguments);
            }
            SemanticType::List(inner) => {
                reserve(&mut pending, 1, "semantic type counting work")?;
                pending.push(inner);
            }
            SemanticType::Function { parameters, result } => {
                let additional = parameters.len().checked_add(1).ok_or_else(|| {
                    WorkspaceError::Host(Arc::from("semantic type child count overflow"))
                })?;
                reserve(&mut pending, additional, "semantic type counting work")?;
                pending.push(result);
                pending.extend(parameters);
            }
            SemanticType::Forall { body, .. } => {
                reserve(&mut pending, 1, "semantic type counting work")?;
                pending.push(body);
            }
            _ => {}
        }
    }
    Ok(count)
}

pub(super) fn semantic_trait(
    program: &SemanticProgram,
    indexes: &SnapshotIndexes,
    id: crate::hir::TraitId,
) -> Result<SemanticTrait, WorkspaceError> {
    let definition = program
        .traits
        .get(host_index(id.raw(), "trait")?)
        .filter(|definition| definition.id == id)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("trait")))?;
    if let Some(core) = definition.core {
        return Ok(SemanticTrait::Builtin(match core {
            crate::hir::CoreTrait::Copy => BuiltinTrait::Copy,
            crate::hir::CoreTrait::Clone => BuiltinTrait::Clone,
            crate::hir::CoreTrait::Drop => BuiltinTrait::Drop,
            crate::hir::CoreTrait::Send => BuiltinTrait::Send,
            crate::hir::CoreTrait::Sync => BuiltinTrait::Sync,
        }));
    }
    indexes
        .address_entities
        .get(&EntityAddress::Trait(id.raw()))
        .copied()
        .map(SemanticTrait::Entity)
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("trait")))
}

fn builtin_enum(bytes: [u8; 32]) -> Option<BuiltinEnum> {
    match bytes {
        lkjscript_core::OPTION_ID => Some(BuiltinEnum::Option),
        lkjscript_core::RESULT_ID => Some(BuiltinEnum::Result),
        lkjscript_core::NUMERIC_ERROR_ID => Some(BuiltinEnum::NumericError),
        lkjscript_core::UTF8_ERROR_ID => Some(BuiltinEnum::Utf8Error),
        lkjscript_core::SYSTEM_ERROR_ID => Some(BuiltinEnum::SystemError),
        _ => None,
    }
}

fn builtin_enum_facts(kind: BuiltinEnum) -> (crate::hir::EnumId, usize) {
    match kind {
        BuiltinEnum::Option => (crate::hir::EnumId::new(lkjscript_core::OPTION_ID), 1),
        BuiltinEnum::Result => (crate::hir::EnumId::new(lkjscript_core::RESULT_ID), 2),
        BuiltinEnum::NumericError => (crate::hir::EnumId::new(lkjscript_core::NUMERIC_ERROR_ID), 0),
        BuiltinEnum::Utf8Error => (crate::hir::EnumId::new(lkjscript_core::UTF8_ERROR_ID), 0),
        BuiltinEnum::SystemError => (crate::hir::EnumId::new(lkjscript_core::SYSTEM_ERROR_ID), 0),
    }
}

fn semantic_entity<'a>(
    snapshot: &'a WorkspaceSnapshot,
    entity: EntityId,
    subject: &str,
) -> Result<&'a super::EntityHeader, WorkspaceError> {
    if entity.namespace() != snapshot.namespace() {
        return Err(WorkspaceError::ForeignNamespace(Arc::from(
            subject.to_owned(),
        )));
    }
    snapshot
        .workspace_entity(entity)
        .map_err(|_| WorkspaceError::StaleIdentity(Arc::from(subject.to_owned())))
}

fn entity_address(
    snapshot: &WorkspaceSnapshot,
    entity: EntityId,
) -> Result<EntityAddress, WorkspaceError> {
    snapshot
        .indexes
        .entity_lookup
        .get(&entity)
        .and_then(|index| snapshot.indexes.entity_addresses.get(*index))
        .copied()
        .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("entity")))
}

fn host_index(raw: u64, subject: &str) -> Result<usize, WorkspaceError> {
    usize::try_from(raw).map_err(|_| WorkspaceError::StaleIdentity(Arc::from(subject.to_owned())))
}

fn wrong_kind(operation: &str, expected: &str, actual: EntityKind) -> WorkspaceError {
    WorkspaceError::WrongEntityKind {
        operation: Arc::from(operation.to_owned()),
        expected: Arc::from(expected.to_owned()),
        actual: super::error::SemanticKind::Entity(actual),
    }
}

fn clone_type_name(value: &str, subject: &str) -> Result<String, WorkspaceError> {
    let mut name = String::new();
    name.try_reserve(value.len())
        .map_err(|_| WorkspaceError::Host(Arc::from(format!("{subject} allocation failed"))))?;
    name.push_str(value);
    Ok(name)
}

fn reserve<T>(values: &mut Vec<T>, additional: usize, subject: &str) -> Result<(), WorkspaceError> {
    values
        .try_reserve(additional)
        .map_err(|_| WorkspaceError::Host(Arc::from(format!("{subject} allocation failed"))))
}
