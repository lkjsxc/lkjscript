//! Missing, empty and repeated are distinct across exact offline package boundaries.
use super::*;

#[test]
fn native_forms_selection_tests_detect_silent_first_value_wins() {
    let public = Native::template("command");
    let before = public.revision();
    let source =
        include_str!("../../docs/guides/examples/form-codec.lkjc").replace("LIBRARY_BASE", &before);
    let correct = "(arm selection::present (payload previous Text) (variant selection::repeated))";
    let wrong = "(arm selection::present (payload previous Text) (variant selection::present (local previous)))";
    assert_eq!(source.matches(correct).count(), 1);
    let original = public.input("original.lkjc", &source);
    let plan = public.plan(&original, true);
    let altered = public.input("first-wins.lkjc", &source.replace(correct, wrong));
    public.apply(&altered, &plan, false);
    assert_eq!(public.revision(), before);
    // The mutation remains well typed; native expectations must catch its wrong result.
    public.apply(&altered, &public.plan(&altered, true), true);
    let accepted = public.revision();
    let failed = public.cli(&["check"], false);
    assert_eq!(
        compact_field(compact_record(&failed, "diagnostic"), "code"),
        "normalized_test_failed"
    );
    assert_eq!(public.revision(), accepted);
}

#[test]
fn native_forms_selection_preserves_exact_names_and_rejects_decoded_aliases_without_source() {
    let library = author_library();
    let consumer = import_consumer(&library);
    let artifact = consumer.root.path().join("selection.lkja");
    consumer.cli(&["build", "--output", path(&artifact)], true);

    // Independent literal expectations, not the implementation's own lookup or encoder.
    let cases = [
        ("", "a", "missing", ""),
        ("&&", "", "missing", ""),
        ("a", "a", "present", ""),
        ("a=", "a", "present", ""),
        ("b=other&a=value", "a", "present", "value"),
        ("a=value&b=other", "a", "present", "value"),
        ("b=left&a=value&c=right", "a", "present", "value"),
        ("a=same&a=same", "a", "repeated", ""),
        ("a=first&a=last", "a", "repeated", ""),
        ("a=first&b=other&a=last", "a", "repeated", ""),
        ("b=first&a=only&b=last", "a", "present", "only"),
        ("A=value", "a", "missing", ""),
        ("a=first&%61=last", "a", "repeated", ""),
        ("a+b=first&a%20b=last", "a b", "repeated", ""),
        ("a%2bb=value", "a+b", "present", "value"),
        ("a+b=value", "a+b", "missing", ""),
        ("%2561=value", "a", "missing", ""),
        ("%2561=value", "%61", "present", "value"),
        ("=value", "", "present", "value"),
        ("=&=", "", "repeated", ""),
        (
            "%E7%8C%AB=%F0%9F%98%80%00%0D%0A",
            "猫",
            "present",
            "😀\0\r\n",
        ),
        ("e%CC%81=value", "é", "missing", ""),
        ("e%CC%81=value", "e\u{301}", "present", "value"),
        ("%EF%BB%BFa=value", "a", "missing", ""),
        ("%EF%BB%BFa=value", "\u{feff}a", "present", "value"),
        ("%00=value", "\0", "present", "value"),
        ("a=value&unrelated=%ff", "a", "invalid", "invalid UTF-8"),
        (
            "a=value&unrelated=%",
            "a",
            "invalid",
            "invalid percent escape",
        ),
        // Decode must finish even when lookup could already determine repetition.
        ("a=one&a=two&bad=%ff", "a", "invalid", "invalid UTF-8"),
    ];
    let inputs: Vec<_> = cases
        .iter()
        .map(|(body, name, _, _)| json!({"body":bytes(body.as_bytes()),"name":name}))
        .collect();
    let expected: Vec<_> = cases
        .iter()
        .map(|(_, _, state, value)| json!({"state":state,"value":value}))
        .collect();
    let arguments = consumer.input("selection.json", &json!([inputs]).to_string());
    let descriptor = consumer.input(
        "selection.deployment.json",
        &json!({
            "artifact":"selection.lkja","target":"select-batch","listen":null,
            "http":null,"session":null,"worker":null,
            "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,
                "maximum_total_bytes":1048576,"maximum_live_streams":1024},
            "grants":[],"secrets":[],"configuration":{}
        })
        .to_string(),
    );
    // Direct caller-created records need neither a body codec nor HTTP headers.
    let direct = consumer.cli(
        &["run", "lookup", "--arguments",
          r#"[[{"name":"a","value":"first"},{"name":"b","value":"other"},{"name":"a","value":"last"}],"a"]"#],
        true,
    );
    let value: Value =
        serde_json::from_str(compact_field(compact_record(&direct, "execution"), "value")).unwrap();
    assert_eq!(value, json!({"state":"repeated","value":""}));

    for detached in [false, true] {
        if detached {
            std::fs::remove_dir_all(&library.project).unwrap();
            std::fs::remove_file(library.root.path().join("forms.lkjc")).unwrap();
            std::fs::remove_file(library.root.path().join("forms.lkjp")).unwrap();
            std::fs::remove_dir_all(&consumer.project).unwrap();
            std::fs::remove_file(consumer.root.path().join("consumer.lkjc")).unwrap();
            assert!(!library.project.exists() && !consumer.project.exists());
        }
        let result = consumer.root.path().join(if detached {
            "detached-selection.json"
        } else {
            "project-selection.json"
        });
        let mut command = if detached {
            vec!["run", "--deployment", path(&descriptor)]
        } else {
            vec!["run", "select-batch"]
        };
        command.extend([
            "--arguments-file",
            path(&arguments),
            "--result-file",
            path(&result),
        ]);
        let execution = consumer.cli(&command, true);
        if !detached {
            assert_eq!(
                compact_field(compact_record(&execution, "execution"), "differential"),
                "equal"
            );
        }
        let actual: Value = serde_json::from_slice(&std::fs::read(&result).unwrap()).unwrap();
        assert_eq!(actual, json!(expected));
    }
}
