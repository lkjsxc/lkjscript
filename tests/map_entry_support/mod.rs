use crate::support::{Native, field, path};
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path};

pub fn input(n: i64, shape: u8) -> Vec<Value> {
    let mut keys: Vec<_> = (0..n).collect();
    match shape {
        1 => keys.reverse(),
        2 => keys.sort_by_key(|key| (key % 2, *key)),
        _ => {}
    }
    let mut entries: Vec<_> = keys
        .into_iter()
        .map(|key| {
            json!({
                "key":key,"value":{"items":[key,13-7*key,-key],"marker":101+key}
            })
        })
        .collect();
    if shape == 3 && n > 0 {
        for key in [n - 1, 0, n / 2] {
            entries.push(json!({"key":key,"value":{"items":[],"marker":-99-key}}));
        }
    }
    entries
}
pub fn expected(input: &[Value], repetitions: i64) -> Value {
    // A separate standard ordered map specifies final values, not the VM cursor.
    let mut map = BTreeMap::new();
    for entry in input {
        map.insert(entry["key"].as_i64().unwrap(), entry["value"].clone());
    }
    let count = map.len() as i64;
    let entries: Vec<_> = map
        .into_iter()
        .map(|(key, value)| json!({"key":key,"value":value}))
        .collect();
    json!({"entries":entries,"total":count*repetitions})
}
pub fn input_nodes(input: &[Value]) -> u64 {
    3 + input
        .iter()
        .map(|entry| 5 + entry["value"]["items"].as_array().unwrap().len() as u64)
        .sum::<u64>()
}
pub fn valid_cost(control: &Value, projected: &Value, nodes: u64, n: u64, k: u64) -> bool {
    let Some(extra) = n.checked_mul(k) else {
        return false;
    };
    for work in [control, projected] {
        if work["input_admission_nodes"].as_u64() != Some(nodes)
            || work["raw_result_admission_nodes"].as_u64() != Some(0)
            || work["internal_guard_descendant_visits"].as_u64() != Some(0)
            || work["capture_admission_nodes"].as_u64() != Some(0)
            || work["maps"]["key_bytes_copied"].as_u64() != Some(0)
        {
            return false;
        }
    }
    control["maps"]["node_visits"]
        .as_u64()
        .zip(projected["maps"]["node_visits"].as_u64())
        .is_some_and(|(a, b)| {
            a != u64::MAX && b != u64::MAX && b.checked_sub(a).is_some_and(|delta| delta <= extra)
        })
}
pub fn refusal(
    public: &Native,
    descriptor: &Path,
    name: &str,
    arguments: &Path,
    class: &str,
    code: &str,
) {
    let absent = public.root.join(format!("refused-{name}.json"));
    let output = public.cli(
        &public.root,
        &[
            "run",
            "--deployment",
            path(descriptor),
            "--arguments-file",
            path(arguments),
            "--result-file",
            path(&absent),
        ],
        false,
    );
    assert_eq!(field(&output, "diagnostic", "class"), class);
    assert_eq!(field(&output, "diagnostic", "code"), code);
    assert!(!absent.exists());
}

#[test]
fn map_entry_cost_oracle_rejects_skipped_ingress_repeated_search_and_raw_readmission() {
    let control = json!({"input_admission_nodes":8195,"raw_result_admission_nodes":0,
        "internal_guard_descendant_visits":0,"capture_admission_nodes":0,
        "maps":{"node_visits":2048,"key_bytes_copied":0}});
    let mut projected = control.clone();
    projected["maps"]["node_visits"] = json!(2048 + 8192);
    assert!(valid_cost(&control, &projected, 8195, 1024, 8));
    for (field, value) in [
        ("input_admission_nodes", 8194),
        ("raw_result_admission_nodes", 1),
        ("internal_guard_descendant_visits", 1),
        ("capture_admission_nodes", 1),
    ] {
        let mut wrong = projected.clone();
        wrong[field] = json!(value);
        assert!(!valid_cost(&control, &wrong, 8195, 1024, 8));
    }
    // A later optimizer may reuse the immutable result rather than rescan it.
    for visits in [2048, 2048 + 8191] {
        let mut better = projected.clone();
        better["maps"]["node_visits"] = json!(visits);
        assert!(valid_cost(&control, &better, 8195, 1024, 8));
    }
    for visits in [2047, 2048 + 8193, 2048 + 10252 * 8, u64::MAX] {
        let mut wrong = projected.clone();
        wrong["maps"]["node_visits"] = json!(visits);
        assert!(!valid_cost(&control, &wrong, 8195, 1024, 8));
    }
    assert!(!valid_cost(&control, &projected, 8195, u64::MAX, 8));
}
