//! Reproducible inventory of the complete authoritative compiler source.
//! This observation is not source admission, lowering or a custody proof.
use super::{compiler, graph};
use crate::platform::kernel;
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

#[path = "selfhost_source_profile_flow_tests.rs"]
mod flow;

type Histogram = BTreeMap<String, usize>;

fn variant(value: &impl Serialize) -> String {
    let value = serde_json::to_value(value).unwrap();
    value
        .as_str()
        .or_else(|| value.get("kind").and_then(Value::as_str))
        .unwrap()
        .to_owned()
}

fn count(histogram: &mut Histogram, label: String) {
    *histogram.entry(label).or_default() += 1;
}

fn declaration(
    source: &kernel::KernelSnapshot,
    reference: kernel::DeclarationReference,
) -> &kernel::DeclarationRecord {
    assert_eq!(reference.package, source.root.package_id);
    match source
        .owners
        .get(&kernel::OwnerKey::Declaration(reference.declaration))
    {
        Some(kernel::OwnerRecord::Declaration(record)) => record,
        _ => panic!("inventory application has no exact local declaration"),
    }
}

fn substitutions(
    source: &kernel::KernelSnapshot,
    reference: kernel::DeclarationReference,
    arguments: &[kernel::TypeObjectDigest],
) -> Vec<Value> {
    let parameters = declaration(source, reference).payload.type_parameters();
    assert_eq!(
        parameters.len(),
        arguments.len(),
        "inventory application must retain exact ordered type arity"
    );
    parameters
        .iter()
        .zip(arguments)
        .map(|(parameter, argument)| {
            assert!(source.types.contains_key(argument));
            json!({
                "parameter": parameter,
                "argument": argument,
            })
        })
        .collect()
}

fn closed_type(source: &kernel::KernelSnapshot, root: kernel::TypeObjectDigest) -> bool {
    let mut pending = vec![root];
    let mut seen = BTreeSet::new();
    while let Some(digest) = pending.pop() {
        if !seen.insert(digest) {
            continue;
        }
        let object = source.types.get(&digest).unwrap();
        if matches!(&object.form, kernel::TypeForm::TypeParameter { .. }) {
            return false;
        }
        pending.extend(object.child_types());
    }
    true
}

fn assert_active_expression(operation: &kernel::ExpressionOperation) {
    use kernel::ExpressionOperation as E;
    match operation {
        E::Unit {}
        | E::Bool { .. }
        | E::I64 { .. }
        | E::Text { .. }
        | E::StaticText { .. }
        | E::If { .. }
        | E::Let { .. }
        | E::Sequence { .. }
        | E::List { .. } => {}
        E::Local { value } => assert!(matches!(
            value,
            kernel::LocalValueReference::FunctionParameter(_)
                | kernel::LocalValueReference::LexicalBinding(_)
        )),
        E::Call {
            requirement_arguments,
            effect_arguments,
            read_scope_arguments,
            ..
        } => {
            assert!(requirement_arguments.is_empty());
            assert!(effect_arguments.is_empty());
            assert!(read_scope_arguments.is_empty());
        }
        E::Record {
            nominal_type,
            type_arguments,
            ..
        } => {
            assert!(nominal_type.is_none());
            assert!(type_arguments.is_empty());
        }
        E::Field { selector, .. } => {
            assert!(matches!(selector, kernel::FieldSelector::Structural(_)));
        }
        _ => panic!(
            "maintained compiler source profile expanded to expression {}",
            variant(operation)
        ),
    }
}

