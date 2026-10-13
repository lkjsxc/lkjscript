//! Exact predecessor summary schema. The owner-kind enum is deliberately closed
//! to the original 28 variants, independent of the current kernel enum.

use super::{OwnerSummary, SemanticDigest};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{OwnerKey, OwnerKind, OwnerObjectDigest};
use bincode::{Decode, Encode};

macro_rules! frozen_owner_kinds {
    ($($kind:ident),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, Decode, Encode, Eq, PartialEq)]
        pub(super) enum OwnerKind11 { $($kind),+ }

        impl From<OwnerKind11> for OwnerKind {
            fn from(value: OwnerKind11) -> Self {
                match value { $(OwnerKind11::$kind => Self::$kind),+ }
            }
        }

        impl TryFrom<OwnerKind> for OwnerKind11 {
            type Error = Diagnostic;
            fn try_from(value: OwnerKind) -> Result<Self, Self::Error> {
                match value {
                    $(OwnerKind::$kind => Ok(Self::$kind)),+,
                    OwnerKind::ReadView => Err(Diagnostic::new(
                        DiagnosticClass::Corrupt, "witness_summary_contract",
                        "a read-view owner kind cannot be represented by frozen Summary11",
                    )),
                }
            }
        }
    };
}

// Original bincode variant order, rather than OwnerKind::ALL discovery/tag order.
frozen_owner_kinds!(
    Module,
    Record,
    Variant,
    Interface,
    External,
    PureFunction,
    TaskFunction,
    Constant,
    Component,
    Test,
    TypeParameter,
    Field,
    Case,
    Operation,
    Parameter,
    Binding,
    Expression,
    Requirement,
    Port,
    Target,
    Documentation,
    Annotation,
    HttpRoute,
    EffectParameter,
    RequirementParameter,
    OwnedContract,
    OwnedImplementation,
    OwnedRecord,
);

#[derive(Clone, Debug, Decode, Encode, Eq, PartialEq)]
pub(super) struct OwnerSummary11 {
    pub contract_version: u16,
    pub owner: OwnerKey,
    pub kind: OwnerKind11,
    pub record: OwnerObjectDigest,
    pub semantic_interface: SemanticDigest,
    pub implementation: SemanticDigest,
    pub type_digest: SemanticDigest,
    pub effect: SemanticDigest,
    pub capability: SemanticDigest,
    pub relations: SemanticDigest,
    pub presentation: SemanticDigest,
    pub test: Option<SemanticDigest>,
    pub validation_dependencies: SemanticDigest,
}

impl TryFrom<&OwnerSummary> for OwnerSummary11 {
    type Error = Diagnostic;
    fn try_from(value: &OwnerSummary) -> Result<Self, Self::Error> {
        Ok(Self {
            contract_version: value.contract_version,
            owner: value.owner,
            kind: value.kind.try_into()?,
            record: value.record,
            semantic_interface: value.semantic_interface,
            implementation: value.implementation,
            type_digest: value.type_digest,
            effect: value.effect,
            capability: value.capability,
            relations: value.relations,
            presentation: value.presentation,
            test: value.test,
            validation_dependencies: value.validation_dependencies,
        })
    }
}

impl From<OwnerSummary11> for OwnerSummary {
    fn from(value: OwnerSummary11) -> Self {
        Self {
            contract_version: value.contract_version,
            owner: value.owner,
            kind: value.kind.into(),
            record: value.record,
            semantic_interface: value.semantic_interface,
            implementation: value.implementation,
            type_digest: value.type_digest,
            effect: value.effect,
            capability: value.capability,
            relations: value.relations,
            presentation: value.presentation,
            test: value.test,
            validation_dependencies: value.validation_dependencies,
        }
    }
}
