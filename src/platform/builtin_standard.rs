//! Exact first-party standard package material embedded in the released executable.
//!
//! The maintained Graph 8 package owns both generated assets. This module validates the complete
//! package transport and artifact closure before exposing either one to project creation, linking,
//! inspection, or export.

use super::compiler::{LoadedArtifact, load_artifact};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::kernel::{
    DeclarationPayload, DeclarationReference, DeclarationVisibility, ExternalVisibility,
    Idempotency, OperationReference, OwnerKey, OwnerRecord, PackageId,
    PackageInterfaceDeclarationPayload, PackageInterfaceRecord, PackageRevisionDigest,
    PackageTransportDigest, ParameterParent, TypeForm, TypeObject, TypeObjectDigest,
    TypeObjectInterner,
};
use super::package_interface::PackageInterfaceOwner;
use super::package_transport::{PackageTransportBinding, validate_package_transport_closure};
use super::persistent_map::MapWork;
use super::publication::InitialPackageTransport;
use super::semantic_id::{DeclarationId, RevisionId};
use super::session::{
    SESSION_CLOSE_NAME, SESSION_DECISION_KIND_NAME, SESSION_EVENT_NAME, SESSION_MESSAGE_KIND_NAME,
    SESSION_OUTBOUND_NAME, SESSION_REJECT_NAME, SessionStandardDeclarations,
};
use super::storage::memory::MemoryPackedStore;
use super::storage::object::{StoreError, StoreErrorClass, StoreWork};
use super::storage::pack::{PackId, PackMetadata, SealedPack};
use std::collections::BTreeMap;
use std::str::FromStr;
use std::sync::OnceLock;

const STANDARD_TRANSPORT_PACK: &[u8] =
    include_bytes!("../../packages/standard/generated/standard.lkjp");
const STANDARD_ARTIFACT: &[u8] = include_bytes!("../../packages/standard/generated/standard.lkja");
const STANDARD_PACKAGE: &str = "pkg_10000000000000000000000000000001";
const STANDARD_SEMANTIC_REVISION: &str =
    "rev_c9502434e3b0ce4434fddf7ce56e18f3d7bf5a197ac242878d819554a040bdde";
const STANDARD_PACKAGE_REVISION: &str =
    "package_revision_64569dc96f354374a465c95b4287861716a57e9093c58184b425831be04da562";
const STANDARD_PACKAGE_TRANSPORT: &str =
    "package_transport_c2698e7d88f16120e6aef4215ef1704183eab1e623492021a6ccc290877b6d96";
const COMMAND_TEXT_FROM_STATIC: &str = "text-from-static";
const COMMAND_TEXT_FROM_STATIC_IMPLEMENTATION: &str = "core.text.from-static";
const HTTP_BYTES_FROM_TEXT: &str = "bytes-from-text";
const HTTP_BYTES_FROM_TEXT_IMPLEMENTATION: &str = "core.bytes.from-text";
const HTTP_MEDIA_TYPE_IS: &str = "media-type-is";
const HTTP_MEDIA_TYPE_IS_IMPLEMENTATION: &str = "core.http.media-type-is";
const HTTP_BYTE_STREAM_INTERFACE: &str = "ByteStream";
const HTTP_CLIENT_INTERFACE: &str = "HttpClient";

static BUILTIN_STANDARD: OnceLock<Result<BuiltinStandard, Diagnostic>> = OnceLock::new();

pub(crate) fn builtin_standard_package() -> Result<PackageId, Diagnostic> {
    PackageId::from_str(STANDARD_PACKAGE)
}