#[test]
fn selfhost_source_profile_inventories_every_owner_and_type() {
    let source = &compiler().0;
    assert!(source.dependencies.is_empty());
    assert!(source.dependency_interfaces.is_empty());
    assert!(source.dependency_types.is_empty());
    assert!(source.representation_dependencies.is_empty());
    assert!(source.blobs.is_empty());
    let carrier = graph::packet(source);
    let semantic_state = kernel::semantic_state_digest(source).unwrap();
    let carrier_sha256 = crate::platform::semantic_id::encode_hex(&Sha256::digest(&carrier));
    let word = |at: usize| {
        usize::try_from(u32::from_le_bytes(carrier[at..at + 4].try_into().unwrap())).unwrap()
    };
    assert_eq!(&carrier[..8], b"LKJSG001");
    assert_eq!(word(8), carrier.len());
    assert_eq!(word(12), source.owners.len());
    assert_eq!(word(16), source.types.len());
    assert_eq!(word(20), source.retirements.len());
    assert_eq!(&carrier[48..80], semantic_state.bytes());

    let mut owner_kinds = Histogram::new();
    let mut expression_kinds = Histogram::new();
    let mut type_forms = Histogram::new();
    let mut parameter_modes = Histogram::new();
    let mut callable_effects = Histogram::new();
    let mut modules = Vec::new();
    let mut declarations = Vec::new();
    let mut callables = Vec::new();
    let mut bindings = Vec::new();
    let mut ports = Vec::new();
    let mut targets = Vec::new();
    let mut generic_applications = Vec::new();
    let mut supplied_parameters = BTreeSet::new();
    let mut declared_parameters = BTreeSet::new();
    let mut supplied_type_parameters = BTreeSet::new();
    let mut declared_type_parameters = BTreeSet::new();
    let mut intrinsic_instances: BTreeMap<
        (kernel::DeclarationReference, Vec<kernel::TypeObjectDigest>),
        Vec<crate::platform::semantic_id::ExpressionId>,
    > = BTreeMap::new();

    for (owner, record) in &source.owners {
        assert_eq!(*owner, record.owner());
        let owner_kind = record.kind();
        count(&mut owner_kinds, variant(&owner_kind));
        match record {
            kernel::OwnerRecord::Module(module) => {
                modules.push(serde_json::to_value(module).unwrap());
            }
            kernel::OwnerRecord::Declaration(record) => {
                declarations.push(serde_json::to_value(record).unwrap());
                let kernel::OwnerKey::Declaration(id) = *owner else {
                    panic!("declaration inventory key");
                };
                let reference = kernel::DeclarationReference {
                    package: source.root.package_id,
                    declaration: id,
                };
                let signature = match &record.payload {
                    kernel::DeclarationPayload::Function(function) => {
                        assert!(matches!(&function.effect, kernel::FunctionEffect::Pure));
                        assert!(matches!(
                            function.result.relation,
                            kernel::ResultRelation::Value
                        ));
                        assert!(function.implementation_parameters.is_empty());
                        assert!(function.requirement_parameters.is_empty());
                        assert!(function.effect_parameters.is_empty());
                        assert!(function.read_scope_parameters.is_empty());
                        assert!(function.read_scope_sources.is_empty());
                        count(&mut callable_effects, variant(&function.effect));
                        Some((
                            function.type_parameters.as_slice(),
                            function.parameters.as_slice(),
                        ))
                    }
                    kernel::DeclarationPayload::External(external) => {
                        count(&mut callable_effects, "external".to_owned());
                        Some((
                            external.type_parameters.as_slice(),
                            external.parameters.as_slice(),
                        ))
                    }
                    kernel::DeclarationPayload::Component { requirements, .. } => {
                        assert!(requirements.is_empty());
                        None
                    }
                    _ => panic!("maintained compiler declaration profile expanded: {owner}"),
                };
                if let Some((type_parameters, parameters)) = signature {
                    let type_parameters: Vec<_> = type_parameters
                        .iter()
                        .map(|id| {
                            let key = kernel::OwnerKey::TypeParameter(*id);
                            assert!(declared_type_parameters.insert(key));
                            match source.owners.get(&key) {
                                Some(kernel::OwnerRecord::TypeParameter(record)) => {
                                    assert_eq!(record.declaration, reference.declaration);
                                    serde_json::to_value(record).unwrap()
                                }
                                _ => panic!("missing exact type-parameter record"),
                            }
                        })
                        .collect();
                    let parameters: Vec<_> = parameters
                        .iter()
                        .map(|id| {
                            let key = kernel::OwnerKey::Parameter(*id);
                            assert!(declared_parameters.insert(key));
                            match source.owners.get(&key) {
                                Some(kernel::OwnerRecord::Parameter(record)) => {
                                    assert_eq!(
                                        record.parent,
                                        kernel::ParameterParent::Function(reference.declaration)
                                    );
                                    serde_json::to_value(record).unwrap()
                                }
                                _ => panic!("missing exact parameter record"),
                            }
                        })
                        .collect();
                    callables.push(json!({
                        "reference": reference,
                        "owner_kind": owner_kind,
                        "declaration": record,
                        "ordered_type_parameters": type_parameters,
                        "ordered_parameters": parameters,
                    }));
                }
            }
            kernel::OwnerRecord::TypeParameter(record) => {
                assert_eq!(record.constraints, kernel::TypeParameterConstraints::None);
                assert!(supplied_type_parameters.insert(*owner));
            }
            kernel::OwnerRecord::Parameter(record) => {
                assert!(matches!(
                    record.parent,
                    kernel::ParameterParent::Function(_)
                ));
                assert!(record.resource_requirement.is_none());
                assert!(supplied_parameters.insert(*owner));
                count(&mut parameter_modes, variant(&record.use_mode));
            }
            kernel::OwnerRecord::Binding(record) => {
                assert_eq!(record.kind, kernel::BindingKind::Let);
                bindings.push(serde_json::to_value(record).unwrap());
            }
            kernel::OwnerRecord::Expression(record) => {
                assert_active_expression(&record.operation);
                count(&mut expression_kinds, variant(&record.operation));
                if let kernel::ExpressionOperation::Call {
                    function,
                    type_arguments,
                    ..
                } = &record.operation
                {
                    let ordered = substitutions(source, *function, type_arguments);
                    let closed = type_arguments
                        .iter()
                        .all(|argument| closed_type(source, *argument));
                    assert!(
                        closed,
                        "maintained compiler application has free type parameters"
                    );
                    if !type_arguments.is_empty() {
                        generic_applications.push(json!({
                            "expression": record.id,
                            "operation": &record.operation,
                            "ordered_substitution": ordered,
                            "closed_type_arguments": closed,
                        }));
                    }
                    if matches!(
                        &declaration(source, *function).payload,
                        kernel::DeclarationPayload::External(_)
                    ) {
                        intrinsic_instances
                            .entry((*function, type_arguments.clone()))
                            .or_default()
                            .push(record.id);
                    }
                }
            }
            kernel::OwnerRecord::Port(record) => {
                ports.push(serde_json::to_value(record).unwrap());
            }
            kernel::OwnerRecord::Target(record) => {
                targets.push(serde_json::to_value(record).unwrap());
            }
            _ => panic!(
                "maintained compiler owner profile expanded to {}",
                variant(&record.kind())
            ),
        }
    }

    let types: Vec<_> = source
        .types
        .iter()
        .map(|(digest, object)| {
            assert!(
                matches!(
                    &object.form,
                    kernel::TypeForm::Unit
                        | kernel::TypeForm::Bool
                        | kernel::TypeForm::I64
                        | kernel::TypeForm::Bytes
                        | kernel::TypeForm::Text
                        | kernel::TypeForm::StaticText
                        | kernel::TypeForm::ByteBuffer
                        | kernel::TypeForm::TypeParameter { .. }
                        | kernel::TypeForm::StructuralRecord { .. }
                        | kernel::TypeForm::List { .. }
                        | kernel::TypeForm::Function { .. }
                ),
                "maintained compiler type profile expanded to {}",
                variant(&object.form)
            );
            count(&mut type_forms, variant(&object.form));
            json!({
                "digest": digest,
                "object": object,
            })
        })
        .collect();
    let intrinsic_instances: Vec<_> = intrinsic_instances
        .into_iter()
        .map(|((reference, arguments), sites)| {
            json!({
                "declaration": reference,
                "ordered_substitution": substitutions(source, reference, &arguments),
                "call_sites": sites,
            })
        })
        .collect();

    assert_eq!(supplied_parameters, declared_parameters);
    assert_eq!(supplied_type_parameters, declared_type_parameters);
    assert_eq!(owner_kinds.values().sum::<usize>(), source.owners.len());
    assert_eq!(type_forms.values().sum::<usize>(), source.types.len());
    assert_eq!(
        expression_kinds.values().sum::<usize>(),
        owner_kinds.get("expression").copied().unwrap_or(0)
    );
    assert_eq!(
        parameter_modes.values().sum::<usize>(),
        supplied_parameters.len()
    );
    assert_eq!(callable_effects.values().sum::<usize>(), callables.len());

    let control_flow = flow::inventory(source);
    assert_eq!(
        control_flow["summary"]["call_expression_owners"]
            .as_u64()
            .unwrap(),
        u64::try_from(expression_kinds.get("call").copied().unwrap_or(0)).unwrap()
    );
    let summary = json!({
        "control_flow": &control_flow["summary"],
        "carrier_bytes": carrier.len(),
        "carrier_sha256": carrier_sha256,
        "semantic_state": semantic_state.to_string(),
        "owners": source.owners.len(),
        "types": source.types.len(),
        "retirements": source.retirements.len(),
        "dependency_packages": source.dependencies.len(),
        "dependency_interfaces": source.dependency_interfaces.len(),
        "dependency_types": source.dependency_types.len(),
        "representation_dependencies": source.representation_dependencies.len(),
        "blobs": source.blobs.len(),
        "modules": modules.len(),
        "declarations": declarations.len(),
        "callables": callables.len(),
        "ports": ports.len(),
        "targets": targets.len(),
        "generic_application_sites": generic_applications.len(),
        "intrinsic_instances": intrinsic_instances.len(),
        "owner_kinds": owner_kinds,
        "expression_kinds": expression_kinds,
        "type_forms": type_forms,
        "parameter_modes": parameter_modes,
        "callable_effects": callable_effects,
    });
    let report = json!({
        "schema": "lkjscript.selfhost.canonical-source-profile.v1",
        "scope": "Complete authoritative compiler snapshot and exact LKJSG001 carrier",
        "summary": &summary,
        "control_flow": &control_flow,
        "repository": source.root.repository_id,
        "package": source.root.package_id,
        "package_name": source.root.package_name,
        "modules": modules,
        "declarations": declarations,
        "callables": callables,
        "types": types,
        "bindings": bindings,
        "ports": ports,
        "targets": targets,
        "generic_applications": generic_applications,
        "intrinsic_signature_encoding":
            "Exact declaration templates plus ordered type substitutions; parameter use remains separate",
        "intrinsic_instances": intrinsic_instances,
        "analysis_not_performed": [
            "inferred local types",
            "branch consumption and cleanup obligations",
            "borrow lifetime analysis",
            "native allocation and resident memory measurement",
        ],
    });
    let mut encoded = serde_json::to_vec_pretty(&report).unwrap();
    encoded.push(b'\n');
    let report_sha256 = crate::platform::semantic_id::encode_hex(&Sha256::digest(&encoded));

    if let Some(path) = std::env::var_os("LKJSCRIPT_SELFHOST_PROFILE_OUTPUT") {
        let path = std::path::PathBuf::from(path);
        assert!(
            !path.as_os_str().is_empty(),
            "inventory output path is empty"
        );
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .unwrap_or_else(|error| panic!("create new inventory {}: {error}", path.display()));
        output.write_all(&encoded).unwrap();
        output.sync_all().unwrap();
    }

    println!(
        "selfhost canonical source profile report_bytes={} report_sha256={} {}",
        encoded.len(),
        report_sha256,
        serde_json::to_string(&summary).unwrap()
    );
}
