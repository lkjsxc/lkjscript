//! Maintained generation transition observations; the accepted writer remains GraphRepository.
use super::*;

pub(crate) fn neutral_effect_fields(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(fields) => {
            fields.remove("contract_version");
            fields.remove("graph_contract_version");
            for key in ["effect_parameters", "effect_arguments"] {
                if fields
                    .get(key)
                    .is_some_and(|value| value.as_array().is_some_and(Vec::is_empty))
                {
                    fields.remove(key);
                }
            }
            for value in fields.values_mut() {
                neutral_effect_fields(value);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                neutral_effect_fields(value);
            }
        }
        _ => {}
    }
}

fn meaning_hash(mut value: serde_json::Value) -> String {
    neutral_effect_fields(&mut value);
    blake3::hash(&serde_json::to_vec(&value).unwrap())
        .to_hex()
        .to_string()
}

#[test]
#[ignore = "one-time capture of the explicitly preserved Graph 13 baseline and reviewed Graph 14 transition"]
fn retain_effect_transition_inventory() {
    use crate::platform::kernel::{
        OwnerKey, OwnerRecord, TypeObject, TypeObjectDigest, encode_type_object,
    };
    use serde_json::{Value, json};
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut projects = Vec::new();
    for (name, project) in [
        ("standard", "packages/standard"),
        ("lkjournal", "applications/lkjournal"),
    ] {
        let baseline: Value = serde_json::from_slice(
            &std::fs::read(root.join(format!(
                ".artifacts/effects-202609111843/baseline/{name}.json"
            )))
            .unwrap(),
        )
        .unwrap();
        let snapshot = GraphRepository::open(&root.join(project))
            .unwrap()
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        let old_owners = baseline["owners"].as_array().unwrap();
        let old_keys = old_owners
            .iter()
            .map(|row| serde_json::from_value::<OwnerKey>(row[0].clone()).unwrap())
            .collect::<BTreeSet<_>>();
        let added = snapshot
            .owners
            .keys()
            .filter(|key| !old_keys.contains(key))
            .collect::<Vec<_>>();
        let mut ports = Vec::new();
        let mut retained = Vec::new();
        for row in old_owners {
            let key: OwnerKey = serde_json::from_value(row[0].clone()).unwrap();
            let current = snapshot
                .owners
                .get(&key)
                .expect("every baseline owner must remain");
            let mut observed = serde_json::to_value(current).unwrap();
            if let OwnerRecord::Port(port) = current {
                let old = &row[1]["record"]["function_type"];
                if *old != observed["record"]["function_type"] {
                    ports.push(
                        json!({"owner":key,"previous_type":old,"current_type":port.function_type}),
                    );
                    observed["record"]["function_type"] = old.clone();
                }
            }
            retained.push(json!([key, observed]));
        }
        assert_eq!(
            meaning_hash(json!(retained)),
            meaning_hash(baseline["owners"].clone()),
            "only reviewed port meaning may change: {project}"
        );
        assert_eq!(
            meaning_hash(json!(snapshot.retirements.iter().collect::<Vec<_>>())),
            meaning_hash(baseline["retirements"].clone())
        );
        let types = baseline["types"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| {
                let expected: TypeObjectDigest = serde_json::from_value(row[0].clone()).unwrap();
                let object: TypeObject = serde_json::from_value(row[1].clone()).unwrap();
                let (digest, bytes) = encode_type_object(&object).unwrap();
                assert_eq!(
                    digest, expected,
                    "predecessor canonical type bytes must retain their digest"
                );
                json!({"type":expected,"bytes":crate::platform::semantic_id::encode_hex(&bytes)})
            })
            .collect::<Vec<_>>();
        projects.push(json!({"project":project,"observed_predecessor_root":baseline["root"],"owner_ids":old_keys,"owner_meaning_blake3":meaning_hash(baseline["owners"].clone()),
            "added_owners":added,"port_changes":ports,"type_bytes":types,"retirement_meaning_blake3":meaning_hash(baseline["retirements"].clone())}));
    }
    let destination = root.join("tests/fixtures/graph13-effect-cutover.json");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .unwrap();
    std::io::Write::write_all(&mut file, &serde_json::to_vec_pretty(&json!({"predecessor_checkout_observation":"f5bc5db6485affe92daf4ced2c026027121d5bec","predecessor_graph":13,"successor_graph":14,"projects":projects})).unwrap()).unwrap();
}

