//! Content-addressed, structural Graph 5 type objects.

use super::contract::{GRAPH_CONTRACT_VERSION, MAXIMUM_CHILDREN};
use super::digest::TypeObjectDigest;
use super::name::Name;
use super::reference::DeclarationReference;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::semantic_id::TypeParameterId;
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
            contract_version: GRAPH_CONTRACT_VERSION,
            form,
        };
        object.validate_local()?;
        Ok(object)
    }

    pub(crate) fn validate_local(&self) -> Result<(), Diagnostic> {
        if self.contract_version != GRAPH_CONTRACT_VERSION {
            return Err(type_error(
                "kernel_type_contract",
                format!(
                    "type object contract {} is not Graph Contract {GRAPH_CONTRACT_VERSION}",
                    self.contract_version
                ),
            ));
        }
        match &self.form {
            TypeForm::StructuralRecord { fields } => {
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
            TypeForm::Unit
            | TypeForm::Bool
            | TypeForm::I64
            | TypeForm::Bytes
            | TypeForm::Text
            | TypeForm::StaticText
            | TypeForm::Secret
            | TypeForm::TypeParameter { .. }
            | TypeForm::Named { .. }
            | TypeForm::List { .. }
            | TypeForm::Map { .. }
            | TypeForm::Option { .. }
            | TypeForm::Result { .. }
            | TypeForm::Stream { .. } => {}
        }
        Ok(())
    }

    pub fn child_types(&self) -> Vec<TypeObjectDigest> {
        match &self.form {
            TypeForm::StructuralRecord { fields } => fields.iter().map(|field| field.ty).collect(),
            TypeForm::List { item } | TypeForm::Option { item } | TypeForm::Stream { item } => {
                vec![*item]
            }
            TypeForm::Map { key, value }
            | TypeForm::Result {
                ok: key,
                error: value,
            } => {
                vec![*key, *value]
            }
            TypeForm::Function { parameters, result } => {
                let mut children = parameters.clone();
                children.push(*result);
                children
            }
            TypeForm::Unit
            | TypeForm::Bool
            | TypeForm::I64
            | TypeForm::Bytes
            | TypeForm::Text
            | TypeForm::StaticText
            | TypeForm::Secret
            | TypeForm::TypeParameter { .. }
            | TypeForm::Named { .. } => Vec::new(),
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
            format!("{label} count {count} is outside the Graph 5 bound"),
        ));
    }
    Ok(())
}

fn type_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Semantic, code, message)
}
