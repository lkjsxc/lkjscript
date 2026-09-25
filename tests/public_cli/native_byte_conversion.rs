//! Public authoring, strict failures, lossless Bytes and detached conversion.
use super::*;
use base64::Engine;
use serde_json::json;

const PROGRAM: &str = r#"declarations.begin
(units (use std builtin)
  (module create conversions
    (function create construct (visibility public)
      (parameter create input (type (list I64))) (returns Bytes) (effect pure)
      (body (call std::bytes-from-list (local input))))
    (function create decode (visibility public)
      (parameter create input (type Bytes)) (returns (record (valid Bool) (value Text))) (effect pure)
      (body (call std::bytes-to-text-result (local input))))
    (component create console (visibility private)
      (port create construct (type (function ((list I64)) Bytes)) (function construct))
      (port create decode (type (function (Bytes) (record (valid Bool) (value Text)))) (function decode))))
  (target create construct (component conversions::console) (runner command) (port conversions::console::construct))
  (target create decode (component conversions::console) (runner command) (port conversions::console::decode)))
declarations.end
"#;

fn byte_value(bytes: &[u8]) -> Value {
    json!({"$bytes":base64::engine::general_purpose::STANDARD.encode(bytes)})
}

#[test]
fn byte_conversions_run_from_copied_product_and_after_source_removal() {
    let public = Native::template("command");
    let source = public.input(
        "conversion.lkjc",
        &format!("request base={}\n{PROGRAM}", public.revision()),
    );
    let plan = public.plan(&source, true);
    public.apply(&source, &plan, true);
    public.cli(&["check"], true);
    let artifact = public.root.path().join("conversion.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let mut cases = Vec::new();
    for bytes in [
        vec![],
        (0..=255).collect(),
        vec![0, 255, 128, 231, 140, 171],
    ] {
        cases.push(("construct", json!([bytes]), byte_value(&bytes)));
    }
    for (bytes, valid, text) in [
        (vec![], true, ""),
        (b"A\0Z".to_vec(), true, "A\0Z"),
        ("\u{feff}猫😀".as_bytes().to_vec(), true, "\u{feff}猫😀"),
        (vec![255], false, ""),
        (vec![65, 128, 66], false, ""),
        (vec![237, 160, 128], false, ""),
        (vec![240, 144, 128], false, ""),
    ] {
        cases.push((
            "decode",
            json!([byte_value(&bytes)]),
            json!({"valid":valid,"value":text}),
        ));
    }
    for detached in [false, true] {
        if detached {
            std::fs::rename(&public.project, public.root.path().join("retained-source")).unwrap();
            std::fs::rename(&source, public.root.path().join("retained-input.lkjc")).unwrap();
        }
        for (index, (target, input, expected)) in cases.iter().enumerate() {
            let input = public.input(
                &format!("input-{detached}-{index}.json"),
                &input.to_string(),
            );
            let result = public
                .root
                .path()
                .join(format!("result-{detached}-{index}.json"));
            let descriptor = public.input(&format!("{detached}-{index}.deployment.json"),&json!({
                "artifact":"conversion.lkja","target":target,"listen":null,
                "http":null,"session":null,"worker":null,
                "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,"maximum_total_bytes":1048576,"maximum_live_streams":1024},
                "grants":[],"secrets":[],"configuration":{}
            }).to_string());
            let mut arguments = if detached {
                vec!["run", "--deployment", path(&descriptor)]
            } else {
                vec!["run", target]
            };
            arguments.extend([
                "--arguments-file",
                path(&input),
                "--result-file",
                path(&result),
            ]);
            public.cli(&arguments, true);
            assert_eq!(
                &serde_json::from_slice::<Value>(&std::fs::read(result).unwrap()).unwrap(),
                expected
            );
        }
    }
}

#[test]
fn byte_construction_bad_octets_and_foreign_inputs_publish_no_result() {
    let public = Native::template("command");
    let source = public.input(
        "conversion.lkjc",
        &format!("request base={}\n{PROGRAM}", public.revision()),
    );
    let plan = public.plan(&source, true);
    public.apply(&source, &plan, true);
    let before = public.revision();
    for (index, input) in [
        json!([[i64::MIN]]),
        json!([[65, -1]]),
        json!([[256, 65]]),
        json!([[i64::MAX]]),
        json!([[true]]),
        json!(["ABC"]),
        json!([[null]]),
    ]
    .into_iter()
    .enumerate()
    {
        let input = public.input(&format!("bad-{index}.json"), &input.to_string());
        let result = public.root.path().join(format!("bad-result-{index}.json"));
        let failed = public.cli(
            &[
                "run",
                "construct",
                "--arguments-file",
                path(&input),
                "--result-file",
                path(&result),
            ],
            false,
        );
        if index < 4 {
            assert_eq!(
                compact_field(compact_record(&failed, "diagnostic"), "code"),
                "normalized_bytes_octet"
            );
        }
        assert!(!result.exists());
    }
    assert_eq!(before, public.revision());
}

#[test]
fn byte_conversion_foreign_signatures_reject_before_publication() {
    let public = Native::new();
    let before = public.revision();
    for (index, (name, parameters, result)) in [
        (
            "from-list",
            "(parameter create input (type Bytes))",
            "Bytes",
        ),
        (
            "from-list",
            "(parameter create input (type (list Bool)))",
            "Bytes",
        ),
        (
            "from-list",
            "(parameter create input (type (list I64)))",
            "Text",
        ),
        ("from-list", "", "Bytes"),
        (
            "to-text-result",
            "(parameter create input (type Text))",
            "(record (valid Bool) (value Text))",
        ),
        (
            "to-text-result",
            "(parameter create input (type Bytes))",
            "Text",
        ),
        (
            "to-text-result",
            "(parameter create input (type Bytes))",
            "(record (valid Bool) (value I64))",
        ),
        (
            "to-text-result",
            "(parameter create input (type Bytes))",
            "(record (valid Bool) (value Text) (extra Bool))",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let input = public.input(
            &format!("signature-{index}.lkjc"),
            &format!(
                r#"request base={before}
declarations.begin
(units (module create invalid (external create value (visibility public)
  (implementation core.bytes.{name}) {parameters} (returns {result}))))
declarations.end
"#
            ),
        );
        let failed = public.plan(&input, false);
        assert_eq!(
            compact_field(compact_record(&failed, "diagnostic"), "code"),
            "intrinsic_signature"
        );
        assert_eq!(before, public.revision());
    }
}