#[test]
fn maintained_effect_transition_preserves_every_owner_and_unchanged_type_byte() {
    use crate::platform::kernel::{
        OwnerKey, OwnerRecord, TypeObjectDigest, decode_type_object, encode_type_object,
    };
    let fixture: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/graph13-effect-cutover.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(fixture["predecessor_graph"], 13);
    assert_eq!(
        fixture["successor_graph"],
        crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
    );
    for project in fixture["projects"].as_array().unwrap() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(project["project"].as_str().unwrap());
        let before = std::fs::read(path.join("HEAD")).unwrap();
        let snapshot = GraphRepository::open(&path)
            .unwrap()
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        assert_eq!(
            serde_json::to_value(snapshot.root.package_id).unwrap(),
            project["observed_predecessor_root"]["package_id"]
        );
        assert_eq!(
            serde_json::to_value(snapshot.root.repository_id).unwrap(),
            project["observed_predecessor_root"]["repository_id"]
        );
        let old_keys: BTreeSet<OwnerKey> =
            serde_json::from_value(project["owner_ids"].clone()).unwrap();
        let added: BTreeSet<OwnerKey> =
            serde_json::from_value(project["added_owners"].clone()).unwrap();
        assert_eq!(
            snapshot.owners.keys().copied().collect::<BTreeSet<_>>(),
            old_keys.union(&added).copied().collect()
        );
        let mut retained = Vec::new();
        for key in old_keys {
            let owner = &snapshot.owners[&key];
            let mut value = serde_json::to_value(owner).unwrap();
            if let Some(change) = project["port_changes"]
                .as_array()
                .unwrap()
                .iter()
                .find(|change| change["owner"] == serde_json::to_value(key).unwrap())
            {
                assert!(matches!(owner, OwnerRecord::Port(_)));
                assert_eq!(value["record"]["function_type"], change["current_type"]);
                value["record"]["function_type"] = change["previous_type"].clone();
            }
            retained.push(serde_json::json!([key, value]));
        }
        assert_eq!(
            meaning_hash(serde_json::json!(retained)),
            project["owner_meaning_blake3"]
        );
        assert_eq!(
            meaning_hash(serde_json::json!(
                snapshot.retirements.iter().collect::<Vec<_>>()
            )),
            project["retirement_meaning_blake3"]
        );
        for entry in project["type_bytes"].as_array().unwrap() {
            let digest: TypeObjectDigest = serde_json::from_value(entry["type"].clone()).unwrap();
            let bytes = entry["bytes"]
                .as_str()
                .unwrap()
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect::<Vec<_>>();
            let object = decode_type_object(&bytes, digest).unwrap();
            assert_eq!(encode_type_object(&object).unwrap(), (digest, bytes));
            if let Some(current) = snapshot
                .types
                .get(&digest)
                .or_else(|| snapshot.dependency_types.get(&digest))
            {
                assert_eq!(current, &object);
            } else {
                assert!(
                    project["port_changes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|change| change["previous_type"] == entry["type"]),
                    "unreviewed type retirement"
                );
            }
        }
        assert_eq!(std::fs::read(path.join("HEAD")).unwrap(), before);
    }
}

