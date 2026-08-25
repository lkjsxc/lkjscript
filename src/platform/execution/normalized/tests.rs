//! Focused normalized artifact-preparation and dense-execution tests.

use super::capability::{
    NormalizedAdapterKind, NormalizedCallPolicy, NormalizedCapabilities,
    NormalizedCapabilityAdapter, NormalizedCapabilityGrant, NormalizedCapabilityGrantDescriptor,
    NormalizedCapabilityTransaction, NormalizedGrantAuthorityRevision,
    NormalizedGrantDescriptorDigest, NormalizedGrantLimit, NormalizedSharingDomain,
    NormalizedTransactionPolicy,
};
use super::codec::{decode_typed, encode_typed};
use super::deployment::{
    NormalizedAdapterDescriptor, NormalizedDeploymentGrant, NormalizedDeploymentResourcePolicy,
    NormalizedPreparedDeployment,
};
use super::http::NormalizedHttpApplication;
use super::prepare::{NormalizedFunctionBody, NormalizedInstruction, NormalizedProgram};
use super::reference::{
    NormalizedReferenceBinding, NormalizedReferenceInterpreter, NormalizedReferenceOwnerRead,
    NormalizedReferenceRead,
};
use super::resident::NormalizedResidentDeployment;
use super::resource::NormalizedResourceScope;
use super::runner::{
    NormalizedCommandPolicy, run_effectful_command, run_graph_tests, run_pure_command,
};
use super::value::{NormalizedMapKey, NormalizedValue};
use super::vm::{NormalizedRunPolicy, NormalizedVm};
use super::worker::NormalizedWorkerApplication;
use crate::platform::change::{
    AuthoredChange, AuthoredChangeSet, AuthoredDeclarationReference, AuthoredExpression,
    AuthoredExpressionOperation, AuthoredFunctionEffect, AuthoredType, ChangeBudget,
    ModuleSelector,
};
use crate::platform::compiler::{OptimizationPolicy, build_clean, link_artifact, load_artifact};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::json::JsonLimits;
use crate::platform::kernel::{
    DeclarationPayload, DeclarationRecord, DeclarationReference, DeclarationVisibility,
    ExpressionOperation, ExpressionRecord, ExternalVisibility, FieldSelector, FunctionDeclaration,
    FunctionEffect, Idempotency, LocalValueReference, Name, OperationRecord, OwnerHeader, OwnerKey,
    OwnerKind, OwnerRecord, ParameterParent, ParameterRecord, PortImplementation, PortRecord,
    PortReference, RecordExpressionField, ResourceLimit, ResourceUnit, StructuralTypeField,
    TargetRecord, TypeForm, TypeObject, TypeObjectDigest, encode_type_object,
};
use crate::platform::package::RunnerKind;
use crate::platform::persistent_map::{MapRoot, PageDigest};
use crate::platform::publication::{
    GraphRepository, PublicationOptions, PublicationOutcome, RepositoryView,
};
use crate::platform::secrets::SecretCatalog;
use crate::platform::semantic_id::{
    DeclarationId, ExpressionId, OperationId, ParameterId, PortId, RevisionId, TargetId,
};
use crate::platform::stream::{StreamLimits, StreamRegistry};
use crate::platform::{HttpHeader, HttpLimits, HttpRequest, ResidentLimits, WorkerLimits};
use axum::body::{Body, to_bytes};
use axum::http::Request;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

fn declaration_named(
    snapshot: &crate::platform::kernel::KernelSnapshot,
    name: &str,
) -> DeclarationReference {
    let declaration = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record))
                if record.name.as_str() == name =>
            {
                Some(*declaration)
            }
            _ => None,
        })
        .expect("named fixture declaration");
    DeclarationReference {
        package: snapshot.root.package_id,
        declaration,
    }
}

fn prepare_snapshot(snapshot: &crate::platform::kernel::KernelSnapshot) -> NormalizedProgram {
    prepare_repository(snapshot).2
}

fn prepare_repository(
    snapshot: &crate::platform::kernel::KernelSnapshot,
) -> (tempfile::TempDir, GraphRepository, NormalizedProgram) {
    let temporary = tempfile::tempdir().expect("normalized runtime parent");
    let created = GraphRepository::create(&temporary.path().join("repository"), snapshot, None)
        .expect("Graph 5 repository");
    let compilation = build_clean(
        &created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("normalized compilation");
    let linked = link_artifact(&created.repository, compilation.manifest_digest, &[])
        .expect("Graph 5 artifact");
    let loaded = load_artifact(&linked.artifact.bytes).expect("strict Graph 5 artifact");
    let program = NormalizedProgram::prepare(loaded).expect("dense runtime preparation");
    (temporary, created.repository, program)
}

fn empty_normalized_snapshot(seed: &[u8]) -> crate::platform::kernel::KernelSnapshot {
    let empty = MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        0,
        crate::platform::persistent_map::MapContentDigest::from_bytes([0; 32]),
    );
    crate::platform::kernel::KernelSnapshot {
        root: crate::platform::kernel::SemanticRoot {
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            repository_id: crate::platform::semantic_id::RepositoryId::migrate(seed, 0),
            package_id: crate::platform::kernel::PackageId::migrate(seed, 0),
            package_name: Name::new("linked_fixture").expect("linked fixture package name"),
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

fn linked_pure_program() -> (
    tempfile::TempDir,
    GraphRepository,
    GraphRepository,
    NormalizedProgram,
    DeclarationReference,
    DeclarationReference,
) {
    let temporary = tempfile::tempdir().expect("linked reference parent");
    let source_created = GraphRepository::create(
        &temporary.path().join("source"),
        &empty_normalized_snapshot(b"normalized-reference-source"),
        None,
    )
    .expect("linked source repository");
    let source_change = AuthoredChangeSet {
        base: source_created.current.head.revision,
        preconditions: Vec::new(),
        changes: vec![
            AuthoredChange::CreateModule {
                symbol: "$source_module".to_owned(),
                name: Name::new("library").unwrap(),
            },
            AuthoredChange::CreateFunction {
                symbol: "$source_function".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$source_module".to_owned(),
                },
                name: Name::new("produce").unwrap(),
                visibility: DeclarationVisibility::Public,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: AuthoredType::Unit {},
                effect: AuthoredFunctionEffect::Pure {},
                body: AuthoredExpression {
                    symbol: Some("$source_body".to_owned()),
                    operation: AuthoredExpressionOperation::Unit {},
                },
            },
        ],
        budget: ChangeBudget::default(),
    };
    let prepared_source = source_created
        .repository
        .prepare_authored_change(&source_change, PublicationOptions::default())
        .expect("prepare linked source");
    let OwnerKey::Declaration(source_declaration) = prepared_source.allocated["$source_function"]
    else {
        panic!("source function allocation kind")
    };
    assert!(matches!(
        source_created
            .repository
            .publish(&prepared_source.publication)
            .expect("publish linked source"),
        PublicationOutcome::Accepted { .. }
    ));
    let source_reference = DeclarationReference {
        package: source_created.current.semantic_root.package_id,
        declaration: source_declaration,
    };
    let exported = source_created
        .repository
        .export_package_transport()
        .expect("export linked source package");
    let source_compilation = build_clean(
        &source_created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("compile linked source");
    let source_artifact = link_artifact(
        &source_created.repository,
        source_compilation.manifest_digest,
        &[],
    )
    .expect("link source artifact");
    let source_loaded =
        load_artifact(&source_artifact.artifact.bytes).expect("load source artifact");

    let target_created = GraphRepository::create(
        &temporary.path().join("target"),
        &empty_normalized_snapshot(b"normalized-reference-target"),
        None,
    )
    .expect("linked target repository");
    target_created
        .repository
        .stage_package_transport(exported.transport_digest, &exported.packs)
        .expect("stage exact linked source package");
    let target_change = AuthoredChangeSet {
        base: target_created.current.head.revision,
        preconditions: Vec::new(),
        changes: vec![
            AuthoredChange::AddDependency {
                package: exported.revision.package,
                semantic_revision: exported.revision.revision.revision_id().unwrap(),
                package_revision: exported.revision_digest,
            },
            AuthoredChange::CreateModule {
                symbol: "$target_module".to_owned(),
                name: Name::new("application").unwrap(),
            },
            AuthoredChange::CreateFunction {
                symbol: "$caller".to_owned(),
                module: ModuleSelector::Symbol {
                    symbol: "$target_module".to_owned(),
                },
                name: Name::new("call_library").unwrap(),
                visibility: DeclarationVisibility::Private,
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: AuthoredType::Unit {},
                effect: AuthoredFunctionEffect::Pure {},
                body: AuthoredExpression {
                    symbol: Some("$call".to_owned()),
                    operation: AuthoredExpressionOperation::Call {
                        function: AuthoredDeclarationReference::Exact {
                            package: source_reference.package,
                            declaration: source_reference.declaration,
                        },
                        type_arguments: Vec::new(),
                        arguments: Vec::new(),
                    },
                },
            },
        ],
        budget: ChangeBudget::default(),
    };
    let prepared_target = target_created
        .repository
        .prepare_authored_change(&target_change, PublicationOptions::default())
        .expect("prepare linked target");
    let OwnerKey::Declaration(caller_declaration) = prepared_target.allocated["$caller"] else {
        panic!("caller allocation kind")
    };
    assert!(matches!(
        target_created
            .repository
            .publish(&prepared_target.publication)
            .expect("publish linked target"),
        PublicationOutcome::Accepted { .. }
    ));
    let caller_reference = DeclarationReference {
        package: target_created.current.semantic_root.package_id,
        declaration: caller_declaration,
    };
    let target_compilation = build_clean(
        &target_created.repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .expect("compile linked target");
    let target_artifact = link_artifact(
        &target_created.repository,
        target_compilation.manifest_digest,
        std::slice::from_ref(&source_loaded),
    )
    .expect("link exact two-package artifact");
    let target_loaded =
        load_artifact(&target_artifact.artifact.bytes).expect("load linked target artifact");
    let program = NormalizedProgram::prepare(target_loaded).expect("prepare linked program");
    (
        temporary,
        source_created.repository,
        target_created.repository,
        program,
        source_reference,
        caller_reference,
    )
}

fn pure_command_snapshot() -> crate::platform::kernel::KernelSnapshot {
    const SEED: &[u8] = b"normalized-pure-command-runner";

    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let implementation = declaration_named(&snapshot, "with_binding");
    let module = match snapshot
        .owners
        .get(&OwnerKey::Declaration(implementation.declaration))
        .expect("pure command implementation")
    {
        OwnerRecord::Declaration(record) => record.module,
        _ => panic!("pure command implementation owner kind"),
    };
    let function_type = snapshot
        .types
        .iter()
        .find_map(|(digest, object)| match &object.form {
            TypeForm::Function { parameters, result }
                if parameters.is_empty()
                    && snapshot
                        .types
                        .get(result)
                        .is_some_and(|object| matches!(object.form, TypeForm::Unit)) =>
            {
                Some(*digest)
            }
            _ => None,
        })
        .expect("fixture unit command function type");
    let component = DeclarationId::migrate(SEED, 0);
    let port = PortId::migrate(SEED, 0);
    let target = TargetId::migrate(SEED, 0);
    let package = snapshot.root.package_id;

    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Port(port),
                OwnerRecord::Port(PortRecord {
                    header: OwnerHeader::new(OwnerKey::Port(port), OwnerKind::Port),
                    declaration: component,
                    name: Name::new("run").unwrap(),
                    function_type,
                    implementation: PortImplementation::Function(implementation),
                }),
            )
            .is_none()
    );
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Declaration(component),
                OwnerRecord::Declaration(DeclarationRecord {
                    header: OwnerHeader::new(
                        OwnerKey::Declaration(component),
                        OwnerKind::Component,
                    ),
                    module,
                    name: Name::new("PureApplication").unwrap(),
                    visibility: DeclarationVisibility::Public,
                    payload: DeclarationPayload::Component {
                        requirements: Vec::new(),
                        ports: vec![port],
                    },
                }),
            )
            .is_none()
    );
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Target(target),
                OwnerRecord::Target(TargetRecord {
                    header: OwnerHeader::new(OwnerKey::Target(target), OwnerKind::Target),
                    name: Name::new("pure").unwrap(),
                    component: DeclarationReference {
                        package,
                        declaration: component,
                    },
                    port: PortReference { package, port },
                    runner: RunnerKind::Command,
                }),
            )
            .is_none()
    );
    snapshot.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    snapshot
}

