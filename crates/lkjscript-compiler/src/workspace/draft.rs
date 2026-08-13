use std::fmt;
use std::sync::Arc;

use lkjscript_contracts::{CapabilityKind, ResourceKind};

use super::{EntityId, SemanticEnum, SemanticType, WorkspaceError};
use crate::operation::Operation;

/// Dense identity into one flat expression draft. It is never a workspace identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DraftNodeId(u64);

impl DraftNodeId {
    pub const fn new(index: u64) -> Self {
        Self(index)
    }

    pub(super) fn index(self) -> Option<usize> {
        usize::try_from(self.0).ok()
    }
}

/// Dense identity into one flat match-pattern draft. It is never a workspace identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DraftPatternNodeId(u64);

impl DraftPatternNodeId {
    pub const fn new(index: u64) -> Self {
        Self(index)
    }

    pub(super) fn index(self) -> Option<usize> {
        usize::try_from(self.0).ok()
    }
}

/// Identity of one local binding inside a single [`ExpressionDraft`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DraftBindingId(u64);

impl DraftBindingId {
    pub const fn new(index: u64) -> Self {
        Self(index)
    }

    pub(super) const fn raw(self) -> u64 {
        self.0
    }
}

/// A binding reference whose identity domain is explicit.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DraftBindingRef {
    /// A stable parameter or local already published in this workspace.
    Entity(EntityId),
    /// A transaction-local binding in this draft.
    Local(DraftBindingId),
}

/// Identity of one type parameter inside a single function declaration draft.
///
/// This invocation-local handle is not a workspace identity and never enters a
/// published snapshot, query, projection, or semantic diff.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DraftTypeParameterId(u64);

impl DraftTypeParameterId {
    pub const fn new(index: u64) -> Self {
        Self(index)
    }

    pub(super) const fn raw(self) -> u64 {
        self.0
    }
}

/// A creation-time type for one function declaration.
///
/// Published type parameters use stable [`EntityId`] values. A type parameter
/// declared by the same creation edit instead uses [`DraftTypeParameterId`]
/// until staging allocates its stable entity. Recursive operations are
/// iterative so declaration depth does not consume unbounded native stack.
#[non_exhaustive]
pub enum DeclarationType {
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
        arguments: Vec<DeclarationType>,
    },
    TypeParameter(EntityId),
    DraftTypeParameter(DraftTypeParameterId),
    List(Box<DeclarationType>),
    Function {
        parameters: Vec<DeclarationType>,
        result: Box<DeclarationType>,
    },
}

