//! Current recipes compared with literal public declarations, independently of recipe intent.

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

#[derive(Debug, Eq, PartialEq)]
struct RecipeProjection {
    meaning: Vec<String>,
    owners: usize,
    types: usize,
    expressions: usize,
    relations: usize,
}

#[test]
fn recipes_match_complete_current_literal_meaning() {
    let temporary = tempfile::TempDir::new().expect("temporary recipe projection root");
    let cases = [
        (ProjectTemplate::Minimal, None, None, 0, 0, 0, 0),
        (
            ProjectTemplate::Command,
            Some(include_str!("fixtures/command.lkjc")),
            Some(include_str!("fixtures/command.deployment.json")),
            10,
            2,
            4,
            17,
        ),
        (
            ProjectTemplate::Http,
            Some(include_str!("fixtures/http.lkjc")),
            Some(include_str!("fixtures/http.deployment.json")),
            21,
            12,
            10,
            37,
        ),
        (
            ProjectTemplate::NostrRelayInfo,
            Some(include_str!("fixtures/nostr-relay-info.lkjc")),
            Some(include_str!("fixtures/nostr-relay-info.deployment.json")),
            56,
            12,
            43,
            87,
        ),
    ];
    for (template, literal, descriptor, owners, types, expressions, relations) in cases {
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
        let expected_path = temporary
            .path()
            .join(format!("{}-literal", template.name()));
        create_literal_project(&expected_path, template.name(), literal);
        assert_eq!(
            projection,
            recipe_projection(&expected_path),
            "{} complete current owner/type/body/dependency/relation meaning",
            template.name()
        );
        assert_eq!(
            deployment_projection(&path),
            descriptor.map(|value| {
                serde_json::from_str::<Value>(value).expect("literal current descriptor")
            })
        );
    }
}