fn successor_snapshot(name: &str) -> crate::platform::kernel::KernelSnapshot {
    use crate::platform::kernel::*;
    use serde_json::Value;
    fn evolve(value: &mut Value) {
        match value {
            Value::Object(fields) => {
                for key in ["contract_version", "graph_contract_version"] {
                    if fields.get(key) == Some(&Value::from(13)) {
                        fields.insert(key.into(), Value::from(14));
                    }
                }
                let kind = fields
                    .get("kind")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_owned();
                if kind == "task" || (kind == "function" && fields.contains_key("body")) {
                    fields.insert("effect_parameters".into(), Value::Array(Vec::new()));
                }
                if matches!(kind.as_str(), "call" | "function_value") {
                    fields.insert("effect_arguments".into(), Value::Array(Vec::new()));
                }
                for value in fields.values_mut() {
                    evolve(value);
                }
            }
            Value::Array(values) => {
                for value in values {
                    evolve(value);
                }
            }
            _ => {}
        }
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        ".artifacts/effects-202609111843/baseline/{name}.json"
    ));
    let mut value: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    evolve(&mut value);
    let mut snapshot = KernelSnapshot {
        root: serde_json::from_value(value["root"].clone()).unwrap(),
        owners: serde_json::from_value::<Vec<_>>(value["owners"].clone())
            .unwrap()
            .into_iter()
            .collect(),
        types: serde_json::from_value::<Vec<_>>(value["types"].clone())
            .unwrap()
            .into_iter()
            .collect(),
        dependency_types: BTreeMap::new(),
        dependency_interfaces: BTreeMap::new(),
        dependencies: BTreeMap::new(),
        retirements: serde_json::from_value::<Vec<_>>(value["retirements"].clone())
            .unwrap()
            .into_iter()
            .collect(),
        blobs: serde_json::from_value::<Vec<_>>(value["blobs"].clone())
            .unwrap()
            .into_iter()
            .collect(),
    };
    let mut replacements = Vec::new();
    for (key, record) in &snapshot.owners {
        let OwnerRecord::Port(port) = record else {
            continue;
        };
        let target = match port.implementation {
            PortImplementation::Function(target) => target,
            PortImplementation::Expression(expression) => match snapshot
                .owners
                .get(&OwnerKey::Expression(expression))
            {
                Some(OwnerRecord::Expression(record)) => match record.operation {
                    ExpressionOperation::FunctionValue { function, .. } => function,
                    _ => panic!("maintained port expression requires an inspected named target"),
                },
                _ => panic!("maintained port expression is absent"),
            },
        };
        let Some(OwnerRecord::Declaration(declaration)) = snapshot
            .owners
            .get(&OwnerKey::Declaration(target.declaration))
        else {
            panic!("maintained port target missing");
        };
        let DeclarationPayload::Function(function) = &declaration.payload else {
            panic!("maintained port target not a function");
        };
        if let FunctionEffect::Task { .. } = function.effect {
            let previous = snapshot.types.get(&port.function_type).unwrap();
            let TypeForm::Function { parameters, result } = &previous.form else {
                panic!("predecessor port is not pure representation");
            };
            let object = TypeObject::new(TypeForm::TaskFunction {
                parameters: parameters.clone(),
                result: *result,
                effect: function.effect.row(),
            })
            .unwrap();
            let (digest, _) = encode_type_object(&object).unwrap();
            replacements.push((*key, digest, object));
        }
    }
    for (key, digest, object) in replacements {
        snapshot.types.insert(digest, object);
        let Some(OwnerRecord::Port(port)) = snapshot.owners.get_mut(&key) else {
            unreachable!()
        };
        port.function_type = digest;
    }
    let mut reachable = BTreeSet::new();
    let mut pending = snapshot
        .owners
        .values()
        .flat_map(OwnerRecord::type_roots)
        .collect::<Vec<_>>();
    while let Some(ty) = pending.pop() {
        if reachable.insert(ty)
            && let Some(object) = snapshot.types.get(&ty)
        {
            pending.extend(object.child_types());
        }
    }
    snapshot.types.retain(|ty, _| reachable.contains(ty));
    snapshot
}