impl Clone for DeclarationType {
    fn clone(&self) -> Self {
        enum Work<'a> {
            Visit(&'a DeclarationType),
            Enum(SemanticEnum, usize),
            List,
            Function(usize),
        }
        let mut work = vec![Work::Visit(self)];
        let mut completed = Vec::new();
        while let Some(item) = work.pop() {
            match item {
                Work::Visit(ty) => match ty {
                    DeclarationType::Never => completed.push(DeclarationType::Never),
                    DeclarationType::Unit => completed.push(DeclarationType::Unit),
                    DeclarationType::Bool => completed.push(DeclarationType::Bool),
                    DeclarationType::I64 => completed.push(DeclarationType::I64),
                    DeclarationType::F64 => completed.push(DeclarationType::F64),
                    DeclarationType::String => completed.push(DeclarationType::String),
                    DeclarationType::Bytes => completed.push(DeclarationType::Bytes),
                    DeclarationType::ByteVector => completed.push(DeclarationType::ByteVector),
                    DeclarationType::ByteSlice => completed.push(DeclarationType::ByteSlice),
                    DeclarationType::ByteSliceMut => {
                        completed.push(DeclarationType::ByteSliceMut);
                    }
                    DeclarationType::Path => completed.push(DeclarationType::Path),
                    DeclarationType::Capability(kind) => {
                        completed.push(DeclarationType::Capability(*kind));
                    }
                    DeclarationType::Symbol => completed.push(DeclarationType::Symbol),
                    DeclarationType::Resource(kind) => {
                        completed.push(DeclarationType::Resource(*kind));
                    }
                    DeclarationType::Product(entity) => {
                        completed.push(DeclarationType::Product(*entity));
                    }
                    DeclarationType::Enum {
                        constructor,
                        arguments,
                    } => {
                        work.push(Work::Enum(*constructor, arguments.len()));
                        work.extend(arguments.iter().rev().map(Work::Visit));
                    }
                    DeclarationType::TypeParameter(entity) => {
                        completed.push(DeclarationType::TypeParameter(*entity));
                    }
                    DeclarationType::DraftTypeParameter(parameter) => {
                        completed.push(DeclarationType::DraftTypeParameter(*parameter));
                    }
                    DeclarationType::List(inner) => {
                        work.push(Work::List);
                        work.push(Work::Visit(inner));
                    }
                    DeclarationType::Function { parameters, result } => {
                        work.push(Work::Function(parameters.len()));
                        work.push(Work::Visit(result));
                        work.extend(parameters.iter().rev().map(Work::Visit));
                    }
                },
                Work::Enum(constructor, count) => {
                    let Some(split) = completed.len().checked_sub(count) else {
                        unreachable!("declaration type clone completion order")
                    };
                    let arguments = completed.split_off(split);
                    completed.push(DeclarationType::Enum {
                        constructor,
                        arguments,
                    });
                }
                Work::List => {
                    let Some(inner) = completed.pop() else {
                        unreachable!("declaration type clone list completion order")
                    };
                    completed.push(DeclarationType::List(Box::new(inner)));
                }
                Work::Function(count) => {
                    let Some(result) = completed.pop() else {
                        unreachable!("declaration type clone function completion order")
                    };
                    let Some(split) = completed.len().checked_sub(count) else {
                        unreachable!("declaration type clone parameter completion order")
                    };
                    let parameters = completed.split_off(split);
                    completed.push(DeclarationType::Function {
                        parameters,
                        result: Box::new(result),
                    });
                }
            }
        }
        match completed.pop() {
            Some(ty) => ty,
            None => unreachable!("declaration type clone omitted its root"),
        }
    }
}

impl TryFrom<&SemanticType> for DeclarationType {
    type Error = WorkspaceError;

