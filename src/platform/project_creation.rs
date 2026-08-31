//! Atomic typed creation of the executable's closed normalized project recipe set.

use super::deployment::{
    STARTER_HTTP_ARTIFACT_DIRECTORY, STARTER_HTTP_ARTIFACT_PATH, STARTER_HTTP_DESCRIPTOR_PATH,
    STARTER_HTTP_LISTENER, STARTER_HTTP_TARGET, encode_deployment, starter_http_deployment,
};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::kernel::{
    BindingKind, BindingRecord, ComparisonPolicy, DeclarationPayload, DeclarationRecord,
    DeclarationReference, DeclarationVisibility, DependencyRecord, ExpressionOperation,
    ExpressionRecord, FieldSelector, FunctionDeclaration, FunctionEffect, KernelSnapshot,
    LocalValueReference, Name, OwnerHeader, OwnerKey, OwnerKind, OwnerRecord, PackageId,
    ParameterParent, ParameterRecord, PortImplementation, PortRecord, PortReference,
    RecordExpressionField, RequirementRecord, RequirementReference, ResourceLimit, ResourceUnit,
    SemanticRoot, SemanticRootDigest, SemanticStateDigest, TargetRecord, TextValue, TypeForm,
    TypeObjectDigest, TypeObjectInterner, validate_full,
};
use super::package::RunnerKind;
use super::persistent_map::{MapContentDigest, MapRoot, PageDigest};
use super::publication::{
    GraphRepository, InitialPackageTransport, ReceiptObjectDigest, RevisionObjectDigest,
};
use super::semantic_id::{
    BindingId, DeclarationId, ExpressionId, ModuleId, ParameterId, PortId, RepositoryId,
    RequirementId, RevisionId, TargetId,
};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

pub const PROJECT_CREATION_CONTRACT_IDENTITY: &str = "lkjscript-project-creation-3";
pub const PROJECT_CREATION_CONTRACT_VERSION: u16 = 3;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ProjectTemplate {
    Minimal,
    Command,
    Http,
    NostrRelayInfo,
}

