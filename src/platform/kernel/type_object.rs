//! Content-addressed structural type objects under the current canonical schema.

use super::ReadScopeArgument;
use super::contract::{MAXIMUM_CHILDREN, TYPE_OBJECT_CONTRACT_VERSION};
use super::digest::TypeObjectDigest;
use super::name::Name;
use super::reference::DeclarationReference;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, BindingId, ImplementationParameterId, TypeParameterId,
};
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TypeObject {
    pub contract_version: u16,
    pub form: TypeForm,
}

impl TypeObject {
    pub fn new(form: TypeForm) -> Result<Self, Diagnostic> {
        let object = Self {
            contract_version: TYPE_OBJECT_CONTRACT_VERSION,
            form,
        };
        object.validate_local()?;
        Ok(object)
    }

    pub(crate) fn validate_local(&self) -> Result<(), Diagnostic> {
        if self.contract_version != TYPE_OBJECT_CONTRACT_VERSION {
            return Err(type_error(
                "kernel_type_contract",
                format!(
                    "type object contract {} does not match the current contract {}",
                    self.contract_version, TYPE_OBJECT_CONTRACT_VERSION
                ),
            ));
        }
        match &self.form {
            TypeForm::RegionBrand { .. }
            | TypeForm::Region { .. }
            | TypeForm::RelPtr { .. }
            | TypeForm::RegionSlice { .. }
            | TypeForm::RegionCap { .. } => {}
            TypeForm::Applied { arguments, .. } => {
                require_count("nominal type arguments", arguments.len(), false)?;
            }
            TypeForm::ReadViewApplied { arguments, .. } => {
                require_count("read-view type arguments", arguments.len(), true)?;
            }
            TypeForm::StructuralRecord { fields }
            | TypeForm::OwnedProduct { fields }
            | TypeForm::OwnedChoice { cases: fields } => {
                require_count("structural fields", fields.len(), false)?;
                if fields.windows(2).any(|pair| pair[0].name >= pair[1].name) {
                    return Err(type_error(
                        "kernel_type_field_order",
                        "structural record fields must be strictly ordered by canonical name",
                    ));
                }
            }
            TypeForm::Function { parameters, .. } => {
                require_count("function parameters", parameters.len(), true)?;
            }
            TypeForm::TaskFunction {
                parameters, effect, ..
            } => {
                require_count("task function parameters", parameters.len(), true)?;
                effect.validate()?;
            }
            TypeForm::Unit
            | TypeForm::Bool
            | TypeForm::I64
            | TypeForm::F64
            | TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
            | TypeForm::Bytes
            | TypeForm::Text
            | TypeForm::StaticText
            | TypeForm::Secret
            | TypeForm::TypeParameter { .. }
            | TypeForm::Named { .. }
            | TypeForm::CapabilityResource { .. }
            | TypeForm::List { .. }
            | TypeForm::Map { .. }
            | TypeForm::Option { .. }
            | TypeForm::Result { .. }
            | TypeForm::Stream { .. } => {}
            TypeForm::AssociatedContractMember { .. }
            | TypeForm::AssociatedWitnessProjection { .. }
            | TypeForm::AssociatedReadViewMember { .. }
            | TypeForm::AssociatedReadViewProjection { .. } => {}
            TypeForm::OwnedSequence { .. } => {}
        }
        Ok(())
    }

    pub fn child_type_count(&self) -> usize {
        match &self.form {
            TypeForm::Region { .. } | TypeForm::RegionCap { .. } => 1,
            TypeForm::RelPtr { .. } | TypeForm::RegionSlice { .. } => 2,
            TypeForm::Applied { arguments, .. } | TypeForm::ReadViewApplied { arguments, .. } => {
                arguments.len()
            }
            TypeForm::StructuralRecord { fields }
            | TypeForm::OwnedProduct { fields }
            | TypeForm::OwnedChoice { cases: fields } => fields.len(),
            TypeForm::List { .. }
            | TypeForm::Option { .. }
            | TypeForm::Stream { .. }
            | TypeForm::OwnedSequence { .. } => 1,
            TypeForm::Map { .. } | TypeForm::Result { .. } => 2,
            TypeForm::Function { parameters, .. } | TypeForm::TaskFunction { parameters, .. } => {
                parameters.len().saturating_add(1)
            }
            _ => 0,
        }
    }

