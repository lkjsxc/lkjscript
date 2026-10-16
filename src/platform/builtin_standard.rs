//! Exact first-party standard package material embedded in the released executable.
//!
//! The maintained standard package owns both generated assets. This module validates the complete
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
use super::package_transport::PackageTransportBinding;
use super::package_transport::source::AdmittedClosure;
use super::package_transport::validate_package_transport_closure;
use super::persistent_map::MapWork;
use super::publication::InitialPackageTransport;
use super::semantic_id::{DeclarationId, RevisionId};
use super::session::{
    SESSION_CLOSE_NAME, SESSION_DECISION_KIND_NAME, SESSION_EVENT_NAME, SESSION_MESSAGE_KIND_NAME,
    SESSION_OUTBOUND_NAME, SESSION_REJECT_NAME, SessionStandardDeclarations,
};
use super::storage::object::StoreWork;
use std::collections::BTreeMap;
use std::io::Read;
use std::str::FromStr;
use std::sync::{Arc, Mutex, OnceLock, TryLockError};

#[path = "builtin_standard_retention.rs"]
mod retention;

const STANDARD_TRANSPORT_COMPRESSED: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/standard.lkjp.deflate"));
const STANDARD_ARTIFACT_COMPRESSED: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/standard.lkja.deflate"));
const STANDARD_PACKAGE: &str = "pkg_10000000000000000000000000000001";
const STANDARD_SEMANTIC_REVISION: &str =
    "rev_2c307fbdefe23a178a2762148447fdac853f8e9b198b53294da8302f28270ced";
const STANDARD_PACKAGE_REVISION: &str =
    "package_revision_438773e160af5a00bdd72e4c576286d9add6bf5e9948e7b013f37b3228f9b591";
const STANDARD_PACKAGE_TRANSPORT: &str =
    "package_transport_a0646e9a61973a84b9b29bebf2d3f520a5cb981921b5637e3a38f3956b93bbc7";
const COMMAND_TEXT_FROM_STATIC: &str = "text-from-static";
const COMMAND_TEXT_FROM_STATIC_IMPLEMENTATION: &str = "core.text.from-static";
const HTTP_BYTES_FROM_TEXT: &str = "bytes-from-text";
const HTTP_BYTES_FROM_TEXT_IMPLEMENTATION: &str = "core.bytes.from-text";
const HTTP_MEDIA_TYPE_IS: &str = "media-type-is";
const HTTP_MEDIA_TYPE_IS_IMPLEMENTATION: &str = "core.http.media-type-is";
const HTTP_BYTE_STREAM_INTERFACE: &str = "ByteStream";
const HTTP_CLIENT_INTERFACE: &str = "HttpClient";

static BUILTIN_STANDARD: OnceLock<Result<BuiltinStandard, Diagnostic>> = OnceLock::new();
const MAXIMUM_RETAINED_BUILTIN_BYTES: usize = 64 * 1_048_576;
const MAXIMUM_RETAINED_BUILTIN_OBJECTS: usize = 100_000;

#[cfg(test)]
std::thread_local! {
    static SHARED_SOURCE_DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(crate) fn without_shared_source<T>(run: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            SHARED_SOURCE_DISABLED.with(|disabled| disabled.set(self.0));
        }
    }
    let _reset = Reset(SHARED_SOURCE_DISABLED.with(|disabled| disabled.replace(true)));
    run()
}

pub(crate) fn builtin_standard_package() -> Result<PackageId, Diagnostic> {
    PackageId::from_str(STANDARD_PACKAGE)
}

