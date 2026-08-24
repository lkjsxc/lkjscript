//! Canonical Graph 5 authoring namespaces.
//!
//! Names are selector and presentation inputs. Accepted references use exact identities, so the
//! namespace rule is deliberately separate from executable relation and dependency summaries.

use super::{Name, OwnerKey, OwnerRecord, ParameterParent};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, Ord, PartialEq, PartialOrd, Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum NamespaceClass {
    Module,
    Declaration,
    TypeParameter,
    Field,
    Case,
    Operation,
    Parameter,
    Requirement,
    Port,
    Target,
}

impl NamespaceClass {
    pub const ALL: [Self; 10] = [
        Self::Module,
        Self::Declaration,
        Self::TypeParameter,
        Self::Field,
        Self::Case,
        Self::Operation,
        Self::Parameter,
        Self::Requirement,
        Self::Port,
        Self::Target,
    ];

    pub const fn tag(self) -> u8 {
        match self {
            Self::Module => 1,
            Self::Declaration => 2,
            Self::TypeParameter => 3,
            Self::Field => 4,
            Self::Case => 5,
            Self::Operation => 6,
            Self::Parameter => 7,
            Self::Requirement => 8,
            Self::Port => 9,
            Self::Target => 10,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NamespaceEntryRef<'a> {
    pub parent: Option<OwnerKey>,
    pub class: NamespaceClass,
    pub name: &'a Name,
}

/// Returns the one canonical uniqueness namespace entry for an owner, when its name participates
/// in accepted namespace selection. Lexical binding names are presentation only after exact local
/// references have been normalized, so they intentionally do not enter this package namespace.
pub fn owner_namespace(record: &OwnerRecord) -> Option<NamespaceEntryRef<'_>> {
    let (parent, class, name) = match record {
        OwnerRecord::Module(module) => (None, NamespaceClass::Module, &module.name),
        OwnerRecord::Declaration(declaration) => (
            Some(OwnerKey::Module(declaration.module)),
            NamespaceClass::Declaration,
            &declaration.name,
        ),
        OwnerRecord::TypeParameter(parameter) => (
            Some(OwnerKey::Declaration(parameter.declaration)),
            NamespaceClass::TypeParameter,
            &parameter.name,
        ),
        OwnerRecord::Field(field) => (
            Some(OwnerKey::Declaration(field.declaration)),
            NamespaceClass::Field,
            &field.name,
        ),
        OwnerRecord::Case(case) => (
            Some(OwnerKey::Declaration(case.declaration)),
            NamespaceClass::Case,
            &case.name,
        ),
        OwnerRecord::Operation(operation) => (
            Some(OwnerKey::Declaration(operation.declaration)),
            NamespaceClass::Operation,
            &operation.name,
        ),
        OwnerRecord::Parameter(parameter) => (
            Some(match parameter.parent {
                ParameterParent::Function(declaration) => OwnerKey::Declaration(declaration),
                ParameterParent::Operation(operation) => OwnerKey::Operation(operation),
            }),
            NamespaceClass::Parameter,
            &parameter.name,
        ),
        OwnerRecord::Requirement(requirement) => (
            Some(OwnerKey::Declaration(requirement.declaration)),
            NamespaceClass::Requirement,
            &requirement.name,
        ),
        OwnerRecord::Port(port) => (
            Some(OwnerKey::Declaration(port.declaration)),
            NamespaceClass::Port,
            &port.name,
        ),
        OwnerRecord::Target(target) => (None, NamespaceClass::Target, &target.name),
        OwnerRecord::Binding(_)
        | OwnerRecord::Expression(_)
        | OwnerRecord::Documentation(_)
        | OwnerRecord::Annotation(_) => return None,
    };
    Some(NamespaceEntryRef {
        parent,
        class,
        name,
    })
}