impl ProjectTemplate {
    pub(crate) const ALL: [Self; 4] = [
        Self::Minimal,
        Self::Command,
        Self::Http,
        Self::NostrRelayInfo,
    ];

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|template| template.name() == value)
    }

    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Command => "command",
            Self::Http => "http",
            Self::NostrRelayInfo => "nostr-relay-info",
        }
    }

    pub(crate) const fn purpose(self) -> &'static str {
        match self {
            Self::Minimal => "Create the smallest normalized accepted project authority.",
            Self::Command => {
                "Create a tested pure command with one exact built-in standard dependency."
            }
            Self::Http => {
                "Create an editable tested HTTP application and loopback starter deployment."
            }
            Self::NostrRelayInfo => {
                "Create a tested NIP-11 relay-information proxy with one deployment-bound endpoint."
            }
        }
    }

    pub(crate) const fn runner(self) -> &'static str {
        match self {
            Self::Minimal => "none",
            Self::Command => "command",
            Self::Http => "http",
            Self::NostrRelayInfo => "http",
        }
    }

    pub(crate) const fn emits_deployment(self) -> bool {
        matches!(self, Self::Http | Self::NostrRelayInfo)
    }

    pub(crate) const fn recommended_artifact_output(self) -> Option<&'static str> {
        match self {
            Self::Http | Self::NostrRelayInfo => Some(STARTER_HTTP_ARTIFACT_PATH),
            Self::Minimal | Self::Command => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatedDeployment {
    pub descriptor: PathBuf,
    pub recommended_artifact_output: PathBuf,
    pub target: &'static str,
    pub runner: &'static str,
    pub configured_listener: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectCreation {
    pub project: PathBuf,
    pub package_name: Name,
    pub repository: RepositoryId,
    pub package: PackageId,
    pub revision: RevisionId,
    pub semantic_state: SemanticStateDigest,
    pub semantic_root: SemanticRootDigest,
    pub revision_record: RevisionObjectDigest,
    pub receipt: ReceiptObjectDigest,
    pub(crate) template: ProjectTemplate,
    pub owners: u64,
    pub dependencies: u64,
    pub targets: u64,
    pub tests: u64,
    pub deployment: Option<CreatedDeployment>,
}

pub(crate) fn create_project(
    destination: &Path,
    package_name: &str,
    template: ProjectTemplate,
) -> Result<ProjectCreation, Diagnostic> {
    create_project_with_hook(destination, package_name, template, |_, _, _| Ok(()))
}

pub(crate) fn create_project_with_relay(
    destination: &Path,
    package_name: &str,
    relay_url: &str,
) -> Result<ProjectCreation, Diagnostic> {
    create_project_with_options(
        destination,
        package_name,
        ProjectTemplate::NostrRelayInfo,
        Some(relay_url),
        |_, _, _| Ok(()),
    )
}

struct ProjectRecipe {
    snapshot: KernelSnapshot,
    transports: Vec<InitialPackageTransport>,
    template: ProjectTemplate,
    targets: u64,
    tests: u64,
    auxiliary: Option<ProjectAuxiliary>,
}

struct ProjectAuxiliary {
    descriptor: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CreationPoint {
    GraphPublished,
    BeforeDescriptor,
    DescriptorPublished,
    GeneratedDirectoryPublished,
    BeforeVisibility,
}

fn create_project_with_hook<F>(
    destination: &Path,
    package_name: &str,
    template: ProjectTemplate,
    hook: F,
) -> Result<ProjectCreation, Diagnostic>
where
    F: FnMut(CreationPoint, &Path, &Path) -> Result<(), Diagnostic>,
{
    create_project_with_options(destination, package_name, template, None, hook)
}

fn create_project_with_options<F>(
    destination: &Path,
    package_name: &str,
    template: ProjectTemplate,
    relay_url: Option<&str>,
    mut hook: F,
) -> Result<ProjectCreation, Diagnostic>
where
    F: FnMut(CreationPoint, &Path, &Path) -> Result<(), Diagnostic>,
{
    let destination = safe_destination(destination)?;
    let parent = destination.parent().ok_or_else(|| {
        creation_error(
            DiagnosticClass::Source,
            "new_destination_parent",
            "project destination has no existing parent",
        )
    })?;
    let package_name = Name::new(package_name)?;
    let repository = RepositoryId::generate()?;
    let package = PackageId::generate()?;
    let recipe = match template {
        ProjectTemplate::Minimal => minimal_recipe(repository, package, package_name.clone()),
        ProjectTemplate::Command => command_recipe(repository, package, package_name.clone()),
        ProjectTemplate::Http => http_recipe(repository, package, package_name.clone()),
        ProjectTemplate::NostrRelayInfo => nostr_relay_info_recipe(
            repository,
            package,
            package_name.clone(),
            relay_url.ok_or_else(|| {
                creation_error(
                    DiagnosticClass::Source,
                    "new_relay_url_required",
                    "nostr-relay-info requires one exact relay URL",
                )
            })?,
        ),
    }?;
    validate_auxiliary_inventory(recipe.auxiliary.as_ref())?;
    validate_full(&recipe.snapshot).map_err(|diagnostics| {
        diagnostics.into_iter().next().unwrap_or_else(|| {
            creation_error(
                DiagnosticClass::Corrupt,
                "new_recipe_validation",
                "typed project recipe failed without an exact diagnostic",
            )
        })
    })?;
    let private = parent.join(format!(".lkjscript-project-stage-{repository}"));

    let created = GraphRepository::create_with_package_transports(
        &private,
        &recipe.snapshot,
        Some(format!(
            "public {} project bootstrap",
            recipe.template.name()
        )),
        &recipe.transports,
    )?;
    let staged = (|| {
        hook(CreationPoint::GraphPublished, &private, &destination)?;
        if let Some(auxiliary) = &recipe.auxiliary {
            publish_auxiliary(&private, auxiliary, &mut hook, &destination)?;
        }
        sync_stage_directory(&private)?;
        hook(CreationPoint::BeforeVisibility, &private, &destination)?;
        Ok(())
    })();
    if let Err(error) = staged {
        remove_owned_stage(&private);
        return Err(error);
    }
    if let Err(error) = fs::rename(&private, &destination) {
        remove_owned_stage(&private);
        return Err(creation_error(
            DiagnosticClass::Infrastructure,
            "new_destination_publish",
            format!(
                "normalized project could not be published at '{}': {error}",
                destination.display()
            ),
        ));
    }
    sync_parent_directory(parent)?;

    let visible = GraphRepository::open(&destination)?;
    let current = visible.current()?;
    if current.head != created.current.head
        || current.semantic_root.package_id != package
        || current.semantic_root.repository_id != repository
    {
        return Err(creation_error(
            DiagnosticClass::Infrastructure,
            "new_destination_reconcile",
            "visible normalized project disagrees with its privately accepted publication",
        ));
    }
    reconcile_auxiliary(&destination, recipe.auxiliary.as_ref())?;
    let deployment = recipe.auxiliary.as_ref().map(|_| CreatedDeployment {
        descriptor: destination.join(STARTER_HTTP_DESCRIPTOR_PATH),
        recommended_artifact_output: destination.join(STARTER_HTTP_ARTIFACT_PATH),
        target: STARTER_HTTP_TARGET,
        runner: "http",
        configured_listener: STARTER_HTTP_LISTENER,
    });
    Ok(ProjectCreation {
        project: destination,
        package_name,
        repository,
        package,
        revision: current.head.revision,
        semantic_state: current.accepted.semantic_state,
        semantic_root: current.accepted.semantic_root,
        revision_record: current.head.record,
        receipt: current.accepted.receipt,
        template,
        owners: recipe.snapshot.owners.len() as u64,
        dependencies: recipe.snapshot.dependencies.len() as u64,
        targets: recipe.targets,
        tests: recipe.tests,
        deployment,
    })
}

fn publish_auxiliary<F>(
    private: &Path,
    auxiliary: &ProjectAuxiliary,
    hook: &mut F,
    destination: &Path,
) -> Result<(), Diagnostic>
where
    F: FnMut(CreationPoint, &Path, &Path) -> Result<(), Diagnostic>,
{
    hook(CreationPoint::BeforeDescriptor, private, destination)?;
    let descriptor = private.join(STARTER_HTTP_DESCRIPTOR_PATH);
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&descriptor)
        .map_err(|error| auxiliary_io("new_descriptor_create", &descriptor, error))?;
    output
        .write_all(&auxiliary.descriptor)
        .and_then(|()| output.sync_all())
        .map_err(|error| auxiliary_io("new_descriptor_publish", &descriptor, error))?;
    hook(CreationPoint::DescriptorPublished, private, destination)?;

    let generated = private.join(STARTER_HTTP_ARTIFACT_DIRECTORY);
    fs::create_dir(&generated)
        .map_err(|error| auxiliary_io("new_generated_directory", &generated, error))?;
    sync_stage_directory(&generated)?;
    hook(
        CreationPoint::GeneratedDirectoryPublished,
        private,
        destination,
    )?;
    Ok(())
}

fn reconcile_auxiliary(
    destination: &Path,
    auxiliary: Option<&ProjectAuxiliary>,
) -> Result<(), Diagnostic> {
    let Some(auxiliary) = auxiliary else {
        return Ok(());
    };
    let descriptor = destination.join(STARTER_HTTP_DESCRIPTOR_PATH);
    let metadata = fs::symlink_metadata(&descriptor)
        .map_err(|error| auxiliary_io("new_destination_reconcile", &descriptor, error))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || fs::read(&descriptor)
            .map(|bytes| bytes != auxiliary.descriptor)
            .unwrap_or(true)
    {
        return Err(creation_error(
            DiagnosticClass::Infrastructure,
            "new_destination_reconcile",
            "visible starter deployment disagrees with its synchronized private publication",
        ));
    }
    let generated = destination.join(STARTER_HTTP_ARTIFACT_DIRECTORY);
    let metadata = fs::symlink_metadata(&generated)
        .map_err(|error| auxiliary_io("new_destination_reconcile", &generated, error))?;
    let empty = metadata.is_dir()
        && !metadata.file_type().is_symlink()
        && fs::read_dir(&generated)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
    if !empty {
        return Err(creation_error(
            DiagnosticClass::Infrastructure,
            "new_destination_reconcile",
            "visible generated output directory is absent, foreign, or unexpectedly populated",
        ));
    }
    Ok(())
}

fn auxiliary_io(code: &'static str, path: &Path, error: std::io::Error) -> Diagnostic {
    creation_error(
        DiagnosticClass::Infrastructure,
        code,
        format!(
            "project auxiliary path '{}' failed: {error}",
            path.display()
        ),
    )
}

fn validate_auxiliary_inventory(auxiliary: Option<&ProjectAuxiliary>) -> Result<(), Diagnostic> {
    let Some(_) = auxiliary else {
        return Ok(());
    };
    for (path, label) in [
        (STARTER_HTTP_DESCRIPTOR_PATH, "starter descriptor"),
        (STARTER_HTTP_ARTIFACT_DIRECTORY, "generated directory"),
        (STARTER_HTTP_ARTIFACT_PATH, "recommended artifact"),
    ] {
        let path = Path::new(path);
        if path.is_absolute()
            || path.as_os_str().is_empty()
            || path.components().any(
                |component| !matches!(component, Component::Normal(value) if !value.is_empty()),
            )
        {
            return Err(creation_error(
                DiagnosticClass::Corrupt,
                "new_auxiliary_path",
                format!("{label} path is not a canonical relative path"),
            ));
        }
    }
    if Path::new(STARTER_HTTP_ARTIFACT_PATH).parent()
        != Some(Path::new(STARTER_HTTP_ARTIFACT_DIRECTORY))
        || STARTER_HTTP_DESCRIPTOR_PATH == STARTER_HTTP_ARTIFACT_DIRECTORY
        || STARTER_HTTP_DESCRIPTOR_PATH == STARTER_HTTP_ARTIFACT_PATH
    {
        return Err(creation_error(
            DiagnosticClass::Corrupt,
            "new_auxiliary_inventory",
            "starter descriptor, generated directory, and artifact output paths overlap",
        ));
    }
    Ok(())
}

fn minimal_recipe(
    repository: RepositoryId,
    package: PackageId,
    package_name: Name,
) -> Result<ProjectRecipe, Diagnostic> {
    Ok(ProjectRecipe {
        snapshot: empty_snapshot(repository, package, package_name),
        transports: Vec::new(),
        template: ProjectTemplate::Minimal,
        targets: 0,
        tests: 0,
        auxiliary: None,
    })
}

fn command_recipe(
    repository: RepositoryId,
    package: PackageId,
    package_name: Name,
) -> Result<ProjectRecipe, Diagnostic> {
    let standard = super::builtin_standard::BuiltinStandard::load()?;
    let (text_from_static, static_text_type, text_type) = standard.command_text_signature()?;
    let seed = repository.bytes();
    let module = ModuleId::migrate(&seed, 0);
    let function = DeclarationId::migrate(&seed, 0);
    let component = DeclarationId::migrate(&seed, 1);
    let test = DeclarationId::migrate(&seed, 2);
    let literal = ExpressionId::migrate(&seed, 0);
    let body = ExpressionId::migrate(&seed, 1);
    let test_actual = ExpressionId::migrate(&seed, 2);
    let test_expected = ExpressionId::migrate(&seed, 3);
    let port = PortId::migrate(&seed, 0);
    let target = TargetId::migrate(&seed, 0);

    let mut interner = TypeObjectInterner::default();
    let local_text = interner.intern(TypeForm::Text)?;
    if local_text != text_type
        || !matches!(
            standard
                .interface_types
                .get(&static_text_type)
                .map(|value| &value.form),
            Some(TypeForm::StaticText)
        )
    {
        return Err(creation_error(
            DiagnosticClass::Corrupt,
            "new_command_standard_types",
            "built-in standard primitive types disagree with canonical Graph 5 types",
        ));
    }
    let function_type = interner.intern(TypeForm::Function {
        parameters: Vec::new(),
        result: text_type,
    })?;

    let mut owners = BTreeMap::new();
    insert_owner(
        &mut owners,
        OwnerRecord::Module(super::kernel::ModuleRecord {
            header: OwnerHeader::new(OwnerKey::Module(module), OwnerKind::Module),
            name: Name::new("application")?,
        }),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Expression(ExpressionRecord::new(
            literal,
            ExpressionOperation::StaticText {
                value: TextValue::Inline {
                    text: "hello".to_owned(),
                },
            },
        )?),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Expression(ExpressionRecord::new(
            body,
            ExpressionOperation::Call {
                function: text_from_static,
                type_arguments: Vec::new(),
                arguments: vec![literal],
            },
        )?),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(function), OwnerKind::PureFunction),
            module,
            name: Name::new("greet")?,
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: text_type,
                effect: FunctionEffect::Pure,
                body,
            }),
        }),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Port(PortRecord {
            header: OwnerHeader::new(OwnerKey::Port(port), OwnerKind::Port),
            declaration: component,
            name: Name::new("main")?,
            function_type,
            implementation: PortImplementation::Function(DeclarationReference {
                package,
                declaration: function,
            }),
        }),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(component), OwnerKind::Component),
            module,
            name: Name::new("application")?,
            visibility: DeclarationVisibility::Package,
            payload: DeclarationPayload::Component {
                requirements: Vec::new(),
                ports: vec![port],
            },
        }),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Target(TargetRecord {
            header: OwnerHeader::new(OwnerKey::Target(target), OwnerKind::Target),
            name: Name::new("main")?,
            component: DeclarationReference {
                package,
                declaration: component,
            },
            port: PortReference { package, port },
            runner: RunnerKind::Command,
        }),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Expression(ExpressionRecord::new(
            test_actual,
            ExpressionOperation::Call {
                function: DeclarationReference {
                    package,
                    declaration: function,
                },
                type_arguments: Vec::new(),
                arguments: Vec::new(),
            },
        )?),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Expression(ExpressionRecord::new(
            test_expected,
            ExpressionOperation::Text {
                value: TextValue::Inline {
                    text: "hello".to_owned(),
                },
            },
        )?),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(test), OwnerKind::Test),
            module,
            name: Name::new("main-returns-hello")?,
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Test {
                actual: test_actual,
                expected: test_expected,
                comparison: ComparisonPolicy::Exact,
            },
        }),
    )?;

    let dependency = DependencyRecord {
        graph_contract_version: super::kernel::contract::GRAPH_CONTRACT_VERSION,
        package: standard.package,
        semantic_revision: standard.semantic_revision,
        package_revision: standard.package_revision,
    };
    let dependency_interfaces = BTreeMap::from([(
        standard.package_revision,
        standard
            .interface_owners
            .iter()
            .map(|(owner, value)| (*owner, value.record.clone()))
            .collect(),
    )]);
    let dependency_types = standard.interface_types.clone();
    let owners_len = owners.len();
    Ok(ProjectRecipe {
        snapshot: KernelSnapshot {
            root: SemanticRoot {
                graph_contract_version: super::kernel::contract::GRAPH_CONTRACT_VERSION,
                repository_id: repository,
                package_id: package,
                package_name,
                owners: placeholder_map(owners_len),
                dependencies: placeholder_map(1),
                retirements: placeholder_map(0),
            },
            owners,
            types: interner.into_objects(),
            dependency_interfaces,
            dependency_types,
            blobs: BTreeMap::new(),
            dependencies: BTreeMap::from([(standard.package, dependency)]),
            retirements: BTreeMap::new(),
        },
        transports: vec![standard.transport()],
        template: ProjectTemplate::Command,
        targets: 1,
        tests: 1,
        auxiliary: None,
    })
}