#[derive(Clone, Debug)]
pub struct BuiltinStandard {
    transport_bytes: Arc<[u8]>,
    artifact_bytes: Arc<[u8]>,
    admitted_source: Arc<AdmittedClosure>,
    independent_source: Arc<super::package_transport::oracle::OracleClosure>,
    source_epoch: u16,
    source_implementation: [u8; 32],
    artifact_admission: [u8; 32],
    program: Arc<Mutex<Option<Arc<super::execution::normalized::NormalizedProgram>>>>,
    retained_material: retention::RetainedMaterial,
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
            container: self.transport_bytes.to_vec(),
        }
    }

    pub fn transport_bytes(&self) -> &[u8] {
        &self.transport_bytes
    }

    pub fn artifact_bytes(&self) -> &[u8] {
        &self.artifact_bytes
    }

    /// A nonblocking lookup: source admission also runs inside builtin initialization.
    /// Exact historical transports never resolve to the current embedded package.
    pub(crate) fn initialized_for(binding: PackageTransportBinding) -> Option<&'static Self> {
        #[cfg(test)]
        if SHARED_SOURCE_DISABLED.with(std::cell::Cell::get) {
            return None;
        }
        let standard = BUILTIN_STANDARD.get()?.as_ref().ok()?;
        standard
            .matches_source(
                binding,
                super::canonical_schema::SCHEMA_EPOCH,
                super::canonical_schema::VALIDATOR_IMPLEMENTATION_HASH,
            )
            .then_some(standard)
    }

    fn matches_source(
        &self,
        binding: PackageTransportBinding,
        epoch: u16,
        implementation: [u8; 32],
    ) -> bool {
        binding == self.admitted_source.container.root
            && self.source_epoch == epoch
            && self.source_implementation == implementation
    }

    pub(crate) fn admitted_source(&self) -> &Arc<AdmittedClosure> {
        &self.admitted_source
    }

    pub(crate) fn independent_source(
        &self,
    ) -> &Arc<super::package_transport::oracle::OracleClosure> {
        &self.independent_source
    }

    /// The builtin has one immutable dense program per process. Invocation resources,
    /// policies, grants and cancellation remain with each caller, never in this cache.
    #[cfg(test)]
    pub(crate) fn prepared_program(
        &self,
    ) -> Result<Arc<super::execution::normalized::NormalizedProgram>, Diagnostic> {
        self.prepared_program_with_control(&super::execution::ExecutionControl::uncancelled())
    }

    fn prepared_program_with_control(
        &self,
        control: &super::execution::ExecutionControl,
    ) -> Result<Arc<super::execution::normalized::NormalizedProgram>, Diagnostic> {
        let checkpoint = || {
            control
                .check()
                .map_err(super::execution::normalized::execution_diagnostic)
        };
        checkpoint()?;
        self.retained_material.require_within(
            MAXIMUM_RETAINED_BUILTIN_BYTES,
            MAXIMUM_RETAINED_BUILTIN_OBJECTS,
        )?;
        if self.artifact.admitted_content_digest(checkpoint)? != self.artifact_admission {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_program_binding",
                "builtin program requires its exact independently compiled artifact",
            ));
        }
        let mut retained = loop {
            checkpoint()?;
            match self.program.try_lock() {
                Ok(retained) => break retained,
                Err(TryLockError::WouldBlock) => {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(TryLockError::Poisoned(_)) => {
                    return Err(builtin_error(
                        DiagnosticClass::Corrupt,
                        "builtin_standard_program_binding",
                        "shared builtin program preparation was interrupted by an internal panic",
                    ));
                }
            }
        };
        if let Some(program) = retained.as_ref() {
            return Ok(Arc::clone(program));
        }
        let program = super::execution::normalized::NormalizedProgram::prepare_with_control(
            self.artifact.clone(),
            control,
        )?;
        retention::program_material(self.artifact_bytes.len(), &program)
            .ok_or_else(builtin_retention_limit)?
            .require_within(
                retention::PROGRAM_MATERIAL_RESERVATION,
                MAXIMUM_RETAINED_BUILTIN_OBJECTS
                    .checked_sub(self.retained_material.records)
                    .ok_or_else(builtin_retention_limit)?,
            )?;
        checkpoint()?;
        let program = Arc::new(program);
        *retained = Some(Arc::clone(&program));
        Ok(program)
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
        let transport_bytes = inflate_embedded(
            STANDARD_TRANSPORT_COMPRESSED,
            super::canonical_schema::STANDARD_TRANSPORT_DECOMPRESSED_BYTES,
        )?;
        let artifact_bytes = inflate_embedded(
            STANDARD_ARTIFACT_COMPRESSED,
            super::canonical_schema::STANDARD_ARTIFACT_DECOMPRESSED_BYTES,
        )?;
        if transport_bytes.len() > MAXIMUM_RETAINED_BUILTIN_BYTES
            || artifact_bytes.len() > MAXIMUM_RETAINED_BUILTIN_BYTES
        {
            return Err(builtin_retention_limit());
        }
        let package = PackageId::from_str(STANDARD_PACKAGE)?;
        let semantic_revision = RevisionId::from_str(STANDARD_SEMANTIC_REVISION)?;
        let package_revision = PackageRevisionDigest::from_str(STANDARD_PACKAGE_REVISION)?;
        let package_transport = PackageTransportDigest::from_str(STANDARD_PACKAGE_TRANSPORT)?;

        let container = crate::platform::package_transport::source::PackageContainer::decode(
            &transport_bytes,
            package_transport,
        )?;
        if container.objects.len() > MAXIMUM_RETAINED_BUILTIN_OBJECTS {
            return Err(builtin_retention_limit());
        }
        let admitted = Arc::new(container.admit()?);
        // This independent decoder never consumes the production admission verdict. Its
        // own complete read set and private-body checks prove the same original input.
        let independent_source =
            Arc::new(super::package_transport::oracle::reconstruct(&container)?);
        if admitted.packages.len() != 1 || independent_source.snapshots.len() != 1 {
            return Err(builtin_retention_limit());
        }
        let mut work = StoreWork::default();
        let validated = validate_package_transport_closure(
            &container.objects,
            package_revision,
            &container.selections,
            None,
            &mut work,
        )?;
        let source = admitted.packages.get(&package_revision).ok_or_else(|| {
            builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_source",
                "embedded source omits its selected package",
            )
        })?;
        let clean = super::compiler::compile_immutable(source, &container.objects, &[])?;
        if clean.artifact.bytes.as_slice() != artifact_bytes.as_ref() {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_source_artifact",
                "embedded derived artifact differs from clean compilation of its exact canonical source",
            ));
        }
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

        let artifact = load_artifact(&artifact_bytes)?;
        let artifact_admission = artifact.admitted_content_digest(|| Ok(()))?;
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
        let public_interface_records = validated
            .root_interface
            .owners
            .len()
            .checked_add(validated.root_interface.type_objects.len())
            .ok_or_else(builtin_retention_limit)?;
        let retained_material = retention::retained_material(
            transport_bytes.len(),
            artifact_bytes.len(),
            &admitted,
            &independent_source,
            &artifact,
            public_interface_records,
        )
        .ok_or_else(builtin_retention_limit)?
        .require_within(
            MAXIMUM_RETAINED_BUILTIN_BYTES,
            MAXIMUM_RETAINED_BUILTIN_OBJECTS,
        )?;
        let value = Self {
            transport_bytes,
            artifact_bytes,
            admitted_source: admitted,
            independent_source,
            source_epoch: super::canonical_schema::SCHEMA_EPOCH,
            source_implementation: super::canonical_schema::VALIDATOR_IMPLEMENTATION_HASH,
            artifact_admission,
            program: Arc::new(Mutex::new(None)),
            retained_material,
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

fn builtin_retention_limit() -> Diagnostic {
    builtin_error(
        DiagnosticClass::Resource,
        "builtin_standard_retention_limit",
        "shared builtin exceeds its single-package, 100000-record or aggregate 64 MiB modeled retained-material bounds",
    )
}

/// Called only after strict fresh artifact admission. Exact bytes, implementation and
/// the private admitted-content binding must agree before sharing the embedded program.
pub(crate) fn prepare_admitted_program(
    bytes: &[u8],
    artifact: LoadedArtifact,
    control: &super::execution::ExecutionControl,
) -> Result<Arc<super::execution::normalized::NormalizedProgram>, Diagnostic> {
    control
        .check()
        .map_err(super::execution::normalized::execution_diagnostic)?;
    if let Some(Ok(standard)) = BUILTIN_STANDARD.get()
        && standard.artifact_bytes() == bytes
        && BuiltinStandard::initialized_for(standard.admitted_source.container.root).is_some()
        && artifact.admitted_content_digest(|| {
            control
                .check()
                .map_err(super::execution::normalized::execution_diagnostic)
        })? == standard.artifact_admission
    {
        return standard.prepared_program_with_control(control);
    }
    super::execution::normalized::NormalizedProgram::prepare_with_control(artifact, control)
        .map(Arc::new)
}

#[cfg(test)]
#[path = "builtin_standard_shared_tests.rs"]
mod shared_source_tests;

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

fn inflate_embedded(bytes: &[u8], expected: usize) -> Result<Arc<[u8]>, Diagnostic> {
    if expected > super::packed::MAXIMUM_PACKED_PAYLOAD_BYTES {
        return Err(builtin_error(
            DiagnosticClass::Resource,
            "builtin_standard_compressed_limit",
            "embedded standard exceeds its decompressed byte bound",
        ));
    }
    let mut decoder = flate2::bufread::DeflateDecoder::new(bytes);
    let mut decoded = Vec::with_capacity(expected);
    decoder
        .by_ref()
        .take(expected as u64 + 1)
        .read_to_end(&mut decoded)
        .map_err(|error| {
            builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_compressed_decode",
                format!("embedded standard decompression failed: {error}"),
            )
        })?;
    if decoded.len() != expected || decoder.total_in() != bytes.len() as u64 {
        return Err(builtin_error(
            DiagnosticClass::Corrupt,
            "builtin_standard_compressed_length",
            "embedded standard has a foreign decompressed length or trailing compressed bytes",
        ));
    }
    Ok(decoded.into())
}

