//! Exact Graph 5 semantic references without mutable locators.

use super::id::PackageId;
use crate::platform::semantic_id::{
    CaseId, DeclarationId, FieldId, ModuleId, OperationId, PortId, RequirementId, TargetId,
};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

macro_rules! exact_reference {
    ($name:ident, $field:ident, $id:ty) => {
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

exact_reference!(DeclarationReference, declaration, DeclarationId);
exact_reference!(ModuleReference, module, ModuleId);
exact_reference!(FieldReference, field, FieldId);
exact_reference!(CaseReference, case, CaseId);
exact_reference!(OperationReference, operation, OperationId);
exact_reference!(RequirementReference, requirement, RequirementId);
exact_reference!(PortReference, port, PortId);
exact_reference!(TargetReference, target, TargetId);