// This side uses literal public syntax rather than recipes.rs, RecipeExpressions or
// create_project's lowering. Exact current supplier revisions remain in the projection.
fn create_literal_project(path: &Path, name: &str, literal: Option<&str>) {
    let snapshot = empty_snapshot(
        RepositoryId::generate().expect("literal repository"),
        PackageId::generate().expect("literal package"),
        Name::new(name).expect("literal package name"),
    );
    let supplier = literal.map(|_| {
        crate::platform::builtin_standard::BuiltinStandard::load()
            .expect("current literal supplier")
    });
    let transports = supplier
        .iter()
        .map(|supplier| supplier.transport())
        .collect::<Vec<_>>();
    let created =
        GraphRepository::create_with_package_transports(path, &snapshot, None, &transports)
            .expect("create independent literal repository");
    if let Some(literal) = literal {
        let tail = literal
            .strip_prefix("request base=BASE\n")
            .expect("literal fixture has exactly one current request binding");
        let supplier = supplier.as_ref().expect("literal supplier selection");
        let source = format!(
            "request base={}\nadd.dependency package={} semantic-revision={} package-revision={}\n{tail}",
            created.current.head.revision,
            supplier.package,
            supplier.semantic_revision,
            supplier.package_revision,
        );
        let request = crate::platform::control::decode_compact_change_in_repository(
            "current-recipe-oracle.lkjc",
            source.as_bytes(),
            &created.repository,
        )
        .expect("decode independent literal recipe");
        let prepared = created
            .repository
            .prepare_authored_change(&request.semantic, request.options)
            .expect("independently prepare literal recipe");
        assert!(matches!(
            created
                .repository
                .publish(&prepared.publication)
                .expect("publish independent literal recipe"),
            PublicationOutcome::Accepted { .. }
        ));
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
            (OwnerKey::EffectParameter(id), OwnerRecord::EffectParameter(record)) => Some((
                id.to_string(),
                format!(
                    "effect-parameter:{}/{}",
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
            (OwnerKey::HttpRoute(id), OwnerRecord::HttpRoute(record)) => Some((
                id.to_string(),
                format!(
                    "http-route:{}/{} {}",
                    identity(&identities, record.target),
                    record.method,
                    record.selector.display()
                ),
            )),
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
            "recipe projection has no semantic identity label for {owner}"
        );
    }

    // Every type identity depends on current canonical bytes. Label all types by their
    // complete current structure after child and recipe-local identity substitution.
    let mut pending = snapshot
        .types
        .keys()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    while !pending.is_empty() {
        let before = pending.len();
        for ty in pending.clone() {
            let object = &snapshot.types[&ty];
            let children = object.child_types();
            if children.iter().any(|child| pending.contains(child)) {
                continue;
            }
            let meaning = normalized_json(object, &identities);
            let digest = blake3::hash(meaning.as_bytes()).to_hex();
            identities.insert(ty.to_string(), format!("type:{digest}"));
            pending.remove(&ty);
        }
        assert!(pending.len() < before, "recipe type projection cycle");
    }

    assert!(
        snapshot.retirements.is_empty(),
        "fresh recipe has no retirements"
    );
    let mut lines = vec![
        format!("package-name {}", snapshot.root.package_name),
        format!("graph-contract {}", snapshot.root.graph_contract_version),
    ];
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
    lines.sort();
    RecipeProjection {
        meaning: lines,
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
        ExpressionOperation::Region { .. }
        | ExpressionOperation::PackOwned { .. }
        | ExpressionOperation::UnpackOwned { .. }
        | ExpressionOperation::PackOwnedRecord { .. }
        | ExpressionOperation::UnpackOwnedRecord { .. }
        | ExpressionOperation::BorrowOwnedRecordField { .. }
        | ExpressionOperation::ChooseOwned { .. }
        | ExpressionOperation::MatchOwned { .. }
        | ExpressionOperation::BorrowOwnedField { .. }
        | ExpressionOperation::SequenceEmpty { .. }
        | ExpressionOperation::SequenceLength { .. }
        | ExpressionOperation::SequencePush { .. }
        | ExpressionOperation::SequencePop { .. }
        | ExpressionOperation::SequenceGet { .. }
        | ExpressionOperation::SequenceReplace { .. }
        | ExpressionOperation::BorrowOwnedItem { .. }
        | ExpressionOperation::BorrowCall { .. }
        | ExpressionOperation::MatchBorrowedOwned { .. }
        | ExpressionOperation::CreateReadView { .. }
        | ExpressionOperation::BorrowReadView { .. }
        | ExpressionOperation::ReborrowReadView { .. }
        | ExpressionOperation::MoveReadView { .. }
        | ExpressionOperation::DropReadView { .. }
        | ExpressionOperation::ReadViewDataField { .. }
        | ExpressionOperation::BorrowReadViewField { .. } => {
            panic!("owned products are outside the ordinary web template projection");
        }
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
        ExpressionOperation::Parallel { left, right } => {
            visit_expression(
                snapshot,
                *left,
                format!("{label}/parallel-left"),
                identities,
            );
            visit_expression(
                snapshot,
                *right,
                format!("{label}/parallel-right"),
                identities,
            );
        }
        ExpressionOperation::ImplementationCall { arguments, .. }
        | ExpressionOperation::MethodCall { arguments, .. }
        | ExpressionOperation::Call { arguments, .. } => {
            visit_many(snapshot, arguments, &label, "argument", identities)
        }
        ExpressionOperation::Invoke { callee, arguments }
        | ExpressionOperation::Bind { callee, arguments } => {
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
        ExpressionOperation::Transaction { binding, body, .. }
        | ExpressionOperation::TransactionOutcome { binding, body, .. } => {
            visit_binding(snapshot, *binding, format!("{label}/binding"), identities);
            visit_expression(snapshot, *body, format!("{label}/body"), identities);
        }
        ExpressionOperation::Unit {}
        | ExpressionOperation::Bool { .. }
        | ExpressionOperation::I64 { .. }
        | ExpressionOperation::F64 { .. }
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
        .unwrap_or_else(|| panic!("missing semantic identity for {value}"))
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
                Some("task") => {
                    sort_json_array(fields.get_mut("requirements"));
                    sort_json_array(fields.get_mut("effect_parameters"));
                }
                _ => {}
            }
            if fields.len() == 2
                && fields.contains_key("requirements")
                && fields.contains_key("parameters")
            {
                sort_json_array(fields.get_mut("requirements"));
                sort_json_array(fields.get_mut("parameters"));
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

#[test]
fn recipe_projection_retains_every_current_field_and_ordered_operand() {
    use serde_json::json;
    let current = json!({
        "kind":"function", "contract_version":crate::platform::canonical_schema::SCHEMA_EPOCH,
        "result":{"ty":"T","relation":{"kind":"value"}},
        "read_scope_parameters":[], "read_scope_sources":[],
        "implementation_parameters":[], "effect_parameters":[],
        "requirement_parameters":[], "type_parameters":[],
        "parameters":["first", "second"],
        "body":{"kind":"call", "function":"function-a", "arguments":["left", "right"],
            "type_arguments":[], "effect_arguments":[], "requirement_arguments":[],
            "read_scope_arguments":[], "implementations":[]}
    });
    let observe = |value: &Value| normalized_json(value, &BTreeMap::new());
    assert_eq!(observe(&current), serde_json::to_string(&current).unwrap());
    for field in [
        "contract_version",
        "result",
        "read_scope_parameters",
        "read_scope_sources",
        "implementation_parameters",
        "effect_parameters",
        "requirement_parameters",
        "type_parameters",
    ] {
        let mut changed = current.clone();
        changed.as_object_mut().unwrap().remove(field);
        assert_ne!(
            observe(&current),
            observe(&changed),
            "retain explicit current {field}"
        );
    }
    for field in [
        "type_arguments",
        "effect_arguments",
        "requirement_arguments",
        "read_scope_arguments",
        "implementations",
    ] {
        let mut changed = current.clone();
        changed["body"].as_object_mut().unwrap().remove(field);
        assert_ne!(
            observe(&current),
            observe(&changed),
            "retain current call {field}"
        );
    }
    let mut results = std::collections::BTreeSet::from([observe(&current)]);
    for relation in [
        json!({"kind":"read_from","source":"source-a"}),
        json!({"kind":"read_from","source":"source-b"}),
        json!({"kind":"view_from","source":"source-a","scope":"scope-a"}),
        json!({"kind":"view_from","source":"source-a","scope":"scope-b"}),
    ] {
        let mut changed = current.clone();
        changed["result"]["relation"] = relation;
        assert_ne!(observe(&current), observe(&changed));
        assert!(
            results.insert(observe(&changed)),
            "result source and scope stay exact"
        );
    }
    let mut unused_scope = current.clone();
    unused_scope["read_scope_parameters"] = json!([{"id":"unused", "name":"scope"}]);
    assert_ne!(observe(&current), observe(&unused_scope));
    let mut reversed = current.clone();
    reversed["parameters"].as_array_mut().unwrap().reverse();
    assert_ne!(observe(&current), observe(&reversed));
    let mut reversed = current.clone();
    reversed["body"]["arguments"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert_ne!(observe(&current), observe(&reversed));
    let dependency = json!({"graph_contract_version":crate::platform::canonical_schema::SCHEMA_EPOCH, "package":"supplier",
        "semantic_revision":"revision-a", "package_revision":"package-a"});
    for field in [
        "graph_contract_version",
        "semantic_revision",
        "package_revision",
    ] {
        let mut changed = dependency.clone();
        changed.as_object_mut().unwrap().remove(field);
        assert_ne!(
            observe(&dependency),
            observe(&changed),
            "retain supplier {field}"
        );
    }
}

#[test]
fn current_literal_oracle_detects_recipe_body_and_requirement_changes() {
    let temporary = tempfile::tempdir().unwrap();
    for (name, literal, from, to) in [
        (
            "command",
            include_str!("fixtures/command.lkjc"),
            "(static-text \"hello\")",
            "(static-text \"goodbye\")",
        ),
        (
            "http",
            include_str!("fixtures/http.lkjc"),
            "maximum_calls 10000",
            "maximum_calls 9999",
        ),
    ] {
        assert!(literal.contains(from));
        let original = temporary.path().join(format!("{name}-original"));
        let changed = temporary.path().join(format!("{name}-changed"));
        create_literal_project(&original, name, Some(literal));
        create_literal_project(&changed, name, Some(&literal.replace(from, to)));
        assert_ne!(
            recipe_projection(&original),
            recipe_projection(&changed),
            "{name} mutation"
        );
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

fn deployment_projection(path: &Path) -> Option<Value> {
    let command = path.join(STARTER_COMMAND_DESCRIPTOR_PATH);
    let http = path.join(STARTER_HTTP_DESCRIPTOR_PATH);
    let descriptor = if command.exists() {
        assert!(!http.exists());
        command
    } else if http.exists() {
        http
    } else {
        assert!(!path.join(STARTER_HTTP_ARTIFACT_DIRECTORY).exists());
        return None;
    };
    assert!(path.join(STARTER_HTTP_ARTIFACT_DIRECTORY).is_dir());
    assert!(!path.join(STARTER_HTTP_ARTIFACT_PATH).exists());
    let bytes = std::fs::read(&descriptor).expect("current deployment descriptor");
    crate::platform::deployment::decode_deployment(&bytes).expect("strict current descriptor");
    let mut value: Value = serde_json::from_slice(&bytes).expect("current descriptor JSON");
    let mut revisions = std::collections::BTreeSet::new();
    for grant in value["grants"]
        .as_array_mut()
        .expect("current grant inventory")
    {
        let revision = grant["authority_revision"]
            .as_str()
            .expect("fresh grant authority");
        assert_eq!(revision.len(), 64);
        assert!(revision.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_ne!(revision, "0".repeat(64));
        assert!(
            revisions.insert(revision.to_owned()),
            "grant authorities must be distinct"
        );
        grant["authority_revision"] = Value::String(format!(
            "authority:{}",
            grant["requirement"]
                .as_str()
                .expect("exact grant requirement")
        ));
    }
    Some(value)
}
