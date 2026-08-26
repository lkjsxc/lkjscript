//! Exact first-party standard package material embedded in the released executable.
//!
//! The maintained Graph 5 package owns both generated assets. This module validates the complete
//! package transport and artifact closure before exposing either one to project creation, linking,
//! inspection, or export.

use super::compiler::{LoadedArtifact, load_artifact};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::kernel::{
    DeclarationReference, OwnerKey, PackageId, PackageInterfaceDeclarationPayload,
    PackageInterfaceRecord, PackageRevisionDigest, PackageTransportDigest, TypeForm, TypeObject,
    TypeObjectDigest,
};
use super::package_interface::PackageInterfaceOwner;
use super::package_transport::{PackageTransportBinding, validate_package_transport_closure};
use super::publication::InitialPackageTransport;
use super::semantic_id::{DeclarationId, RevisionId};
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
    "rev_27c3a79c798fe402d114e0000fefa0d628916808062d63d1782a6d9ed5e5aa83";
const STANDARD_PACKAGE_REVISION: &str =
    "package_revision_4290e78132570943c17a9cd800af0742dfc8c16baa6f471354792dab1d0db981";
const STANDARD_PACKAGE_TRANSPORT: &str =
    "package_transport_76566ff6df6024e573d3fc7f868cbc74760170dbd2111805c4c8c30a3a95b154";
const COMMAND_TEXT_FROM_STATIC: &str = "text-from-static";

static BUILTIN_STANDARD: OnceLock<Result<BuiltinStandard, Diagnostic>> = OnceLock::new();

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

    pub fn command_text_from_static(&self) -> Result<DeclarationReference, Diagnostic> {
        let mut selected = None;
        for (owner, value) in &self.interface_owners {
            let OwnerKey::Declaration(declaration) = owner else {
                continue;
            };
            let PackageInterfaceRecord::Declaration(record) = &value.record else {
                continue;
            };
            if record.name.as_str() != COMMAND_TEXT_FROM_STATIC {
                continue;
            }
            if !matches!(
                record.payload,
                PackageInterfaceDeclarationPayload::External(_)
            ) || selected.replace(*declaration).is_some()
            {
                return Err(builtin_error(
                    DiagnosticClass::Corrupt,
                    "builtin_standard_command_declaration",
                    "built-in standard interface has an ambiguous command text constructor",
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
                    "builtin_standard_command_declaration",
                    "built-in standard interface omits its command text constructor",
                )
            })
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
        let record = self.interface_declaration(declaration.declaration)?;
        let PackageInterfaceDeclarationPayload::External(signature) = &record.payload else {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_command_signature",
                "built-in command text constructor is not an external function",
            ));
        };
        let [parameter] = signature.parameters.as_slice() else {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_command_signature",
                "built-in command text constructor must accept one exact parameter",
            ));
        };
        let parameter_type = match self
            .interface_owners
            .get(&OwnerKey::Parameter(*parameter))
            .map(|value| &value.record)
        {
            Some(PackageInterfaceRecord::Parameter(record)) => record.ty,
            _ => {
                return Err(builtin_error(
                    DiagnosticClass::Corrupt,
                    "builtin_standard_command_parameter",
                    "built-in command text constructor parameter is unavailable",
                ));
            }
        };
        if !matches!(
            self.interface_types
                .get(&parameter_type)
                .map(|value| &value.form),
            Some(TypeForm::StaticText)
        ) || !matches!(
            self.interface_types
                .get(&signature.result)
                .map(|value| &value.form),
            Some(TypeForm::Text)
        ) {
            return Err(builtin_error(
                DiagnosticClass::Corrupt,
                "builtin_standard_command_type",
                "built-in command text constructor must map static_text to text",
            ));
        }
        Ok((declaration, parameter_type, signature.result))
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
        Ok(value)
    }
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
    }

    #[test]
    fn maintained_standard_is_the_byte_owner_of_embedded_assets() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("packages/standard");
        let repository = GraphRepository::open(&root).expect("open maintained Graph 5 standard");
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