    pub fn child_types(&self) -> Vec<TypeObjectDigest> {
        match &self.form {
            TypeForm::Region { region } | TypeForm::RegionCap { region, .. } => vec![*region],
            TypeForm::RelPtr { region, item } | TypeForm::RegionSlice { region, item } => {
                vec![*region, *item]
            }
            TypeForm::RegionBrand { .. } => Vec::new(),
            TypeForm::Applied { arguments, .. } | TypeForm::ReadViewApplied { arguments, .. } => {
                arguments.clone()
            }
            TypeForm::StructuralRecord { fields }
            | TypeForm::OwnedProduct { fields }
            | TypeForm::OwnedChoice { cases: fields } => {
                fields.iter().map(|field| field.ty).collect()
            }
            TypeForm::List { item }
            | TypeForm::Option { item }
            | TypeForm::Stream { item }
            | TypeForm::OwnedSequence { item } => {
                vec![*item]
            }
            TypeForm::Map { key, value }
            | TypeForm::Result {
                ok: key,
                error: value,
            } => {
                vec![*key, *value]
            }
            TypeForm::Function { parameters, result }
            | TypeForm::TaskFunction {
                parameters, result, ..
            } => {
                let mut children = parameters.clone();
                children.push(*result);
                children
            }
            TypeForm::Unit
            | TypeForm::Bool
            | TypeForm::I64
            | TypeForm::F64
            | TypeForm::ByteBuffer
            | TypeForm::OwnedI64Cell
            | TypeForm::Bytes
            | TypeForm::Text
            | TypeForm::StaticText
            | TypeForm::Secret
            | TypeForm::TypeParameter { .. }
            | TypeForm::Named { .. }
            | TypeForm::CapabilityResource { .. } => Vec::new(),
            TypeForm::AssociatedContractMember { .. }
            | TypeForm::AssociatedWitnessProjection { .. }
            | TypeForm::AssociatedReadViewMember { .. }
            | TypeForm::AssociatedReadViewProjection { .. } => Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TypeForm {
    Unit,
    Bool,
    I64,
    Bytes,
    Text,
    StaticText,
    Secret,
    TypeParameter {
        parameter: TypeParameterId,
    },
    Named {
        declaration: DeclarationReference,
    },
    CapabilityResource {
        interface: DeclarationReference,
    },
    StructuralRecord {
        fields: Vec<StructuralTypeField>,
    },
    List {
        item: TypeObjectDigest,
    },
    Map {
        key: TypeObjectDigest,
        value: TypeObjectDigest,
    },
    Option {
        item: TypeObjectDigest,
    },
    Result {
        ok: TypeObjectDigest,
        error: TypeObjectDigest,
    },
    Stream {
        item: TypeObjectDigest,
    },
    Function {
        parameters: Vec<TypeObjectDigest>,
        result: TypeObjectDigest,
    },
    /// Positive-arity nominal identity with ordered type arguments.
    Applied {
        declaration: DeclarationReference,
        arguments: Vec<TypeObjectDigest>,
    },
    TaskFunction {
        parameters: Vec<TypeObjectDigest>,
        result: TypeObjectDigest,
        effect: super::EffectRow,
    },
    F64,
    ByteBuffer,
    OwnedI64Cell,
    /// Explicit affine product structure.
    OwnedProduct {
        fields: Vec<StructuralTypeField>,
    },
    /// An explicit affine sum. Exactly one named case carries one typed payload.
    /// Its ownership is invariant under case selection and generic substitution.
    OwnedChoice {
        cases: Vec<StructuralTypeField>,
    },
    /// One affine owner of a runtime-sized sequence of admitted owned or ordinary data.
    OwnedSequence {
        item: TypeObjectDigest,
    },
    /// Contract-local abstract member, legal only in its exact contract's method signatures.
    AssociatedContractMember {
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
    },
    /// Exact lexical witness identity. Equal Self types confer no selection authority.
    AssociatedWitnessProjection {
        scope: DeclarationReference,
        parameter: ImplementationParameterId,
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
    },
    /// Positive affine descriptor identity, including its complete source-scope actual.
    ReadViewApplied {
        declaration: DeclarationReference,
        arguments: Vec<TypeObjectDigest>,
        read_scope: ReadScopeArgument,
    },
    AssociatedReadViewMember {
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
        read_scope: ReadScopeArgument,
    },
    AssociatedReadViewProjection {
        scope: DeclarationReference,
        parameter: ImplementationParameterId,
        contract: DeclarationReference,
        member: AssociatedTypeMemberId,
        read_scope: ReadScopeArgument,
    },
    /// A generative type-level identity. It has no runtime value inhabitants.
    RegionBrand {
        callable: DeclarationReference,
        binding: BindingId,
    },
    /// A copyable region identifier, without storage custody or access rights.
    Region {
        region: TypeObjectDigest,
    },
    /// A copyable 32-bit offset. Only a matching live capability may dereference it.
    RelPtr {
        region: TypeObjectDigest,
        item: TypeObjectDigest,
    },
    /// A copyable pair of 32-bit offset and element count.
    RegionSlice {
        region: TypeObjectDigest,
        item: TypeObjectDigest,
    },
    /// Linear storage custody or a scoped read loan over one region identity.
    RegionCap {
        region: TypeObjectDigest,
        mode: RegionMode,
    },
}

#[derive(Clone, Copy, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RegionMode {
    Unique,
    Shared { scope: ReadScopeArgument },
    Frozen,
    Pinned,
}

impl RegionMode {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Unique => "unique",
            Self::Shared { .. } => "shared",
            Self::Frozen => "frozen",
            Self::Pinned => "pinned",
        }
    }
}

