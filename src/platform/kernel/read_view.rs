//! Canonical source-tied view identities and callable result contracts.
//!
//! These finite symbolic coordinates carry no live access permission. Local inventory checks
//! inspect every entry; dependency, lexical containment, category and provenance admission remain
//! independent checks over the complete candidate and each invocation.

use super::contract::MAXIMUM_CHILDREN;
use super::{DeclarationReference, Name, TypeObjectDigest};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, BindingId, ExpressionId, FieldId, MethodId, ReadScopeParameterId,
    TypeParameterId,
};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CallableResult<S, T = TypeObjectDigest> {
    pub ty: T,
    pub relation: ResultRelation<S>,
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ResultRelation<S> {
    Value,
    ReadFrom { source: S },
    ViewFrom { source: S, scope: ReadScopeArgument },
}

impl<S, T> CallableResult<S, T> {
    pub fn try_map_source<U>(
        self,
        mut map: impl FnMut(S) -> Result<U, Diagnostic>,
    ) -> Result<CallableResult<U, T>, Diagnostic> {
        Ok(CallableResult {
            ty: self.ty,
            relation: self.relation.try_map_source(&mut map)?,
        })
    }

    pub fn try_map_type<U>(
        self,
        map: impl FnOnce(T) -> Result<U, Diagnostic>,
    ) -> Result<CallableResult<S, U>, Diagnostic> {
        Ok(CallableResult {
            ty: map(self.ty)?,
            relation: self.relation,
        })
    }
}

impl<S> ResultRelation<S> {
    pub fn source(&self) -> Option<&S> {
        match self {
            Self::Value => None,
            Self::ReadFrom { source } | Self::ViewFrom { source, .. } => Some(source),
        }
    }

    pub fn try_map_source<U>(
        self,
        mut map: impl FnMut(S) -> Result<U, Diagnostic>,
    ) -> Result<ResultRelation<U>, Diagnostic> {
        Ok(match self {
            Self::Value => ResultRelation::Value,
            Self::ReadFrom { source } => ResultRelation::ReadFrom {
                source: map(source)?,
            },
            Self::ViewFrom { source, scope } => ResultRelation::ViewFrom {
                source: map(source)?,
                scope,
            },
        })
    }
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadScopeParameter {
    pub id: ReadScopeParameterId,
    pub name: Name,
}

#[derive(
    Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReadScopeBinder {
    Declaration(DeclarationReference),
    ContractMethod {
        contract: DeclarationReference,
        method: MethodId,
    },
    AssociatedMember {
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
    },
    AssociatedBinding {
        implementation: DeclarationReference,
        member: AssociatedTypeMemberId,
    },
}

impl ReadScopeBinder {
    pub fn declaration(self) -> DeclarationReference {
        match self {
            Self::Declaration(declaration) => declaration,
            Self::ContractMethod { contract, .. } | Self::AssociatedMember { contract, .. } => {
                contract
            }
            Self::AssociatedBinding { implementation, .. } => implementation,
        }
    }
}

#[derive(
    Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(deny_unknown_fields)]
pub struct ReadScopeReference {
    pub binder: ReadScopeBinder,
    pub parameter: ReadScopeParameterId,
}

#[derive(
    Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReadScopeArgument {
    Parameter(ReadScopeReference),
    Lexical {
        callable: DeclarationReference,
        binding: BindingId,
    },
}

impl ReadScopeArgument {
    pub fn declaration(self) -> DeclarationReference {
        match self {
            Self::Parameter(reference) => reference.binder.declaration(),
            Self::Lexical { callable, .. } => callable,
        }
    }
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadScopeSource<S> {
    pub source: S,
    pub scope: ReadScopeArgument,
}

impl<S> ReadScopeSource<S> {
    pub fn try_map_source<U>(
        self,
        mut map: impl FnMut(S) -> Result<U, Diagnostic>,
    ) -> Result<ReadScopeSource<U>, Diagnostic> {
        Ok(ReadScopeSource {
            source: map(self.source)?,
            scope: self.scope,
        })
    }
}

/// A positive semantic category. Absence of Owned never establishes ordinary-data permission.
#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueCategory {
    OrdinaryData,
    Owned,
    ReadView,
    Capability,
    Callable,
    /// A type-level identity, never an inhabitable runtime value.
    RegionIdentity,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadViewDeclaration {
    pub type_parameters: Vec<TypeParameterId>,
    pub scope_parameter: ReadScopeParameter,
    pub fields: Vec<ReadViewField>,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadViewField {
    pub id: FieldId,
    pub name: Name,
    pub ty: TypeObjectDigest,
    pub kind: ReadViewFieldKind,
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReadViewFieldKind {
    Ordinary,
    ReadReference { read_scope: ReadScopeArgument },
}

#[derive(
    Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(deny_unknown_fields)]
pub struct ReadViewFieldReference {
    pub declaration: DeclarationReference,
    pub field: FieldId,
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadViewFieldInitializer {
    pub field: FieldId,
    pub value: ExpressionId,
}

impl ReadViewDeclaration {
    pub(crate) fn validate_local(&self) -> Result<(), Diagnostic> {
        let mut parameters = BTreeSet::new();
        if self.type_parameters.len() > MAXIMUM_CHILDREN
            || self
                .type_parameters
                .iter()
                .any(|id| !parameters.insert(*id))
        {
            return Err(view_error(
                "kernel_read_view_scope",
                "view type parameters must be bounded and distinct",
            ));
        }
        let mut ids = BTreeSet::new();
        let mut names = BTreeSet::new();
        if self.fields.len() > MAXIMUM_CHILDREN
            || self
                .fields
                .iter()
                .any(|field| !ids.insert(field.id) || !names.insert(&field.name))
        {
            return Err(view_error(
                "kernel_read_view_field",
                "inline view fields require bounded distinct identities and names",
            ));
        }
        for field in &self.fields {
            if let ReadViewFieldKind::ReadReference { read_scope } = field.kind {
                validate_formal_scope(
                    read_scope,
                    std::slice::from_ref(&self.scope_parameter),
                    None,
                )?;
            }
        }
        Ok(())
    }

    pub(crate) fn validate_local_at(
        &self,
        declaration: DeclarationReference,
    ) -> Result<(), Diagnostic> {
        self.validate_local()?;
        for field in &self.fields {
            if let ReadViewFieldKind::ReadReference { read_scope } = field.kind {
                validate_formal_scope(
                    read_scope,
                    std::slice::from_ref(&self.scope_parameter),
                    Some(ReadScopeBinder::Declaration(declaration)),
                )?;
            }
        }
        Ok(())
    }
}

/// Checks the complete local inventory without granting declaration or invocation authority.
pub(crate) fn validate_read_scope_inventory<S: Copy + Ord>(
    parameters: &[ReadScopeParameter],
    sources: &[ReadScopeSource<S>],
    mut source_exists: impl FnMut(S) -> bool,
) -> Result<(), Diagnostic> {
    validate_inventory(parameters, sources, None, &mut source_exists)
}

/// Additionally requires every formal relationship to name its exact enclosing binder.
pub(crate) fn validate_read_scope_inventory_at<S: Copy + Ord>(
    binder: ReadScopeBinder,
    parameters: &[ReadScopeParameter],
    sources: &[ReadScopeSource<S>],
    mut source_exists: impl FnMut(S) -> bool,
) -> Result<(), Diagnostic> {
    validate_inventory(parameters, sources, Some(binder), &mut source_exists)
}

fn validate_inventory<S: Copy + Ord>(
    parameters: &[ReadScopeParameter],
    sources: &[ReadScopeSource<S>],
    binder: Option<ReadScopeBinder>,
    source_exists: &mut impl FnMut(S) -> bool,
) -> Result<(), Diagnostic> {
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    if parameters.len() > MAXIMUM_CHILDREN
        || sources.len() > MAXIMUM_CHILDREN
        || parameters
            .iter()
            .any(|p| !ids.insert(p.id) || !names.insert(&p.name))
    {
        return Err(view_error(
            "kernel_read_view_scope",
            "scope parameters require bounded distinct identities and names",
        ));
    }
    let mut pairs = BTreeSet::new();
    for relationship in sources {
        if !source_exists(relationship.source)
            || !pairs.insert((relationship.source, relationship.scope))
        {
            return Err(view_error(
                "kernel_read_view_source",
                "scope relationships require existing exact sources and distinct pairs",
            ));
        }
        validate_formal_scope(relationship.scope, parameters, binder)?;
    }
    Ok(())
}

pub(crate) fn validate_formal_scope(
    scope: ReadScopeArgument,
    parameters: &[ReadScopeParameter],
    binder: Option<ReadScopeBinder>,
) -> Result<(), Diagnostic> {
    match scope {
        ReadScopeArgument::Parameter(reference)
            if parameters.iter().any(|p| p.id == reference.parameter)
                && binder.is_none_or(|expected| expected == reference.binder) =>
        {
            Ok(())
        }
        _ => Err(view_error(
            "kernel_read_view_scope",
            "signature scope must name an exact declared formal in its binder",
        )),
    }
}

pub(crate) fn view_error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}

#[cfg(test)]
#[path = "read_view_tests.rs"]
mod tests;
