//! Generation-neutral recipe parity oracle independent of recipe intent construction.

use super::*;
use crate::platform::kernel::{
    DeclarationPayload, ExpressionOperation, OwnerKey, OwnerRecord, ParameterParent,
    PortImplementation, RelationEndpoint, extract_relations,
};
use crate::platform::publication::GraphRepository;
use crate::platform::semantic_id::{BindingId, ExpressionId};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

const PROJECTION_DOMAIN: &str = "lkjscript.recipe.generation-neutral.v1";

#[derive(Debug)]
struct RecipeProjection {
    digest: String,
    bytes: usize,
    owners: usize,
    types: usize,
    expressions: usize,
    relations: usize,
}

#[test]
fn recipes_match_captured_generation_neutral_predecessor_projections() {
    let temporary = tempfile::TempDir::new().expect("temporary recipe projection root");
    let cases = [
        (
            ProjectTemplate::Minimal,
            "recipe_projection_1959117ddc6915b382b7f0ae8bd7a51ea160cbee0780c9633d6fb681d43e4654",
            0,
            0,
            0,
            0,
        ),
        (
            ProjectTemplate::Command,
            "recipe_projection_b46f48f1b1a1966cb55d480bd1ab76f18f48c7ca1b3a79948ba459496f57970c",
            10,
            2,
            4,
            17,
        ),
        (
            ProjectTemplate::Http,
            "recipe_projection_ec764297d4d69b5704b67f817fcf3e1f019309285f1bf5180b0f69fa2c28501e",
            20,
            12,
            10,
            35,
        ),
        (
            ProjectTemplate::NostrRelayInfo,
            "recipe_projection_59c724a352718e8e13a636ce651997f493954bbcf8781d5acbba82bdb2b90182",
            86,
            12,
            69,
            130,
        ),
    ];
    for (template, expected_digest, owners, types, expressions, relations) in cases {
        let path = temporary.path().join(template.name());
        if template == ProjectTemplate::NostrRelayInfo {
            create_project_with_relay(&path, template.name(), "ws://127.0.0.1:7447/nostr")
                .expect("create relay-information recipe");
        } else {
            create_project(&path, template.name(), template).expect("create recipe");
        }
        let projection = recipe_projection(&path);
        assert_eq!(projection.owners, owners, "{} owners", template.name());
        assert_eq!(projection.types, types, "{} types", template.name());
        assert_eq!(
            projection.expressions,
            expressions,
            "{} expressions",
            template.name()
        );
        assert_eq!(
            projection.relations,
            relations,
            "{} relations",
            template.name()
        );
        assert_eq!(
            projection.digest,
            expected_digest,
            "{} projection ({} bytes)",
            template.name(),
            projection.bytes
        );
    }
}

