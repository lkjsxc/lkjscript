//! Ordinary byte programs through a copied executable, without an authoring checkout.
use super::*;
use base64::Engine;
use serde_json::json;

fn bytes(value: &[u8]) -> Value {
    json!({"$bytes": base64::engine::general_purpose::STANDARD.encode(value)})
}

#[test]
fn native_byte_reuse_preserves_recursive_generic_and_retained_consumers() {
    let public = Native::template("command");
    let input = public.input(
        "byte-buffer.lkjc",
        &include_str!("../fixtures/reusable-byte-buffer.lkjc").replacen(
            "base=BASE",
            &format!("base={}", public.revision()),
            1,
        ),
    );
    public.apply(&input, &public.plan(&input, true), true);
    public.cli(&["check"], true);
    let artifact = public.root.path().join("bytes.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let mut deployment: Value = serde_json::from_slice(
        &std::fs::read(public.project.join("command.deployment.json")).unwrap(),
    )
    .unwrap();
    deployment["artifact"] = json!("bytes.lkja");
    let segment = [0, 128, 255, 17];
    let mut cases = Vec::new();
    for count in [0_usize, 4, 1024] {
        cases.push((
            "grow",
            json!([count, bytes(&segment)]),
            bytes(&segment.repeat(count)),
            count,
        ));
    }
    cases.push((
        "retain",
        json!([bytes(&segment)]),
        json!({
            "prefix": bytes(&segment.repeat(3)), "next": bytes(&segment.repeat(4)),
            "captured": bytes(&segment.repeat(3)), "old": 17, "enlarged": -1,
            "roundtrip": bytes(&segment.repeat(4))
        }),
        4,
    ));
    let original_head = std::fs::read(public.project.join("HEAD")).unwrap();
    for detached in [false, true] {
        if detached {
            std::fs::rename(&public.project, public.root.path().join("retained-project")).unwrap();
            std::fs::remove_file(&input).unwrap();
        }
        for (target, arguments, expected, count) in &cases {
            let input_arguments = public.input("arguments.json", &arguments.to_string());
            deployment["target"] = json!(target);
            let descriptor = public.input("bytes.deployment.json", &deployment.to_string());
            let result = public.root.path().join("result.json");
            let mut route = if detached {
                vec!["run", "--deployment", path(&descriptor)]
            } else {
                vec!["run", target]
            };
            route.extend([
                "--arguments-file",
                path(&input_arguments),
                "--result-file",
                path(&result),
            ]);
            let records = public.cli(&route, true);
            let actual: Value = serde_json::from_slice(&std::fs::read(&result).unwrap()).unwrap();
            assert_eq!(&actual, expected, "{target}/{count}, detached={detached}");
            let execution = compact_record(&records, "execution");
            if detached {
                let observation: Value =
                    serde_json::from_str(compact_field(execution, "production-observation"))
                        .unwrap();
                let work = &observation["value_work"]["bytes"];
                assert_eq!(observation["live_handles_after"], 0);
                assert_eq!(work["concatenations"], *count as u64);
                if *target == "grow" && *count > 1 {
                    assert_eq!(work["empty_operand_reuses"], 1);
                    assert_eq!(work["fresh_buffers"], 1);
                    assert!(work["in_place_appends"].as_u64().unwrap() > 0);
                    assert!(
                        work["payload_bytes_copied"].as_u64().unwrap()
                            < (3 * count * segment.len()) as u64
                    );
                }
                if *target == "retain" {
                    assert_eq!(
                        work["fresh_buffers"], 2,
                        "map/closure retention forbids prefix mutation"
                    );
                }
                println!("byte-reuse target={target} count={count} work={work}");
            } else {
                assert_eq!(compact_field(execution, "differential"), "equal");
            }
            std::fs::remove_file(result).unwrap();
        }
    }
    assert_eq!(
        std::fs::read(public.root.path().join("retained-project/HEAD")).unwrap(),
        original_head
    );
}