    fn try_from(root: &SemanticType) -> Result<Self, Self::Error> {
        enum Work<'a> {
            Visit(&'a SemanticType),
            Enum(SemanticEnum, usize),
            List,
            Function(usize),
        }
        let mut work = Vec::new();
        work.try_reserve(1).map_err(|_| {
            WorkspaceError::Host(Arc::from(
                "declaration type conversion work allocation failed",
            ))
        })?;
        work.push(Work::Visit(root));
        let mut completed = Vec::new();
        while let Some(item) = work.pop() {
            match item {
                Work::Visit(ty) => {
                    completed.try_reserve(1).map_err(|_| {
                        WorkspaceError::Host(Arc::from(
                            "declaration type conversion allocation failed",
                        ))
                    })?;
                    match ty {
                        SemanticType::Never => completed.push(DeclarationType::Never),
                        SemanticType::Unit => completed.push(DeclarationType::Unit),
                        SemanticType::Bool => completed.push(DeclarationType::Bool),
                        SemanticType::I64 => completed.push(DeclarationType::I64),
                        SemanticType::F64 => completed.push(DeclarationType::F64),
                        SemanticType::String => completed.push(DeclarationType::String),
                        SemanticType::Bytes => completed.push(DeclarationType::Bytes),
                        SemanticType::ByteVector => completed.push(DeclarationType::ByteVector),
                        SemanticType::ByteSlice => completed.push(DeclarationType::ByteSlice),
                        SemanticType::ByteSliceMut => completed.push(DeclarationType::ByteSliceMut),
                        SemanticType::Path => completed.push(DeclarationType::Path),
                        SemanticType::Capability(kind) => {
                            completed.push(DeclarationType::Capability(*kind));
                        }
                        SemanticType::Symbol => completed.push(DeclarationType::Symbol),
                        SemanticType::Resource(kind) => {
                            completed.push(DeclarationType::Resource(*kind));
                        }
                        SemanticType::Product(entity) => {
                            completed.push(DeclarationType::Product(*entity));
                        }
                        SemanticType::Enum {
                            constructor,
                            arguments,
                        } => {
                            let additional = arguments.len().checked_add(1).ok_or_else(|| {
                                WorkspaceError::Host(Arc::from(
                                    "declaration enum child count overflow",
                                ))
                            })?;
                            work.try_reserve(additional).map_err(|_| {
                                WorkspaceError::Host(Arc::from(
                                    "declaration type conversion work allocation failed",
                                ))
                            })?;
                            work.push(Work::Enum(*constructor, arguments.len()));
                            work.extend(arguments.iter().rev().map(Work::Visit));
                        }
                        SemanticType::TypeParameter(entity) => {
                            completed.push(DeclarationType::TypeParameter(*entity));
                        }
                        SemanticType::List(inner) => {
                            work.try_reserve(2).map_err(|_| {
                                WorkspaceError::Host(Arc::from(
                                    "declaration type conversion work allocation failed",
                                ))
                            })?;
                            work.push(Work::List);
                            work.push(Work::Visit(inner));
                        }
                        SemanticType::Function { parameters, result } => {
                            let additional = parameters.len().checked_add(2).ok_or_else(|| {
                                WorkspaceError::Host(Arc::from(
                                    "declaration function child count overflow",
                                ))
                            })?;
                            work.try_reserve(additional).map_err(|_| {
                                WorkspaceError::Host(Arc::from(
                                    "declaration type conversion work allocation failed",
                                ))
                            })?;
                            work.push(Work::Function(parameters.len()));
                            work.push(Work::Visit(result));
                            work.extend(parameters.iter().rev().map(Work::Visit));
                        }
                        SemanticType::Forall { .. } => {
                            return Err(WorkspaceError::InvalidSemanticType {
                                position: Arc::from("function declaration"),
                                reason: Arc::from(
                                    "nested universal types are not valid in declaration input",
                                ),
                            });
                        }
                    }
                }
                Work::Enum(constructor, count) => {
                    let split = completed.len().checked_sub(count).ok_or_else(|| {
                        WorkspaceError::Validation(Arc::from(
                            "declaration enum conversion order is invalid",
                        ))
                    })?;
                    let arguments = completed.split_off(split);
                    completed.push(DeclarationType::Enum {
                        constructor,
                        arguments,
                    });
                }
                Work::List => {
                    let inner = completed.pop().ok_or_else(|| {
                        WorkspaceError::Validation(Arc::from(
                            "declaration list conversion omitted its child",
                        ))
                    })?;
                    completed.push(DeclarationType::List(Box::new(inner)));
                }
                Work::Function(count) => {
                    let result = completed.pop().ok_or_else(|| {
                        WorkspaceError::Validation(Arc::from(
                            "declaration function conversion omitted its result",
                        ))
                    })?;
                    let split = completed.len().checked_sub(count).ok_or_else(|| {
                        WorkspaceError::Validation(Arc::from(
                            "declaration function conversion omitted parameters",
                        ))
                    })?;
                    let parameters = completed.split_off(split);
                    completed.push(DeclarationType::Function {
                        parameters,
                        result: Box::new(result),
                    });
                }
            }
        }
        let result = completed.pop().ok_or_else(|| {
            WorkspaceError::Validation(Arc::from("declaration type conversion omitted its root"))
        })?;
        if completed.is_empty() {
            Ok(result)
        } else {
            Err(WorkspaceError::Validation(Arc::from(
                "declaration type conversion left disconnected results",
            )))
        }
    }
}

impl Drop for DeclarationType {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        take_declaration_type_children(self, &mut pending);
        while let Some(mut ty) = pending.pop() {
            take_declaration_type_children(&mut ty, &mut pending);
        }
    }
}

fn take_declaration_type_children(ty: &mut DeclarationType, pending: &mut Vec<DeclarationType>) {
    match ty {
        DeclarationType::Enum { arguments, .. } => pending.append(arguments),
        DeclarationType::List(inner) => {
            pending.push(std::mem::replace(inner.as_mut(), DeclarationType::Unit));
        }
        DeclarationType::Function { parameters, result } => {
            pending.append(parameters);
            pending.push(std::mem::replace(result.as_mut(), DeclarationType::Unit));
        }
        _ => {}
    }
}