fn normalized_worker_snapshot() -> crate::platform::kernel::KernelSnapshot {
    const SEED: &[u8] = b"normalized-worker-runner";

    let mut snapshot = pure_command_snapshot();
    let bool_type = admit_snapshot_type(&mut snapshot, TypeForm::Bool);
    let port_type = admit_snapshot_type(
        &mut snapshot,
        TypeForm::Function {
            parameters: Vec::new(),
            result: bool_type,
        },
    );
    let implementation = declaration_named(&snapshot, "with_binding");
    let module = match snapshot
        .owners
        .get(&OwnerKey::Declaration(implementation.declaration))
        .expect("worker fixture implementation")
    {
        OwnerRecord::Declaration(record) => record.module,
        _ => panic!("worker fixture implementation kind"),
    };
    let function = DeclarationId::migrate(SEED, 0);
    let body = ExpressionId::migrate(SEED, 0);
    let package = snapshot.root.package_id;
    let (target, port) = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Target(target), OwnerRecord::Target(record))
                if record.name.as_str() == "pure" =>
            {
                Some((*target, record.port.port))
            }
            _ => None,
        })
        .expect("pure command target and port");
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Expression(body),
                OwnerRecord::Expression(
                    ExpressionRecord::new(body, ExpressionOperation::Bool { value: false })
                        .expect("worker idle result"),
                ),
            )
            .is_none()
    );
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Declaration(function),
                OwnerRecord::Declaration(DeclarationRecord {
                    header: OwnerHeader::new(
                        OwnerKey::Declaration(function),
                        OwnerKind::PureFunction,
                    ),
                    module,
                    name: Name::new("worker_iteration").unwrap(),
                    visibility: DeclarationVisibility::Package,
                    payload: DeclarationPayload::Function(FunctionDeclaration {
                        type_parameters: Vec::new(),
                        parameters: Vec::new(),
                        result: bool_type,
                        effect: FunctionEffect::Pure,
                        body,
                    }),
                }),
            )
            .is_none()
    );
    let OwnerRecord::Port(port_record) = snapshot
        .owners
        .get_mut(&OwnerKey::Port(port))
        .expect("pure command port")
    else {
        panic!("pure command port owner kind")
    };
    port_record.function_type = port_type;
    port_record.implementation = PortImplementation::Function(DeclarationReference {
        package,
        declaration: function,
    });
    let OwnerRecord::Target(target_record) = snapshot
        .owners
        .get_mut(&OwnerKey::Target(target))
        .expect("pure command target")
    else {
        panic!("pure command target owner kind")
    };
    target_record.name = Name::new("work").unwrap();
    target_record.runner = RunnerKind::Worker;
    snapshot.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    snapshot
}

fn wall_clock_command_snapshot() -> crate::platform::kernel::KernelSnapshot {
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let i64_object = TypeObject::new(TypeForm::I64).expect("I64 type object");
    let (i64_type, _) = encode_type_object(&i64_object).expect("I64 type encoding");
    snapshot.types.insert(i64_type, i64_object);
    let function_object = TypeObject::new(TypeForm::Function {
        parameters: Vec::new(),
        result: i64_type,
    })
    .expect("wall-clock port type");
    let (function_type, _) =
        encode_type_object(&function_object).expect("wall-clock port type encoding");
    snapshot.types.retain(|digest, object| {
        !matches!(object.form, TypeForm::Function { .. }) || *digest == function_type
    });
    snapshot.types.insert(function_type, function_object);

    for record in snapshot.owners.values_mut() {
        match record {
            OwnerRecord::Operation(operation) if operation.name.as_str() == "read" => {
                operation.name = Name::new("utc-milliseconds").unwrap();
                operation.result = i64_type;
            }
            OwnerRecord::Declaration(declaration) if declaration.name.as_str() == "caller" => {
                let DeclarationPayload::Function(function) = &mut declaration.payload else {
                    panic!("caller must remain a function")
                };
                function.result = i64_type;
            }
            OwnerRecord::Port(port) => {
                port.function_type = function_type;
            }
            _ => {}
        }
    }
    snapshot
}

fn admit_snapshot_type(
    snapshot: &mut crate::platform::kernel::KernelSnapshot,
    form: TypeForm,
) -> TypeObjectDigest {
    let object = TypeObject::new(form).expect("valid fixture type");
    let (digest, _) = encode_type_object(&object).expect("canonical fixture type");
    if let Some(existing) = snapshot.types.insert(digest, object.clone()) {
        assert_eq!(existing, object);
    }
    digest
}

fn byte_stream_command_snapshot() -> crate::platform::kernel::KernelSnapshot {
    const SEED: &[u8] = b"normalized-byte-stream-command";

    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let package = snapshot.root.package_id;
    let unit = snapshot
        .types
        .iter()
        .find_map(|(digest, object)| matches!(object.form, TypeForm::Unit).then_some(*digest))
        .expect("fixture unit type");
    let bool_type = admit_snapshot_type(&mut snapshot, TypeForm::Bool);
    let i64_type = admit_snapshot_type(&mut snapshot, TypeForm::I64);
    let bytes_type = admit_snapshot_type(&mut snapshot, TypeForm::Bytes);
    let stream_type = admit_snapshot_type(&mut snapshot, TypeForm::Stream { item: bytes_type });
    let read_result = admit_snapshot_type(
        &mut snapshot,
        TypeForm::StructuralRecord {
            fields: vec![
                StructuralTypeField {
                    name: Name::new("chunk").unwrap(),
                    ty: bytes_type,
                },
                StructuralTypeField {
                    name: Name::new("done").unwrap(),
                    ty: bool_type,
                },
            ],
        },
    );
    let port_type = admit_snapshot_type(
        &mut snapshot,
        TypeForm::Function {
            parameters: vec![stream_type],
            result: bytes_type,
        },
    );
    snapshot.types.retain(|digest, object| {
        !matches!(object.form, TypeForm::Function { .. }) || *digest == port_type
    });

    let read_all = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Operation(operation), OwnerRecord::Operation(record))
                if record.name.as_str() == "read" =>
            {
                Some(*operation)
            }
            _ => None,
        })
        .expect("fixture capability operation");
    let interface = match &snapshot.owners[&OwnerKey::Operation(read_all)] {
        OwnerRecord::Operation(record) => record.declaration,
        _ => panic!("fixture operation owner kind"),
    };
    let requirement = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| {
            matches!(record, OwnerRecord::Requirement(_)).then(|| match owner {
                OwnerKey::Requirement(requirement) => *requirement,
                _ => unreachable!(),
            })
        })
        .expect("fixture requirement");
    let caller = declaration_named(&snapshot, "caller").declaration;
    let read = OperationId::migrate(SEED, 0);
    let close = OperationId::migrate(SEED, 1);
    let read_stream = ParameterId::migrate(SEED, 0);
    let close_stream = ParameterId::migrate(SEED, 1);
    let read_all_stream = ParameterId::migrate(SEED, 2);
    let read_all_maximum = ParameterId::migrate(SEED, 3);
    let caller_stream = ParameterId::migrate(SEED, 4);
    let stream_expression = ExpressionId::migrate(SEED, 0);
    let maximum_expression = ExpressionId::migrate(SEED, 1);

    for (parameter, parent, name, ty) in [
        (
            read_stream,
            ParameterParent::Operation(read),
            "stream",
            stream_type,
        ),
        (
            close_stream,
            ParameterParent::Operation(close),
            "stream",
            stream_type,
        ),
        (
            read_all_stream,
            ParameterParent::Operation(read_all),
            "stream",
            stream_type,
        ),
        (
            read_all_maximum,
            ParameterParent::Operation(read_all),
            "maximum_bytes",
            i64_type,
        ),
        (
            caller_stream,
            ParameterParent::Function(caller),
            "body",
            stream_type,
        ),
    ] {
        assert!(
            snapshot
                .owners
                .insert(
                    OwnerKey::Parameter(parameter),
                    OwnerRecord::Parameter(ParameterRecord {
                        header: OwnerHeader::new(
                            OwnerKey::Parameter(parameter),
                            OwnerKind::Parameter,
                        ),
                        parent,
                        name: Name::new(name).unwrap(),
                        ty,
                    }),
                )
                .is_none()
        );
    }
    for (operation, name, parameters, result) in [
        (read, "read", vec![read_stream], read_result),
        (close, "close", vec![close_stream], unit),
    ] {
        assert!(
            snapshot
                .owners
                .insert(
                    OwnerKey::Operation(operation),
                    OwnerRecord::Operation(OperationRecord {
                        header: OwnerHeader::new(
                            OwnerKey::Operation(operation),
                            OwnerKind::Operation,
                        ),
                        declaration: interface,
                        name: Name::new(name).unwrap(),
                        parameters,
                        result,
                        idempotency: Idempotency::Idempotent,
                        external_visibility: ExternalVisibility::None,
                    }),
                )
                .is_none()
        );
    }
    let OwnerRecord::Operation(operation) = snapshot
        .owners
        .get_mut(&OwnerKey::Operation(read_all))
        .expect("read-all operation")
    else {
        panic!("read-all operation owner kind")
    };
    operation.name = Name::new("read-all").unwrap();
    operation.parameters = vec![read_all_stream, read_all_maximum];
    operation.result = bytes_type;

    let OwnerRecord::Declaration(interface_record) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(interface))
        .expect("stream interface")
    else {
        panic!("stream interface owner kind")
    };
    let DeclarationPayload::Interface { operations } = &mut interface_record.payload else {
        panic!("stream interface declaration kind")
    };
    *operations = vec![read, close, read_all];
    operations.sort();

    let OwnerRecord::Requirement(requirement_record) = snapshot
        .owners
        .get_mut(&OwnerKey::Requirement(requirement))
        .expect("stream requirement")
    else {
        panic!("stream requirement owner kind")
    };
    requirement_record.name = Name::new("streams").unwrap();
    requirement_record.operations = vec![
        crate::platform::kernel::OperationReference {
            package,
            operation: read,
        },
        crate::platform::kernel::OperationReference {
            package,
            operation: close,
        },
        crate::platform::kernel::OperationReference {
            package,
            operation: read_all,
        },
    ];
    requirement_record.operations.sort();

    let OwnerRecord::Declaration(caller_record) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(caller))
        .expect("stream caller")
    else {
        panic!("stream caller owner kind")
    };
    let DeclarationPayload::Function(function) = &mut caller_record.payload else {
        panic!("stream caller declaration kind")
    };
    function.parameters = vec![caller_stream];
    function.result = bytes_type;

    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Expression(stream_expression),
                OwnerRecord::Expression(
                    ExpressionRecord::new(
                        stream_expression,
                        ExpressionOperation::Local {
                            value: LocalValueReference::FunctionParameter(caller_stream),
                        },
                    )
                    .expect("stream parameter expression"),
                ),
            )
            .is_none()
    );
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Expression(maximum_expression),
                OwnerRecord::Expression(
                    ExpressionRecord::new(
                        maximum_expression,
                        ExpressionOperation::I64 { value: 64 },
                    )
                    .expect("stream limit expression"),
                ),
            )
            .is_none()
    );
    let OwnerRecord::Expression(capability) = snapshot
        .owners
        .values_mut()
        .find(|record| {
            matches!(
                record,
                OwnerRecord::Expression(ExpressionRecord {
                    operation: ExpressionOperation::CapabilityCall { operation, .. },
                    ..
                }) if operation.operation == read_all
            )
        })
        .expect("stream capability expression")
    else {
        panic!("stream capability expression owner kind")
    };
    let ExpressionOperation::CapabilityCall { arguments, .. } = &mut capability.operation else {
        unreachable!()
    };
    *arguments = vec![stream_expression, maximum_expression];

    for record in snapshot.owners.values_mut() {
        if let OwnerRecord::Port(port) = record {
            port.function_type = port_type;
        }
    }
    snapshot.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    snapshot
}