fn builtin_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
pub(crate) fn copy_maintained_repository(source: &std::path::Path, destination: &std::path::Path) {
    let source_metadata = std::fs::symlink_metadata(source).expect("inspect copied tree source");
    assert!(
        source_metadata.is_dir() && !source_metadata.file_type().is_symlink(),
        "copied tree source must be a regular directory"
    );
    std::fs::create_dir(destination).expect("create copied tree destination");
    let mut entries = std::fs::read_dir(source)
        .expect("read copied tree source")
        .collect::<Result<Vec<_>, _>>()
        .expect("read copied tree entries");
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = std::fs::symlink_metadata(&source_path).expect("inspect copied tree entry");
        assert!(
            !metadata.file_type().is_symlink(),
            "copied maintained authority must not contain symbolic links"
        );
        if metadata.is_dir() {
            copy_maintained_repository(&source_path, &destination_path);
        } else {
            assert!(metadata.is_file(), "copied tree entry must be regular");
            std::fs::copy(&source_path, &destination_path).expect("copy regular tree entry");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::compiler::{OptimizationPolicy, build_clean, link_artifact};
    use crate::platform::publication::GraphRepository;
    use std::path::PathBuf;

    #[test]
    fn embedded_compression_preserves_bytes_and_rejects_wrong_length_or_trailing_data() {
        use std::io::Write;
        let original = b"exact standard artifact bytes, repeated repeated repeated";
        let mut encoder =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::best());
        encoder.write_all(original).unwrap();
        let compressed = encoder.finish().unwrap();
        assert_eq!(
            inflate_embedded(&compressed, original.len())
                .unwrap()
                .as_ref(),
            original
        );
        assert!(inflate_embedded(&compressed, original.len() - 1).is_err());
        assert!(inflate_embedded(&compressed, original.len() + 1).is_err());
        let mut trailing = compressed.clone();
        trailing.push(0);
        assert!(inflate_embedded(&trailing, original.len()).is_err());
        assert!(
            inflate_embedded(
                &compressed,
                super::super::packed::MAXIMUM_PACKED_PAYLOAD_BYTES + 1
            )
            .is_err()
        );
    }

    #[test]
    fn embedded_standard_transport_and_artifact_bind_one_exact_package() {
        let standard = BuiltinStandard::load().expect("validate embedded standard");
        let repeated = BuiltinStandard::load().unwrap();
        assert!(Arc::ptr_eq(
            &standard.transport_bytes,
            &repeated.transport_bytes
        ));
        assert!(Arc::ptr_eq(
            &standard.artifact_bytes,
            &repeated.artifact_bytes
        ));
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
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("packages/standard");
        let temporary = tempfile::tempdir().expect("maintained standard copy parent");
        let root = temporary.path().join("standard");
        super::copy_maintained_repository(&source, &root);
        let repository = GraphRepository::open(&root).expect("open maintained Graph 10 standard");
        let exported = repository
            .export_package_transport()
            .expect("export maintained standard transport");
        let embedded = BuiltinStandard::load().expect("admit compressed builtin");
        assert_eq!(exported.container.as_slice(), embedded.transport_bytes());
        assert_eq!(
            exported.transport_digest.to_string(),
            STANDARD_PACKAGE_TRANSPORT
        );

        let compilation = build_clean(&repository, OptimizationPolicy::DeterministicBaseline)
            .expect("clean compile maintained standard");
        let linked = link_artifact(&repository, compilation.manifest_digest, &[])
            .expect("link maintained standard");
        assert_eq!(linked.artifact.bytes.as_slice(), embedded.artifact_bytes());
    }
}