impl PartialEq for DeclarationType {
    fn eq(&self, other: &Self) -> bool {
        let mut pending = vec![(self, other)];
        while let Some((left, right)) = pending.pop() {
            match (left, right) {
                (DeclarationType::Never, DeclarationType::Never)
                | (DeclarationType::Unit, DeclarationType::Unit)
                | (DeclarationType::Bool, DeclarationType::Bool)
                | (DeclarationType::I64, DeclarationType::I64)
                | (DeclarationType::F64, DeclarationType::F64)
                | (DeclarationType::String, DeclarationType::String)
                | (DeclarationType::Bytes, DeclarationType::Bytes)
                | (DeclarationType::ByteVector, DeclarationType::ByteVector)
                | (DeclarationType::ByteSlice, DeclarationType::ByteSlice)
                | (DeclarationType::ByteSliceMut, DeclarationType::ByteSliceMut)
                | (DeclarationType::Path, DeclarationType::Path)
                | (DeclarationType::Symbol, DeclarationType::Symbol) => {}
                (DeclarationType::Capability(left), DeclarationType::Capability(right))
                    if left == right => {}
                (DeclarationType::Resource(left), DeclarationType::Resource(right))
                    if left == right => {}
                (DeclarationType::Product(left), DeclarationType::Product(right))
                    if left == right => {}
                (DeclarationType::TypeParameter(left), DeclarationType::TypeParameter(right))
                    if left == right => {}
                (
                    DeclarationType::DraftTypeParameter(left),
                    DeclarationType::DraftTypeParameter(right),
                ) if left == right => {}
                (
                    DeclarationType::Enum {
                        constructor: left_constructor,
                        arguments: left_arguments,
                    },
                    DeclarationType::Enum {
                        constructor: right_constructor,
                        arguments: right_arguments,
                    },
                ) if left_constructor == right_constructor
                    && left_arguments.len() == right_arguments.len() =>
                {
                    pending.extend(left_arguments.iter().zip(right_arguments));
                }
                (DeclarationType::List(left), DeclarationType::List(right)) => {
                    pending.push((left, right));
                }
                (
                    DeclarationType::Function {
                        parameters: left_parameters,
                        result: left_result,
                    },
                    DeclarationType::Function {
                        parameters: right_parameters,
                        result: right_result,
                    },
                ) if left_parameters.len() == right_parameters.len() => {
                    pending.push((left_result, right_result));
                    pending.extend(left_parameters.iter().zip(right_parameters));
                }
                _ => return false,
            }
        }
        true
    }
}

impl Eq for DeclarationType {}