impl TypeForm {
    pub(crate) fn is_associated(&self) -> bool {
        self.is_owned_associated()
            || matches!(
                self,
                Self::AssociatedReadViewMember { .. } | Self::AssociatedReadViewProjection { .. }
            )
    }

    pub(crate) fn is_owned_associated(&self) -> bool {
        matches!(
            self,
            Self::AssociatedContractMember { .. } | Self::AssociatedWitnessProjection { .. }
        )
    }

    pub(crate) fn is_read_view(&self) -> bool {
        self.read_scope().is_some()
    }

    pub(crate) fn read_scope(&self) -> Option<ReadScopeArgument> {
        match self {
            Self::ReadViewApplied { read_scope, .. }
            | Self::AssociatedReadViewMember { read_scope, .. }
            | Self::AssociatedReadViewProjection { read_scope, .. }
            | Self::RegionCap {
                mode: RegionMode::Shared { scope: read_scope },
                ..
            } => Some(*read_scope),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Decode, Deserialize, Encode, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StructuralTypeField {
    pub name: Name,
    pub ty: TypeObjectDigest,
}

/// Request-local structural type interner. Equal canonical values reuse one digest, and child
/// availability is checked by exact digest lookup rather than a global scan.
#[derive(Clone, Debug)]
pub struct TypeObjectInterner {
    objects: BTreeMap<TypeObjectDigest, TypeObject>,
    maximum_objects: usize,
}

impl Default for TypeObjectInterner {
    fn default() -> Self {
        Self {
            objects: BTreeMap::new(),
            maximum_objects: usize::MAX,
        }
    }
}

impl TypeObjectInterner {
    pub fn with_maximum_objects(maximum_objects: usize) -> Self {
        Self {
            objects: BTreeMap::new(),
            maximum_objects,
        }
    }

    pub fn admit(
        &mut self,
        digest: TypeObjectDigest,
        object: TypeObject,
    ) -> Result<(), Diagnostic> {
        let (canonical, _) = super::codec::encode_type_object(&object)?;
        if canonical != digest {
            return Err(type_error(
                "kernel_type_digest_mismatch",
                "admitted type object does not match its exact digest",
            ));
        }
        self.insert_if_absent(digest, object)?;
        Ok(())
    }

    pub fn intern(&mut self, form: TypeForm) -> Result<TypeObjectDigest, Diagnostic> {
        let object = TypeObject::new(form)?;
        for child in object.child_types() {
            if !self.objects.contains_key(&child) {
                return Err(type_error(
                    "kernel_type_child_missing",
                    format!("type child {child} has not been interned in this request"),
                ));
            }
        }
        let (digest, _) = super::codec::encode_type_object(&object)?;
        self.insert_if_absent(digest, object)?;
        Ok(digest)
    }

    pub fn len(&self) -> usize {
        self.objects.len()
    }

    pub fn get(&self, digest: TypeObjectDigest) -> Option<&TypeObject> {
        self.objects.get(&digest)
    }

    pub fn into_objects(self) -> BTreeMap<TypeObjectDigest, TypeObject> {
        self.objects
    }

    fn insert_if_absent(
        &mut self,
        digest: TypeObjectDigest,
        object: TypeObject,
    ) -> Result<(), Diagnostic> {
        if self.objects.contains_key(&digest) {
            return Ok(());
        }
        if self.objects.len() >= self.maximum_objects {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "kernel_type_interner_exhausted",
                format!(
                    "request-local type interning exceeds the declared {}-type-object budget",
                    self.maximum_objects
                ),
            ));
        }
        self.objects.insert(digest, object);
        Ok(())
    }
}

fn require_count(label: &str, count: usize, allow_zero: bool) -> Result<(), Diagnostic> {
    if (!allow_zero && count == 0) || count > MAXIMUM_CHILDREN {
        return Err(type_error(
            "kernel_type_child_count",
            format!("{label} count {count} is outside the current type-object bound"),
        ));
    }
    Ok(())
}

fn type_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}
