use super::super::semantic_change::ChangeRequest;
use super::registry::{
    PROTOCOL_SCHEMA_DIGEST_DOMAIN, PublicOperation, RegistryManifest, SchemaId,
    operation_descriptors,
};
use schemars::{JsonSchema, schema_for};
use serde::Serialize;
use serde_json::{Map, Value, json};

const PROTOCOL_SCHEMA_ID: &str = "https://lkjscript.org/schema/protocol-1.json";

#[derive(JsonSchema)]
#[schemars(rename = "lkjscript.ProtocolSchemaDocumentV1")]
#[allow(dead_code)]
struct ProtocolSchemaDocument {
    registry: RegistryManifest,
    authored_change: ChangeRequest,
    request: PublicRequestEnvelope,
    success: CliSuccessSchema,
    failure: CliFailureSchema,
    runtime_event: RuntimeEventSchema,
}

#[derive(JsonSchema)]
#[schemars(rename = "lkjscript.PublicRequestEnvelopeV1")]
#[allow(dead_code)]
struct PublicRequestEnvelope {
    operation: PublicOperation,
    arguments: Vec<String>,
    project: Option<String>,
}

#[derive(JsonSchema)]
#[schemars(rename = "lkjscript.CliSuccessV4")]
#[allow(dead_code)]
struct CliSuccessSchema {
    contract_version: u16,
    ok: bool,
    status: String,
    command: String,
    result: Value,
}

#[derive(JsonSchema)]
#[schemars(rename = "lkjscript.CliFailureV4")]
#[allow(dead_code)]
struct CliFailureSchema {
    contract_version: u16,
    ok: bool,
    status: String,
    error: DiagnosticSchema,
}

#[derive(JsonSchema)]
#[schemars(rename = "lkjscript.DiagnosticV1")]
#[allow(dead_code)]
struct DiagnosticSchema {
    class: String,
    code: String,
    message: String,
    location: Option<SourceLocationSchema>,
    notes: Vec<String>,
}

#[derive(JsonSchema)]
#[schemars(rename = "lkjscript.SourceLocationV1")]
#[allow(dead_code)]
struct SourceLocationSchema {
    path: String,
    byte_offset: usize,
    line: usize,
    column: usize,
}

#[derive(JsonSchema)]
#[schemars(rename = "lkjscript.RuntimeEventV1")]
#[allow(dead_code)]
struct RuntimeEventSchema {
    contract_version: u16,
    ok: bool,
    event: String,
    result: Value,
}

pub fn protocol_schema() -> Result<Value, String> {
    let schema = schema_for!(ProtocolSchemaDocument);
    let mut value = serde_json::to_value(schema)
        .map_err(|error| format!("protocol schema could not be encoded: {error}"))?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| "derived protocol schema is not an object".to_owned())?;
    object.insert(
        "$id".to_owned(),
        Value::String(PROTOCOL_SCHEMA_ID.to_owned()),
    );
    object.insert(
        "title".to_owned(),
        Value::String("lkjscript executable protocol registry".to_owned()),
    );
    let definitions = object
        .entry("$defs")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .ok_or_else(|| "derived protocol definitions are not an object".to_owned())?;

    for descriptor in operation_descriptors() {
        let request_name = descriptor.request_schema.name();
        if descriptor.request_schema == SchemaId::ChangeRequest {
            definitions.insert(
                request_name.to_owned(),
                json!({"$ref": "#/$defs/lkjscript.ChangeRequestV3"}),
            );
        } else {
            definitions.insert(
                request_name.to_owned(),
                operation_request_schema(descriptor.operation),
            );
        }
        let response_name = descriptor.response_schema.name();
        definitions
            .entry(response_name.to_owned())
            .or_insert_with(|| {
                let target = if descriptor.response_schema == SchemaId::RuntimeEvent {
                    "lkjscript.RuntimeEventV1"
                } else {
                    "lkjscript.CliSuccessV4"
                };
                json!({"$ref": format!("#/$defs/{target}")})
            });
    }

    let available = definitions
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    for descriptor in operation_descriptors() {
        for schema in [descriptor.request_schema, descriptor.response_schema] {
            if !available.contains(schema.name()) {
                return Err(format!(
                    "operation '{}' references missing schema '{}'",
                    descriptor.operation.name(),
                    schema.name()
                ));
            }
        }
    }
    Ok(value)
}

fn operation_request_schema(operation: PublicOperation) -> Value {
    json!({
        "type": "object",
        "properties": {
            "operation": {"type": "string", "const": operation.name()},
            "arguments": {
                "type": "array",
                "items": {"type": "string"},
                "maxItems": 10000
            },
            "project": {"type": ["string", "null"]}
        },
        "required": ["operation", "arguments"],
        "additionalProperties": false
    })
}

pub fn protocol_schema_bytes() -> Result<Vec<u8>, String> {
    let value = protocol_schema()?;
    let mut bytes = serde_json::to_vec_pretty(&value)
        .map_err(|error| format!("protocol schema could not be encoded: {error}"))?;
    bytes.push(b'\n');
    Ok(bytes)
}

pub fn protocol_schema_digest() -> Result<String, String> {
    let value = protocol_schema()?;
    let bytes = canonical_json(&value)?;
    let mut hasher = blake3::Hasher::new_derive_key(PROTOCOL_SCHEMA_DIGEST_DOMAIN);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(&bytes);
    Ok(hasher.finalize().to_hex().to_string())
}

fn canonical_json(value: &impl Serialize) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|error| format!("schema JSON encoding failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_deterministic_and_covers_every_descriptor() {
        let first = protocol_schema_bytes().expect("schema");
        let second = protocol_schema_bytes().expect("schema");
        assert_eq!(first, second);
        assert_eq!(protocol_schema_digest().expect("digest").len(), 64);
        let text = String::from_utf8(first).expect("UTF-8 schema");
        for descriptor in operation_descriptors() {
            assert!(text.contains(descriptor.request_schema.name()));
            assert!(text.contains(descriptor.response_schema.name()));
        }
    }
}