#[derive(Clone, Debug)]
pub struct BuiltinStandard {
    pub package: PackageId,
    pub semantic_revision: RevisionId,
    pub package_revision: PackageRevisionDigest,
    pub package_transport: PackageTransportDigest,
    pub artifact: LoadedArtifact,
    pub interface_owners: BTreeMap<OwnerKey, PackageInterfaceOwner>,
    pub interface_types: BTreeMap<TypeObjectDigest, TypeObject>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuiltinHttpRecipeContract {
    pub text_from_static: DeclarationReference,
    pub bytes_from_text: DeclarationReference,
    pub static_text_type: TypeObjectDigest,
    pub text_type: TypeObjectDigest,
    pub bytes_type: TypeObjectDigest,
    pub media_type_is: DeclarationReference,
    pub text_equal: DeclarationReference,
    pub i64_equal: DeclarationReference,
    pub bool_and: DeclarationReference,
    pub bool_or: DeclarationReference,
    pub list_fold_left: DeclarationReference,
    pub byte_stream_interface: DeclarationReference,
    pub byte_stream_read: OperationReference,
    pub byte_stream_close: OperationReference,
    pub byte_stream_read_all: OperationReference,
    pub byte_stream_operations: Vec<OperationReference>,
    pub http_client_interface: DeclarationReference,
    pub http_client_get: OperationReference,
    pub http_client_operations: Vec<OperationReference>,
}

impl BuiltinStandard {
    pub fn load() -> Result<&'static Self, Diagnostic> {
        match BUILTIN_STANDARD.get_or_init(Self::validate_embedded) {
            Ok(value) => Ok(value),
            Err(error) => Err(error.clone()),
        }
    }

    pub fn transport(&self) -> InitialPackageTransport {
        InitialPackageTransport {
            digest: self.package_transport,
            packs: vec![STANDARD_TRANSPORT_PACK.to_vec()],
        }
    }