fn normalized_http_snapshot() -> crate::platform::kernel::KernelSnapshot {
    const SEED: &[u8] = b"normalized-http-runner";

    let mut snapshot = byte_stream_command_snapshot();
    let text_type = admit_snapshot_type(&mut snapshot, TypeForm::Text);
    let bytes_type = admit_snapshot_type(&mut snapshot, TypeForm::Bytes);
    let i64_type = admit_snapshot_type(&mut snapshot, TypeForm::I64);
    let stream_type = admit_snapshot_type(&mut snapshot, TypeForm::Stream { item: bytes_type });
    let header_type = admit_snapshot_type(
        &mut snapshot,
        TypeForm::StructuralRecord {
            fields: vec![
                StructuralTypeField {
                    name: Name::new("name").unwrap(),
                    ty: text_type,
                },
                StructuralTypeField {
                    name: Name::new("value").unwrap(),
                    ty: bytes_type,
                },
            ],
        },
    );
    let header_list = admit_snapshot_type(&mut snapshot, TypeForm::List { item: header_type });
    let text_list = admit_snapshot_type(&mut snapshot, TypeForm::List { item: text_type });
    let query_map = admit_snapshot_type(
        &mut snapshot,
        TypeForm::Map {
            key: text_type,
            value: text_list,
        },
    );
    let request_type = admit_snapshot_type(
        &mut snapshot,
        TypeForm::StructuralRecord {
            fields: vec![
                StructuralTypeField {
                    name: Name::new("body").unwrap(),
                    ty: stream_type,
                },
                StructuralTypeField {
                    name: Name::new("headers").unwrap(),
                    ty: header_list,
                },
                StructuralTypeField {
                    name: Name::new("method").unwrap(),
                    ty: text_type,
                },
                StructuralTypeField {
                    name: Name::new("path").unwrap(),
                    ty: text_type,
                },
                StructuralTypeField {
                    name: Name::new("query").unwrap(),
                    ty: text_type,
                },
                StructuralTypeField {
                    name: Name::new("query_parameters").unwrap(),
                    ty: query_map,
                },
            ],
        },
    );
    let response_type = admit_snapshot_type(
        &mut snapshot,
        TypeForm::StructuralRecord {
            fields: vec![
                StructuralTypeField {
                    name: Name::new("body").unwrap(),
                    ty: bytes_type,
                },
                StructuralTypeField {
                    name: Name::new("headers").unwrap(),
                    ty: header_list,
                },
                StructuralTypeField {
                    name: Name::new("status").unwrap(),
                    ty: i64_type,
                },
            ],
        },
    );
    let port_type = admit_snapshot_type(
        &mut snapshot,
        TypeForm::Function {
            parameters: vec![request_type],
            result: response_type,
        },
    );
    snapshot.types.retain(|digest, object| {
        !matches!(object.form, TypeForm::Function { .. }) || *digest == port_type
    });

    let caller = declaration_named(&snapshot, "caller").declaration;
    let (request_parameter, body_expression) = match snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(caller))
        .expect("HTTP handler declaration")
    {
        OwnerRecord::Declaration(record) => match &mut record.payload {
            DeclarationPayload::Function(function) => {
                function.result = response_type;
                (function.parameters[0], function.body)
            }
            _ => panic!("HTTP handler declaration kind"),
        },
        _ => panic!("HTTP handler owner kind"),
    };
    let OwnerRecord::Parameter(parameter) = snapshot
        .owners
        .get_mut(&OwnerKey::Parameter(request_parameter))
        .expect("HTTP request parameter")
    else {
        panic!("HTTP request parameter owner kind")
    };
    parameter.name = Name::new("request").unwrap();
    parameter.ty = request_type;

    let root_operation = match snapshot
        .owners
        .get(&OwnerKey::Expression(body_expression))
        .expect("HTTP prior body expression")
    {
        OwnerRecord::Expression(record) => record.operation.clone(),
        _ => panic!("HTTP prior body owner kind"),
    };
    let ExpressionOperation::Sequence { items } = root_operation else {
        panic!("HTTP prior body must be the fixture sequence")
    };
    let capability = *items.last().expect("HTTP prior body sequence item");
    let arguments = match snapshot
        .owners
        .get(&OwnerKey::Expression(capability))
        .expect("HTTP stream read expression")
    {
        OwnerRecord::Expression(ExpressionRecord {
            operation: ExpressionOperation::CapabilityCall { arguments, .. },
            ..
        }) => arguments.clone(),
        _ => panic!("HTTP prior body must end by reading the stream"),
    };
    assert_eq!(arguments.len(), 2);
    let body_field = arguments[0];
    let maximum = arguments[1];

    let mut prior_descendants = BTreeSet::new();
    let mut pending = items;
    while let Some(expression) = pending.pop() {
        if !prior_descendants.insert(expression) {
            continue;
        }
        let OwnerRecord::Expression(record) = snapshot
            .owners
            .get(&OwnerKey::Expression(expression))
            .expect("HTTP prior expression closure")
        else {
            panic!("HTTP prior expression owner kind")
        };
        pending.extend(record.children().into_iter().map(|child| child.expression));
    }
    let mut retained = BTreeSet::new();
    let mut pending = vec![capability];
    while let Some(expression) = pending.pop() {
        if !retained.insert(expression) {
            continue;
        }
        let OwnerRecord::Expression(record) = snapshot
            .owners
            .get(&OwnerKey::Expression(expression))
            .expect("HTTP retained expression closure")
        else {
            panic!("HTTP retained expression owner kind")
        };
        pending.extend(record.children().into_iter().map(|child| child.expression));
    }
    for expression in prior_descendants.difference(&retained) {
        assert!(
            snapshot
                .owners
                .remove(&OwnerKey::Expression(*expression))
                .is_some()
        );
    }

    let request_value = ExpressionId::migrate(SEED, 0);
    let headers = ExpressionId::migrate(SEED, 1);
    let status = ExpressionId::migrate(SEED, 2);

    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Expression(request_value),
                OwnerRecord::Expression(
                    ExpressionRecord::new(
                        request_value,
                        ExpressionOperation::Local {
                            value: LocalValueReference::FunctionParameter(request_parameter),
                        },
                    )
                    .expect("HTTP request local expression"),
                ),
            )
            .is_none()
    );
    let OwnerRecord::Expression(field) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(body_field))
        .expect("HTTP body-field expression")
    else {
        panic!("HTTP body-field owner kind")
    };
    field.operation = ExpressionOperation::Field {
        value: request_value,
        selector: FieldSelector::Structural(Name::new("body").unwrap()),
    };
    let OwnerRecord::Expression(capability_record) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(capability))
        .expect("HTTP retained stream read")
    else {
        panic!("HTTP retained stream read owner kind")
    };
    let ExpressionOperation::CapabilityCall { arguments, .. } = &mut capability_record.operation
    else {
        panic!("HTTP retained stream read operation")
    };
    *arguments = vec![body_field, maximum];
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Expression(headers),
                OwnerRecord::Expression(
                    ExpressionRecord::new(
                        headers,
                        ExpressionOperation::List {
                            item_type: header_type,
                            items: Vec::new(),
                        },
                    )
                    .expect("HTTP response headers expression"),
                ),
            )
            .is_none()
    );
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Expression(status),
                OwnerRecord::Expression(
                    ExpressionRecord::new(status, ExpressionOperation::I64 { value: 200 })
                        .expect("HTTP response status expression"),
                ),
            )
            .is_none()
    );
    let OwnerRecord::Expression(body) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(body_expression))
        .expect("HTTP response expression")
    else {
        panic!("HTTP response owner kind")
    };
    body.operation = ExpressionOperation::Record {
        nominal_type: None,
        fields: vec![
            RecordExpressionField {
                selector: FieldSelector::Structural(Name::new("body").unwrap()),
                value: capability,
            },
            RecordExpressionField {
                selector: FieldSelector::Structural(Name::new("headers").unwrap()),
                value: headers,
            },
            RecordExpressionField {
                selector: FieldSelector::Structural(Name::new("status").unwrap()),
                value: status,
            },
        ],
    };

    for record in snapshot.owners.values_mut() {
        match record {
            OwnerRecord::Port(port) => port.function_type = port_type,
            OwnerRecord::Target(target) => {
                target.name = Name::new("serve").unwrap();
                target.runner = RunnerKind::Http;
            }
            _ => {}
        }
    }
    snapshot.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    snapshot
}

#[derive(Clone)]
struct UnitAdapter {
    interface: DeclarationReference,
    operations: BTreeSet<crate::platform::kernel::OperationReference>,
    calls: Arc<AtomicU64>,
}

struct WrongRevisionReader<'a>(&'a RepositoryView);

impl NormalizedReferenceRead for WrongRevisionReader<'_> {
    fn binding(&self) -> Result<NormalizedReferenceBinding, ExecutionError> {
        let mut binding = NormalizedReferenceRead::binding(self.0)?;
        binding.revision = Some(RevisionId::from_digest([0x55; 32]));
        Ok(binding)
    }

    fn owner(&self, owner: OwnerKey) -> Result<NormalizedReferenceOwnerRead, ExecutionError> {
        NormalizedReferenceRead::owner(self.0, owner)
    }
}

impl NormalizedCapabilityAdapter for UnitAdapter {
    fn kind(&self) -> NormalizedAdapterKind {
        NormalizedAdapterKind::Configuration
    }

    fn interface(&self) -> DeclarationReference {
        self.interface
    }

    fn operations(&self) -> &BTreeSet<crate::platform::kernel::OperationReference> {
        &self.operations
    }

    fn call(
        &self,
        _policy: &NormalizedCallPolicy,
        _arguments: Vec<NormalizedValue>,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(NormalizedValue::Unit)
    }

    fn begin_transaction(
        &self,
        _policy: &NormalizedTransactionPolicy,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<Box<dyn NormalizedCapabilityTransaction>, ExecutionError> {
        control.check()?;
        Ok(Box::new(UnitTransaction))
    }
}

struct UnitTransaction;

impl NormalizedCapabilityTransaction for UnitTransaction {
    fn call(
        &mut self,
        _policy: &NormalizedCallPolicy,
        _arguments: Vec<NormalizedValue>,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        Ok(NormalizedValue::Unit)
    }

    fn commit(&mut self, control: &ExecutionControl) -> Result<(), ExecutionError> {
        control.check()
    }

    fn rollback(&mut self) -> Result<(), ExecutionError> {
        Ok(())
    }
}

#[derive(Default)]
struct TransactionStats {
    begins: AtomicU64,
    calls: AtomicU64,
    commits: AtomicU64,
    rollbacks: AtomicU64,
    call_policies: Mutex<Vec<NormalizedCallPolicy>>,
    transaction_policies: Mutex<Vec<NormalizedTransactionPolicy>>,
}

#[derive(Clone)]
struct TrackingAdapter {
    interface: DeclarationReference,
    operations: BTreeSet<crate::platform::kernel::OperationReference>,
    stats: Arc<TransactionStats>,
    fail_transaction_call: bool,
}

impl NormalizedCapabilityAdapter for TrackingAdapter {
    fn kind(&self) -> NormalizedAdapterKind {
        NormalizedAdapterKind::Configuration
    }

    fn interface(&self) -> DeclarationReference {
        self.interface
    }

    fn operations(&self) -> &BTreeSet<crate::platform::kernel::OperationReference> {
        &self.operations
    }

    fn call(
        &self,
        policy: &NormalizedCallPolicy,
        _arguments: Vec<NormalizedValue>,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        self.stats
            .call_policies
            .lock()
            .expect("tracking call policy lock")
            .push(policy.clone());
        Ok(NormalizedValue::Unit)
    }