fn http_recipe(
    repository: RepositoryId,
    package: PackageId,
    package_name: Name,
) -> Result<ProjectRecipe, Diagnostic> {
    let standard = super::builtin_standard::BuiltinStandard::load()?;
    let contract = standard.http_recipe_contract()?;
    let seed = repository.bytes();
    let module = ModuleId::migrate(&seed, 0);
    let response_function = DeclarationId::migrate(&seed, 0);
    let status_function = DeclarationId::migrate(&seed, 1);
    let handler_function = DeclarationId::migrate(&seed, 2);
    let component = DeclarationId::migrate(&seed, 3);
    let test = DeclarationId::migrate(&seed, 4);
    let request_parameter = ParameterId::migrate(&seed, 0);
    let streams = RequirementId::migrate(&seed, 0);
    let port = PortId::migrate(&seed, 0);
    let target = TargetId::migrate(&seed, 0);

    let response_literal = ExpressionId::migrate(&seed, 0);
    let status_literal = ExpressionId::migrate(&seed, 1);
    let response_call = ExpressionId::migrate(&seed, 2);
    let text_conversion = ExpressionId::migrate(&seed, 3);
    let bytes_conversion = ExpressionId::migrate(&seed, 4);
    let empty_headers = ExpressionId::migrate(&seed, 5);
    let status_call = ExpressionId::migrate(&seed, 6);
    let response_record = ExpressionId::migrate(&seed, 7);
    let test_actual = ExpressionId::migrate(&seed, 8);
    let test_expected = ExpressionId::migrate(&seed, 9);

    let mut interner = TypeObjectInterner::default();
    let semantic_http = super::http::semantic_http_types(&mut interner)?;
    let static_text_type = interner.intern(TypeForm::StaticText)?;
    if contract.static_text_type != static_text_type
        || contract.text_type != semantic_http.text_type
        || contract.bytes_type != semantic_http.bytes_type
    {
        return Err(creation_error(
            DiagnosticClass::Corrupt,
            "new_http_standard_types",
            "built-in HTTP recipe declarations disagree with canonical Graph 5 primitive types",
        ));
    }

    let local_stream_requirement = RequirementReference {
        package,
        requirement: streams,
    };
    let local_response_function = DeclarationReference {
        package,
        declaration: response_function,
    };
    let local_status_function = DeclarationReference {
        package,
        declaration: status_function,
    };
    let mut owners = BTreeMap::new();
    insert_owner(
        &mut owners,
        OwnerRecord::Module(super::kernel::ModuleRecord {
            header: OwnerHeader::new(OwnerKey::Module(module), OwnerKind::Module),
            name: Name::new("application")?,
        }),
    )?;

    insert_owner(
        &mut owners,
        OwnerRecord::Expression(ExpressionRecord::new(
            response_literal,
            ExpressionOperation::StaticText {
                value: TextValue::Inline {
                    text: "hello from lkjscript".to_owned(),
                },
            },
        )?),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(response_function),
                OwnerKind::PureFunction,
            ),
            module,
            name: Name::new("response-text")?,
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: static_text_type,
                effect: FunctionEffect::Pure,
                body: response_literal,
            }),
        }),
    )?;

    insert_owner(
        &mut owners,
        OwnerRecord::Expression(ExpressionRecord::new(
            status_literal,
            ExpressionOperation::I64 { value: 200 },
        )?),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(status_function),
                OwnerKind::PureFunction,
            ),
            module,
            name: Name::new("status-code")?,
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: semantic_http.i64_type,
                effect: FunctionEffect::Pure,
                body: status_literal,
            }),
        }),
    )?;

    insert_owner(
        &mut owners,
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(request_parameter), OwnerKind::Parameter),
            parent: ParameterParent::Function(handler_function),
            name: Name::new("request")?,
            ty: semantic_http.request_type,
        }),
    )?;
    for (id, operation) in [
        (
            response_call,
            ExpressionOperation::Call {
                function: local_response_function,
                type_arguments: Vec::new(),
                arguments: Vec::new(),
            },
        ),
        (
            text_conversion,
            ExpressionOperation::Call {
                function: contract.text_from_static,
                type_arguments: Vec::new(),
                arguments: vec![response_call],
            },
        ),
        (
            bytes_conversion,
            ExpressionOperation::Call {
                function: contract.bytes_from_text,
                type_arguments: Vec::new(),
                arguments: vec![text_conversion],
            },
        ),
        (
            empty_headers,
            ExpressionOperation::List {
                item_type: semantic_http.header_type,
                items: Vec::new(),
            },
        ),
        (
            status_call,
            ExpressionOperation::Call {
                function: local_status_function,
                type_arguments: Vec::new(),
                arguments: Vec::new(),
            },
        ),
    ] {
        insert_owner(
            &mut owners,
            OwnerRecord::Expression(ExpressionRecord::new(id, operation)?),
        )?;
    }
    insert_owner(
        &mut owners,
        OwnerRecord::Expression(ExpressionRecord::new(
            response_record,
            ExpressionOperation::Record {
                nominal_type: None,
                fields: vec![
                    RecordExpressionField {
                        selector: FieldSelector::Structural(Name::new("body")?),
                        value: bytes_conversion,
                    },
                    RecordExpressionField {
                        selector: FieldSelector::Structural(Name::new("headers")?),
                        value: empty_headers,
                    },
                    RecordExpressionField {
                        selector: FieldSelector::Structural(Name::new("status")?),
                        value: status_call,
                    },
                ],
            },
        )?),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(handler_function),
                OwnerKind::TaskFunction,
            ),
            module,
            name: Name::new("handle")?,
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: Vec::new(),
                parameters: vec![request_parameter],
                result: semantic_http.response_type,
                effect: FunctionEffect::Task {
                    requirements: vec![local_stream_requirement],
                },
                body: response_record,
            }),
        }),
    )?;

    insert_owner(
        &mut owners,
        OwnerRecord::Requirement(RequirementRecord {
            header: OwnerHeader::new(OwnerKey::Requirement(streams), OwnerKind::Requirement),
            declaration: component,
            name: Name::new("streams")?,
            interface: contract.byte_stream_interface,
            operations: contract.byte_stream_operations.clone(),
            limits: vec![ResourceLimit {
                name: Name::new("maximum_calls")?,
                maximum: 10_000,
                unit: ResourceUnit::Calls,
            }],
        }),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Port(PortRecord {
            header: OwnerHeader::new(OwnerKey::Port(port), OwnerKind::Port),
            declaration: component,
            name: Name::new("http")?,
            function_type: semantic_http.function_type,
            implementation: PortImplementation::Function(DeclarationReference {
                package,
                declaration: handler_function,
            }),
        }),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(component), OwnerKind::Component),
            module,
            name: Name::new("application")?,
            visibility: DeclarationVisibility::Package,
            payload: DeclarationPayload::Component {
                requirements: vec![streams],
                ports: vec![port],
            },
        }),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Target(TargetRecord {
            header: OwnerHeader::new(OwnerKey::Target(target), OwnerKind::Target),
            name: Name::new(STARTER_HTTP_TARGET)?,
            component: DeclarationReference {
                package,
                declaration: component,
            },
            port: PortReference { package, port },
            runner: RunnerKind::Http,
        }),
    )?;

    insert_owner(
        &mut owners,
        OwnerRecord::Expression(ExpressionRecord::new(
            test_actual,
            ExpressionOperation::Call {
                function: local_status_function,
                type_arguments: Vec::new(),
                arguments: Vec::new(),
            },
        )?),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Expression(ExpressionRecord::new(
            test_expected,
            ExpressionOperation::I64 { value: 200 },
        )?),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(test), OwnerKind::Test),
            module,
            name: Name::new("status-is-200")?,
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Test {
                actual: test_actual,
                expected: test_expected,
                comparison: ComparisonPolicy::Exact,
            },
        }),
    )?;

    let dependency = DependencyRecord {
        graph_contract_version: super::kernel::contract::GRAPH_CONTRACT_VERSION,
        package: standard.package,
        semantic_revision: standard.semantic_revision,
        package_revision: standard.package_revision,
    };
    let dependency_interfaces = BTreeMap::from([(
        standard.package_revision,
        standard
            .interface_owners
            .iter()
            .map(|(owner, value)| (*owner, value.record.clone()))
            .collect(),
    )]);
    let owners_len = owners.len();
    let deployment = starter_http_deployment()?;
    let descriptor = encode_deployment(&deployment)?;
    Ok(ProjectRecipe {
        snapshot: KernelSnapshot {
            root: SemanticRoot {
                graph_contract_version: super::kernel::contract::GRAPH_CONTRACT_VERSION,
                repository_id: repository,
                package_id: package,
                package_name,
                owners: placeholder_map(owners_len),
                dependencies: placeholder_map(1),
                retirements: placeholder_map(0),
            },
            owners,
            types: interner.into_objects(),
            dependency_interfaces,
            dependency_types: standard.interface_types.clone(),
            blobs: BTreeMap::new(),
            dependencies: BTreeMap::from([(standard.package, dependency)]),
            retirements: BTreeMap::new(),
        },
        transports: vec![standard.transport()],
        template: ProjectTemplate::Http,
        targets: 1,
        tests: 1,
        auxiliary: Some(ProjectAuxiliary { descriptor }),
    })
}

