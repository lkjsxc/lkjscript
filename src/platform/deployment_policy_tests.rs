//! Descriptor policy tests use literal legacy values, not the new policy default as oracle.
use super::*;
use serde_json::{Value, json};

const QUOTAS: [&str; 4] = [
    "instruction_fuel",
    "maximum_allocated_bytes",
    "maximum_collection_items",
    "maximum_capability_calls",
];

fn descriptor(execution: Value) -> Value {
    let mut value = serde_json::to_value(starter_http_deployment().unwrap()).unwrap();
    value["execution"] = execution;
    value
}

fn decode(value: &Value) -> Result<DeploymentDescriptor, Diagnostic> {
    decode_deployment(&serde_json::to_vec(value).unwrap())
}

fn quota_values(policy: NormalizedRunPolicy) -> [Option<u64>; 4] {
    [
        policy.instruction_steps,
        policy.maximum_allocated_bytes,
        policy.maximum_collection_items,
        policy.maximum_capability_calls,
    ]
}

fn explicit() -> Value {
    json!({"instruction_fuel": 7, "maximum_call_depth": 19, "maximum_value_stack": 43,
        "maximum_allocated_bytes": 11, "maximum_collection_items": 13, "maximum_capability_calls": 17})
}

#[test]
fn duplicate_configuration_keys_reject_instead_of_selecting_a_value() {
    let mut value = descriptor(explicit());
    value["configuration"] = json!("CONFIGURATION_FIXTURE");
    let raw = serde_json::to_string(&value).unwrap();
    for key in ["alpha", r"\u0061lpha", r"a\u006cpha"] {
        for second in ["first", "second"] {
            let configuration = format!(
                r#"{{"alpha":{{"kind":"text","value":"first"}},"{key}":{{"kind":"text","value":"{second}"}}}}"#
            );
            let input = raw.replace("\"CONFIGURATION_FIXTURE\"", &configuration);
            let error = decode_deployment(input.as_bytes()).unwrap_err();
            assert_eq!(error.code, "deployment_json", "{key}: {second}");
            assert!(error.message.contains("duplicate JSON object field"));
        }
    }
}

#[test]
fn strict_descriptor_projection_retains_integer_extremes_and_string_contents() {
    let mut value = descriptor(explicit());
    value["execution"]["instruction_fuel"] = json!(u64::MAX);
    let quoted = r#"{"alpha":1,"alpha":2} 18446744073709551616 1.25"#;
    value["configuration"] = json!({
        "minimum": {"kind":"i64","value":i64::MIN},
        "maximum": {"kind":"i64","value":i64::MAX},
        "quoted": {"kind":"text","value":quoted},
        "enabled": {"kind":"bool","value":true}
    });
    let decoded = decode(&value).unwrap();
    assert_eq!(decoded.execution.unwrap().instruction_fuel, Some(u64::MAX));
    assert_eq!(
        decoded.configuration["minimum"],
        ConfigurationValue::I64(i64::MIN)
    );
    assert_eq!(
        decoded.configuration["maximum"],
        ConfigurationValue::I64(i64::MAX)
    );
    assert_eq!(
        decoded.configuration["quoted"],
        ConfigurationValue::Text(quoted.to_owned())
    );
    assert_eq!(
        decoded.configuration["enabled"],
        ConfigurationValue::Bool(true)
    );
    assert_eq!(decoded.configuration.len(), 4);
}

#[test]
fn legacy_numeric_descriptors_keep_the_previously_implicit_quotas() {
    let value = descriptor(json!({"instruction_fuel": 123, "maximum_call_depth": 19,
        "maximum_value_stack": 43}));
    let decoded = decode(&value).unwrap();
    let normalized = normalized_run_policy(decoded.execution.unwrap());
    assert_eq!(
        quota_values(normalized),
        [Some(123), Some(268_435_456), Some(1_000_000), Some(100_000)]
    );
    assert_eq!(normalized.maximum_call_depth, 19);
    assert_eq!(normalized.maximum_value_stack, 43);
    let encoded = encode_deployment(&decoded).unwrap();
    let roundtrip = decode_deployment(&encoded).unwrap();
    assert_eq!(decoded.execution, roundtrip.execution);
    let materialized: Value = serde_json::from_slice(&encoded).unwrap();
    for name in QUOTAS {
        assert!(materialized["execution"].get(name).is_some());
    }
}

#[test]
fn explicit_null_disables_only_the_selected_quota() {
    for (index, name) in QUOTAS.iter().enumerate() {
        for choice in [None, Some(1), Some(u64::MAX)] {
            let mut execution = explicit();
            execution[name] = json!(choice);
            let decoded = decode(&descriptor(execution)).unwrap();
            let normalized = normalized_run_policy(decoded.execution.unwrap());
            let mut expected = [Some(7), Some(11), Some(13), Some(17)];
            expected[index] = choice;
            assert_eq!(quota_values(normalized), expected, "{name}: {choice:?}");
            assert_eq!(normalized.maximum_call_depth, 19);
            assert_eq!(normalized.maximum_value_stack, 43);
        }
    }
    let legacy_null = descriptor(json!({"instruction_fuel": null,
        "maximum_call_depth": 19, "maximum_value_stack": 43}));
    assert_eq!(
        quota_values(normalized_run_policy(
            decode(&legacy_null).unwrap().execution.unwrap()
        )),
        [None, Some(268_435_456), Some(1_000_000), Some(100_000)]
    );
}