    fn begin_transaction(
        &self,
        policy: &NormalizedTransactionPolicy,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<Box<dyn NormalizedCapabilityTransaction>, ExecutionError> {
        control.check()?;
        self.stats.begins.fetch_add(1, Ordering::Relaxed);
        self.stats
            .transaction_policies
            .lock()
            .expect("tracking transaction policy lock")
            .push(policy.clone());
        Ok(Box::new(TrackingTransaction {
            stats: Arc::clone(&self.stats),
            fail_call: self.fail_transaction_call,
        }))
    }
}

struct TrackingTransaction {
    stats: Arc<TransactionStats>,
    fail_call: bool,
}

impl NormalizedCapabilityTransaction for TrackingTransaction {
    fn call(
        &mut self,
        policy: &NormalizedCallPolicy,
        _arguments: Vec<NormalizedValue>,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        self.stats.calls.fetch_add(1, Ordering::Relaxed);
        self.stats
            .call_policies
            .lock()
            .expect("tracking transaction call policy lock")
            .push(policy.clone());
        if self.fail_call {
            Err(ExecutionError::new(
                ExecutionFailureClass::PossibleVisibility,
                "normalized_test_possible_visibility",
                "injected capability failure after possible visibility",
            ))
        } else {
            Ok(NormalizedValue::Unit)
        }
    }

    fn commit(&mut self, control: &ExecutionControl) -> Result<(), ExecutionError> {
        control.check()?;
        self.stats.commits.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn rollback(&mut self) -> Result<(), ExecutionError> {
        self.stats.rollbacks.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

fn bind_fixture_capability(
    program: &NormalizedProgram,
    maximum_calls: u64,
) -> (NormalizedCapabilities, Arc<AtomicU64>) {
    let target = program
        .root_target(&Name::new("command").unwrap())
        .expect("fixture target");
    let component = &program.components[target.component.0 as usize];
    let requirement = component.requirements[0];
    let requirement_record = &program.requirements[requirement.0 as usize];
    let calls = Arc::new(AtomicU64::new(0));
    let operations = requirement_record
        .operations
        .iter()
        .map(|operation| program.operations[operation.0 as usize].reference)
        .collect::<BTreeSet<_>>();
    let grant = NormalizedCapabilityGrant {
        requirement: requirement_record.reference,
        descriptor: exact_grant_descriptor(
            requirement_record,
            operations.clone(),
            exact_grant_limits(requirement_record, maximum_calls),
        ),
        adapter: Arc::new(UnitAdapter {
            interface: requirement_record.interface,
            operations,
            calls: Arc::clone(&calls),
        }),
    };
    (
        NormalizedCapabilities::bind(program, target.component, vec![grant])
            .expect("exact fixture grant"),
        calls,
    )
}

fn bind_tracking_capability(
    program: &NormalizedProgram,
    maximum_calls: u64,
    fail_transaction_call: bool,
) -> (NormalizedCapabilities, Arc<TransactionStats>) {
    let target = program
        .root_target(&Name::new("command").unwrap())
        .expect("fixture target");
    let requirement = program.components[target.component.0 as usize].requirements[0];
    let requirement = &program.requirements[requirement.0 as usize];
    let stats = Arc::new(TransactionStats::default());
    let operations = requirement
        .operations
        .iter()
        .map(|operation| program.operations[operation.0 as usize].reference)
        .collect::<BTreeSet<_>>();
    let grant = NormalizedCapabilityGrant {
        requirement: requirement.reference,
        descriptor: exact_grant_descriptor(
            requirement,
            operations.clone(),
            exact_grant_limits(requirement, maximum_calls),
        ),
        adapter: Arc::new(TrackingAdapter {
            interface: requirement.interface,
            operations,
            stats: Arc::clone(&stats),
            fail_transaction_call,
        }),
    };
    (
        NormalizedCapabilities::bind(program, target.component, vec![grant])
            .expect("exact tracked fixture grant"),
        stats,
    )
}

fn exact_grant_descriptor(
    requirement: &super::prepare::NormalizedRequirement,
    operations: BTreeSet<crate::platform::kernel::OperationReference>,
    limits: BTreeMap<Name, NormalizedGrantLimit>,
) -> NormalizedCapabilityGrantDescriptor {
    NormalizedCapabilityGrantDescriptor::for_test(
        requirement.interface,
        NormalizedAdapterKind::Configuration,
        operations,
        limits,
    )
}

fn exact_grant_limits(
    requirement: &super::prepare::NormalizedRequirement,
    maximum_calls: u64,
) -> BTreeMap<Name, NormalizedGrantLimit> {
    let mut limits = requirement
        .limits
        .iter()
        .map(|limit| {
            (
                limit.name.clone(),
                NormalizedGrantLimit {
                    maximum: limit.maximum,
                    unit: limit.unit,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    limits.insert(
        Name::new("maximum_calls").unwrap(),
        NormalizedGrantLimit {
            maximum: maximum_calls,
            unit: ResourceUnit::Calls,
        },
    );
    limits
}

#[test]
fn normalized_grant_metadata_uses_distinct_typed_domains() {
    let bytes = b"same external identity bytes";
    assert_ne!(
        NormalizedGrantAuthorityRevision::of(bytes).bytes(),
        NormalizedGrantDescriptorDigest::of(bytes).bytes(),
    );
    assert_eq!(
        NormalizedSharingDomain::new("service-test")
            .expect("bounded sharing domain")
            .as_name()
            .as_str(),
        "service-test"
    );
    assert!(NormalizedSharingDomain::new("service tenant").is_err());
}

fn transaction_call_snapshot(
    external_visibility: ExternalVisibility,
) -> crate::platform::kernel::KernelSnapshot {
    let mut snapshot = crate::platform::compiler::tests::complete_expression_snapshot();
    let (body, requirement) = snapshot
        .owners
        .values()
        .find_map(|record| match record {
            OwnerRecord::Expression(record) => match record.operation {
                ExpressionOperation::Transaction {
                    requirement, body, ..
                } => Some((body, requirement)),
                _ => None,
            },
            _ => None,
        })
        .expect("coverage transaction");
    let operation = snapshot
        .owners
        .values()
        .find_map(|record| match record {
            OwnerRecord::Expression(record) => match record.operation {
                ExpressionOperation::CapabilityCall { operation, .. } => Some(operation),
                _ => None,
            },
            _ => None,
        })
        .expect("coverage capability operation");
    let OwnerRecord::Operation(operation_record) = snapshot
        .owners
        .get_mut(&OwnerKey::Operation(operation.operation))
        .expect("coverage operation owner")
    else {
        panic!("coverage operation owner kind")
    };
    operation_record.external_visibility = external_visibility;
    let OwnerRecord::Requirement(requirement_record) = snapshot
        .owners
        .get_mut(&OwnerKey::Requirement(requirement.requirement))
        .expect("coverage requirement owner")
    else {
        panic!("coverage requirement owner kind")
    };
    requirement_record.limits = vec![ResourceLimit {
        name: Name::new("maximum_input_bytes").unwrap(),
        maximum: 64,
        unit: ResourceUnit::Bytes,
    }];
    let OwnerRecord::Expression(body) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(body))
        .expect("transaction body expression")
    else {
        panic!("transaction body owner kind")
    };
    body.operation = ExpressionOperation::CapabilityCall {
        requirement,
        operation,
        arguments: Vec::new(),
    };
    snapshot
}

fn transaction_result_snapshot() -> crate::platform::kernel::KernelSnapshot {
    let mut snapshot = crate::platform::compiler::tests::complete_expression_snapshot();
    let transaction = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Expression(expression), OwnerRecord::Expression(record))
                if matches!(record.operation, ExpressionOperation::Transaction { .. }) =>
            {
                Some(*expression)
            }
            _ => None,
        })
        .expect("coverage transaction expression");
    let caller = declaration_named(&snapshot, "caller").declaration;
    let root = match &snapshot.owners[&OwnerKey::Declaration(caller)] {
        OwnerRecord::Declaration(record) => match record.payload {
            crate::platform::kernel::DeclarationPayload::Function(ref function) => function.body,
            _ => panic!("caller declaration kind"),
        },
        _ => panic!("caller owner kind"),
    };
    let OwnerRecord::Expression(root) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(root))
        .expect("caller root expression")
    else {
        panic!("caller root owner kind")
    };
    let ExpressionOperation::Sequence { items } = &mut root.operation else {
        panic!("caller root operation")
    };
    let position = items
        .iter()
        .position(|expression| *expression == transaction)
        .expect("transaction sequence item");
    items.remove(position);
    items.push(transaction);
    snapshot
}

#[test]
fn strict_graph5_artifact_prepares_only_dense_runtime_bindings() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let caller = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record))
                if record.name.as_str() == "caller" =>
            {
                Some(*declaration)
            }
            _ => None,
        })
        .expect("caller declaration");
    let program = prepare_snapshot(&snapshot);

    assert_eq!(program.work.packages, 1);
    assert_eq!(program.work.compiler_units, 11);
    assert_eq!(program.work.runtime_owners, 8);
    assert_eq!(program.work.type_objects, 2);
    assert_eq!(program.work.functions, 5);
    assert_eq!(program.work.record_layouts, 1);
    assert_eq!(program.work.variant_layouts, 1);
    assert_eq!(program.work.requirements, 1);
    assert_eq!(program.work.operations, 1);
    assert_eq!(program.work.components, 1);
    assert_eq!(program.work.ports, 1);
    assert_eq!(program.work.targets, 1);
    assert_eq!(program.work.tests, 1);
    assert_eq!(program.records[0].fields[0].name.as_str(), "value");
    assert_eq!(program.variants[0].cases[0].name.as_str(), "Ready");
    assert_eq!(program.operations[0].name.as_str(), "read");
    assert_eq!(program.requirements[0].name.as_str(), "store");
    assert_eq!(program.ports[0].name.as_str(), "run");
    assert_eq!(
        program
            .root_target(&crate::platform::kernel::Name::new("command").unwrap())
            .expect("root command target")
            .runner,
        crate::platform::package::RunnerKind::Command
    );
    let caller = program
        .function(DeclarationReference {
            package: snapshot.root.package_id,
            declaration: caller,
        })
        .expect("dense caller function");
    let function = &program.functions[caller.0 as usize];
    let NormalizedFunctionBody::Code(code) = &function.body else {
        panic!("caller must have normalized code")
    };
    assert!(
        code.instructions
            .iter()
            .any(|instruction| matches!(instruction, NormalizedInstruction::Call { .. }))
    );
    assert!(code.instructions.iter().any(|instruction| matches!(
        instruction,
        NormalizedInstruction::Record {
            layout: Some(_),
            ..
        }
    )));
    assert!(
        code.instructions
            .iter()
            .any(|instruction| matches!(instruction, NormalizedInstruction::Variant { .. }))
    );
    assert!(
        code.instructions
            .iter()
            .any(|instruction| matches!(instruction, NormalizedInstruction::Perform { .. }))
    );
}

fn admit_runtime_type(program: &mut NormalizedProgram, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).expect("valid runtime boundary type");
    let (digest, _) = encode_type_object(&object).expect("canonical runtime boundary type");
    assert!(program.types.insert(digest, object).is_none());
    digest
}