    pub const fn transport_bytes(&self) -> &'static [u8] {
        STANDARD_TRANSPORT_PACK
    }

    pub const fn artifact_bytes(&self) -> &'static [u8] {
        STANDARD_ARTIFACT
    }

    pub(crate) fn session_contract(&self) -> Result<SessionStandardDeclarations, Diagnostic> {
        let mut declarations = BTreeMap::new();
        for (owner, value) in &self.interface_owners {
            let (OwnerKey::Declaration(declaration), PackageInterfaceRecord::Declaration(record)) =
                (owner, &value.record)
            else {
                continue;
            };
            if [
                SESSION_EVENT_NAME,
                SESSION_MESSAGE_KIND_NAME,
                SESSION_DECISION_KIND_NAME,
                SESSION_OUTBOUND_NAME,
                SESSION_REJECT_NAME,
                SESSION_CLOSE_NAME,
            ]
            .contains(&record.name.as_str())
                && declarations
                    .insert(record.name.as_str(), *declaration)
                    .is_some()
            {
                return Err(builtin_error(
                    DiagnosticClass::Corrupt,
                    "builtin_standard_session_ambiguous",
                    "built-in standard repeats a canonical session declaration",
                ));
            }
        }
        let reference = |name: &'static str| {
            declarations
                .get(name)
                .copied()
                .map(|declaration| DeclarationReference {
                    package: self.package,
                    declaration,
                })
                .ok_or_else(|| {
                    builtin_error(
                        DiagnosticClass::Corrupt,
                        "builtin_standard_session_missing",
                        format!("built-in standard omits canonical {name}"),
                    )
                })
        };
        Ok(SessionStandardDeclarations {
            event: reference(SESSION_EVENT_NAME)?,
            message_kind: reference(SESSION_MESSAGE_KIND_NAME)?,
            decision_kind: reference(SESSION_DECISION_KIND_NAME)?,
            outbound: reference(SESSION_OUTBOUND_NAME)?,
            reject: reference(SESSION_REJECT_NAME)?,
            close: reference(SESSION_CLOSE_NAME)?,
        })
    }

    pub fn command_text_from_static(&self) -> Result<DeclarationReference, Diagnostic> {
        self.external_reference_by_implementation(
            COMMAND_TEXT_FROM_STATIC,
            COMMAND_TEXT_FROM_STATIC_IMPLEMENTATION,
            "builtin_standard_command_declaration",
            "built-in standard command text constructor is absent, ambiguous, private, or foreign",
        )
    }

    pub fn interface_declaration(
        &self,
        declaration: DeclarationId,
    ) -> Result<&super::kernel::PackageInterfaceDeclaration, Diagnostic> {
        let value = self
            .interface_owners
            .get(&OwnerKey::Declaration(declaration))
            .ok_or_else(|| {
                builtin_error(
                    DiagnosticClass::Corrupt,
                    "builtin_standard_interface_declaration",
                    "built-in standard declaration is absent from its exact interface",
                )
            })?;
        match &value.record {
            PackageInterfaceRecord::Declaration(record) => Ok(record),
            _ => Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_interface_declaration",
                "built-in standard declaration is absent from its exact interface",
            )),
        }
    }

    pub fn command_text_signature(
        &self,
    ) -> Result<(DeclarationReference, TypeObjectDigest, TypeObjectDigest), Diagnostic> {
        let declaration = self.command_text_from_static()?;
        let (parameter, result) = self.external_signature(
            declaration,
            "value",
            TypeForm::StaticText,
            TypeForm::Text,
            "builtin_standard_command_signature",
            "built-in command text constructor must be public core.text.from-static with exact StaticText -> Text shape",
        )?;
        Ok((declaration, parameter, result))
    }

    pub fn http_recipe_contract(&self) -> Result<BuiltinHttpRecipeContract, Diagnostic> {
        let (text_from_static, static_text_type, text_type) = self.command_text_signature()?;
        let bytes_from_text = self.external_reference_by_implementation(
            HTTP_BYTES_FROM_TEXT,
            HTTP_BYTES_FROM_TEXT_IMPLEMENTATION,
            "builtin_standard_http_bytes_declaration",
            "built-in standard core.bytes.from-text declaration is absent, ambiguous, private, or foreign",
        )?;
        let (bytes_parameter, bytes_type) = self.external_signature(
            bytes_from_text,
            "value",
            TypeForm::Text,
            TypeForm::Bytes,
            "builtin_standard_http_bytes_signature",
            "built-in standard core.bytes.from-text must have exact Text -> Bytes shape",
        )?;
        if bytes_parameter != text_type {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_http_text_type",
                "built-in HTTP conversion declarations disagree on the exact Text type",
            ));
        }
        let stream = self.byte_stream_contract(bytes_type)?;
        let media_type_is = self.http_media_type_contract(bytes_type, text_type)?;
        let text_equal = self.external_reference_by_implementation(
            "text-equal",
            "core.text.equal",
            "builtin_standard_http_text_equal",
            "built-in standard text equality predicate is absent or foreign",
        )?;
        let i64_equal = self.external_reference_by_implementation(
            "i64-equal",
            "core.i64.equal",
            "builtin_standard_http_i64_equal",
            "built-in standard integer equality predicate is absent or foreign",
        )?;
        let bool_and = self.external_reference_by_implementation(
            "bool-and",
            "core.bool.and",
            "builtin_standard_http_bool_and",
            "built-in standard boolean conjunction is absent or foreign",
        )?;
        let bool_or = self.external_reference_by_implementation(
            "bool-or",
            "core.bool.or",
            "builtin_standard_http_bool_or",
            "built-in standard boolean disjunction is absent or foreign",
        )?;
        let list_fold_left = self.public_pure_function_reference("list-fold-left")?;
        let client = self.http_client_contract()?;
        Ok(BuiltinHttpRecipeContract {
            text_from_static,
            bytes_from_text,
            static_text_type,
            text_type,
            bytes_type,
            media_type_is,
            text_equal,
            i64_equal,
            bool_and,
            bool_or,
            list_fold_left,
            byte_stream_interface: stream.interface,
            byte_stream_read: stream.read,
            byte_stream_close: stream.close,
            byte_stream_read_all: stream.read_all,
            byte_stream_operations: stream.operations,
            http_client_interface: client.interface,
            http_client_get: client.get,
            http_client_operations: vec![client.get],
        })
    }

    fn public_pure_function_reference(
        &self,
        expected_name: &str,
    ) -> Result<DeclarationReference, Diagnostic> {
        let mut selected = None;
        for (owner, value) in &self.interface_owners {
            let OwnerKey::Declaration(declaration) = owner else {
                continue;
            };
            let PackageInterfaceRecord::Declaration(interface) = &value.record else {
                continue;
            };
            if interface.name.as_str() != expected_name
                || !matches!(
                    interface.payload,
                    PackageInterfaceDeclarationPayload::Function(_)
                )
            {
                continue;
            }
            let mut map_work = MapWork::default();
            let mut store_work = StoreWork::default();
            let Some(OwnerRecord::Declaration(canonical)) = self.artifact.reference_owner(
                self.package,
                OwnerKey::Declaration(*declaration),
                &mut map_work,
                &mut store_work,
            )?
            else {
                return Err(builtin_error(
                    DiagnosticClass::Corrupt,
                    "builtin_standard_http_function",
                    "built-in standard pure helper is absent from canonical authority",
                ));
            };
            if canonical.visibility != DeclarationVisibility::Public
                || !matches!(canonical.payload, DeclarationPayload::Function(_))
                || selected.replace(*declaration).is_some()
            {
                return Err(builtin_error(
                    DiagnosticClass::Corrupt,
                    "builtin_standard_http_function",
                    "built-in standard pure helper is ambiguous, private, or foreign",
                ));
            }
        }
        selected
            .map(|declaration| DeclarationReference {
                package: self.package,
                declaration,
            })
            .ok_or_else(|| {
                builtin_error(
                    DiagnosticClass::Corrupt,
                    "builtin_standard_http_function",
                    "built-in standard pure helper is absent",
                )
            })
    }

    fn http_media_type_contract(
        &self,
        bytes_type: TypeObjectDigest,
        text_type: TypeObjectDigest,
    ) -> Result<DeclarationReference, Diagnostic> {
        let declaration = self.external_reference_by_implementation(
            HTTP_MEDIA_TYPE_IS,
            HTTP_MEDIA_TYPE_IS_IMPLEMENTATION,
            "builtin_standard_http_media_declaration",
            "built-in standard media-type predicate is absent, ambiguous, private, or foreign",
        )?;
        let record = self.interface_declaration(declaration.declaration)?;
        let PackageInterfaceDeclarationPayload::External(signature) = &record.payload else {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_http_media_signature",
                "built-in standard media-type predicate is not an exact external declaration",
            ));
        };
        let mut interner = TypeObjectInterner::default();
        let bool_type = interner.intern(TypeForm::Bool)?;
        if !signature.type_parameters.is_empty()
            || signature.result != bool_type
            || signature.parameters.len() != 2
        {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_http_media_signature",
                "built-in standard media-type predicate must have exact Bytes, Text -> Bool shape",
            ));
        }
        for (parameter, (name, ty)) in signature
            .parameters
            .iter()
            .zip([("value", bytes_type), ("expected", text_type)])
        {
            match self
                .interface_owners
                .get(&OwnerKey::Parameter(*parameter))
                .map(|value| &value.record)
            {
                Some(PackageInterfaceRecord::Parameter(parameter))
                    if parameter.parent == ParameterParent::Function(declaration.declaration)
                        && parameter.name.as_str() == name
                        && parameter.ty == ty => {}
                _ => {
                    return Err(builtin_error(
                        DiagnosticClass::Corrupt,
                        "builtin_standard_http_media_parameter",
                        "built-in standard media-type predicate has a foreign exact parameter",
                    ));
                }
            }
        }
        Ok(declaration)
    }

    fn http_client_contract(&self) -> Result<HttpClientContract, Diagnostic> {
        let mut declarations = self.interface_owners.iter().filter_map(|(owner, value)| {
            let OwnerKey::Declaration(declaration) = owner else {
                return None;
            };
            let PackageInterfaceRecord::Declaration(record) = &value.record else {
                return None;
            };
            (record.name.as_str() == HTTP_CLIENT_INTERFACE).then_some((*declaration, record))
        });
        let (declaration, record) = declarations.next().ok_or_else(|| {
            builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_http_client_interface",
                "built-in standard interface omits its exact HttpClient declaration",
            )
        })?;
        if declarations.next().is_some()
            || declaration.to_string() != "decl_f1084ba5dca02ba338140747d0ea9d46"
        {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_http_client_interface",
                "built-in standard HttpClient declaration is ambiguous or has a foreign identity",
            ));
        }
        let PackageInterfaceDeclarationPayload::Interface { operations } = &record.payload else {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_http_client_interface",
                "built-in standard HttpClient locator names another declaration kind",
            ));
        };
        let [operation] = operations.as_slice() else {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_http_client_operation",
                "built-in standard HttpClient must expose exactly get",
            ));
        };
        let mut interner = TypeObjectInterner::default();
        let types = super::http_client::semantic_http_client_types(&mut interner)?;
        for (digest, object) in interner.into_objects() {
            if self.interface_types.get(&digest) != Some(&object) {
                return Err(builtin_error(
                    DiagnosticClass::Corrupt,
                    "builtin_standard_http_client_types",
                    "built-in standard HttpClient omits an exact canonical type object",
                ));
            }
        }
        let operation_record = match self
            .interface_owners
            .get(&OwnerKey::Operation(*operation))
            .map(|value| &value.record)
        {
            Some(PackageInterfaceRecord::Operation(operation_record)) => operation_record,
            _ => {
                return Err(builtin_error(
                    DiagnosticClass::Corrupt,
                    "builtin_standard_http_client_operation",
                    "built-in standard HttpClient get operation is absent",
                ));
            }
        };
        if operation_record.declaration != declaration
            || operation_record.name.as_str() != "get"
            || operation_record.idempotency != Idempotency::Idempotent
            || operation_record.external_visibility != ExternalVisibility::Possible
            || operation_record.result != types.response_type
            || operation_record.parameters.len() != 1
        {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_http_client_operation",
                "built-in standard HttpClient get operation has a foreign policy or signature",
            ));
        }
        let parameter = operation_record.parameters[0];
        match self
            .interface_owners
            .get(&OwnerKey::Parameter(parameter))
            .map(|value| &value.record)
        {
            Some(PackageInterfaceRecord::Parameter(parameter))
                if parameter.parent == ParameterParent::Operation(*operation)
                    && parameter.name.as_str() == "headers"
                    && parameter.ty == types.header_list_type => {}
            _ => {
                return Err(builtin_error(
                    DiagnosticClass::Corrupt,
                    "builtin_standard_http_client_parameter",
                    "built-in standard HttpClient get parameter has a foreign exact shape",
                ));
            }
        }
        Ok(HttpClientContract {
            interface: DeclarationReference {
                package: self.package,
                declaration,
            },
            get: OperationReference {
                package: self.package,
                operation: *operation,
            },
        })
    }

    fn external_reference_by_implementation(
        &self,
        expected_name: &str,
        expected_implementation: &str,
        code: &'static str,
        message: &'static str,
    ) -> Result<DeclarationReference, Diagnostic> {
        let mut selected = None;
        for (owner, value) in &self.interface_owners {
            let OwnerKey::Declaration(declaration) = owner else {
                continue;
            };
            let PackageInterfaceRecord::Declaration(interface) = &value.record else {
                continue;
            };
            if !matches!(
                interface.payload,
                PackageInterfaceDeclarationPayload::External(_)
            ) {
                continue;
            }
            let mut map_work = MapWork::default();
            let mut store_work = StoreWork::default();
            let Some(OwnerRecord::Declaration(canonical)) = self.artifact.reference_owner(
                self.package,
                OwnerKey::Declaration(*declaration),
                &mut map_work,
                &mut store_work,
            )?
            else {
                return Err(builtin_error(DiagnosticClass::Corrupt, code, message));
            };
            let DeclarationPayload::External(external) = &canonical.payload else {
                return Err(builtin_error(DiagnosticClass::Corrupt, code, message));
            };
            if external.implementation.as_str() != expected_implementation {
                continue;
            }
            if canonical.visibility != DeclarationVisibility::Public
                || canonical.name.as_str() != expected_name
                || interface.name != canonical.name
                || selected.replace(*declaration).is_some()
            {
                return Err(builtin_error(DiagnosticClass::Corrupt, code, message));
            }
        }
        selected
            .map(|declaration| DeclarationReference {
                package: self.package,
                declaration,
            })
            .ok_or_else(|| builtin_error(DiagnosticClass::Corrupt, code, message))
    }

    #[allow(clippy::too_many_arguments)]
    fn external_signature(
        &self,
        declaration: DeclarationReference,
        parameter_name: &str,
        parameter_form: TypeForm,
        result_form: TypeForm,
        code: &'static str,
        message: &'static str,
    ) -> Result<(TypeObjectDigest, TypeObjectDigest), Diagnostic> {
        let record = self.interface_declaration(declaration.declaration)?;
        let PackageInterfaceDeclarationPayload::External(signature) = &record.payload else {
            return Err(builtin_error(DiagnosticClass::Corrupt, code, message));
        };
        if !signature.type_parameters.is_empty() {
            return Err(builtin_error(DiagnosticClass::Corrupt, code, message));
        }
        let [parameter] = signature.parameters.as_slice() else {
            return Err(builtin_error(DiagnosticClass::Corrupt, code, message));
        };
        let parameter = match self
            .interface_owners
            .get(&OwnerKey::Parameter(*parameter))
            .map(|value| &value.record)
        {
            Some(PackageInterfaceRecord::Parameter(parameter))
                if parameter.parent == ParameterParent::Function(declaration.declaration)
                    && parameter.name.as_str() == parameter_name =>
            {
                parameter
            }
            _ => return Err(builtin_error(DiagnosticClass::Corrupt, code, message)),
        };
        if !self.type_has_form(parameter.ty, &parameter_form)
            || !self.type_has_form(signature.result, &result_form)
        {
            return Err(builtin_error(DiagnosticClass::Corrupt, code, message));
        }
        Ok((parameter.ty, signature.result))
    }

    fn type_has_form(&self, digest: TypeObjectDigest, expected: &TypeForm) -> bool {
        self.interface_types
            .get(&digest)
            .is_some_and(|object| &object.form == expected)
    }

    fn byte_stream_contract(
        &self,
        bytes_type: TypeObjectDigest,
    ) -> Result<ByteStreamContract, Diagnostic> {
        let mut declarations = self.interface_owners.iter().filter_map(|(owner, value)| {
            let OwnerKey::Declaration(declaration) = owner else {
                return None;
            };
            let PackageInterfaceRecord::Declaration(record) = &value.record else {
                return None;
            };
            (record.name.as_str() == HTTP_BYTE_STREAM_INTERFACE).then_some((*declaration, record))
        });
        let (declaration, record) = declarations.next().ok_or_else(|| {
            builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_http_stream_interface",
                "built-in standard interface omits its exact ByteStream declaration",
            )
        })?;
        if declarations.next().is_some() {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_http_stream_interface",
                "built-in standard interface has an ambiguous ByteStream declaration",
            ));
        }
        let PackageInterfaceDeclarationPayload::Interface { operations } = &record.payload else {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_http_stream_interface",
                "built-in standard ByteStream locator names another declaration kind",
            ));
        };
        if operations.len() != 3 {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_http_stream_operations",
                "built-in standard ByteStream interface must expose exactly read, close, and read-all",
            ));
        }

        let mut interner = TypeObjectInterner::default();
        let canonical_bytes = interner.intern(TypeForm::Bytes)?;
        let unit_type = interner.intern(TypeForm::Unit)?;
        let bool_type = interner.intern(TypeForm::Bool)?;
        let i64_type = interner.intern(TypeForm::I64)?;
        if canonical_bytes != bytes_type {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_http_stream_bytes",
                "built-in standard ByteStream and core.bytes.from-text disagree on Bytes",
            ));
        }
        let stream_type = interner.intern(TypeForm::Stream { item: bytes_type })?;
        let read_result = interner.intern(TypeForm::StructuralRecord {
            fields: vec![
                super::kernel::StructuralTypeField {
                    name: super::kernel::Name::new("chunk")?,
                    ty: bytes_type,
                },
                super::kernel::StructuralTypeField {
                    name: super::kernel::Name::new("done")?,
                    ty: bool_type,
                },
            ],
        })?;
        for (digest, object) in interner.into_objects() {
            if self.interface_types.get(&digest) != Some(&object) {
                return Err(builtin_error(
                    DiagnosticClass::Corrupt,
                    "builtin_standard_http_stream_types",
                    "built-in standard ByteStream interface omits an exact canonical type object",
                ));
            }
        }

        let mut read = None;
        let mut close = None;
        let mut read_all = None;
        for operation in operations {
            let record = match self
                .interface_owners
                .get(&OwnerKey::Operation(*operation))
                .map(|value| &value.record)
            {
                Some(PackageInterfaceRecord::Operation(record))
                    if record.declaration == declaration
                        && record.external_visibility == ExternalVisibility::None =>
                {
                    record
                }
                _ => {
                    return Err(builtin_error(
                        DiagnosticClass::Corrupt,
                        "builtin_standard_http_stream_operation",
                        "built-in standard ByteStream operation ownership or policy is invalid",
                    ));
                }
            };
            let expected = match record.name.as_str() {
                "read" if read.replace(*operation).is_none() => (
                    &[("stream", stream_type)][..],
                    read_result,
                    Idempotency::NonIdempotent,
                ),
                "close" if close.replace(*operation).is_none() => (
                    &[("stream", stream_type)][..],
                    unit_type,
                    Idempotency::Idempotent,
                ),
                "read-all" if read_all.replace(*operation).is_none() => (
                    &[("stream", stream_type), ("maximum-bytes", i64_type)][..],
                    bytes_type,
                    Idempotency::NonIdempotent,
                ),
                _ => {
                    return Err(builtin_error(
                        DiagnosticClass::Corrupt,
                        "builtin_standard_http_stream_operation",
                        "built-in standard ByteStream operation set is foreign or ambiguous",
                    ));
                }
            };
            if record.result != expected.1
                || record.idempotency != expected.2
                || record.parameters.len() != expected.0.len()
            {
                return Err(builtin_error(
                    DiagnosticClass::Corrupt,
                    "builtin_standard_http_stream_signature",
                    "built-in standard ByteStream operation has a foreign exact signature",
                ));
            }
            for (parameter, (name, ty)) in record.parameters.iter().zip(expected.0) {
                match self
                    .interface_owners
                    .get(&OwnerKey::Parameter(*parameter))
                    .map(|value| &value.record)
                {
                    Some(PackageInterfaceRecord::Parameter(parameter))
                        if parameter.parent == ParameterParent::Operation(*operation)
                            && parameter.name.as_str() == *name
                            && parameter.ty == *ty => {}
                    _ => {
                        return Err(builtin_error(
                            DiagnosticClass::Corrupt,
                            "builtin_standard_http_stream_parameter",
                            "built-in standard ByteStream parameter has a foreign exact shape",
                        ));
                    }
                }
            }
        }
        let (Some(read), Some(close), Some(read_all)) = (read, close, read_all) else {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_http_stream_operations",
                "built-in standard ByteStream interface is incomplete",
            ));
        };
        let mut selected = vec![
            OperationReference {
                package: self.package,
                operation: read,
            },
            OperationReference {
                package: self.package,
                operation: close,
            },
            OperationReference {
                package: self.package,
                operation: read_all,
            },
        ];
        selected.sort();
        Ok(ByteStreamContract {
            interface: DeclarationReference {
                package: self.package,
                declaration,
            },
            read: OperationReference {
                package: self.package,
                operation: read,
            },
            close: OperationReference {
                package: self.package,
                operation: close,
            },
            read_all: OperationReference {
                package: self.package,
                operation: read_all,
            },
            operations: selected,
        })
    }

    fn validate_embedded() -> Result<Self, Diagnostic> {
        let package = PackageId::from_str(STANDARD_PACKAGE)?;
        let semantic_revision = RevisionId::from_str(STANDARD_SEMANTIC_REVISION)?;
        let package_revision = PackageRevisionDigest::from_str(STANDARD_PACKAGE_REVISION)?;
        let package_transport = PackageTransportDigest::from_str(STANDARD_PACKAGE_TRANSPORT)?;

        let metadata =
            PackMetadata::decode(STANDARD_TRANSPORT_PACK, true).map_err(store_diagnostic)?;
        let mut store = MemoryPackedStore::default();
        store
            .install(SealedPack {
                id: PackId::of(STANDARD_TRANSPORT_PACK),
                bytes: STANDARD_TRANSPORT_PACK.to_vec(),
                metadata,
            })
            .map_err(store_diagnostic)?;
        let duplicates = store.rebuild_catalog().map_err(store_diagnostic)?;
        if !duplicates.is_empty() {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_transport_duplicate",
                "built-in standard transport repeats an immutable object",
            ));
        }
        let mut work = StoreWork::default();
        let validated = validate_package_transport_closure(
            &store,
            package_revision,
            &[PackageTransportBinding {
                package_revision,
                transport: package_transport,
            }],
            None,
            &mut work,
        )?;
        let revision = validated.root_revision;
        if validated.root_transport_digest != package_transport
            || revision.package != package
            || revision.revision.revision_id()? != semantic_revision
            || revision.encode()?.0 != package_revision
            || !revision.dependencies.is_empty()
        {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_transport_binding",
                "built-in standard transport disagrees with its exact package identities",
            ));
        }

        let artifact = load_artifact(STANDARD_ARTIFACT)?;
        let root = artifact.root_package().ok_or_else(|| {
            builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_artifact_root",
                "built-in standard artifact has no exact root package",
            )
        })?;
        if artifact.manifest.root_package != package
            || artifact.manifest.packages.len() != 1
            || root.package_revision != package_revision
            || root.semantic_revision != semantic_revision
        {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_artifact_binding",
                "built-in standard artifact disagrees with its validated package transport",
            ));
        }
        let value = Self {
            package,
            semantic_revision,
            package_revision,
            package_transport,
            artifact,
            interface_owners: validated.root_interface.owners,
            interface_types: validated.root_interface.type_objects,
        };
        let _ = value.command_text_signature()?;
        let _ = value.http_recipe_contract()?;
        Ok(value)
    }
}