fn nostr_relay_info_recipe(
    repository: RepositoryId,
    package: PackageId,
    package_name: Name,
    relay_url: &str,
) -> Result<ProjectRecipe, Diagnostic> {
    let relay = super::http_client::normalize_nostr_relay_url(relay_url)?;
    let standard = super::builtin_standard::BuiltinStandard::load()?;
    let contract = standard.http_recipe_contract()?;
    let seed = repository.bytes();
    let module = ModuleId::migrate(&seed, 0);
    let content_type_reducer = DeclarationId::migrate(&seed, 0);
    let route_function = DeclarationId::migrate(&seed, 1);
    let handler_function = DeclarationId::migrate(&seed, 2);
    let component = DeclarationId::migrate(&seed, 3);
    let valid_test = DeclarationId::migrate(&seed, 4);
    let invalid_test = DeclarationId::migrate(&seed, 5);
    let reducer_state = ParameterId::migrate(&seed, 0);
    let reducer_header = ParameterId::migrate(&seed, 1);
    let route_method = ParameterId::migrate(&seed, 2);
    let route_path = ParameterId::migrate(&seed, 3);
    let request_parameter = ParameterId::migrate(&seed, 4);
    let streams = RequirementId::migrate(&seed, 0);
    let relay_requirement = RequirementId::migrate(&seed, 1);
    let remote_binding = BindingId::migrate(&seed, 0);
    let port = PortId::migrate(&seed, 0);
    let target = TargetId::migrate(&seed, 0);

    let mut interner = TypeObjectInterner::default();
    let semantic_http = super::http::semantic_http_types(&mut interner)?;
    let semantic_client = super::http_client::semantic_http_client_types(&mut interner)?;
    let bool_type = interner.intern(TypeForm::Bool)?;
    if contract.text_type != semantic_http.text_type
        || contract.bytes_type != semantic_http.bytes_type
        || semantic_client.i64_type != semantic_http.i64_type
        || semantic_client.bytes_type != semantic_http.bytes_type
        || semantic_client.text_type != semantic_http.text_type
        || semantic_client.header_type != semantic_http.header_type
        || semantic_client.header_list_type != semantic_http.header_list_type
        || semantic_client.response_type != semantic_http.response_type
    {
        return Err(creation_error(
            DiagnosticClass::Corrupt,
            "new_nostr_standard_types",
            "built-in standard HTTP server and client types disagree with canonical Graph 5 types",
        ));
    }

    let local_stream_requirement = RequirementReference {
        package,
        requirement: streams,
    };
    let local_relay_requirement = RequirementReference {
        package,
        requirement: relay_requirement,
    };
    let local_route = DeclarationReference {
        package,
        declaration: route_function,
    };
    let local_reducer = DeclarationReference {
        package,
        declaration: content_type_reducer,
    };
    let mut owners = BTreeMap::new();
    insert_owner(
        &mut owners,
        OwnerRecord::Module(super::kernel::ModuleRecord {
            header: OwnerHeader::new(OwnerKey::Module(module), OwnerKind::Module),
            name: Name::new("application")?,
        }),
    )?;
    let mut expression_ordinal = 0_u64;

    for (parameter, parent, name, ty) in [
        (reducer_state, content_type_reducer, "matched", bool_type),
        (
            reducer_header,
            content_type_reducer,
            "header",
            semantic_client.header_type,
        ),
        (
            route_method,
            route_function,
            "method",
            semantic_http.text_type,
        ),
        (route_path, route_function, "path", semantic_http.text_type),
        (
            request_parameter,
            handler_function,
            "request",
            semantic_http.request_type,
        ),
    ] {
        insert_owner(
            &mut owners,
            OwnerRecord::Parameter(ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
                parent: ParameterParent::Function(parent),
                name: Name::new(name)?,
                ty,
            }),
        )?;
    }

    let header_for_name = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(reducer_header),
        },
    )?;
    let header_name = recipe_field(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        header_for_name,
        "name",
    )?;
    let content_type_name =
        recipe_text(&mut owners, &seed, &mut expression_ordinal, "content-type")?;
    let name_matches = recipe_call(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        contract.text_equal,
        Vec::new(),
        vec![header_name, content_type_name],
    )?;
    let header_for_value = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(reducer_header),
        },
    )?;
    let header_value = recipe_field(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        header_for_value,
        "value",
    )?;
    let expected_media = recipe_text(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        "application/nostr+json",
    )?;
    let media_matches = recipe_call(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        contract.media_type_is,
        Vec::new(),
        vec![header_value, expected_media],
    )?;
    let this_header_matches = recipe_call(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        contract.bool_and,
        Vec::new(),
        vec![name_matches, media_matches],
    )?;
    let prior_match = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(reducer_state),
        },
    )?;
    let reducer_body = recipe_call(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        contract.bool_or,
        Vec::new(),
        vec![prior_match, this_header_matches],
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(content_type_reducer),
                OwnerKind::PureFunction,
            ),
            module,
            name: Name::new("content-type-is-nostr")?,
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: Vec::new(),
                parameters: vec![reducer_state, reducer_header],
                result: bool_type,
                effect: FunctionEffect::Pure,
                body: reducer_body,
            }),
        }),
    )?;

    let route_method_value = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(route_method),
        },
    )?;
    let get_text = recipe_text(&mut owners, &seed, &mut expression_ordinal, "GET")?;
    let method_matches = recipe_call(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        contract.text_equal,
        Vec::new(),
        vec![route_method_value, get_text],
    )?;
    let route_path_value = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(route_path),
        },
    )?;
    let relay_path_text = recipe_text(&mut owners, &seed, &mut expression_ordinal, "/relay-info")?;
    let path_matches = recipe_call(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        contract.text_equal,
        Vec::new(),
        vec![route_path_value, relay_path_text],
    )?;
    let route_body = recipe_call(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        contract.bool_and,
        Vec::new(),
        vec![method_matches, path_matches],
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(route_function),
                OwnerKind::PureFunction,
            ),
            module,
            name: Name::new("is-relay-info-request")?,
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: Vec::new(),
                parameters: vec![route_method, route_path],
                result: bool_type,
                effect: FunctionEffect::Pure,
                body: route_body,
            }),
        }),
    )?;

    let accept_name = recipe_text(&mut owners, &seed, &mut expression_ordinal, "accept")?;
    let accept_value = recipe_bytes(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        contract.bytes_from_text,
        "application/nostr+json",
    )?;
    let accept_header = recipe_record(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        vec![("name", accept_name), ("value", accept_value)],
    )?;
    let request_headers = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::List {
            item_type: semantic_client.header_type,
            items: vec![accept_header],
        },
    )?;
    let remote_call = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::CapabilityCall {
            requirement: local_relay_requirement,
            operation: contract.http_client_get,
            arguments: vec![request_headers],
        },
    )?;

    let remote_for_status = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(remote_binding),
        },
    )?;
    let remote_status = recipe_field(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        remote_for_status,
        "status",
    )?;
    let status_200 = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::I64 { value: 200 },
    )?;
    let status_matches = recipe_call(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        contract.i64_equal,
        Vec::new(),
        vec![remote_status, status_200],
    )?;
    let remote_for_headers = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(remote_binding),
        },
    )?;
    let remote_headers = recipe_field(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        remote_for_headers,
        "headers",
    )?;
    let no_media_match = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::Bool { value: false },
    )?;
    let reducer_value = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::FunctionValue {
            function: local_reducer,
            type_arguments: Vec::new(),
        },
    )?;
    let media_matches = recipe_call(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        contract.list_fold_left,
        vec![semantic_client.header_type, bool_type],
        vec![remote_headers, no_media_match, reducer_value],
    )?;
    let remote_is_valid = recipe_call(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        contract.bool_and,
        Vec::new(),
        vec![status_matches, media_matches],
    )?;

    let remote_for_body = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(remote_binding),
        },
    )?;
    let remote_body = recipe_field(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        remote_for_body,
        "body",
    )?;
    let response_content_name =
        recipe_text(&mut owners, &seed, &mut expression_ordinal, "content-type")?;
    let response_content_value = recipe_bytes(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        contract.bytes_from_text,
        "application/nostr+json",
    )?;
    let response_content_header = recipe_record(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        vec![
            ("name", response_content_name),
            ("value", response_content_value),
        ],
    )?;
    let response_headers = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::List {
            item_type: semantic_http.header_type,
            items: vec![response_content_header],
        },
    )?;
    let response_200 = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::I64 { value: 200 },
    )?;
    let successful_response = recipe_record(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        vec![
            ("body", remote_body),
            ("headers", response_headers),
            ("status", response_200),
        ],
    )?;
    let bad_gateway = recipe_http_response(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        contract.bytes_from_text,
        semantic_http.header_type,
        502,
        "bad gateway",
    )?;
    let selected_remote = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::If {
            condition: remote_is_valid,
            when_true: successful_response,
            when_false: bad_gateway,
        },
    )?;
    let remote_scope = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::Let {
            bindings: vec![remote_binding],
            body: selected_remote,
        },
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Binding(BindingRecord {
            header: OwnerHeader::new(OwnerKey::Binding(remote_binding), OwnerKind::Binding),
            name: Name::new("remote-response")?,
            kind: BindingKind::Let,
            value: Some(remote_call),
            declared_type: Some(semantic_client.response_type),
        }),
    )?;

    let request_for_method = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(request_parameter),
        },
    )?;
    let request_method = recipe_field(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        request_for_method,
        "method",
    )?;
    let request_for_path = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(request_parameter),
        },
    )?;
    let request_path = recipe_field(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        request_for_path,
        "path",
    )?;
    let route_matches = recipe_call(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        local_route,
        Vec::new(),
        vec![request_method, request_path],
    )?;
    let not_found = recipe_http_response(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        contract.bytes_from_text,
        semantic_http.header_type,
        404,
        "not found",
    )?;
    let handler_body = recipe_expression(
        &mut owners,
        &seed,
        &mut expression_ordinal,
        ExpressionOperation::If {
            condition: route_matches,
            when_true: remote_scope,
            when_false: not_found,
        },
    )?;
    let mut task_requirements = vec![local_stream_requirement, local_relay_requirement];
    task_requirements.sort();
    insert_owner(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(handler_function),
                OwnerKind::TaskFunction,
            ),
            module,
            name: Name::new("handle")?,
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: Vec::new(),
                parameters: vec![request_parameter],
                result: semantic_http.response_type,
                effect: FunctionEffect::Task {
                    requirements: task_requirements,
                },
                body: handler_body,
            }),
        }),
    )?;

    for (id, name, interface, operations, maximum_calls) in [
        (
            streams,
            "streams",
            contract.byte_stream_interface,
            contract.byte_stream_operations.clone(),
            10_000,
        ),
        (
            relay_requirement,
            "relay",
            contract.http_client_interface,
            contract.http_client_operations.clone(),
            64,
        ),
    ] {
        insert_owner(
            &mut owners,
            OwnerRecord::Requirement(RequirementRecord {
                header: OwnerHeader::new(OwnerKey::Requirement(id), OwnerKind::Requirement),
                declaration: component,
                name: Name::new(name)?,
                interface,
                operations,
                limits: vec![ResourceLimit {
                    name: Name::new("maximum_calls")?,
                    maximum: maximum_calls,
                    unit: ResourceUnit::Calls,
                }],
            }),
        )?;
    }
    insert_owner(
        &mut owners,
        OwnerRecord::Port(PortRecord {
            header: OwnerHeader::new(OwnerKey::Port(port), OwnerKind::Port),
            declaration: component,
            name: Name::new("http")?,
            function_type: semantic_http.function_type,
            implementation: PortImplementation::Function(DeclarationReference {
                package,
                declaration: handler_function,
            }),
        }),
    )?;
    let mut component_requirements = vec![streams, relay_requirement];
    component_requirements.sort();
    insert_owner(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(component), OwnerKind::Component),
            module,
            name: Name::new("application")?,
            visibility: DeclarationVisibility::Package,
            payload: DeclarationPayload::Component {
                requirements: component_requirements,
                ports: vec![port],
            },
        }),
    )?;
    insert_owner(
        &mut owners,
        OwnerRecord::Target(TargetRecord {
            header: OwnerHeader::new(OwnerKey::Target(target), OwnerKind::Target),
            name: Name::new(STARTER_HTTP_TARGET)?,
            component: DeclarationReference {
                package,
                declaration: component,
            },
            port: PortReference { package, port },
            runner: RunnerKind::Http,
        }),
    )?;

    for (test, name, method, path, expected) in [
        (
            valid_test,
            "relay-info-route-is-exact",
            "GET",
            "/relay-info",
            true,
        ),
        (
            invalid_test,
            "relay-info-route-rejects-other-method",
            "POST",
            "/relay-info",
            false,
        ),
    ] {
        let method = recipe_text(&mut owners, &seed, &mut expression_ordinal, method)?;
        let path = recipe_text(&mut owners, &seed, &mut expression_ordinal, path)?;
        let actual = recipe_call(
            &mut owners,
            &seed,
            &mut expression_ordinal,
            local_route,
            Vec::new(),
            vec![method, path],
        )?;
        let expected = recipe_expression(
            &mut owners,
            &seed,
            &mut expression_ordinal,
            ExpressionOperation::Bool { value: expected },
        )?;
        insert_owner(
            &mut owners,
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(OwnerKey::Declaration(test), OwnerKind::Test),
                module,
                name: Name::new(name)?,
                visibility: DeclarationVisibility::Private,
                payload: DeclarationPayload::Test {
                    actual,
                    expected,
                    comparison: ComparisonPolicy::Exact,
                },
            }),
        )?;
    }

    let dependency = DependencyRecord {
        graph_contract_version: super::kernel::contract::GRAPH_CONTRACT_VERSION,
        package: standard.package,
        semantic_revision: standard.semantic_revision,
        package_revision: standard.package_revision,
    };
    let dependency_interfaces = BTreeMap::from([(
        standard.package_revision,
        standard
            .interface_owners
            .iter()
            .map(|(owner, value)| (*owner, value.record.clone()))
            .collect(),
    )]);
    let owners_len = owners.len();
    let descriptor = encode_deployment(&super::deployment::starter_nostr_relay_deployment(
        &relay.endpoint,
        relay.address_policy,
    )?)?;
    Ok(ProjectRecipe {
        snapshot: KernelSnapshot {
            root: SemanticRoot {
                graph_contract_version: super::kernel::contract::GRAPH_CONTRACT_VERSION,
                repository_id: repository,
                package_id: package,
                package_name,
                owners: placeholder_map(owners_len),
                dependencies: placeholder_map(1),
                retirements: placeholder_map(0),
            },
            owners,
            types: interner.into_objects(),
            dependency_interfaces,
            dependency_types: standard.interface_types.clone(),
            blobs: BTreeMap::new(),
            dependencies: BTreeMap::from([(standard.package, dependency)]),
            retirements: BTreeMap::new(),
        },
        transports: vec![standard.transport()],
        template: ProjectTemplate::NostrRelayInfo,
        targets: 1,
        tests: 2,
        auxiliary: Some(ProjectAuxiliary { descriptor }),
    })
}