fn recipe_projection(path: &Path) -> RecipeProjection {
    let repository = GraphRepository::open(path).expect("open recipe projection repository");
    let read = repository
        .view_current()
        .expect("recipe projection view")
        .reconstruct_full_oracle()
        .expect("recipe projection full oracle");
    validate_full(&read.value).expect("recipe projection full validation");
    let snapshot = read.value;
    let mut identities = BTreeMap::new();
    identities.insert(
        snapshot.root.package_id.to_string(),
        "package:<root>".to_owned(),
    );

    for (owner, record) in &snapshot.owners {
        if let (OwnerKey::Module(module), OwnerRecord::Module(record)) = (owner, record) {
            identities.insert(module.to_string(), format!("module:{}", record.name));
        }
    }
    for (owner, record) in &snapshot.owners {
        if let (OwnerKey::Declaration(declaration), OwnerRecord::Declaration(record)) =
            (owner, record)
        {
            let module = identity(&identities, record.module);
            identities.insert(
                declaration.to_string(),
                format!(
                    "declaration:{module}/{}:{}",
                    record.header.kind.name(),
                    record.name
                ),
            );
        }
    }
    for (owner, record) in &snapshot.owners {
        let label = match (owner, record) {
            (OwnerKey::TypeParameter(id), OwnerRecord::TypeParameter(record)) => Some((
                id.to_string(),
                format!(
                    "type-parameter:{}/{}",
                    identity(&identities, record.declaration),
                    record.name
                ),
            )),
            (OwnerKey::Field(id), OwnerRecord::Field(record)) => Some((
                id.to_string(),
                format!(
                    "field:{}/{}",
                    identity(&identities, record.declaration),
                    record.name
                ),
            )),
            (OwnerKey::Case(id), OwnerRecord::Case(record)) => Some((
                id.to_string(),
                format!(
                    "case:{}/{}",
                    identity(&identities, record.declaration),
                    record.name
                ),
            )),
            (OwnerKey::Operation(id), OwnerRecord::Operation(record)) => Some((
                id.to_string(),
                format!(
                    "operation:{}/{}",
                    identity(&identities, record.declaration),
                    record.name
                ),
            )),
            (OwnerKey::Requirement(id), OwnerRecord::Requirement(record)) => Some((
                id.to_string(),
                format!(
                    "requirement:{}/{}",
                    identity(&identities, record.declaration),
                    record.name
                ),
            )),
            (OwnerKey::Port(id), OwnerRecord::Port(record)) => Some((
                id.to_string(),
                format!(
                    "port:{}/{}",
                    identity(&identities, record.declaration),
                    record.name
                ),
            )),
            (OwnerKey::Target(id), OwnerRecord::Target(record)) => {
                Some((id.to_string(), format!("target:{}", record.name)))
            }
            _ => None,
        };
        if let Some((id, label)) = label {
            assert!(identities.insert(id, label).is_none());
        }
    }
    for (owner, record) in &snapshot.owners {
        if let (OwnerKey::Parameter(id), OwnerRecord::Parameter(record)) = (owner, record) {
            let parent = match record.parent {
                ParameterParent::Function(declaration) => identity(&identities, declaration),
                ParameterParent::Operation(operation) => identity(&identities, operation),
            };
            assert!(
                identities
                    .insert(
                        id.to_string(),
                        format!("parameter:{parent}/{}", record.name),
                    )
                    .is_none()
            );
        }
    }

    for record in snapshot.owners.values() {
        match record {
            OwnerRecord::Declaration(record) => {
                let declaration = identity(&identities, record.header.owner);
                match &record.payload {
                    DeclarationPayload::Function(function) => visit_expression(
                        &snapshot,
                        function.body,
                        format!("expression:{declaration}/body"),
                        &mut identities,
                    ),
                    DeclarationPayload::Constant { value, .. } => visit_expression(
                        &snapshot,
                        *value,
                        format!("expression:{declaration}/value"),
                        &mut identities,
                    ),
                    DeclarationPayload::Test {
                        actual, expected, ..
                    } => {
                        visit_expression(
                            &snapshot,
                            *actual,
                            format!("expression:{declaration}/actual"),
                            &mut identities,
                        );
                        visit_expression(
                            &snapshot,
                            *expected,
                            format!("expression:{declaration}/expected"),
                            &mut identities,
                        );
                    }
                    _ => {}
                }
            }
            OwnerRecord::Port(record) => {
                if let PortImplementation::Expression(expression) = record.implementation {
                    visit_expression(
                        &snapshot,
                        expression,
                        format!(
                            "expression:{}/implementation",
                            identity(&identities, record.header.owner)
                        ),
                        &mut identities,
                    );
                }
            }
            _ => {}
        }
    }
    for owner in snapshot.owners.keys() {
        assert!(
            identities.contains_key(&owner.to_string()),
            "recipe projection has no generation-neutral label for {owner}"
        );
    }

    let mut lines = vec![format!("package-name {}", snapshot.root.package_name)];
    for (owner, record) in &snapshot.owners {
        let label = identity(&identities, *owner);
        lines.push(format!(
            "owner {label} {}",
            normalized_json(record, &identities)
        ));
    }
    let mut types = snapshot
        .types
        .values()
        .map(|object| normalized_json(object, &identities))
        .collect::<Vec<_>>();
    types.sort();
    lines.extend(types.into_iter().map(|value| format!("type {value}")));
    for dependency in snapshot.dependencies.values() {
        lines.push(format!(
            "dependency {}",
            normalized_json(dependency, &identities)
        ));
    }
    for (blob, bytes) in &snapshot.blobs {
        lines.push(format!("blob {blob} {bytes}"));
    }
    let relations = extract_relations(
        snapshot.root.package_id,
        &snapshot.owners,
        &snapshot.types,
        &snapshot.dependencies,
    )
    .expect("extract recipe projection relations");
    for relation in &relations {
        lines.push(format!(
            "relation {} {:?} {}",
            endpoint(relation.source, snapshot.root.package_id, &identities),
            relation.kind,
            endpoint(relation.target, snapshot.root.package_id, &identities),
        ));
    }
    lines.push(format!("deployment {}", deployment_projection(path)));
    lines.sort();
    let projection = lines.join("\n") + "\n";
    let mut hasher = blake3::Hasher::new_derive_key(PROJECTION_DOMAIN);
    hasher.update(&(projection.len() as u64).to_be_bytes());
    hasher.update(projection.as_bytes());
    RecipeProjection {
        digest: format!("recipe_projection_{}", hasher.finalize().to_hex()),
        bytes: projection.len(),
        owners: snapshot.owners.len(),
        types: snapshot.types.len(),
        expressions: snapshot
            .owners
            .values()
            .filter(|record| matches!(record, OwnerRecord::Expression(_)))
            .count(),
        relations: relations.len(),
    }
}

