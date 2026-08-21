//! Runtime values with deterministic collection ordering and opaque live authority handles.

use super::semantic::{NominalShape, OwnerId, ResolvedType, ValidatedPackage};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

pub const MAXIMUM_VALUE_DEPTH: usize = 256;
pub const MAXIMUM_COLLECTION_ITEMS: usize = 1_000_000;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ResourceId(pub(crate) u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceKind {
    ByteStream,
    Transaction,
    Secret,
}

#[derive(Clone)]
pub enum Value {
    Unit,
    Bool(bool),
    I64(i64),
    Bytes(Arc<[u8]>),
    Text(Arc<str>),
    StaticText(Arc<str>),
    Record {
        owner: Option<OwnerId>,
        fields: BTreeMap<String, Value>,
    },
    Variant {
        owner: OwnerId,
        case: String,
        payload: Option<Box<Value>>,
    },
    List(Arc<Vec<Value>>),
    Map(Arc<BTreeMap<MapKey, Value>>),
    Function(OwnerId),
    Resource {
        id: ResourceId,
        kind: ResourceKind,
    },
}

impl fmt::Debug for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unit => formatter.write_str("Unit"),
            Self::Bool(value) => formatter.debug_tuple("Bool").field(value).finish(),
            Self::I64(value) => formatter.debug_tuple("I64").field(value).finish(),
            Self::Bytes(value) => formatter
                .debug_struct("Bytes")
                .field("length", &value.len())
                .finish(),
            Self::Text(value) => formatter.debug_tuple("Text").field(value).finish(),
            Self::StaticText(value) => formatter.debug_tuple("StaticText").field(value).finish(),
            Self::Record { owner, fields } => formatter
                .debug_struct("Record")
                .field("owner", owner)
                .field("fields", fields)
                .finish(),
            Self::Variant {
                owner,
                case,
                payload,
            } => formatter
                .debug_struct("Variant")
                .field("owner", owner)
                .field("case", case)
                .field("payload", payload)
                .finish(),
            Self::List(value) => formatter.debug_tuple("List").field(value).finish(),
            Self::Map(value) => formatter.debug_tuple("Map").field(value).finish(),
            Self::Function(owner) => formatter.debug_tuple("Function").field(owner).finish(),
            Self::Resource {
                kind: ResourceKind::Secret,
                ..
            } => formatter.write_str("Secret(<redacted>)"),
            Self::Resource { id, kind } => formatter
                .debug_struct("Resource")
                .field("id", id)
                .field("kind", kind)
                .finish(),
        }
    }
}

impl Value {
    pub fn text(value: impl Into<Arc<str>>) -> Self {
        Self::Text(value.into())
    }

    pub fn static_text(value: impl Into<Arc<str>>) -> Self {
        Self::StaticText(value.into())
    }

    pub fn bytes(value: impl Into<Arc<[u8]>>) -> Self {
        Self::Bytes(value.into())
    }

    pub fn record(
        owner: Option<OwnerId>,
        fields: impl IntoIterator<Item = (String, Value)>,
    ) -> Self {
        Self::Record {
            owner,
            fields: fields.into_iter().collect(),
        }
    }

    pub fn variant(owner: OwnerId, case: impl Into<String>, payload: Option<Value>) -> Self {
        Self::Variant {
            owner,
            case: case.into(),
            payload: payload.map(Box::new),
        }
    }

    pub fn is_durable(&self) -> bool {
        match self {
            Self::Resource { .. } | Self::Function(_) => false,
            Self::Record { fields, .. } => fields.values().all(Self::is_durable),
            Self::Variant { payload, .. } => {
                payload.as_ref().is_none_or(|payload| payload.is_durable())
            }
            Self::List(items) => items.iter().all(Self::is_durable),
            Self::Map(entries) => entries.values().all(Self::is_durable),
            Self::Unit
            | Self::Bool(_)
            | Self::I64(_)
            | Self::Bytes(_)
            | Self::Text(_)
            | Self::StaticText(_) => true,
        }
    }

    pub fn field(&self, name: &str) -> Option<&Value> {
        match self {
            Self::Record { fields, .. } => fields.get(name),
            _ => None,
        }
    }