fn recipe_expression(
    owners: &mut BTreeMap<OwnerKey, OwnerRecord>,
    seed: &[u8],
    ordinal: &mut u64,
    operation: ExpressionOperation,
) -> Result<ExpressionId, Diagnostic> {
    let id = ExpressionId::migrate(seed, *ordinal);
    *ordinal = ordinal.saturating_add(1);
    insert_owner(
        owners,
        OwnerRecord::Expression(ExpressionRecord::new(id, operation)?),
    )?;
    Ok(id)
}

fn recipe_text(
    owners: &mut BTreeMap<OwnerKey, OwnerRecord>,
    seed: &[u8],
    ordinal: &mut u64,
    value: &str,
) -> Result<ExpressionId, Diagnostic> {
    recipe_expression(
        owners,
        seed,
        ordinal,
        ExpressionOperation::Text {
            value: TextValue::Inline {
                text: value.to_owned(),
            },
        },
    )
}

fn recipe_call(
    owners: &mut BTreeMap<OwnerKey, OwnerRecord>,
    seed: &[u8],
    ordinal: &mut u64,
    function: DeclarationReference,
    type_arguments: Vec<TypeObjectDigest>,
    arguments: Vec<ExpressionId>,
) -> Result<ExpressionId, Diagnostic> {
    recipe_expression(
        owners,
        seed,
        ordinal,
        ExpressionOperation::Call {
            function,
            type_arguments,
            arguments,
        },
    )
}