#[test]
fn normalized_json_codec_uses_exact_runtime_layouts_and_bounds() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let mut program = prepare_snapshot(&snapshot);
    let unit = program
        .types
        .iter()
        .find_map(|(digest, object)| matches!(object.form, TypeForm::Unit).then_some(*digest))
        .expect("unit type");
    assert_eq!(
        decode_typed(&program, b"null", unit, JsonLimits::default()).unwrap(),
        NormalizedValue::Unit
    );
    assert_eq!(
        encode_typed(
            &program,
            &NormalizedValue::Unit,
            unit,
            JsonLimits::default(),
        )
        .unwrap(),
        b"null"
    );

    let record_declaration = program.records[0].declaration;
    let record = admit_runtime_type(
        &mut program,
        TypeForm::Named {
            declaration: record_declaration,
        },
    );
    let record_value = decode_typed(
        &program,
        br#"{"value":null}"#,
        record,
        JsonLimits::default(),
    )
    .expect("decode exact nominal record");
    assert_eq!(
        encode_typed(&program, &record_value, record, JsonLimits::default()).unwrap(),
        br#"{"value":null}"#
    );
    assert_eq!(
        decode_typed(
            &program,
            br#"{"other":null}"#,
            record,
            JsonLimits::default(),
        )
        .expect_err("wrong nominal field must reject")
        .code,
        "normalized_json_type"
    );

    let variant_declaration = program.variants[0].declaration;
    let variant = admit_runtime_type(
        &mut program,
        TypeForm::Named {
            declaration: variant_declaration,
        },
    );
    let variant_value = decode_typed(
        &program,
        br#"{"case":"Ready"}"#,
        variant,
        JsonLimits::default(),
    )
    .expect("decode exact nominal variant");
    assert_eq!(
        encode_typed(&program, &variant_value, variant, JsonLimits::default(),).unwrap(),
        br#"{"case":"Ready"}"#
    );

    let boolean = admit_runtime_type(&mut program, TypeForm::Bool);
    let structural = admit_runtime_type(
        &mut program,
        TypeForm::StructuralRecord {
            fields: vec![StructuralTypeField {
                name: Name::new("flag").unwrap(),
                ty: boolean,
            }],
        },
    );
    let structural_value = decode_typed(
        &program,
        br#"{"flag":true}"#,
        structural,
        JsonLimits::default(),
    )
    .expect("decode structural record");
    assert_eq!(
        encode_typed(
            &program,
            &structural_value,
            structural,
            JsonLimits::default(),
        )
        .unwrap(),
        br#"{"flag":true}"#
    );

    let option = admit_runtime_type(&mut program, TypeForm::Option { item: unit });
    assert_eq!(
        decode_typed(&program, b"null", option, JsonLimits::default())
            .expect_err("unrepresented Option boundary must reject exactly")
            .code,
        "normalized_json_type"
    );
    assert_eq!(
        encode_typed(
            &program,
            &NormalizedValue::Unit,
            unit,
            JsonLimits {
                maximum_bytes: 3,
                ..JsonLimits::default()
            },
        )
        .expect_err("output byte bound must reject")
        .code,
        "normalized_json_output_bytes"
    );

    let registry = StreamRegistry::new(StreamLimits::default()).expect("stream registry");
    let resources = NormalizedResourceScope::new().expect("resource scope");
    let authority = program.requirements[0].reference;
    let handle = resources
        .register_byte_stream(
            authority,
            registry
                .register_memory(b"runtime-only".to_vec())
                .expect("memory stream"),
        )
        .expect("resource handle");
    let resource = NormalizedValue::Resource(handle);
    assert!(!resource.is_durable());
    assert!(NormalizedMapKey::from_value(resource.clone()).is_none());
    assert_eq!(
        encode_typed(&program, &resource, unit, JsonLimits::default())
            .expect_err("live resource must not cross JSON boundary")
            .code,
        "normalized_json_type"
    );
}