    pub fn canonical_json(&self) -> serde_json::Value {
        match self {
            Self::Unit => serde_json::Value::Null,
            Self::Bool(value) => serde_json::Value::Bool(*value),
            Self::I64(value) => serde_json::Value::from(*value),
            Self::Bytes(value) => serde_json::json!({
                "kind": "bytes",
                "length": value.len(),
                "blake3": blake3::hash(value).to_hex().to_string(),
            }),
            Self::Text(value) => serde_json::Value::String(value.to_string()),
            Self::StaticText(value) => {
                serde_json::json!({"kind": "static_text", "value": value.as_ref()})
            }
            Self::Record { owner, fields } => serde_json::json!({
                "kind": "record",
                "owner": owner,
                "fields": fields.iter().map(|(name, value)| (name.clone(), value.canonical_json())).collect::<serde_json::Map<_, _>>(),
            }),
            Self::Variant {
                owner,
                case,
                payload,
            } => serde_json::json!({
                "kind": "variant",
                "owner": owner,
                "case": case,
                "payload": payload.as_ref().map(|value| value.canonical_json()),
            }),
            Self::List(items) => {
                serde_json::Value::Array(items.iter().map(Self::canonical_json).collect())
            }
            Self::Map(entries) => serde_json::Value::Array(
                entries
                    .iter()
                    .map(|(key, value)| serde_json::json!([key, value.canonical_json()]))
                    .collect(),
            ),
            Self::Function(owner) => serde_json::json!({"kind": "function", "owner": owner}),
            Self::Resource { kind, .. } => serde_json::json!({
                "kind": "resource",
                "resource_kind": match kind {
                    ResourceKind::ByteStream => "byte_stream",
                    ResourceKind::Transaction => "transaction",
                    ResourceKind::Secret => "secret",
                },
                "value": "<opaque>",
            }),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum MapKey {
    Bool(bool),
    I64(i64),
    Bytes(Vec<u8>),
    Text(String),
}

impl MapKey {
    pub fn from_value(value: Value) -> Result<Self, ValueError> {
        match value {
            Value::Bool(value) => Ok(Self::Bool(value)),
            Value::I64(value) => Ok(Self::I64(value)),
            Value::Bytes(value) => Ok(Self::Bytes(value.to_vec())),
            Value::Text(value) => Ok(Self::Text(value.to_string())),
            Value::StaticText(value) => Ok(Self::Text(value.to_string())),
            _ => Err(ValueError::new(
                "value_map_key",
                "map key is not a deterministically ordered primitive",
            )),
        }
    }

    pub fn to_value(&self) -> Value {
        match self {
            Self::Bool(value) => Value::Bool(*value),
            Self::I64(value) => Value::I64(*value),
            Self::Bytes(value) => Value::bytes(value.clone()),
            Self::Text(value) => Value::text(value.clone()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValueError {
    pub code: &'static str,
    pub message: String,
}

impl ValueError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

pub fn value_matches_type(
    value: &Value,
    ty: &ResolvedType,
    packages: &BTreeMap<super::package::PackageId, ValidatedPackage>,
) -> Result<(), ValueError> {
    let mut pending = vec![(value, ty, 0usize)];
    let mut items = 0usize;
    while let Some((value, ty, depth)) = pending.pop() {
        if depth > MAXIMUM_VALUE_DEPTH {
            return Err(ValueError::new(
                "value_depth",
                format!("value exceeds maximum depth {MAXIMUM_VALUE_DEPTH}"),
            ));
        }
        items = items.saturating_add(1);
        if items > MAXIMUM_COLLECTION_ITEMS {
            return Err(ValueError::new(
                "value_items",
                format!("value exceeds maximum item count {MAXIMUM_COLLECTION_ITEMS}"),
            ));
        }
        match (value, ty) {
            (Value::Unit, ResolvedType::Unit)
            | (Value::Bool(_), ResolvedType::Bool)
            | (Value::I64(_), ResolvedType::I64)
            | (Value::Bytes(_), ResolvedType::Bytes)
            | (Value::Text(_), ResolvedType::Text)
            | (Value::StaticText(_), ResolvedType::StaticText)
            | (
                Value::Resource {
                    kind: ResourceKind::Secret,
                    ..
                },
                ResolvedType::Secret,
            )
            | (
                Value::Resource {
                    kind: ResourceKind::ByteStream,
                    ..
                },
                ResolvedType::Stream(_),
            ) => {}
            (Value::Function(_), ResolvedType::Function(_, _)) => {}
            (
                Value::Record {
                    owner: Some(actual),
                    fields,
                },
                ResolvedType::Nominal(expected),
            ) if actual == expected => {
                let shape = nominal_shape(packages, expected)?;
                let NominalShape::Record(expected_fields) = shape else {
                    return Err(ValueError::new(
                        "value_nominal_kind",
                        "nominal variant was represented as a record",
                    ));
                };
                if fields.len() != expected_fields.len() {
                    return Err(ValueError::new(
                        "value_record_fields",
                        "record field count does not match its nominal type",
                    ));
                }
                for field in expected_fields {
                    let field_value = fields.get(&field.name).ok_or_else(|| {
                        ValueError::new(
                            "value_record_field",
                            format!("record omits field '{}'", field.name),
                        )
                    })?;
                    pending.push((field_value, &field.ty, depth + 1));
                }
            }
            (
                Value::Variant {
                    owner: actual,
                    case,
                    payload,
                },
                ResolvedType::Nominal(expected),
            ) if actual == expected => {
                let shape = nominal_shape(packages, expected)?;
                let NominalShape::Variant(cases) = shape else {
                    return Err(ValueError::new(
                        "value_nominal_kind",
                        "nominal record was represented as a variant",
                    ));
                };
                let expected_payload = cases.get(case).ok_or_else(|| {
                    ValueError::new(
                        "value_variant_case",
                        format!("variant type has no case '{case}'"),
                    )
                })?;
                match (payload, expected_payload) {
                    (None, None) => {}
                    (Some(actual), Some(expected)) => {
                        pending.push((actual, expected, depth + 1));
                    }
                    _ => {
                        return Err(ValueError::new(
                            "value_variant_payload",
                            "variant payload presence does not match its case",
                        ));
                    }
                }
            }
            (
                Value::Record {
                    owner: None,
                    fields,
                },
                ResolvedType::Record(expected),
            ) => {
                if fields.len() != expected.len() {
                    return Err(ValueError::new(
                        "value_record_fields",
                        "structural record field count does not match its type",
                    ));
                }
                for field in expected {
                    let field_value = fields.get(&field.name).ok_or_else(|| {
                        ValueError::new(
                            "value_record_field",
                            format!("record omits field '{}'", field.name),
                        )
                    })?;
                    pending.push((field_value, &field.ty, depth + 1));
                }
            }
            (Value::List(values), ResolvedType::List(item_type)) => {
                for value in values.iter() {
                    pending.push((value, item_type, depth + 1));
                }
            }
            (Value::Map(values), ResolvedType::Map(key_type, value_type)) => {
                for (key, value) in values.iter() {
                    let key_value = key.to_value();
                    value_matches_type(&key_value, key_type, packages)?;
                    pending.push((value, value_type, depth + 1));
                }
            }
            _ => {
                return Err(ValueError::new(
                    "value_type",
                    format!("runtime value {value:?} does not match type {ty:?}"),
                ));
            }
        }
    }
    Ok(())
}

fn nominal_shape<'a>(
    packages: &'a BTreeMap<super::package::PackageId, ValidatedPackage>,
    owner: &OwnerId,
) -> Result<&'a NominalShape, ValueError> {
    packages
        .get(&owner.package)
        .and_then(|package| package.nominal_shapes.get(owner))
        .ok_or_else(|| {
            ValueError::new(
                "value_nominal_missing",
                format!("nominal owner '{}' is unavailable", owner.diagnostic_name()),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_key_order_is_closed_and_exact() {
        assert!(MapKey::Bool(false) < MapKey::Bool(true));
        assert!(MapKey::I64(-1) < MapKey::I64(0));
        assert!(MapKey::from_value(Value::Unit).is_err());
    }

    #[test]
    fn live_values_are_never_durable_or_revealed() {
        let secret = Value::Resource {
            id: ResourceId(42),
            kind: ResourceKind::Secret,
        };
        assert!(!secret.is_durable());
        assert_eq!(format!("{secret:?}"), "Secret(<redacted>)");
        assert_eq!(secret.canonical_json()["value"], "<opaque>");
    }
}