fn recipe_field(
    owners: &mut BTreeMap<OwnerKey, OwnerRecord>,
    seed: &[u8],
    ordinal: &mut u64,
    value: ExpressionId,
    name: &str,
) -> Result<ExpressionId, Diagnostic> {
    recipe_expression(
        owners,
        seed,
        ordinal,
        ExpressionOperation::Field {
            value,
            selector: FieldSelector::Structural(Name::new(name)?),
        },
    )
}

fn recipe_record(
    owners: &mut BTreeMap<OwnerKey, OwnerRecord>,
    seed: &[u8],
    ordinal: &mut u64,
    fields: Vec<(&str, ExpressionId)>,
) -> Result<ExpressionId, Diagnostic> {
    recipe_expression(
        owners,
        seed,
        ordinal,
        ExpressionOperation::Record {
            nominal_type: None,
            fields: fields
                .into_iter()
                .map(|(name, value)| {
                    Ok(RecordExpressionField {
                        selector: FieldSelector::Structural(Name::new(name)?),
                        value,
                    })
                })
                .collect::<Result<Vec<_>, Diagnostic>>()?,
        },
    )
}

fn recipe_bytes(
    owners: &mut BTreeMap<OwnerKey, OwnerRecord>,
    seed: &[u8],
    ordinal: &mut u64,
    bytes_from_text: DeclarationReference,
    value: &str,
) -> Result<ExpressionId, Diagnostic> {
    let text = recipe_text(owners, seed, ordinal, value)?;
    recipe_call(
        owners,
        seed,
        ordinal,
        bytes_from_text,
        Vec::new(),
        vec![text],
    )
}

#[allow(clippy::too_many_arguments)]
fn recipe_http_response(
    owners: &mut BTreeMap<OwnerKey, OwnerRecord>,
    seed: &[u8],
    ordinal: &mut u64,
    bytes_from_text: DeclarationReference,
    header_type: TypeObjectDigest,
    status: i64,
    body: &str,
) -> Result<ExpressionId, Diagnostic> {
    let body = recipe_bytes(owners, seed, ordinal, bytes_from_text, body)?;
    let headers = recipe_expression(
        owners,
        seed,
        ordinal,
        ExpressionOperation::List {
            item_type: header_type,
            items: Vec::new(),
        },
    )?;
    let status = recipe_expression(
        owners,
        seed,
        ordinal,
        ExpressionOperation::I64 { value: status },
    )?;
    recipe_record(
        owners,
        seed,
        ordinal,
        vec![("body", body), ("headers", headers), ("status", status)],
    )
}

fn insert_owner(
    owners: &mut BTreeMap<OwnerKey, OwnerRecord>,
    record: OwnerRecord,
) -> Result<(), Diagnostic> {
    let owner = record.owner();
    if owners.insert(owner, record).is_some() {
        return Err(creation_error(
            DiagnosticClass::Corrupt,
            "new_recipe_owner_duplicate",
            format!("typed project recipe repeats owner {owner}"),
        ));
    }
    Ok(())
}

fn placeholder_map(entries: usize) -> MapRoot {
    MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        u64::try_from(entries).unwrap_or(u64::MAX),
        MapContentDigest::from_bytes([0; 32]),
    )
}

fn empty_snapshot(
    repository_id: RepositoryId,
    package_id: PackageId,
    package_name: Name,
) -> KernelSnapshot {
    let empty = MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        0,
        MapContentDigest::from_bytes([0; 32]),
    );
    KernelSnapshot {
        root: SemanticRoot {
            graph_contract_version: super::kernel::contract::GRAPH_CONTRACT_VERSION,
            repository_id,
            package_id,
            package_name,
            owners: empty,
            dependencies: empty,
            retirements: empty,
        },
        owners: BTreeMap::new(),
        types: BTreeMap::new(),
        dependency_interfaces: BTreeMap::new(),
        dependency_types: BTreeMap::new(),
        blobs: BTreeMap::new(),
        dependencies: BTreeMap::new(),
        retirements: BTreeMap::new(),
    }
}