#[test]
fn normalized_runners_execute_pure_commands_and_graph_owned_tests_differentially() {
    let effectful_snapshot = crate::platform::kernel::tests::witness_snapshot();
    let effectful_program = prepare_snapshot(&effectful_snapshot);
    let control = ExecutionControl::uncancelled();
    let policy = NormalizedRunPolicy::default();
    let json_limits = JsonLimits::default();
    let command_policy = NormalizedCommandPolicy {
        execution: policy,
        json: json_limits,
    };
    assert_eq!(
        run_pure_command(
            &effectful_snapshot,
            &effectful_program,
            &Name::new("command").unwrap(),
            b"[]",
            command_policy,
            &control,
        )
        .expect_err("effectful command must not run twice against live grants")
        .code,
        "normalized_runner_grants_required"
    );

    let snapshot = pure_command_snapshot();
    let program = prepare_snapshot(&snapshot);
    let receipt = run_pure_command(
        &snapshot,
        &program,
        &Name::new("pure").unwrap(),
        b"[]",
        command_policy,
        &control,
    )
    .expect("pure command runs through both execution tiers");
    assert_eq!(receipt.target.as_str(), "pure");
    assert_eq!(receipt.revision, None);
    assert_eq!(receipt.result_json, b"null");
    assert_eq!(receipt.differential, "equal");
    assert!(receipt.production.instructions > 0);
    assert!(receipt.reference.expressions > 0);

    assert_eq!(
        run_pure_command(
            &snapshot,
            &program,
            &Name::new("pure").unwrap(),
            b"[null]",
            command_policy,
            &control,
        )
        .expect_err("command argument arity is exact")
        .code,
        "normalized_runner_argument_count"
    );

    let tests = run_graph_tests(&snapshot, &program, None, policy, &control)
        .expect("graph-owned tests agree in both execution tiers");
    assert_eq!(tests.revision, None);
    assert_eq!(tests.passed, 1);
    assert_eq!(tests.failed, 0);
    assert!(tests.production_instructions > 0);
    assert!(tests.reference_expressions > 0);
    assert_eq!(tests.differential, "equal");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn normalized_resident_reuses_the_bounded_execution_kernel() {
    let snapshot = pure_command_snapshot();
    let (_temporary, _repository, program) = prepare_repository(&snapshot);
    let program = Arc::new(program);
    let target = Name::new("pure").unwrap();
    let deployment = NormalizedPreparedDeployment::prepare(
        &program,
        target,
        Vec::new(),
        NormalizedDeploymentResourcePolicy::default(),
        &SecretCatalog::from_environment(&[]).expect("empty exact secret catalog"),
    )
    .expect("pure normalized deployment");
    let resident = NormalizedResidentDeployment::prepare(
        Arc::clone(&program),
        deployment,
        ResidentLimits::default(),
        NormalizedRunPolicy::default(),
    )
    .expect("normalized resident deployment");

    assert_eq!(resident.target().runner, RunnerKind::Command);
    assert_eq!(
        resident.deployment().observation().revision,
        program.root_revision
    );
    let receipt = resident.invoke(Vec::new()).await.expect("resident invoke");
    assert_eq!(receipt.value, NormalizedValue::Unit);
    assert!(receipt.execution.instructions > 0);
    assert_eq!(receipt.task_id, 1);
    assert_eq!(resident.observe().completed, 1);
    assert_eq!(resident.limits(), &ResidentLimits::default());

    let shutdown = resident.shutdown().await;
    assert!(shutdown.drained_before_cancellation);
    assert_eq!(shutdown.remaining_tasks, 0);
    assert_eq!(
        resident
            .invoke(Vec::new())
            .await
            .expect_err("shutdown stops normalized admission")
            .code,
        "resident_shutting_down"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn normalized_worker_uses_shared_structured_topology() {
    let snapshot = normalized_worker_snapshot();
    let (_temporary, _repository, program) = prepare_repository(&snapshot);
    let program = Arc::new(program);
    let deployment = NormalizedPreparedDeployment::prepare(
        &program,
        Name::new("work").unwrap(),
        Vec::new(),
        NormalizedDeploymentResourcePolicy::default(),
        &SecretCatalog::from_environment(&[]).expect("empty exact secret catalog"),
    )
    .expect("pure normalized worker deployment");
    let resident = NormalizedResidentDeployment::prepare(
        Arc::clone(&program),
        deployment,
        ResidentLimits::default(),
        NormalizedRunPolicy::default(),
    )
    .expect("normalized worker resident");
    let application = NormalizedWorkerApplication::new(
        resident,
        WorkerLimits {
            maximum_workers: 1,
            idle_wait_milliseconds: 1,
            ..WorkerLimits::default()
        },
    )
    .expect("normalized worker application");
    let observer = application.resident().clone();
    let shutdown = async move {
        loop {
            if observer.observe().completed >= 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
    };

    let receipt =
        tokio::time::timeout(std::time::Duration::from_secs(2), application.run(shutdown))
            .await
            .expect("bounded normalized worker run")
            .expect("normalized worker topology");
    assert!(receipt.iterations >= 1);
    assert_eq!(receipt.productive_iterations, 0);
    assert_eq!(receipt.idle_iterations, receipt.iterations);
    assert!(receipt.shutdown.admission_stopped);
    assert_eq!(receipt.shutdown.remaining_tasks, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn normalized_http_dispatch_uses_exact_body_resources_and_resident_admission() {
    let snapshot = normalized_http_snapshot();
    let (_temporary, _repository, program) = prepare_repository(&snapshot);
    let program = Arc::new(program);
    let target_name = Name::new("serve").unwrap();
    let target = program
        .root_target(&target_name)
        .expect("normalized HTTP target");
    let requirement_index = program.components[target.component.0 as usize].requirements[0];
    let requirement = &program.requirements[requirement_index.0 as usize];
    let grant = NormalizedDeploymentGrant {
        requirement: requirement.reference,
        sharing_domain: NormalizedSharingDomain::new("http-test").unwrap(),
        authority_revision: NormalizedGrantAuthorityRevision::of(b"http stream revision one"),
        limits: exact_grant_limits(requirement, 4),
        adapter: NormalizedAdapterDescriptor::ByteStream,
    };
    let deployment = NormalizedPreparedDeployment::prepare(
        &program,
        target_name,
        vec![grant],
        NormalizedDeploymentResourcePolicy {
            streams: StreamLimits {
                maximum_chunk_bytes: 4,
                maximum_buffered_chunks: 2,
                maximum_total_bytes: 64,
                maximum_live_streams: 4,
            },
        },
        &SecretCatalog::from_environment(&[]).expect("empty exact secret catalog"),
    )
    .expect("normalized HTTP deployment");
    let resident = NormalizedResidentDeployment::prepare(
        Arc::clone(&program),
        deployment,
        ResidentLimits::default(),
        NormalizedRunPolicy::default(),
    )
    .expect("normalized HTTP resident");
    let application = NormalizedHttpApplication::new(
        resident,
        HttpLimits {
            maximum_request_body_bytes: 64,
            maximum_response_body_bytes: 64,
            ..HttpLimits::default()
        },
    )
    .expect("normalized HTTP application");

    let (response, observation) = application
        .dispatch(HttpRequest {
            method: "POST".to_owned(),
            path: "/echo".to_owned(),
            query: "tag=one&tag=two".to_owned(),
            headers: vec![HttpHeader {
                name: "content-type".to_owned(),
                value: b"application/octet-stream".to_vec(),
            }],
            body: b"payload".to_vec(),
        })
        .await
        .expect("normalized HTTP dispatch");
    assert_eq!(response.status, 200);
    assert!(response.headers.is_empty());
    assert_eq!(response.body, b"payload");
    assert_eq!(observation.task_id, 1);
    assert!(observation.instructions > 0);
    assert_eq!(application.resident().deployment().live_streams(), 0);

    assert_eq!(
        application
            .dispatch(HttpRequest {
                method: "GET".to_owned(),
                path: "/invalid".to_owned(),
                query: "broken=%zz".to_owned(),
                headers: Vec::new(),
                body: Vec::new(),
            })
            .await
            .expect_err("malformed query rejects before resident admission")
            .code,
        "http_query_decode"
    );
    assert_eq!(application.resident().observe().admitted, 1);

    let live_response = application
        .clone()
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/live?tag=transport")
                .header("content-type", "application/octet-stream")
                .body(Body::from("live-body"))
                .expect("live normalized HTTP request"),
        )
        .await
        .expect("live normalized HTTP response");
    assert_eq!(live_response.status(), axum::http::StatusCode::OK);
    assert_eq!(
        to_bytes(live_response.into_body(), 64)
            .await
            .expect("bounded live normalized HTTP body"),
        "live-body"
    );
    assert_eq!(application.resident().observe().admitted, 2);
    assert_eq!(application.resident().deployment().live_streams(), 0);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("normalized HTTP listener");
    let server = application
        .clone()
        .serve(listener, async {})
        .await
        .expect("normalized HTTP graceful shutdown");
    assert!(server.accepted_at_transport);
    assert!(server.shutdown.admission_stopped);
    assert!(server.shutdown.drained_before_cancellation);
    assert_eq!(server.shutdown.remaining_tasks, 0);
}

#[test]
fn normalized_deployment_resolves_and_runs_one_exact_effect_adapter() {
    let snapshot = wall_clock_command_snapshot();
    let (_temporary, repository, program) = prepare_repository(&snapshot);
    let target_name = Name::new("command").unwrap();
    let target = program
        .root_target(&target_name)
        .expect("wall-clock command target");
    let requirement_index = program.components[target.component.0 as usize].requirements[0];
    let requirement = &program.requirements[requirement_index.0 as usize];
    let grant = NormalizedDeploymentGrant {
        requirement: requirement.reference,
        sharing_domain: NormalizedSharingDomain::new("command-test").unwrap(),
        authority_revision: NormalizedGrantAuthorityRevision::of(b"deployment revision one"),
        limits: exact_grant_limits(requirement, 1),
        adapter: NormalizedAdapterDescriptor::WallClock,
    };
    let secrets = SecretCatalog::from_environment(&[]).expect("empty exact secret catalog");
    let deployment = NormalizedPreparedDeployment::prepare(
        &program,
        target_name.clone(),
        vec![grant.clone()],
        NormalizedDeploymentResourcePolicy::default(),
        &secrets,
    )
    .expect("exact normalized deployment");
    let repeated = NormalizedPreparedDeployment::prepare(
        &program,
        target_name.clone(),
        vec![grant.clone()],
        NormalizedDeploymentResourcePolicy::default(),
        &secrets,
    )
    .expect("deterministic normalized deployment descriptor");
    let mut changed_grant = grant.clone();
    changed_grant.sharing_domain = NormalizedSharingDomain::new("other-domain").unwrap();
    let changed = NormalizedPreparedDeployment::prepare(
        &program,
        target_name,
        vec![changed_grant],
        NormalizedDeploymentResourcePolicy::default(),
        &secrets,
    )
    .expect("changed normalized deployment descriptor");
    let mut incompatible_grant = grant;
    incompatible_grant.adapter = NormalizedAdapterDescriptor::Identifier;
    assert_eq!(
        NormalizedPreparedDeployment::prepare(
            &program,
            Name::new("command").unwrap(),
            vec![incompatible_grant],
            NormalizedDeploymentResourcePolicy::default(),
            &secrets,
        )
        .expect_err("adapter must match exact graph operation names and signatures")
        .code,
        "normalized_deployment_adapter_operation"
    );

    assert_eq!(deployment.target().as_str(), "command");
    assert_eq!(deployment.observation(), repeated.observation());
    assert_eq!(
        deployment.observation().artifact_manifest,
        program.artifact().manifest_digest
    );
    let observation = deployment
        .observation()
        .grants
        .get(&requirement.reference)
        .expect("exact observed requirement grant");
    assert_eq!(observation.adapter_kind, NormalizedAdapterKind::WallClock);
    assert_eq!(observation.adapter_kind.as_str(), "wall-clock");
    assert_ne!(
        observation.descriptor_digest,
        changed
            .observation()
            .grants
            .get(&requirement.reference)
            .expect("changed exact requirement grant")
            .descriptor_digest
    );

    let view = repository
        .view_current()
        .expect("wall-clock revision-pinned view");
    assert_eq!(deployment.observation().revision, view.revision());
    assert_eq!(
        deployment.observation().semantic_state,
        program.root_semantic_state
    );
    let receipt = run_effectful_command(
        &view,
        &program,
        deployment.target(),
        b"[]",
        deployment.capabilities(),
        NormalizedCommandPolicy::default(),
        &ExecutionControl::uncancelled(),
    )
    .expect("one exact normalized wall-clock execution");
    let milliseconds = std::str::from_utf8(&receipt.result_json)
        .expect("wall-clock JSON UTF-8")
        .parse::<i64>()
        .expect("wall-clock JSON integer");
    assert!(milliseconds > 0);
    assert_eq!(receipt.production.capability_calls, 1);
    assert_eq!(receipt.verification, "production_only_live_effects");
}

#[test]
fn exact_byte_stream_grant_executes_in_task_scopes_in_both_tiers() {
    let snapshot = byte_stream_command_snapshot();
    let (_temporary, repository, program) = prepare_repository(&snapshot);
    let target_name = Name::new("command").unwrap();
    let target = program
        .root_target(&target_name)
        .expect("byte-stream command target");
    let requirement_index = program.components[target.component.0 as usize].requirements[0];
    let requirement = &program.requirements[requirement_index.0 as usize];
    let grant = NormalizedDeploymentGrant {
        requirement: requirement.reference,
        sharing_domain: NormalizedSharingDomain::new("stream-test").unwrap(),
        authority_revision: NormalizedGrantAuthorityRevision::of(b"stream revision one"),
        limits: exact_grant_limits(requirement, 4),
        adapter: NormalizedAdapterDescriptor::ByteStream,
    };
    let resource_policy = NormalizedDeploymentResourcePolicy {
        streams: StreamLimits {
            maximum_chunk_bytes: 4,
            maximum_buffered_chunks: 2,
            maximum_total_bytes: 64,
            maximum_live_streams: 4,
        },
    };
    let secrets = SecretCatalog::from_environment(&[]).expect("empty exact secret catalog");
    let deployment = NormalizedPreparedDeployment::prepare(
        &program,
        target_name.clone(),
        vec![grant.clone()],
        resource_policy.clone(),
        &secrets,
    )
    .expect("exact byte-stream deployment");
    let mut changed_policy = resource_policy.clone();
    changed_policy.streams.maximum_chunk_bytes = 8;
    let changed = NormalizedPreparedDeployment::prepare(
        &program,
        target_name.clone(),
        vec![grant],
        changed_policy,
        &secrets,
    )
    .expect("changed byte-stream deployment");
    let digest = deployment
        .observation()
        .grants
        .get(&requirement.reference)
        .expect("stream grant observation")
        .descriptor_digest;
    assert_eq!(
        deployment.observation().resources,
        resource_policy,
        "deployment observation binds task resource policy"
    );
    assert_ne!(
        digest,
        changed
            .observation()
            .grants
            .get(&requirement.reference)
            .expect("changed stream grant observation")
            .descriptor_digest,
        "stream registry limits participate in exact grant identity"
    );

    let control = ExecutionControl::uncancelled();
    let production_scope = NormalizedResourceScope::new().expect("production resource scope");
    let production_stream = deployment
        .register_memory_stream(
            requirement.reference,
            &production_scope,
            b"streamed bytes".to_vec(),
        )
        .expect("production memory stream");
    let production = NormalizedVm::new(&program, NormalizedRunPolicy::default())
        .invoke_root_target_scoped(
            &target_name,
            vec![production_stream],
            Some(deployment.capabilities()),
            &production_scope,
            &control,
        )
        .expect("dense stream read-all");
    assert_eq!(
        production.0,
        NormalizedValue::bytes(b"streamed bytes".to_vec())
    );
    assert_eq!(production_scope.live_resources(), 0);
    assert_eq!(deployment.live_streams(), 0);

    let view = repository.view_current().expect("stream repository view");
    let reference_scope = NormalizedResourceScope::new().expect("reference resource scope");
    let reference_stream = deployment
        .register_memory_stream(
            requirement.reference,
            &reference_scope,
            b"streamed bytes".to_vec(),
        )
        .expect("reference memory stream");
    let reference = NormalizedReferenceInterpreter::from_reader(
        &view,
        &program,
        NormalizedRunPolicy::default(),
    )
    .invoke_root_target_scoped(
        &target_name,
        vec![reference_stream],
        Some(deployment.capabilities()),
        &reference_scope,
        &control,
    )
    .expect("reference stream read-all");
    assert_eq!(reference.0, production.0);
    assert_eq!(reference_scope.live_resources(), 0);
    assert_eq!(deployment.live_streams(), 0);

    let owning_scope = NormalizedResourceScope::new().expect("owning resource scope");
    let foreign_scope = NormalizedResourceScope::new().expect("foreign resource scope");
    let foreign_stream = deployment
        .register_memory_stream(requirement.reference, &owning_scope, b"foreign".to_vec())
        .expect("foreign-scope stream");
    let error = NormalizedVm::new(&program, NormalizedRunPolicy::default())
        .invoke_root_target_scoped(
            &target_name,
            vec![foreign_stream],
            Some(deployment.capabilities()),
            &foreign_scope,
            &control,
        )
        .expect_err("stream handle must reject another task scope");
    assert_eq!(error.code, "normalized_resource_foreign_scope");
    drop(owning_scope);
    assert_eq!(deployment.live_streams(), 0);
}

#[test]
fn normalized_reference_runner_uses_revision_pinned_owner_reads() {
    let snapshot = pure_command_snapshot();
    let (_temporary, repository, program) = prepare_repository(&snapshot);
    let view = repository
        .view_current()
        .expect("exact normalized runner repository view");
    let control = ExecutionControl::uncancelled();
    let receipt = run_pure_command(
        &view,
        &program,
        &Name::new("pure").unwrap(),
        b"[]",
        NormalizedCommandPolicy::default(),
        &control,
    )
    .expect("revision-pinned pure command");

    assert_eq!(receipt.revision, Some(view.revision()));
    assert_eq!(receipt.result_json, b"null");
    assert_eq!(receipt.reference.canonical_owner_reads, 7);
    assert!(receipt.reference.canonical_owner_reads < snapshot.owners.len() as u64);
    assert!(receipt.reference.canonical_map_pages_read > 0);
    assert_eq!(
        receipt.reference.canonical_objects_read,
        receipt
            .reference
            .canonical_map_pages_read
            .saturating_add(receipt.reference.canonical_owner_reads),
        "each exact semantic read touches one bounded map path and one owner object"
    );
    assert!(receipt.reference.canonical_bytes_read > 0);

    let tests = run_graph_tests(
        &view,
        &program,
        None,
        NormalizedRunPolicy::default(),
        &control,
    )
    .expect("revision-pinned graph-owned tests");
    assert_eq!(tests.revision, Some(view.revision()));
    assert_eq!(tests.passed, 1);

    let (capabilities, calls) = bind_fixture_capability(&program, 1);
    let effectful = run_effectful_command(
        &view,
        &program,
        &Name::new("command").unwrap(),
        b"[]",
        &capabilities,
        NormalizedCommandPolicy::default(),
        &control,
    )
    .expect("effectful command runs once through production");
    assert_eq!(effectful.revision, Some(view.revision()));
    assert_eq!(effectful.result_json, b"null");
    assert_eq!(effectful.production.capability_calls, 1);
    assert_eq!(effectful.verification, "production_only_live_effects");
    assert_eq!(calls.load(Ordering::Relaxed), 1);

    let pure_target = program
        .root_target(&Name::new("pure").unwrap())
        .expect("pure fixture target");
    let pure_capabilities =
        NormalizedCapabilities::bind(&program, pure_target.component, Vec::new())
            .expect("empty grants bind the pure component");
    assert_eq!(
        run_effectful_command(
            &view,
            &program,
            &Name::new("command").unwrap(),
            b"[]",
            &pure_capabilities,
            NormalizedCommandPolicy::default(),
            &control,
        )
        .expect_err("grants for another component must reject")
        .code,
        "normalized_runner_grant_component"
    );
    assert_eq!(
        run_effectful_command(
            &view,
            &program,
            &Name::new("pure").unwrap(),
            b"[]",
            &pure_capabilities,
            NormalizedCommandPolicy::default(),
            &control,
        )
        .expect_err("pure targets require differential execution")
        .code,
        "normalized_runner_pure_target"
    );

    let (stale_capabilities, stale_calls) = bind_fixture_capability(&program, 1);
    assert_eq!(
        run_effectful_command(
            &WrongRevisionReader(&view),
            &program,
            &Name::new("command").unwrap(),
            b"[]",
            &stale_capabilities,
            NormalizedCommandPolicy::default(),
            &control,
        )
        .expect_err("foreign revision binding must reject before production effects")
        .code,
        "normalized_reference_authority_binding"
    );
    assert_eq!(stale_calls.load(Ordering::Relaxed), 0);
}

#[test]
fn dense_vm_executes_pure_external_test_and_capability_paths() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let program = prepare_snapshot(&snapshot);
    let vm = NormalizedVm::new(&program, NormalizedRunPolicy::default());
    let control = ExecutionControl::uncancelled();

    let (pure, pure_observation) = vm
        .invoke(
            declaration_named(&snapshot, "with_binding"),
            Vec::new(),
            None,
            &control,
        )
        .expect("pure dense invocation");
    assert_eq!(pure, NormalizedValue::Unit);
    assert!(pure_observation.instructions > 0);
    assert_eq!(pure_observation.capability_calls, 0);

    let (external, external_observation) = vm
        .invoke(
            declaration_named(&snapshot, "identity_external"),
            Vec::new(),
            None,
            &control,
        )
        .expect("direct external dense invocation");
    assert_eq!(external, NormalizedValue::Unit);
    assert_eq!(external_observation.external_calls, 1);

    let (actual, expected) = vm
        .invoke_test(declaration_named(&snapshot, "caller_test"), None, &control)
        .expect("graph-owned dense test");
    assert_eq!(actual.0, NormalizedValue::Unit);
    assert_eq!(actual.0, expected.0);

    let (capabilities, calls) = bind_fixture_capability(&program, 1);
    let (result, observation) = vm
        .invoke_root_target(
            &Name::new("command").unwrap(),
            Vec::new(),
            Some(&capabilities),
            &control,
        )
        .expect("effectful dense target");
    assert_eq!(result, NormalizedValue::Unit);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert_eq!(observation.capability_calls, 1);
    assert_eq!(observation.calls, 2);
    assert!(observation.collection_items >= 2);
    assert_eq!(observation.production_tier, "graph5_dense_bytecode_1");
}

#[test]
fn dense_vm_enforces_exact_grants_cancellation_and_separate_budgets() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let program = prepare_snapshot(&snapshot);
    let target_name = Name::new("command").unwrap();
    let control = ExecutionControl::uncancelled();
    let vm = NormalizedVm::new(&program, NormalizedRunPolicy::default());

    let missing = vm
        .invoke_root_target(&target_name, Vec::new(), None, &control)
        .expect_err("task invocation requires an exact deployment grant");
    assert_eq!(missing.class, ExecutionFailureClass::Capability);
    assert_eq!(missing.code, "normalized_capability_unbound");

    let target = program.root_target(&target_name).expect("fixture target");
    let requirement = program.components[target.component.0 as usize].requirements[0];
    let requirement = &program.requirements[requirement.0 as usize];
    let invalid = NormalizedCapabilities::bind(
        &program,
        target.component,
        vec![NormalizedCapabilityGrant {
            requirement: requirement.reference,
            descriptor: exact_grant_descriptor(
                requirement,
                Default::default(),
                exact_grant_limits(requirement, 1),
            ),
            adapter: Arc::new(UnitAdapter {
                interface: requirement.interface,
                operations: Default::default(),
                calls: Arc::new(AtomicU64::new(0)),
            }),
        }],
    )
    .expect_err("partial operation grants are forbidden");
    assert_eq!(invalid.class, ExecutionFailureClass::Capability);
    assert_eq!(invalid.code, "normalized_grant_operation_set");

    let required_operations = requirement
        .operations
        .iter()
        .map(|operation| program.operations[operation.0 as usize].reference)
        .collect::<BTreeSet<_>>();
    let invalid = NormalizedCapabilities::bind(
        &program,
        target.component,
        vec![NormalizedCapabilityGrant {
            requirement: requirement.reference,
            descriptor: exact_grant_descriptor(
                requirement,
                required_operations.clone(),
                exact_grant_limits(requirement, 1),
            ),
            adapter: Arc::new(UnitAdapter {
                interface: requirement.interface,
                operations: Default::default(),
                calls: Arc::new(AtomicU64::new(0)),
            }),
        }],
    )
    .expect_err("adapter operation bindings must equal the exact grant");
    assert_eq!(invalid.class, ExecutionFailureClass::Capability);
    assert_eq!(invalid.code, "normalized_grant_adapter_operations");

    let mut wrong_kind = exact_grant_descriptor(
        requirement,
        required_operations.clone(),
        exact_grant_limits(requirement, 1),
    );
    wrong_kind.adapter_kind = NormalizedAdapterKind::PasswordHash;
    let invalid = NormalizedCapabilities::bind(
        &program,
        target.component,
        vec![NormalizedCapabilityGrant {
            requirement: requirement.reference,
            descriptor: wrong_kind,
            adapter: Arc::new(UnitAdapter {
                interface: requirement.interface,
                operations: required_operations.clone(),
                calls: Arc::new(AtomicU64::new(0)),
            }),
        }],
    )
    .expect_err("adapter kind must equal the exact descriptor");
    assert_eq!(invalid.code, "normalized_grant_adapter_kind");

    for (limits, expected) in [
        (BTreeMap::new(), "normalized_grant_call_limit"),
        (
            BTreeMap::from([(
                Name::new("maximum_calls").unwrap(),
                NormalizedGrantLimit {
                    maximum: 0,
                    unit: ResourceUnit::Calls,
                },
            )]),
            "normalized_grant_limit_zero",
        ),
        (
            BTreeMap::from([(
                Name::new("maximum_calls").unwrap(),
                NormalizedGrantLimit {
                    maximum: 1,
                    unit: ResourceUnit::Bytes,
                },
            )]),
            "normalized_grant_call_unit",
        ),
    ] {
        let invalid = NormalizedCapabilities::bind(
            &program,
            target.component,
            vec![NormalizedCapabilityGrant {
                requirement: requirement.reference,
                descriptor: exact_grant_descriptor(
                    requirement,
                    required_operations.clone(),
                    limits,
                ),
                adapter: Arc::new(UnitAdapter {
                    interface: requirement.interface,
                    operations: required_operations.clone(),
                    calls: Arc::new(AtomicU64::new(0)),
                }),
            }],
        )
        .expect_err("invalid exact grant limit");
        assert_eq!(invalid.code, expected);
    }

    let cancelled = ExecutionControl::uncancelled();
    cancelled.cancel();
    let cancelled_error = vm
        .invoke(
            declaration_named(&snapshot, "identity_external"),
            Vec::new(),
            None,
            &cancelled,
        )
        .expect_err("cancellation reaches normalized external execution");
    assert_eq!(cancelled_error.class, ExecutionFailureClass::Cancelled);

    let step_limited = NormalizedVm::new(
        &program,
        NormalizedRunPolicy {
            instruction_steps: 1,
            ..NormalizedRunPolicy::default()
        },
    )
    .invoke(
        declaration_named(&snapshot, "with_binding"),
        Vec::new(),
        None,
        &control,
    )
    .expect_err("instruction budget is independent");
    assert_eq!(step_limited.code, "normalized_instruction_steps");

    let allocation_limited = NormalizedVm::new(
        &program,
        NormalizedRunPolicy {
            maximum_allocated_bytes: 1,
            ..NormalizedRunPolicy::default()
        },
    )
    .invoke(
        declaration_named(&snapshot, "with_binding"),
        Vec::new(),
        None,
        &control,
    )
    .expect_err("allocation budget is independent");
    assert_eq!(allocation_limited.code, "normalized_allocation");

    let (capabilities, _) = bind_fixture_capability(&program, 1);
    let call_depth_limited = NormalizedVm::new(
        &program,
        NormalizedRunPolicy {
            maximum_call_depth: 1,
            ..NormalizedRunPolicy::default()
        },
    )
    .invoke_root_target(&target_name, Vec::new(), Some(&capabilities), &control)
    .expect_err("call-depth budget is independent");
    assert_eq!(call_depth_limited.code, "normalized_call_depth");
}

#[test]
fn canonical_reference_and_dense_vm_agree_on_fixture_execution() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let program = prepare_snapshot(&snapshot);
    let policy = NormalizedRunPolicy::default();
    let vm = NormalizedVm::new(&program, policy);
    let reference = NormalizedReferenceInterpreter::new(&snapshot, &program, policy);
    let control = ExecutionControl::uncancelled();

    let pure = declaration_named(&snapshot, "with_binding");
    let vm_pure = vm
        .invoke(pure, Vec::new(), None, &control)
        .expect("dense pure execution");
    let reference_pure = reference
        .invoke(pure, Vec::new(), None, &control)
        .expect("canonical pure execution");
    assert_eq!(vm_pure.0, reference_pure.0);
    assert_eq!(
        reference_pure.1.production_tier,
        "graph5_reference_records_1"
    );

    let test = declaration_named(&snapshot, "caller_test");
    let vm_test = vm
        .invoke_test(test, None, &control)
        .expect("dense test execution");
    let reference_test = reference
        .invoke_test(test, None, &control)
        .expect("canonical test execution");
    assert_eq!(vm_test.0.0, reference_test.0.0);
    assert_eq!(vm_test.1.0, reference_test.1.0);

    let (capabilities, calls) = bind_fixture_capability(&program, 1);
    let target = Name::new("command").unwrap();
    let vm_task = vm
        .invoke_root_target(&target, Vec::new(), Some(&capabilities), &control)
        .expect("dense task execution");
    let reference_task = reference
        .invoke_root_target(&target, Vec::new(), Some(&capabilities), &control)
        .expect("canonical task execution");
    assert_eq!(vm_task.0, reference_task.0);
    assert_eq!(vm_task.1.capability_calls, 1);
    assert_eq!(reference_task.1.capability_calls, 1);
    assert_eq!(calls.load(Ordering::Relaxed), 2);
}

#[test]
fn canonical_reference_executes_exact_linked_dependency_bodies_with_shared_budgets() {
    let (_temporary, _source, target, program, source, caller) = linked_pure_program();
    let view = target.view_current().expect("linked target view");
    let policy = NormalizedRunPolicy::default();
    let control = ExecutionControl::uncancelled();

    let production = NormalizedVm::new(&program, policy)
        .invoke(caller, Vec::new(), None, &control)
        .expect("dense linked dependency call");
    let reference = NormalizedReferenceInterpreter::from_reader(&view, &program, policy)
        .invoke(caller, Vec::new(), None, &control)
        .expect("canonical linked dependency call");
    assert_eq!(production.0, NormalizedValue::Unit);
    assert_eq!(production.0, reference.0);
    assert_eq!(reference.1.calls, 2);
    assert_eq!(reference.1.maximum_call_depth, 2);
    assert!(reference.1.canonical_objects_read > 0);
    assert!(reference.1.canonical_map_pages_read > 0);

    let direct_dependency = NormalizedReferenceInterpreter::from_reader(&view, &program, policy)
        .invoke(source, Vec::new(), None, &control)
        .expect("direct exact linked dependency body");
    assert_eq!(direct_dependency.0, NormalizedValue::Unit);

    let absent_package = DeclarationReference {
        package: crate::platform::kernel::PackageId::migrate(
            b"normalized-reference-absent-package",
            0,
        ),
        declaration: source.declaration,
    };
    let missing = NormalizedReferenceInterpreter::from_reader(&view, &program, policy)
        .invoke(absent_package, Vec::new(), None, &control)
        .expect_err("unlinked package authority must reject");
    assert_eq!(missing.code, "normalized_reference_dependency_package");

    let bounded = NormalizedRunPolicy {
        maximum_call_depth: 1,
        ..policy
    };
    let production_error = NormalizedVm::new(&program, bounded)
        .invoke(caller, Vec::new(), None, &control)
        .expect_err("dense linked call depth must be shared");
    let reference_error = NormalizedReferenceInterpreter::from_reader(&view, &program, bounded)
        .invoke(caller, Vec::new(), None, &control)
        .expect_err("reference linked call depth must be shared");
    assert_eq!(production_error.code, "normalized_call_depth");
    assert_eq!(reference_error.code, "normalized_reference_call_depth");
    assert_eq!(production_error.class, ExecutionFailureClass::Resource);
    assert_eq!(reference_error.class, ExecutionFailureClass::Resource);
}

#[test]
fn every_graph5_expression_form_executes_equally_in_both_tiers() {
    let snapshot = transaction_result_snapshot();
    let program = prepare_snapshot(&snapshot);
    let policy = NormalizedRunPolicy::default();
    let vm = NormalizedVm::new(&program, policy);
    let reference = NormalizedReferenceInterpreter::new(&snapshot, &program, policy);
    let control = ExecutionControl::uncancelled();
    let caller = declaration_named(&snapshot, "caller");
    let (capabilities, calls) = bind_fixture_capability(&program, 2);

    let vm_result = vm
        .invoke(caller, Vec::new(), Some(&capabilities), &control)
        .expect("all-form dense execution");
    let reference_result = reference
        .invoke(caller, Vec::new(), Some(&capabilities), &control)
        .expect("all-form canonical execution");

    assert_eq!(vm_result.0, NormalizedValue::Unit);
    assert_eq!(vm_result.0, reference_result.0);
    assert_eq!(vm_result.1.capability_calls, 2);
    assert_eq!(reference_result.1.capability_calls, 2);
    assert!(vm_result.1.collection_items >= 4);
    assert!(reference_result.1.collection_items >= 4);
    assert_eq!(calls.load(Ordering::Relaxed), 2);
}

#[test]
fn both_graph5_execution_tiers_commit_and_rollback_exact_transactions() {
    let snapshot = crate::platform::compiler::tests::complete_expression_snapshot();
    let program = prepare_snapshot(&snapshot);
    let policy = NormalizedRunPolicy::default();
    let control = ExecutionControl::uncancelled();
    let caller = declaration_named(&snapshot, "caller");
    let (capabilities, stats) = bind_tracking_capability(&program, 2, false);

    NormalizedVm::new(&program, policy)
        .invoke(caller, Vec::new(), Some(&capabilities), &control)
        .expect("dense transaction commit");
    NormalizedReferenceInterpreter::new(&snapshot, &program, policy)
        .invoke(caller, Vec::new(), Some(&capabilities), &control)
        .expect("reference transaction commit");
    assert_eq!(stats.begins.load(Ordering::Relaxed), 2);
    assert_eq!(stats.commits.load(Ordering::Relaxed), 2);
    assert_eq!(stats.rollbacks.load(Ordering::Relaxed), 0);

    let failing_snapshot = transaction_call_snapshot(ExternalVisibility::Possible);
    let failing_program = prepare_snapshot(&failing_snapshot);
    let failing_caller = declaration_named(&failing_snapshot, "caller");
    let failing_target = failing_program
        .root_target(&Name::new("command").unwrap())
        .expect("failing fixture target");
    let failing_requirement =
        failing_program.components[failing_target.component.0 as usize].requirements[0];
    let failing_requirement = &failing_program.requirements[failing_requirement.0 as usize];
    let failing_operations = failing_requirement
        .operations
        .iter()
        .map(|operation| failing_program.operations[operation.0 as usize].reference)
        .collect::<BTreeSet<_>>();
    for (input_limit, expected) in [
        (None, "normalized_grant_limit_missing"),
        (
            Some((65, ResourceUnit::Bytes)),
            "normalized_grant_limit_excess",
        ),
        (
            Some((64, ResourceUnit::Calls)),
            "normalized_grant_limit_unit",
        ),
    ] {
        let mut limits = BTreeMap::from([(
            Name::new("maximum_calls").unwrap(),
            NormalizedGrantLimit {
                maximum: 3,
                unit: ResourceUnit::Calls,
            },
        )]);
        if let Some((input_limit, unit)) = input_limit {
            limits.insert(
                Name::new("maximum_input_bytes").unwrap(),
                NormalizedGrantLimit {
                    maximum: input_limit,
                    unit,
                },
            );
        }
        let invalid = NormalizedCapabilities::bind(
            &failing_program,
            failing_target.component,
            vec![NormalizedCapabilityGrant {
                requirement: failing_requirement.reference,
                descriptor: exact_grant_descriptor(
                    failing_requirement,
                    failing_operations.clone(),
                    limits,
                ),
                adapter: Arc::new(UnitAdapter {
                    interface: failing_requirement.interface,
                    operations: failing_operations.clone(),
                    calls: Arc::new(AtomicU64::new(0)),
                }),
            }],
        )
        .expect_err("missing or excessive graph-declared grant limit");
        assert_eq!(invalid.code, expected);
    }
    let (failing_capabilities, failing_stats) = bind_tracking_capability(&failing_program, 3, true);
    let dense_error = NormalizedVm::new(&failing_program, policy)
        .invoke(
            failing_caller,
            Vec::new(),
            Some(&failing_capabilities),
            &control,
        )
        .expect_err("dense transaction failure");
    let reference_error =
        NormalizedReferenceInterpreter::new(&failing_snapshot, &failing_program, policy)
            .invoke(
                failing_caller,
                Vec::new(),
                Some(&failing_capabilities),
                &control,
            )
            .expect_err("reference transaction failure");
    assert_eq!(dense_error.class, ExecutionFailureClass::PossibleVisibility);
    assert_eq!(dense_error, reference_error);
    assert!(dense_error.possibly_visible);
    assert_eq!(failing_stats.begins.load(Ordering::Relaxed), 2);
    assert_eq!(failing_stats.calls.load(Ordering::Relaxed), 2);
    assert_eq!(failing_stats.commits.load(Ordering::Relaxed), 0);
    assert_eq!(failing_stats.rollbacks.load(Ordering::Relaxed), 2);

    let requirement = &failing_program.requirements[0];
    let operation = &failing_program.operations[requirement.operations[0].0 as usize];
    let expected_limits: Arc<[ResourceLimit]> = vec![ResourceLimit {
        name: Name::new("maximum_input_bytes").unwrap(),
        maximum: 64,
        unit: ResourceUnit::Bytes,
    }]
    .into();
    let expected_grant = Arc::new(exact_grant_descriptor(
        requirement,
        failing_operations,
        BTreeMap::from([
            (
                Name::new("maximum_calls").unwrap(),
                NormalizedGrantLimit {
                    maximum: 3,
                    unit: ResourceUnit::Calls,
                },
            ),
            (
                Name::new("maximum_input_bytes").unwrap(),
                NormalizedGrantLimit {
                    maximum: 64,
                    unit: ResourceUnit::Bytes,
                },
            ),
        ]),
    ));
    let expected_call_policy = NormalizedCallPolicy {
        requirement: requirement.reference,
        requirement_name: Name::new("store").unwrap(),
        operation: operation.reference,
        operation_name: Name::new("read").unwrap(),
        idempotency: Idempotency::Idempotent,
        external_visibility: ExternalVisibility::Possible,
        requirement_limits: Arc::clone(&expected_limits),
        grant: Arc::clone(&expected_grant),
    };
    assert_eq!(
        *failing_stats
            .call_policies
            .lock()
            .expect("recorded call policies"),
        vec![expected_call_policy; 2]
    );
    let expected_transaction_policy = NormalizedTransactionPolicy {
        requirement: requirement.reference,
        requirement_name: Name::new("store").unwrap(),
        requirement_limits: expected_limits,
        grant: expected_grant,
    };
    assert_eq!(
        *failing_stats
            .transaction_policies
            .lock()
            .expect("recorded transaction policies"),
        vec![expected_transaction_policy; 2]
    );

    let forbidden_snapshot = transaction_call_snapshot(ExternalVisibility::None);
    let forbidden_program = prepare_snapshot(&forbidden_snapshot);
    let forbidden_caller = declaration_named(&forbidden_snapshot, "caller");
    let (forbidden_capabilities, forbidden_stats) =
        bind_tracking_capability(&forbidden_program, 3, true);
    let forbidden_error = NormalizedVm::new(&forbidden_program, policy)
        .invoke(
            forbidden_caller,
            Vec::new(),
            Some(&forbidden_capabilities),
            &control,
        )
        .expect_err("forbidden possible visibility must become an adapter-contract failure");
    assert_eq!(forbidden_error.class, ExecutionFailureClass::Infrastructure);
    assert_eq!(
        forbidden_error.code,
        "normalized_capability_visibility_contract"
    );
    assert!(!forbidden_error.possibly_visible);
    assert_eq!(forbidden_stats.rollbacks.load(Ordering::Relaxed), 1);
}

#[test]
fn dense_vm_reports_stack_collection_and_capability_budget_dimensions() {
    let snapshot = crate::platform::compiler::tests::complete_expression_snapshot();
    let program = prepare_snapshot(&snapshot);
    let caller = declaration_named(&snapshot, "caller");
    let control = ExecutionControl::uncancelled();
    let (capabilities, _) = bind_fixture_capability(&program, 2);

    let stack = NormalizedVm::new(
        &program,
        NormalizedRunPolicy {
            maximum_value_stack: 1,
            ..NormalizedRunPolicy::default()
        },
    )
    .invoke(caller, Vec::new(), Some(&capabilities), &control)
    .expect_err("value-stack budget is independent");
    assert_eq!(stack.code, "normalized_value_stack");

    let collection = NormalizedVm::new(
        &program,
        NormalizedRunPolicy {
            maximum_collection_items: 1,
            ..NormalizedRunPolicy::default()
        },
    )
    .invoke(caller, Vec::new(), Some(&capabilities), &control)
    .expect_err("collection-item budget is independent");
    assert_eq!(collection.code, "normalized_collection_items");

    let capability = NormalizedVm::new(
        &program,
        NormalizedRunPolicy {
            maximum_capability_calls: 1,
            ..NormalizedRunPolicy::default()
        },
    )
    .invoke(caller, Vec::new(), Some(&capabilities), &control)
    .expect_err("capability-call budget is independent");
    assert_eq!(capability.code, "normalized_capability_calls");
}

#[test]
fn declaration_rename_and_move_do_not_change_dense_runtime_dispatch() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let before = prepare_snapshot(&snapshot);
    let mut moved = snapshot.clone();
    let destination = moved
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Module(module), OwnerRecord::Module(record))
                if record.name.as_str() == "second" =>
            {
                Some(*module)
            }
            _ => None,
        })
        .expect("destination module");
    let callee = declaration_named(&moved, "callee").declaration;
    let OwnerRecord::Declaration(callee_record) = moved
        .owners
        .get_mut(&OwnerKey::Declaration(callee))
        .expect("callee declaration")
    else {
        panic!("callee owner kind")
    };
    callee_record.name = Name::new("renamed_callee").unwrap();
    callee_record.module = destination;
    let after = prepare_snapshot(&moved);

    assert_eq!(before.functions, after.functions);
    assert_eq!(before.records, after.records);
    assert_eq!(before.variants, after.variants);
    assert_eq!(before.requirements, after.requirements);
    assert_eq!(before.operations, after.operations);
    assert_eq!(before.components, after.components);
    assert_eq!(before.ports, after.ports);

    let control = ExecutionControl::uncancelled();
    let target = Name::new("command").unwrap();
    let (before_capabilities, _) = bind_fixture_capability(&before, 1);
    let (after_capabilities, _) = bind_fixture_capability(&after, 1);
    let before_result = NormalizedVm::new(&before, NormalizedRunPolicy::default())
        .invoke_root_target(&target, Vec::new(), Some(&before_capabilities), &control)
        .expect("base dense target");
    let after_result = NormalizedVm::new(&after, NormalizedRunPolicy::default())
        .invoke_root_target(&target, Vec::new(), Some(&after_capabilities), &control)
        .expect("renamed and moved dense target");
    assert_eq!(before_result, after_result);
}