fn visit_expression(
    snapshot: &KernelSnapshot,
    expression: ExpressionId,
    label: String,
    identities: &mut BTreeMap<String, String>,
) {
    if let Some(previous) = identities.insert(expression.to_string(), label.clone()) {
        assert_eq!(
            previous, label,
            "expression tree is shared across projection paths"
        );
        return;
    }
    let OwnerRecord::Expression(record) = snapshot
        .owners
        .get(&OwnerKey::Expression(expression))
        .expect("projected expression owner")
    else {
        panic!("projected expression has another owner kind")
    };
    match &record.operation {
        ExpressionOperation::If {
            condition,
            when_true,
            when_false,
        } => {
            visit_expression(
                snapshot,
                *condition,
                format!("{label}/condition"),
                identities,
            );
            visit_expression(snapshot, *when_true, format!("{label}/true"), identities);
            visit_expression(snapshot, *when_false, format!("{label}/false"), identities);
        }
        ExpressionOperation::Let { bindings, body } => {
            for (index, binding) in bindings.iter().enumerate() {
                visit_binding(
                    snapshot,
                    *binding,
                    format!("{label}/binding:{index}"),
                    identities,
                );
            }
            visit_expression(snapshot, *body, format!("{label}/body"), identities);
        }
        ExpressionOperation::Sequence { items } => {
            visit_many(snapshot, items, &label, "item", identities)
        }
        ExpressionOperation::Call { arguments, .. } => {
            visit_many(snapshot, arguments, &label, "argument", identities)
        }
        ExpressionOperation::Invoke { callee, arguments } => {
            visit_expression(snapshot, *callee, format!("{label}/callee"), identities);
            visit_many(snapshot, arguments, &label, "argument", identities);
        }
        ExpressionOperation::Record { fields, .. } => {
            for (index, field) in fields.iter().enumerate() {
                visit_expression(
                    snapshot,
                    field.value,
                    format!("{label}/field:{index}"),
                    identities,
                );
            }
        }
        ExpressionOperation::Variant { payload, .. } => {
            if let Some(payload) = payload {
                visit_expression(snapshot, *payload, format!("{label}/payload"), identities);
            }
        }
        ExpressionOperation::Field { value, .. } => {
            visit_expression(snapshot, *value, format!("{label}/value"), identities)
        }
        ExpressionOperation::List { items, .. } => {
            visit_many(snapshot, items, &label, "item", identities)
        }
        ExpressionOperation::Map { entries, .. } => {
            for (index, entry) in entries.iter().enumerate() {
                visit_expression(
                    snapshot,
                    entry.key,
                    format!("{label}/entry:{index}/key"),
                    identities,
                );
                visit_expression(
                    snapshot,
                    entry.value,
                    format!("{label}/entry:{index}/value"),
                    identities,
                );
            }
        }
        ExpressionOperation::Match { value, arms } => {
            visit_expression(snapshot, *value, format!("{label}/value"), identities);
            for (index, arm) in arms.iter().enumerate() {
                if let Some(binding) = arm.payload_binding {
                    visit_binding(
                        snapshot,
                        binding,
                        format!("{label}/arm:{index}/binding"),
                        identities,
                    );
                }
                visit_expression(
                    snapshot,
                    arm.body,
                    format!("{label}/arm:{index}/body"),
                    identities,
                );
            }
        }
        ExpressionOperation::CapabilityCall { arguments, .. } => {
            visit_many(snapshot, arguments, &label, "argument", identities)
        }
        ExpressionOperation::Transaction { binding, body, .. } => {
            visit_binding(snapshot, *binding, format!("{label}/binding"), identities);
            visit_expression(snapshot, *body, format!("{label}/body"), identities);
        }
        ExpressionOperation::Unit {}
        | ExpressionOperation::Bool { .. }
        | ExpressionOperation::I64 { .. }
        | ExpressionOperation::Text { .. }
        | ExpressionOperation::StaticText { .. }
        | ExpressionOperation::Local { .. }
        | ExpressionOperation::Constant { .. }
        | ExpressionOperation::FunctionValue { .. } => {}
    }
}