fn safe_destination(destination: &Path) -> Result<PathBuf, Diagnostic> {
    let name = destination.file_name().ok_or_else(|| {
        creation_error(
            DiagnosticClass::Source,
            "new_destination",
            "project destination must name one directory below an existing parent",
        )
    })?;
    if destination
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(creation_error(
            DiagnosticClass::Source,
            "new_destination_traversal",
            "project destination may not contain '..'",
        ));
    }
    let parent = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    reject_symlinked_path(parent)?;
    let metadata = fs::symlink_metadata(parent).map_err(|error| {
        creation_error(
            DiagnosticClass::Source,
            "new_destination_parent",
            format!(
                "project parent '{}' is unavailable: {error}",
                parent.display()
            ),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(creation_error(
            DiagnosticClass::Source,
            "new_destination_parent_type",
            format!(
                "project parent '{}' is not an ordinary directory",
                parent.display()
            ),
        ));
    }
    let parent = parent.canonicalize().map_err(|error| {
        creation_error(
            DiagnosticClass::Source,
            "new_destination_parent",
            format!(
                "project parent '{}' is unavailable: {error}",
                parent.display()
            ),
        )
    })?;
    let destination = parent.join(name);
    match fs::symlink_metadata(&destination) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            Err(creation_error(
                DiagnosticClass::Source,
                "new_destination_type",
                format!(
                    "project destination '{}' is not an ordinary directory",
                    destination.display()
                ),
            ))
        }
        Ok(_) if predecessor_project(&destination) => Err(creation_error(
            DiagnosticClass::Source,
            "predecessor_contract",
            format!(
                "project destination '{}' contains predecessor authority",
                destination.display()
            ),
        )),
        Ok(_) => Err(creation_error(
            DiagnosticClass::Source,
            "new_destination_not_empty",
            format!(
                "project destination '{}' already exists",
                destination.display()
            ),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(destination),
        Err(error) => Err(creation_error(
            DiagnosticClass::Infrastructure,
            "new_destination_inspect",
            format!(
                "project destination '{}' could not be inspected: {error}",
                destination.display()
            ),
        )),
    }
}

fn reject_symlinked_path(path: &Path) -> Result<(), Diagnostic> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| {
                creation_error(
                    DiagnosticClass::Infrastructure,
                    "new_current_directory",
                    format!("current directory is unavailable: {error}"),
                )
            })?
            .join(path)
    };
    let mut checked = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::Prefix(prefix) => checked.push(prefix.as_os_str()),
            Component::RootDir => checked.push(Path::new("/")),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(creation_error(
                    DiagnosticClass::Source,
                    "new_destination_traversal",
                    "project destination may not contain '..'",
                ));
            }
            Component::Normal(value) => {
                checked.push(value);
                let metadata = fs::symlink_metadata(&checked).map_err(|error| {
                    creation_error(
                        DiagnosticClass::Source,
                        "new_destination_parent",
                        format!(
                            "project parent '{}' is unavailable: {error}",
                            checked.display()
                        ),
                    )
                })?;
                if metadata.file_type().is_symlink() {
                    return Err(creation_error(
                        DiagnosticClass::Source,
                        "new_destination_symlink",
                        format!(
                            "project parent '{}' traverses a symbolic link",
                            checked.display()
                        ),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn predecessor_project(path: &Path) -> bool {
    [path.join(".lkjscript"), path.join("lkjscript.package.json")]
        .into_iter()
        .any(|marker| fs::symlink_metadata(marker).is_ok())
}

fn sync_parent_directory(path: &Path) -> Result<(), Diagnostic> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            creation_error(
                DiagnosticClass::Infrastructure,
                "new_parent_sync",
                format!(
                    "project parent '{}' could not be synchronized: {error}",
                    path.display()
                ),
            )
        })
}

fn sync_stage_directory(path: &Path) -> Result<(), Diagnostic> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            creation_error(
                DiagnosticClass::Infrastructure,
                "new_stage_sync",
                format!(
                    "private project path '{}' could not be synchronized: {error}",
                    path.display()
                ),
            )
        })
}

fn remove_owned_stage(stage: &Path) {
    let _ = fs::remove_dir_all(stage);
}