struct ByteStreamContract {
    interface: DeclarationReference,
    read: OperationReference,
    close: OperationReference,
    read_all: OperationReference,
    operations: Vec<OperationReference>,
}

struct HttpClientContract {
    interface: DeclarationReference,
    get: OperationReference,
}

fn store_diagnostic(error: StoreError) -> Diagnostic {
    let class = match error.class {
        StoreErrorClass::Input => DiagnosticClass::Source,
        StoreErrorClass::Resource => DiagnosticClass::Resource,
        StoreErrorClass::Corrupt => DiagnosticClass::Corrupt,
        StoreErrorClass::Io => DiagnosticClass::Infrastructure,
    };
    builtin_error(class, error.code, error.message)
}

fn builtin_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::compiler::{OptimizationPolicy, build_clean, link_artifact};
    use crate::platform::publication::GraphRepository;
    use std::path::PathBuf;

    #[test]
    fn embedded_standard_transport_and_artifact_bind_one_exact_package() {
        let standard = BuiltinStandard::load().expect("validate embedded standard");
        assert_eq!(standard.package.to_string(), STANDARD_PACKAGE);
        assert_eq!(
            standard.semantic_revision.to_string(),
            STANDARD_SEMANTIC_REVISION
        );
        assert_eq!(
            standard.package_revision.to_string(),
            STANDARD_PACKAGE_REVISION
        );
        assert_eq!(
            standard.package_transport.to_string(),
            STANDARD_PACKAGE_TRANSPORT
        );
        let constructor = standard
            .command_text_from_static()
            .expect("exact command constructor");
        assert_eq!(constructor.package, standard.package);

        let http = standard
            .http_recipe_contract()
            .expect("exact HTTP recipe contract");
        assert_eq!(http.text_from_static, constructor);
        assert_eq!(http.bytes_from_text.package, standard.package);
        assert_eq!(http.byte_stream_interface.package, standard.package);
        assert_eq!(http.byte_stream_operations.len(), 3);
        assert_eq!(http.byte_stream_operations, {
            let mut operations = vec![
                http.byte_stream_read,
                http.byte_stream_close,
                http.byte_stream_read_all,
            ];
            operations.sort();
            operations
        });
        assert!(matches!(
            standard.interface_types.get(&http.static_text_type),
            Some(TypeObject {
                form: TypeForm::StaticText,
                ..
            })
        ));
        assert!(matches!(
            standard.interface_types.get(&http.text_type),
            Some(TypeObject {
                form: TypeForm::Text,
                ..
            })
        ));
        assert!(matches!(
            standard.interface_types.get(&http.bytes_type),
            Some(TypeObject {
                form: TypeForm::Bytes,
                ..
            })
        ));
    }

    #[test]
    fn http_recipe_resolution_rejects_interface_shape_drift() {
        let mut standard = BuiltinStandard::load()
            .expect("validate embedded standard")
            .clone();
        let contract = standard
            .http_recipe_contract()
            .expect("exact HTTP recipe contract");
        let operation = contract.byte_stream_read;
        let PackageInterfaceRecord::Operation(record) = &mut standard
            .interface_owners
            .get_mut(&OwnerKey::Operation(operation.operation))
            .expect("read operation")
            .record
        else {
            panic!("read operation must retain its exact owner kind")
        };
        record.result = contract.bytes_type;
        let error = standard
            .http_recipe_contract()
            .expect_err("foreign read result must reject");
        assert_eq!(error.code, "builtin_standard_http_stream_signature");
    }

    #[test]
    fn maintained_standard_is_the_byte_owner_of_embedded_assets() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("packages/standard");
        let repository = GraphRepository::open(&root).expect("open maintained Graph 8 standard");
        let exported = repository
            .export_package_transport()
            .expect("export maintained standard transport");
        let [pack] = exported.packs.as_slice() else {
            panic!("maintained standard transport must contain one exact bounded pack");
        };
        assert_eq!(pack.as_slice(), STANDARD_TRANSPORT_PACK);
        assert_eq!(
            exported.transport_digest.to_string(),
            STANDARD_PACKAGE_TRANSPORT
        );

        let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline)
            .expect("clean compile maintained standard");
        let linked = link_artifact(&repository, compilation.manifest_digest, &[])
            .expect("link maintained standard");
        assert_eq!(linked.artifact.bytes.as_slice(), STANDARD_ARTIFACT);
    }
}
