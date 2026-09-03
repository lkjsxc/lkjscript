//! Exact Graph 8 semantic references without mutable locators.

use super::id::PackageId;
use crate::platform::semantic_id::{
    CaseId, DeclarationId, FieldId, ModuleId, OperationId, PortId, RequirementId, TargetId,
};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

macro_rules! exact_reference {
    ($name:ident, $field:ident, $id:ty, $schema:literal) => {
        #[derive(
            Clone,
            Copy,
            Debug,
            Decode,
            Deserialize,
            Encode,
            Eq,
            Ord,
            PartialEq,
            PartialOrd,
            Serialize,
        )]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            pub package: PackageId,
            pub $field: $id,
        }
    };
}

exact_reference!(
    DeclarationReference,
    declaration,
    DeclarationId,
    "lkjscript.Graph6DeclarationReferenceV1"
);
exact_reference!(
    ModuleReference,
    module,
    ModuleId,
    "lkjscript.Graph6ModuleReferenceV1"
);
exact_reference!(
    FieldReference,
    field,
    FieldId,
    "lkjscript.Graph6FieldReferenceV1"
);
exact_reference!(
    CaseReference,
    case,
    CaseId,
    "lkjscript.Graph6CaseReferenceV1"
);
exact_reference!(
    OperationReference,
    operation,
    OperationId,
    "lkjscript.Graph6OperationReferenceV1"
);
exact_reference!(
    RequirementReference,
    requirement,
    RequirementId,
    "lkjscript.Graph6RequirementReferenceV1"
);
exact_reference!(
    PortReference,
    port,
    PortId,
    "lkjscript.Graph6PortReferenceV1"
);
exact_reference!(
    TargetReference,
    target,
    TargetId,
    "lkjscript.Graph6TargetReferenceV1"
);