#[test]
#[ignore = "one-time explicit maintained transition; creates only the selected campaign staging directory"]
fn stage_effect_successor() {
    use crate::platform::kernel::*;
    use crate::platform::publication::InitialPackageTransport;
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".artifacts/effects-202609111843/cutover");
    std::fs::create_dir_all(&root).unwrap();
    let standard = if root.join("standard").exists() {
        let repository = GraphRepository::open(&root.join("standard")).unwrap();
        let observed = repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        assert_eq!(observed.owners, successor_snapshot("standard").owners);
        repository
    } else {
        GraphRepository::create(
            &root.join("standard"),
            &successor_snapshot("standard"),
            Some("Graph 14 explicit effects and task-callable port transition".into()),
        )
        .unwrap()
        .repository
    };
    let exported = standard.export_package_transport().unwrap();
    std::fs::write(root.join("standard.lkjp"), &exported.container).unwrap();
    let mut application = successor_snapshot("lkjournal");
    let dependency = DependencyRecord {
        graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
        package: exported.revision.package,
        semantic_revision: exported.revision.revision.revision_id().unwrap(),
        package_revision: exported.revision_digest,
    };
    application.dependency_interfaces.insert(
        dependency.package_revision,
        exported
            .interface_owners
            .iter()
            .map(|(key, owner)| (*key, owner.record.clone()))
            .collect(),
    );
    application.dependency_types = exported
        .interface_types
        .iter()
        .map(|(digest, bytes)| (*digest, decode_type_object(bytes, *digest).unwrap()))
        .collect();
    application
        .dependencies
        .insert(dependency.package, dependency);
    let application = GraphRepository::create_with_package_transports(
        &root.join("lkjournal"),
        &application,
        Some("Graph 14 exact successor standard dependency and explicit task ports".into()),
        &[InitialPackageTransport {
            digest: exported.transport_digest,
            container: exported.container,
        }],
    )
    .unwrap();
    assert_eq!(
        standard
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value
            .root
            .repository_id,
        successor_snapshot("standard").root.repository_id
    );
    assert_eq!(
        application.initial.snapshot.root.repository_id,
        successor_snapshot("lkjournal").root.repository_id
    );
}

#[test]
#[ignore = "one-time accepted maintained writer and compiler into the selected campaign staging directory"]
fn stage_effect_standard() {
    use crate::platform::compiler::{OptimizationPolicy, build_clean, link_artifact};
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join(".artifacts/effects-202609111843/cutover");
    std::fs::create_dir_all(&root).unwrap();
    let standard = GraphRepository::create(
        &root.join("standard"),
        &successor_snapshot("standard"),
        Some("Graph 14 explicit effects and task-callable port transition".into()),
    )
    .unwrap()
    .repository;
    let exported = standard.export_package_transport().unwrap();
    let compilation = build_clean(&standard, OptimizationPolicy::DeterministicBaseline).unwrap();
    let artifact = link_artifact(&standard, compilation.manifest_digest, &[]).unwrap();
    std::fs::write(root.join("standard.lkjp"), &exported.container).unwrap();
    std::fs::write(root.join("standard.lkja"), &artifact.artifact.bytes).unwrap();
    std::fs::write(
        root.join("standard-bindings.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "semantic_revision": exported.revision.revision.revision_id().unwrap().to_string(),
            "package_revision": exported.revision_digest.to_string(),
            "package_transport": exported.transport_digest.to_string(),
        }))
        .unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "one-time predecessor observation into the explicitly selected campaign root"]
fn export_effect_predecessor() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let destination = root.join(".artifacts/effects-202609111843/baseline");
    for (name, path) in [
        ("standard", "packages/standard"),
        ("lkjournal", "applications/lkjournal"),
    ] {
        let repository = GraphRepository::open(&root.join(path)).unwrap();
        let snapshot = repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value;
        let value = serde_json::json!({
            "root": snapshot.root,
            "owners": snapshot.owners.iter().collect::<Vec<_>>(),
            "types": snapshot.types.iter().collect::<Vec<_>>(),
            "dependency_types": snapshot.dependency_types.iter().collect::<Vec<_>>(),
            "dependencies": snapshot.dependencies.iter().collect::<Vec<_>>(),
            "retirements": snapshot.retirements.iter().collect::<Vec<_>>(),
            "blobs": snapshot.blobs.iter().collect::<Vec<_>>(),
        });
        std::fs::write(
            destination.join(format!("{name}.json")),
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .unwrap();
    }
}