fn creation_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_recipe_prepares_checks_and_runs_through_both_normalized_tiers() {
        let temporary = tempfile::TempDir::new().expect("temporary command parent");
        let destination = temporary.path().join("command");
        let created = create_project(&destination, "command", ProjectTemplate::Command)
            .expect("command project");
        assert_eq!(created.template, ProjectTemplate::Command);
        assert_eq!(created.dependencies, 1);
        assert_eq!(created.targets, 1);
        assert_eq!(created.tests, 1);
        assert_eq!(created.deployment, None);

        let prepared = super::super::normalized_lifecycle::prepare_application(&destination)
            .expect("prepare normalized command");
        let control = super::super::execution::ExecutionControl::default();
        let checked = prepared.check(&control).expect("check command graph tests");
        assert_eq!(checked.passed, 14);
        assert_eq!(checked.failed, 0);
        assert_eq!(checked.differential, "equal");
        let run = prepared
            .run(
                &Name::new("main").expect("target name"),
                b"[]",
                super::super::execution::normalized::NormalizedCommandPolicy::default(),
                &control,
            )
            .expect("run normalized command");
        assert_eq!(run.result_json, b"\"hello\"");
        assert_eq!(run.differential, "equal");
    }

    #[test]
    fn http_recipe_prepares_and_checks_with_an_atomic_starter_deployment() {
        let temporary = tempfile::TempDir::new().expect("temporary HTTP parent");
        let destination = temporary.path().join("http");
        let created =
            create_project(&destination, "http", ProjectTemplate::Http).expect("HTTP project");
        assert_eq!(created.template, ProjectTemplate::Http);
        assert_eq!(created.dependencies, 1);
        assert_eq!(created.targets, 1);
        assert_eq!(created.tests, 1);
        let deployment = created.deployment.expect("starter deployment");
        assert_eq!(
            deployment.descriptor,
            destination.join(STARTER_HTTP_DESCRIPTOR_PATH)
        );
        assert_eq!(
            deployment.recommended_artifact_output,
            destination.join(STARTER_HTTP_ARTIFACT_PATH)
        );
        assert_eq!(deployment.target, "serve");
        assert_eq!(deployment.runner, "http");
        assert_eq!(deployment.configured_listener, "127.0.0.1:0");
        assert!(destination.join(STARTER_HTTP_ARTIFACT_DIRECTORY).is_dir());
        assert!(
            fs::read_dir(destination.join(STARTER_HTTP_ARTIFACT_DIRECTORY))
                .expect("generated directory")
                .next()
                .is_none()
        );
        assert!(!deployment.recommended_artifact_output.exists());

        let descriptor_bytes = fs::read(&deployment.descriptor).expect("starter descriptor");
        assert_eq!(descriptor_bytes.last(), Some(&b'\n'));
        let descriptor = super::super::deployment::decode_deployment(&descriptor_bytes)
            .expect("strict starter descriptor");
        assert_eq!(descriptor.artifact, STARTER_HTTP_ARTIFACT_PATH);
        assert_eq!(descriptor.target, STARTER_HTTP_TARGET);
        assert_eq!(descriptor.listen.as_deref(), Some(STARTER_HTTP_LISTENER));
        assert_eq!(descriptor.grants.len(), 1);
        assert_eq!(descriptor.grants[0].requirement, "streams");
        assert!(matches!(
            descriptor.grants[0].adapter,
            super::super::deployment::AdapterDescriptor::ByteStream
        ));

        let prepared = super::super::normalized_lifecycle::prepare_application(&destination)
            .expect("prepare normalized HTTP application");
        let checked = prepared
            .check(&super::super::execution::ExecutionControl::default())
            .expect("check HTTP graph tests");
        assert_eq!(checked.passed, 14);
        assert_eq!(checked.failed, 0);
        assert_eq!(checked.differential, "equal");

        fs::write(
            &deployment.recommended_artifact_output,
            &prepared.artifact_bytes,
        )
        .expect("publish test artifact");
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("deployment runtime");
        let prepared_deployment = super::super::deployment::PreparedDeployment::load(
            &deployment.descriptor,
            runtime.handle().clone(),
        )
        .expect("prepare starter deployment");
        let observation = prepared_deployment.observe_redacted();
        assert_eq!(observation.target, "serve");
        assert_eq!(observation.runner, "http");
        assert_eq!(observation.listen.as_deref(), Some("127.0.0.1:0"));
        assert_eq!(
            observation.grants.get("streams").map(String::as_str),
            Some("byte-stream")
        );
        prepared_deployment
            .http_application()
            .expect("prepare exact HTTP runner");

        for (name, mutate, expected) in [
            (
                "missing-target",
                (|descriptor: &mut super::super::deployment::DeploymentDescriptor| {
                    descriptor.target = "foreign".to_owned();
                }) as fn(&mut super::super::deployment::DeploymentDescriptor),
                "deployment_target_missing",
            ),
            (
                "wrong-runner",
                |descriptor: &mut super::super::deployment::DeploymentDescriptor| {
                    descriptor.http = None;
                },
                "deployment_http_incomplete",
            ),
            (
                "missing-grant",
                |descriptor: &mut super::super::deployment::DeploymentDescriptor| {
                    descriptor.grants.clear();
                },
                "deployment_grant_missing",
            ),
        ] {
            let mut invalid = starter_http_deployment().expect("invalid case base");
            mutate(&mut invalid);
            let path = destination.join(format!("{name}.json"));
            fs::write(
                &path,
                super::super::deployment::encode_deployment(&invalid)
                    .expect("encode structurally valid negative descriptor"),
            )
            .expect("publish negative descriptor");
            let error =
                super::super::deployment::PreparedDeployment::load(&path, runtime.handle().clone())
                    .expect_err("deployment mismatch must reject before readiness");
            assert_eq!(error.code, expected, "{name}");
        }

        let mut inconsistent = starter_http_deployment().expect("inconsistent descriptor base");
        inconsistent.streams.maximum_total_bytes = 1;
        let inconsistent_path = destination.join("inconsistent-streams.json");
        fs::write(
            &inconsistent_path,
            super::super::deployment::encode_deployment(&inconsistent)
                .expect("encode independently valid inconsistent limits"),
        )
        .expect("publish inconsistent descriptor");
        let inconsistent = super::super::deployment::PreparedDeployment::load(
            &inconsistent_path,
            runtime.handle().clone(),
        )
        .expect("prepare deployment before HTTP cross-limit validation");
        assert_eq!(
            inconsistent
                .http_application()
                .err()
                .expect("HTTP request/stream inconsistency must reject")
                .code,
            "normalized_http_stream_limit"
        );

        let mut duplicate = starter_http_deployment().expect("duplicate grant base");
        duplicate.grants.push(duplicate.grants[0].clone());
        assert_eq!(
            super::super::deployment::encode_deployment(&duplicate)
                .expect_err("duplicate stream grants must reject")
                .code,
            "deployment_grant_duplicate"
        );
    }

    #[test]
    fn nostr_relay_info_recipe_binds_one_exact_loopback_client_without_network_readiness() {
        let temporary = tempfile::TempDir::new().expect("temporary Nostr parent");
        let destination = temporary.path().join("nostr-relay-info");
        let created = create_project_with_relay(
            &destination,
            "nostr-relay-info",
            "ws://127.0.0.1:7447/nostr",
        )
        .expect("Nostr relay-info project");
        assert_eq!(created.template, ProjectTemplate::NostrRelayInfo);
        assert_eq!(created.dependencies, 1);
        assert_eq!(created.targets, 1);
        assert_eq!(created.tests, 2);
        let deployment = created.deployment.expect("starter deployment");
        let descriptor_bytes = fs::read(&deployment.descriptor).expect("starter descriptor");
        let descriptor = super::super::deployment::decode_deployment(&descriptor_bytes)
            .expect("strict starter descriptor");
        assert_eq!(descriptor.grants.len(), 2);
        let relay = descriptor
            .grants
            .iter()
            .find(|grant| grant.requirement == "relay")
            .expect("relay grant");
        assert!(matches!(
            &relay.adapter,
            super::super::deployment::AdapterDescriptor::HttpClient {
                endpoint,
                address_policy: super::super::http_client::HttpClientAddressPolicy::LoopbackOnly,
                trust: super::super::http_client::HttpClientTrust::WebpkiRoots,
                ..
            } if endpoint == "http://127.0.0.1:7447/nostr"
        ));

        let prepared = super::super::normalized_lifecycle::prepare_application(&destination)
            .expect("prepare Nostr relay-info application");
        let checked = prepared
            .check(&super::super::execution::ExecutionControl::default())
            .expect("check Nostr graph tests");
        assert_eq!(checked.passed, 15);
        assert_eq!(checked.failed, 0);
        assert_eq!(checked.differential, "equal");
        fs::write(
            &deployment.recommended_artifact_output,
            &prepared.artifact_bytes,
        )
        .expect("publish test artifact");
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("deployment runtime");
        let prepared_deployment = super::super::deployment::PreparedDeployment::load(
            &deployment.descriptor,
            runtime.handle().clone(),
        )
        .expect("prepare Nostr starter deployment without network I/O");
        assert_eq!(
            prepared_deployment
                .observe_redacted()
                .grants
                .get("relay")
                .map(String::as_str),
            Some("http-client")
        );
        prepared_deployment
            .http_application()
            .expect("prepare Nostr HTTP runner");
    }

    #[test]
    fn auxiliary_publication_failures_remove_the_complete_owned_stage() {
        for failure_point in [
            CreationPoint::BeforeDescriptor,
            CreationPoint::DescriptorPublished,
            CreationPoint::GeneratedDirectoryPublished,
        ] {
            let temporary = tempfile::TempDir::new().expect("temporary HTTP parent");
            let destination = temporary.path().join("http");
            let error = create_project_with_hook(
                &destination,
                "http",
                ProjectTemplate::Http,
                |point, _, _| {
                    if point == failure_point {
                        Err(creation_error(
                            DiagnosticClass::Infrastructure,
                            "test_auxiliary_failure",
                            "injected auxiliary publication failure",
                        ))
                    } else {
                        Ok(())
                    }
                },
            )
            .expect_err("injected auxiliary failure must reject creation");
            assert_eq!(error.code, "test_auxiliary_failure");
            assert!(!destination.exists());
            let stages = fs::read_dir(temporary.path())
                .expect("creation parent")
                .map(|entry| entry.expect("parent entry").file_name())
                .filter(|name| {
                    name.to_string_lossy()
                        .starts_with(".lkjscript-project-stage-")
                })
                .collect::<Vec<_>>();
            assert!(stages.is_empty(), "owned private stages remain: {stages:?}");
        }
    }

    #[test]
    fn failed_visibility_rename_removes_only_the_owned_private_stage() {
        let temporary = tempfile::TempDir::new().expect("temporary creation parent");
        let destination = temporary.path().join("raced");
        let error = create_project_with_hook(
            &destination,
            "raced",
            ProjectTemplate::Minimal,
            |point, _, visible_destination| {
                if point == CreationPoint::BeforeVisibility {
                    fs::create_dir(visible_destination).map_err(|error| {
                        creation_error(
                            DiagnosticClass::Infrastructure,
                            "test_destination_race",
                            error.to_string(),
                        )
                    })?;
                    fs::write(visible_destination.join("owned.txt"), b"preserve\n").map_err(
                        |error| {
                            creation_error(
                                DiagnosticClass::Infrastructure,
                                "test_destination_race",
                                error.to_string(),
                            )
                        },
                    )?;
                }
                Ok(())
            },
        )
        .expect_err("nonempty raced destination must reject publication");
        assert_eq!(error.code, "new_destination_publish");
        assert_eq!(
            fs::read(destination.join("owned.txt")).expect("preserved destination"),
            b"preserve\n"
        );
        let stages = fs::read_dir(temporary.path())
            .expect("creation parent")
            .map(|entry| entry.expect("parent entry").file_name())
            .filter(|name| {
                name.to_string_lossy()
                    .starts_with(".lkjscript-project-stage-")
            })
            .collect::<Vec<_>>();
        assert!(stages.is_empty(), "owned private stages remain: {stages:?}");
    }
}