#[test]
fn new_http_recipe_has_no_hidden_cumulative_quotas_but_keeps_live_controls() {
    let starter = starter_http_deployment().unwrap();
    let encoded = encode_deployment(&starter).unwrap();
    let value: Value = serde_json::from_slice(&encoded).unwrap();
    for name in QUOTAS {
        assert_eq!(value["execution"].get(name), Some(&Value::Null), "{name}");
    }
    let decoded = decode_deployment(&encoded).unwrap();
    let normalized = normalized_run_policy(decoded.execution.unwrap());
    assert_eq!(quota_values(normalized), [None; 4]);
    assert_eq!(normalized.maximum_call_depth, 4096);
    assert_eq!(normalized.maximum_value_stack, 1_000_000);
    let runtime = decoded.runtime.unwrap();
    assert_eq!(runtime.maximum_concurrent_tasks, 16);
    assert_eq!(runtime.maximum_queued_tasks, 64);
    assert_eq!(runtime.request_deadline_milliseconds, 30_000);
    assert_eq!(runtime.cancellation_grace_milliseconds, 5_000);
    assert_eq!(runtime.shutdown_grace_milliseconds, 30_000);
    assert_eq!(decoded.streams.maximum_live_streams, 1024);
    assert_eq!(
        decoded.http.unwrap().maximum_request_body_bytes,
        8 * 1024 * 1024
    );
    assert_eq!(decoded.grants.len(), 1);
}

#[test]
fn malformed_quotas_and_missing_live_limits_fail_strict_admission() {
    for name in QUOTAS {
        for value in [
            json!(0),
            json!(-1),
            json!(1.5),
            json!(false),
            json!("unlimited"),
            json!([]),
        ] {
            let mut execution = explicit();
            execution[name] = value.clone();
            let error = decode(&descriptor(execution)).unwrap_err();
            assert_eq!(
                error.code,
                if value == json!(0) {
                    "deployment_execution_limit"
                } else {
                    "deployment_json"
                }
            );
        }
    }
    for name in [
        "instruction_fuel",
        "maximum_call_depth",
        "maximum_value_stack",
    ] {
        let mut execution = explicit();
        execution.as_object_mut().unwrap().remove(name);
        assert_eq!(
            decode(&descriptor(execution)).unwrap_err().code,
            "deployment_json",
            "{name}"
        );
    }
    for name in ["maximum_call_depth", "maximum_value_stack"] {
        for value in [Value::Null, json!(0)] {
            let mut execution = explicit();
            execution[name] = value;
            assert!(decode(&descriptor(execution)).is_err());
        }
    }
    let raw = serde_json::to_string(&descriptor(explicit())).unwrap();
    assert!(raw.contains("\"instruction_fuel\":7"));
    for replacement in [
        "\"instruction_fuel\":18446744073709551616",
        "\"instruction_fuel\":null,\"instruction_fuel\":7",
    ] {
        assert_eq!(
            decode_deployment(
                raw.replace("\"instruction_fuel\":7", replacement)
                    .as_bytes()
            )
            .unwrap_err()
            .code,
            "deployment_json"
        );
    }
    let mut unknown = explicit();
    unknown["instruction_steps"] = Value::Null;
    assert_eq!(
        decode(&descriptor(unknown)).unwrap_err().code,
        "deployment_json"
    );
}

#[test]
fn discovered_execution_fields_match_required_and_nullable_decoder_contracts() {
    let fields: Vec<_> = DEPLOYMENT_SCHEMA_FIELDS
        .iter()
        .filter(|field| field.path.starts_with("execution."))
        .collect();
    assert_eq!(fields.len(), 6);
    for field in fields {
        let name = field.path.strip_prefix("execution.").unwrap();
        let mut execution = explicit();
        execution.as_object_mut().unwrap().remove(name);
        assert_eq!(
            decode(&descriptor(execution)).is_err(),
            field.required,
            "{name}"
        );
        let mut execution = explicit();
        execution[name] = Value::Null;
        assert_eq!(
            decode(&descriptor(execution)).is_ok(),
            field.scalar.starts_with("null|"),
            "{name}"
        );
        assert_eq!(field.minimum, Some(1));
    }
}

#[test]
fn every_resident_topology_keeps_complete_live_policy_requirements() {
    for topology in ["http", "session", "worker"] {
        let mut value = serde_json::to_value(starter_http_deployment().unwrap()).unwrap();
        match topology {
            "http" => (),
            "session" => {
                value["http"] = Value::Null;
                value["session"] = serde_json::to_value(SessionLimits::default()).unwrap();
            }
            "worker" => {
                value["listen"] = Value::Null;
                value["http"] = Value::Null;
                value["worker"] = serde_json::to_value(WorkerLimits::default()).unwrap();
            }
            _ => unreachable!(),
        }
        let accepted = decode(&value).unwrap();
        assert_eq!(
            quota_values(normalized_run_policy(accepted.execution.unwrap())),
            [None; 4]
        );
        for name in ["execution", "runtime"] {
            let mut missing = value.clone();
            missing.as_object_mut().unwrap().remove(name);
            assert_eq!(
                decode(&missing).unwrap_err().code,
                "deployment_policy_required",
                "{topology} {name}"
            );
            let mut null = value.clone();
            null[name] = Value::Null;
            assert_eq!(
                decode(&null).unwrap_err().code,
                "deployment_json",
                "{topology} {name}"
            );
        }
    }
}
