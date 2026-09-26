//! Ordinary form library transported to an independent, source-free consumer.
#[path = "native_form_encoding.rs"]
mod encoding;
use super::*;
use base64::Engine;
use serde_json::json;

fn bytes(value: &[u8]) -> Value {
    json!({"$bytes": base64::engine::general_purpose::STANDARD.encode(value)})
}

fn author_library() -> Native {
    let library = Native::template("command");
    let source = include_str!("../../docs/guides/examples/form-codec.lkjc")
        .replace("LIBRARY_BASE", &library.revision());
    let proposal = library.input("forms.lkjc", &source);
    let plan = library.plan(&proposal, true);
    library.apply(&proposal, &plan, true);
    let checked = library.cli(&["check"], true);
    assert_eq!(
        compact_field(compact_record(&checked, "tests"), "passed"),
        "124"
    );
    library
}

fn import_consumer(library: &Native) -> Native {
    let transport = library.root.path().join("forms.lkjp");
    let exported = library.cli(
        &[
            "package",
            "current",
            "export",
            "--kind",
            "transport",
            "--output",
            path(&transport),
        ],
        true,
    );
    let package = compact_record(&exported, "package");
    let consumer = Native::template("command");
    consumer.cli(
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            compact_field(package, "transport"),
            "--input-file",
            path(&transport),
        ],
        true,
    );
    let source = include_str!("../../docs/guides/examples/form-consumer.lkjc")
        .replace("CONSUMER_BASE", &consumer.revision())
        .replace(
            "LIBRARY_PACKAGE_REVISION",
            compact_field(package, "package-revision"),
        )
        .replace("LIBRARY_REVISION", compact_field(package, "revision"))
        .replace("LIBRARY_PACKAGE", compact_field(package, "id"));
    let proposal = consumer.input("consumer.lkjc", &source);
    let plan = consumer.plan(&proposal, true);
    consumer.apply(&proposal, &plan, true);
    let checked = consumer.cli(&["check"], true);
    assert_eq!(
        compact_field(compact_record(&checked, "tests"), "passed"),
        "125"
    );
    consumer
}

fn escaped(value: &str) -> String {
    value
        .as_bytes()
        .iter()
        .map(|byte| format!("%{byte:02X}"))
        .collect()
}

#[test]
fn native_forms_transport_preserves_fields_and_rejects_malformed_input_without_source() {
    let library = author_library();
    let consumer = import_consumer(&library);
    let artifact = consumer.root.path().join("forms.lkja");
    consumer.cli(&["build", "--output", path(&artifact)], true);
    let mut input = Vec::new();
    let mut expected = Vec::new();
    let samples = ["", "a", "a b", "&=+%", "猫", "😀", "\0\r\n", "\u{feff}name"];
    for name in samples {
        for value in samples {
            input.push(bytes(
                format!("{}={}", escaped(name), escaped(value)).as_bytes(),
            ));
            expected.push(json!({"valid":true,"fields":[{"name":name,"value":value}],"error":""}));
        }
    }
    for (body, error) in [
        ("a=%", "invalid percent escape"),
        ("a=%0", "invalid percent escape"),
        ("a=%0z", "invalid percent escape"),
        ("a=%ff", "invalid UTF-8"),
        ("%80=a", "invalid UTF-8"),
        ("a=%ed%a0%80", "invalid UTF-8"),
        ("a=%e2%82", "invalid UTF-8"),
        ("a=%f4%90%80%80", "invalid UTF-8"),
    ] {
        input.push(bytes(body.as_bytes()));
        expected.push(json!({"valid":false,"fields":[],"error":error}));
    }
    input.push(bytes(b"a=first&&a=last&"));
    expected.push(json!({"valid":true,"fields":[{"name":"a","value":"first"},{"name":"a","value":"last"}],"error":""}));
    input.push(bytes(b"ok=one&bad=\xff"));
    expected.push(json!({"valid":false,"fields":[],"error":"invalid UTF-8"}));
    input.push(bytes(format!("a={}", "x".repeat(8190)).as_bytes()));
    expected
        .push(json!({"valid":true,"fields":[{"name":"a","value":"x".repeat(8190)}],"error":""}));
    input.push(bytes(format!("a={}", "x".repeat(8191)).as_bytes()));
    expected.push(json!({"valid":false,"fields":[],"error":"body too large"}));
    let input = consumer.input("batch.json", &json!([input]).to_string());
    let descriptor = consumer.input("forms.deployment.json",&json!({
        "artifact":"forms.lkja","target":"batch","listen":null,
        "http":null,"session":null,"worker":null,
        "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,"maximum_total_bytes":1048576,"maximum_live_streams":1024},
        "grants":[],"secrets":[],"configuration":{}
    }).to_string());
    let expected = json!(expected);
    let result = consumer.root.path().join("project-results.json");
    consumer.cli(
        &[
            "run",
            "batch",
            "--arguments-file",
            path(&input),
            "--result-file",
            path(&result),
        ],
        true,
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(result).unwrap()).unwrap(),
        expected
    );
    let library_path = library.project.clone();
    drop(library);
    assert!(!library_path.exists());
    std::fs::remove_dir_all(&consumer.project).unwrap();
    std::fs::remove_file(consumer.root.path().join("consumer.lkjc")).unwrap();
    let result = consumer.root.path().join("detached-results.json");
    consumer.cli(
        &[
            "run",
            "--deployment",
            path(&descriptor),
            "--arguments-file",
            path(&input),
            "--result-file",
            path(&result),
        ],
        true,
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&std::fs::read(result).unwrap()).unwrap(),
        expected
    );
}