impl fmt::Debug for DeclarationType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        enum Work<'a> {
            Type(&'a DeclarationType),
            Text(&'static str),
        }
        let mut pending = vec![Work::Type(self)];
        while let Some(item) = pending.pop() {
            match item {
                Work::Text(text) => formatter.write_str(text)?,
                Work::Type(ty) => match ty {
                    DeclarationType::Never => formatter.write_str("never")?,
                    DeclarationType::Unit => formatter.write_str("unit")?,
                    DeclarationType::Bool => formatter.write_str("bool")?,
                    DeclarationType::I64 => formatter.write_str("i64")?,
                    DeclarationType::F64 => formatter.write_str("f64")?,
                    DeclarationType::String => formatter.write_str("string")?,
                    DeclarationType::Bytes => formatter.write_str("bytes")?,
                    DeclarationType::ByteVector => formatter.write_str("byte-vector")?,
                    DeclarationType::ByteSlice => formatter.write_str("byte-slice")?,
                    DeclarationType::ByteSliceMut => formatter.write_str("byte-slice-mut")?,
                    DeclarationType::Path => formatter.write_str("path")?,
                    DeclarationType::Capability(kind) => {
                        write!(formatter, "capability {}", kind.as_str())?;
                    }
                    DeclarationType::Symbol => formatter.write_str("symbol")?,
                    DeclarationType::Resource(kind) => formatter.write_str(kind.as_str())?,
                    DeclarationType::Product(entity) => write!(formatter, "product({entity:?})")?,
                    DeclarationType::Enum {
                        constructor,
                        arguments,
                    } => {
                        write!(formatter, "enum({constructor:?})")?;
                        for argument in arguments.iter().rev() {
                            pending.push(Work::Type(argument));
                            pending.push(Work::Text(" "));
                        }
                    }
                    DeclarationType::TypeParameter(entity) => {
                        write!(formatter, "type-parameter({entity:?})")?;
                    }
                    DeclarationType::DraftTypeParameter(parameter) => {
                        write!(formatter, "draft-type-parameter({})", parameter.raw())?;
                    }
                    DeclarationType::List(inner) => {
                        formatter.write_str("list ")?;
                        pending.push(Work::Type(inner));
                    }
                    DeclarationType::Function { parameters, result } => {
                        formatter.write_str("fn inputs")?;
                        pending.push(Work::Type(result));
                        pending.push(Work::Text(" output "));
                        for parameter in parameters.iter().rev() {
                            pending.push(Work::Type(parameter));
                            pending.push(Work::Text(" "));
                        }
                    }
                },
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalDraft {
    pub binding: DraftBindingId,
    pub name: String,
    pub value: DraftNodeId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DraftFieldValue {
    pub field: EntityId,
    pub value: DraftNodeId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeArgumentDraft {
    pub parameter: EntityId,
    pub argument: SemanticType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DraftPatternField {
    pub field: EntityId,
    pub pattern: DraftPatternNodeId,
}

/// One flat non-recursive pattern tree owned by a match arm.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PatternDraft {
    pub nodes: Vec<DraftPatternNode>,
    pub root: DraftPatternNodeId,
}

impl PatternDraft {
    pub fn new(nodes: Vec<DraftPatternNode>, root: DraftPatternNodeId) -> Self {
        Self { nodes, root }
    }

    pub fn wildcard() -> Self {
        Self::new(vec![DraftPatternNode::Wildcard], DraftPatternNodeId::new(0))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum DraftPatternNode {
    Wildcard,
    Binding {
        binding: DraftBindingId,
        name: String,
    },
    Bool(bool),
    I64(i64),
    Product {
        product: EntityId,
        fields: Vec<DraftPatternField>,
    },
    EnumVariant {
        variant: EntityId,
        fields: Vec<DraftPatternField>,
    },
}

impl DraftPatternNode {
    pub(super) fn child_count(&self) -> usize {
        match self {
            Self::Product { fields, .. } | Self::EnumVariant { fields, .. } => fields.len(),
            Self::Wildcard | Self::Binding { .. } | Self::Bool(_) | Self::I64(_) => 0,
        }
    }

    pub(super) fn for_each_child(&self, mut visit: impl FnMut(DraftPatternNodeId)) {
        if let Self::Product { fields, .. } | Self::EnumVariant { fields, .. } = self {
            for field in fields {
                visit(field.pattern);
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MatchArmDraft {
    pub pattern: PatternDraft,
    pub body: DraftNodeId,
}

/// A non-recursive proposed expression graph. Child IDs refer to entries in `nodes`.
#[derive(Clone, Debug, PartialEq)]
pub struct ExpressionDraft {
    pub nodes: Vec<DraftNode>,
    pub root: DraftNodeId,
}

impl ExpressionDraft {
    pub fn new(nodes: Vec<DraftNode>, root: DraftNodeId) -> Self {
        Self { nodes, root }
    }

    pub fn scalar_i64(value: i64) -> Self {
        Self::new(vec![DraftNode::I64(value)], DraftNodeId::new(0))
    }

    pub fn scalar_f64(value: f64) -> Self {
        Self::new(vec![DraftNode::F64(value)], DraftNodeId::new(0))
    }

    pub fn scalar_bool(value: bool) -> Self {
        Self::new(vec![DraftNode::Bool(value)], DraftNodeId::new(0))
    }

    pub fn unit() -> Self {
        Self::new(vec![DraftNode::Unit], DraftNodeId::new(0))
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum DraftNode {
    I64(i64),
    F64(f64),
    Bool(bool),
    Unit,
    Bytes(Vec<u8>),
    Load(DraftBindingRef),
    Move(DraftBindingRef),
    BorrowShared(DraftBindingRef),
    Call {
        callee: EntityId,
        type_arguments: Vec<TypeArgumentDraft>,
        arguments: Vec<DraftNodeId>,
    },
    Operation {
        operation: Operation,
        arguments: Vec<DraftNodeId>,
    },
    If {
        condition: DraftNodeId,
        then_branch: DraftNodeId,
        else_branch: DraftNodeId,
    },
    Sequence(Vec<DraftNodeId>),
    Let {
        bindings: Vec<LocalDraft>,
        body: DraftNodeId,
    },
    MutableLocal {
        binding: DraftBindingId,
        name: String,
        ty: SemanticType,
        initial: DraftNodeId,
        body: DraftNodeId,
    },
    SetLocal {
        target: DraftBindingRef,
        value: DraftNodeId,
    },
    While {
        condition: DraftNodeId,
        body: Vec<DraftNodeId>,
    },
    Loop {
        result_type: SemanticType,
        body: Vec<DraftNodeId>,
    },
    Return {
        value: DraftNodeId,
    },
    Break {
        value: DraftNodeId,
    },
    Continue,
    ProductValue {
        product: EntityId,
        fields: Vec<DraftFieldValue>,
    },
    ProductField {
        field: EntityId,
        value: DraftNodeId,
    },
    EnumValue {
        variant: EntityId,
        type_arguments: Vec<TypeArgumentDraft>,
        fields: Vec<DraftFieldValue>,
    },
    EnumIsVariant {
        variant: EntityId,
        value: DraftNodeId,
    },
    Match {
        scrutinee: DraftNodeId,
        arms: Vec<MatchArmDraft>,
    },
}

impl DraftNode {
    pub(super) fn child_count(&self) -> Option<usize> {
        match self {
            Self::Call { arguments, .. } | Self::Operation { arguments, .. } => {
                Some(arguments.len())
            }
            Self::If { .. } => Some(3),
            Self::Sequence(expressions) => Some(expressions.len()),
            Self::Let { bindings, .. } => bindings.len().checked_add(1),
            Self::MutableLocal { .. } => Some(2),
            Self::SetLocal { .. } | Self::Return { .. } | Self::Break { .. } => Some(1),
            Self::While { body, .. } => body.len().checked_add(1),
            Self::Loop { body, .. } => Some(body.len()),
            Self::ProductValue { fields, .. } | Self::EnumValue { fields, .. } => {
                Some(fields.len())
            }
            Self::ProductField { .. } | Self::EnumIsVariant { .. } => Some(1),
            Self::Match { arms, .. } => arms.len().checked_add(1),
            _ => Some(0),
        }
    }

    pub(super) fn for_each_child(&self, mut visit: impl FnMut(DraftNodeId)) {
        match self {
            Self::Call { arguments, .. } | Self::Operation { arguments, .. } => {
                for child in arguments {
                    visit(*child);
                }
            }
            Self::If {
                condition,
                then_branch,
                else_branch,
            } => {
                visit(*condition);
                visit(*then_branch);
                visit(*else_branch);
            }
            Self::Sequence(expressions) => {
                for expression in expressions {
                    visit(*expression);
                }
            }
            Self::Let { bindings, body } => {
                for binding in bindings {
                    visit(binding.value);
                }
                visit(*body);
            }
            Self::MutableLocal { initial, body, .. } => {
                visit(*initial);
                visit(*body);
            }
            Self::SetLocal { value, .. } | Self::Return { value } | Self::Break { value } => {
                visit(*value)
            }
            Self::While { condition, body } => {
                visit(*condition);
                for expression in body {
                    visit(*expression);
                }
            }
            Self::Loop { body, .. } => {
                for expression in body {
                    visit(*expression);
                }
            }
            Self::ProductValue { fields, .. } | Self::EnumValue { fields, .. } => {
                for field in fields {
                    visit(field.value);
                }
            }
            Self::ProductField { value, .. } | Self::EnumIsVariant { value, .. } => visit(*value),
            Self::Match { scrutinee, arms } => {
                visit(*scrutinee);
                for arm in arms {
                    visit(arm.body);
                }
            }
            _ => {}
        }
    }
}
