//! Native standard joining: independent values, typed failures and source-free use.
use super::*;
use serde_json::json;

const SOURCE: &str = include_str!("../../docs/guides/examples/text-join.lkjc");

fn author() -> Native {
    let public = Native::template("command");
    let input = public.input(
        "text.lkjc",
        &SOURCE.replacen("base=BASE", &format!("base={}", public.revision()), 1),
    );
    let plan = public.plan(&input, true);
    public.apply(&input, &plan, true);
    let check = public.cli(&["check"], true);
    assert_eq!(
        compact_field(compact_record(&check, "tests"), "passed"),
        "66"
    );
    assert_eq!(
        compact_field(compact_record(&check, "tests"), "differential"),
        "equal"
    );
    public
}

fn descriptor(public: &Native, target: &str) -> PathBuf {
    public.input(&format!("{target}.deployment.json"), &json!({
        "artifact":"text.lkja", "target":target, "listen":null,
        "http":null, "session":null, "worker":null,
        "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,"maximum_total_bytes":1048576,"maximum_live_streams":1024},
        "grants":[], "secrets":[], "configuration":{}
    }).to_string())
}

fn cases() -> (Vec<Value>, Vec<String>) {
    let mut inputs = Vec::new();
    let mut expected = Vec::new();
    let mut add = |items: Vec<String>, separator: &str| {
        // Independent standard-library oracle, not the native range algorithm.
        expected.push(items.join(separator));
        inputs.push(json!({"items":items, "separator":separator}));
    };
    for length in 0..=5 {
        for mut encoded in 0_usize..3_usize.pow(length) {
            let mut items = Vec::new();
            for _ in 0..length {
                items.push(["", "a", "猫"][encoded % 3].to_owned());
                encoded /= 3;
            }
            for separator in ["", ":", "\r\n", "🌱", "\0", "<>&\"'"] {
                add(items.clone(), separator);
            }
        }
    }
    add(
        vec![
            "e\u{301}".into(),
            "é".into(),
            "\0\r\n".into(),
            "<>&\"'`".into(),
        ],
        "🌱",
    );
    for length in [
        31, 32, 33, 255, 256, 257, 1023, 1024, 1025, 4097, 8192, 32768,
    ] {
        add(
            (0..length)
                .map(|i| {
                    if i % 7 == 0 {
                        String::new()
                    } else {
                        format!("{i}猫")
                    }
                })
                .collect(),
            "|",
        );
    }
    (inputs, expected)
}

#[test]
fn native_text_join_matches_independent_ordered_values_before_and_after_source_removal() {
    let public = author();
    let discovered = public.cli(
        &[
            "package",
            "builtin",
            "query",
            "owners",
            "--name",
            "text-join",
        ],
        true,
    );
    assert_eq!(
        compact_field(compact_record(&discovered, "owner"), "kind"),
        "pure_function"
    );
    let hidden = public.cli(
        &[
            "package",
            "builtin",
            "query",
            "owners",
            "--name",
            "text-join-range",
        ],
        true,
    );
    assert_eq!(
        compact_field(compact_record(&hidden, "summary"), "returned"),
        "0"
    );
    let owners = public.cli(&["query", "find", "module", "text-output"], true);
    let draft = public.root.path().join("draft.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--owner",
            compact_field(compact_record(&owners, "owner"), "id"),
            "--output",
            path(&draft),
        ],
        true,
    );
    let unchanged = public.plan(&draft, true);
    assert_eq!(
        compact_field(compact_record(&unchanged, "result"), "outcome"),
        "unchanged"
    );
    let artifact = public.root.path().join("text.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let deployment = descriptor(&public, "batch");
    let (inputs, expected) = cases();
    assert_eq!(inputs.len(), 2197);
    let input = public.input("cases.json", &json!([inputs]).to_string());
    for detached in [false, true] {
        if detached {
            std::fs::remove_dir_all(&public.project).unwrap();
            std::fs::remove_file(&draft).unwrap();
            std::fs::remove_file(public.root.path().join("text.lkjc")).unwrap();
        }
        let output = public.root.path().join(format!("values-{detached}.json"));
        let mut arguments = if detached {
            vec!["run", "--deployment", path(&deployment)]
        } else {
            vec!["run", "batch"]
        };
        arguments.extend([
            "--arguments-file",
            path(&input),
            "--result-file",
            path(&output),
        ]);
        public.cli(&arguments, true);
        assert_eq!(
            serde_json::from_slice::<Vec<String>>(&std::fs::read(output).unwrap()).unwrap(),
            expected
        );
    }
}

#[test]
fn native_text_join_rejects_foreign_types_and_preserves_accepted_meaning() {
    let public = author();
    let before = public.revision();
    for (index, body) in [
        r#"(call std::text-join (list I64 (i64 1)) (text ","))"#,
        "(call std::text-join (list Text) (bool false))",
        "(call std::text-join (list Text))",
        r#"(call std::text-join-range (list Text) (text ",") (i64 0) (i64 0))"#,
    ]
    .iter()
    .enumerate()
    {
        let input = public.input(&format!("invalid-{index}.lkjc"), &format!(
            "request base={before}\ndeclarations.begin\n(units (use std builtin) (module create bad (function create bad (visibility public) (returns Text) (effect pure) (body {body}))))\ndeclarations.end\n"
        ));
        let rejected = public.plan(&input, false);
        let code = compact_field(compact_record(&rejected, "diagnostic"), "code");
        assert!(
            !code.starts_with("change_block"),
            "invalid fixture syntax: {code}"
        );
        assert_eq!(public.revision(), before);
    }
    let artifact = public.root.path().join("text.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let deployment = descriptor(&public, "join");
    for (index, input) in [
        json!([[1], ","]),
        json!([["x"], false]),
        json!([null, ","]),
        json!([["x"]]),
        json!([["x"], ",", "extra"]),
    ]
    .iter()
    .enumerate()
    {
        let input = public.input(&format!("bad-{index}.json"), &input.to_string());
        for detached in [false, true] {
            let output = public
                .root
                .path()
                .join(format!("bad-{index}-{detached}.json"));
            let mut arguments = if detached {
                vec!["run", "--deployment", path(&deployment)]
            } else {
                vec!["run", "join"]
            };
            arguments.extend([
                "--arguments-file",
                path(&input),
                "--result-file",
                path(&output),
            ]);
            public.cli(&arguments, false);
            assert!(!output.exists());
        }
    }
    assert_eq!(public.revision(), before);
    let output = public.root.path().join("recovered.json");
    public.cli(
        &[
            "run",
            "join",
            "--arguments",
            "[[\"\",\"猫\",\"\"],\":\"]",
            "--result-file",
            path(&output),
        ],
        true,
    );
    assert_eq!(
        serde_json::from_slice::<String>(&std::fs::read(output).unwrap()).unwrap(),
        ":猫:"
    );
}
