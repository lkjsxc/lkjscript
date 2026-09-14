//! Explicit rank-one requirement operands and minimum-operation constraints.
//!
//! These are semantic references, never adapter bindings or grants. Substitution selects the
//! whole concrete atom and is simultaneous: an inserted caller operand is not substituted again.

use super::{
    DeclarationReference, OperationReference, RequirementParameterReference, RequirementReference,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(
    Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(
    tag = "kind",
    content = "reference",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RequirementOperand {
    Concrete(RequirementReference),
    Parameter(RequirementParameterReference),
}

impl From<RequirementReference> for RequirementOperand {
    fn from(reference: RequirementReference) -> Self {
        Self::Concrete(reference)
    }
}

impl From<RequirementParameterReference> for RequirementOperand {
    fn from(reference: RequirementParameterReference) -> Self {
        Self::Parameter(reference)
    }
}

pub type RequirementSubstitution = BTreeMap<RequirementParameterReference, RequirementOperand>;

impl RequirementOperand {
    pub const fn package(self) -> super::PackageId {
        match self {
            Self::Concrete(reference) => reference.package,
            Self::Parameter(reference) => reference.package,
        }
    }

    pub const fn owner(self) -> super::OwnerKey {
        match self {
            Self::Concrete(reference) => super::OwnerKey::Requirement(reference.requirement),
            Self::Parameter(reference) => {
                super::OwnerKey::RequirementParameter(reference.parameter)
            }
        }
    }

    pub fn substitute(self, bindings: &RequirementSubstitution) -> Result<Self, Diagnostic> {
        match self {
            Self::Concrete(_) => Ok(self),
            Self::Parameter(parameter) => bindings.get(&parameter).copied().ok_or_else(|| {
                error(
                    "kernel_requirement_parameter_scope",
                    "requirement parameter lacks an exact substitution",
                )
            }),
        }
    }

    pub const fn concrete(self) -> Option<RequirementReference> {
        match self {
            Self::Concrete(reference) => Some(reference),
            Self::Parameter(_) => None,
        }
    }
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequirementConstraint {
    pub interface: DeclarationReference,
    pub operations: Vec<OperationReference>,
}

impl RequirementConstraint {
    pub fn validate(&self) -> Result<(), Diagnostic> {
        if self.operations.len() > super::contract::MAXIMUM_CHILDREN
            || self.operations.windows(2).any(|pair| pair[0] >= pair[1])
            || self
                .operations
                .iter()
                .any(|operation| operation.package != self.interface.package)
        {
            return Err(error(
                "kernel_requirement_constraint",
                "requirement constraint operations must be canonical and belong to its exact interface package",
            ));
        }
        Ok(())
    }

    /// The declaration owner separately checks every operation's exact interface membership.
    /// This relation admits additional operations; it does not attenuate the supplied operand.
    pub fn entails(&self, required: &Self) -> Result<bool, Diagnostic> {
        self.validate()?;
        required.validate()?;
        Ok(self.interface == required.interface
            && required
                .operations
                .iter()
                .all(|operation| self.operations.binary_search(operation).is_ok()))
    }
}

fn error(code: &str, message: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}