fn visit_binding(
    snapshot: &KernelSnapshot,
    binding: BindingId,
    label: String,
    identities: &mut BTreeMap<String, String>,
) {
    assert!(
        identities
            .insert(binding.to_string(), label.clone())
            .is_none()
    );
    let OwnerRecord::Binding(record) = snapshot
        .owners
        .get(&OwnerKey::Binding(binding))
        .expect("projected binding owner")
    else {
        panic!("projected binding has another owner kind")
    };
    if let Some(value) = record.value {
        visit_expression(snapshot, value, format!("{label}/value"), identities);
    }
}

fn visit_many(
    snapshot: &KernelSnapshot,
    expressions: &[ExpressionId],
    parent: &str,
    role: &str,
    identities: &mut BTreeMap<String, String>,
) {
    for (index, expression) in expressions.iter().enumerate() {
        visit_expression(
            snapshot,
            *expression,
            format!("{parent}/{role}:{index}"),
            identities,
        );
    }
}

fn identity(identities: &BTreeMap<String, String>, value: impl ToString) -> String {
    let value = value.to_string();
    identities
        .get(&value)
        .unwrap_or_else(|| panic!("missing generation-neutral identity for {value}"))
        .clone()
}

fn normalized_json(value: &impl Serialize, identities: &BTreeMap<String, String>) -> String {
    let mut value = serde_json::to_value(value).expect("serialize recipe projection record");
    normalize_strings(&mut value, identities);
    normalize_semantic_sets(&mut value);
    serde_json::to_string(&value).expect("encode recipe projection record")
}

fn normalize_semantic_sets(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            for value in fields.values_mut() {
                normalize_semantic_sets(value);
            }
            match fields.get("kind").and_then(Value::as_str) {
                Some("component") => {
                    sort_json_array(fields.get_mut("requirements"));
                    sort_json_array(fields.get_mut("ports"));
                }
                Some("task") => sort_json_array(fields.get_mut("requirements")),
                _ => {}
            }
        }
        Value::Array(values) => {
            for value in values {
                normalize_semantic_sets(value);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn sort_json_array(value: Option<&mut Value>) {
    if let Some(Value::Array(values)) = value {
        values.sort_by_key(|value| serde_json::to_string(value).unwrap_or_default());
    }
}

fn normalize_strings(value: &mut Value, identities: &BTreeMap<String, String>) {
    match value {
        Value::String(text) => {
            if let Some(normalized) = identities.get(text) {
                *text = normalized.clone();
            }
        }
        Value::Array(values) => {
            for value in values {
                normalize_strings(value, identities);
            }
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                normalize_strings(value, identities);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn endpoint(
    endpoint: RelationEndpoint,
    root: PackageId,
    identities: &BTreeMap<String, String>,
) -> String {
    match endpoint {
        RelationEndpoint::Package(package) if package == root => "package:<root>".to_owned(),
        RelationEndpoint::Package(package) => format!("package:{package}"),
        RelationEndpoint::Owner(owner) if owner.package == root => {
            identity(identities, owner.owner)
        }
        RelationEndpoint::Owner(owner) => format!("owner:{}/{}", owner.package, owner.owner),
    }
}

fn deployment_projection(path: &Path) -> String {
    let descriptor = path.join(STARTER_HTTP_DESCRIPTOR_PATH);
    if !descriptor.exists() {
        assert!(!path.join(STARTER_HTTP_ARTIFACT_DIRECTORY).exists());
        return "none".to_owned();
    }
    assert!(path.join(STARTER_HTTP_ARTIFACT_DIRECTORY).is_dir());
    let mut value: Value = serde_json::from_slice(
        &std::fs::read(&descriptor).expect("read recipe projection deployment"),
    )
    .expect("decode recipe projection deployment");
    normalize_authority_revisions(&mut value);
    serde_json::to_string(&value).expect("encode recipe projection deployment")
}

fn normalize_authority_revisions(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            for (name, value) in fields {
                if name == "authority_revision" {
                    *value = Value::String("<fresh-authority-revision>".to_owned());
                } else {
                    normalize_authority_revisions(value);
                }
            }
        }
        Value::Array(values) => {
            for value in values {
                normalize_authority_revisions(value);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}
