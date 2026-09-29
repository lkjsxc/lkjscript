//! Public ordinary-value semantics across generic calls, branches and callback iteration.
use super::*;
use serde_json::json;

#[test]
fn native_terminal_values_keep_aliases_and_recursive_payloads_after_source_removal() {
    let public = Native::template("command");
    let input = public.input(
        "values.lkjc",
        &include_str!("../fixtures/terminal-values.lkjc").replacen(
            "base=BASE",
            &format!("base={}", public.revision()),
            1,
        ),
    );
    public.apply(&input, &public.plan(&input, true), true);
    let check = public.cli(&["check"], true);
    assert_eq!(
        compact_field(compact_record(&check, "tests"), "failed"),
        "0"
    );
    assert_eq!(
        compact_field(compact_record(&check, "tests"), "differential"),
        "equal"
    );
    let draft = public.root.path().join("draft.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--module",
            "terminal-values",
            "--output",
            path(&draft),
        ],
        true,
    );
    assert_eq!(
        compact_field(
            compact_record(&public.plan(&draft, true), "result"),
            "outcome"
        ),
        "unchanged"
    );
    let artifact = public.root.path().join("values.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let deployment = public.input(
        "values.deployment.json",
        &json!({
            "artifact":"values.lkja", "target":"duplicate", "listen":null,
            "http":null, "session":null, "worker":null,
            "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,
                "maximum_total_bytes":1048576,"maximum_live_streams":1024},
            "grants":[], "secrets":[], "configuration":{}
        })
        .to_string(),
    );
    for detached in [false, true] {
        if detached {
            std::fs::remove_dir_all(&public.project).unwrap();
            std::fs::remove_file(&input).unwrap();
            std::fs::remove_file(&draft).unwrap();
        }
        for depth in [0, 1, 16, 48] {
            let mut value = json!({"case":"leaf","value":"猫\u{0}<>&"});
            for _ in 0..depth {
                value = json!({"case":"wrap","value":value});
            }
            for direct in [false, true] {
                let arguments = json!([direct, value]).to_string();
                let result = public
                    .root
                    .path()
                    .join(format!("result-{detached}-{depth}-{direct}.json"));
                let mut command = if detached {
                    vec!["run", "--deployment", path(&deployment)]
                } else {
                    vec!["run", "duplicate"]
                };
                command.extend(["--arguments", &arguments, "--result-file", path(&result)]);
                public.cli(&command, true);
                let actual: Value =
                    serde_json::from_slice(&std::fs::read(&result).unwrap()).unwrap();
                assert_eq!(
                    actual,
                    json!({"first":value,"second":value}),
                    "depth={depth} direct={direct} detached={detached}"
                );
            }
        }
    }
}
