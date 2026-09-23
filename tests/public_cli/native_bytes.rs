//! Public byte parsing: exact standard selection, literal expectations and detached use.
use super::*;
use base64::Engine;
use serde_json::json;

const PROGRAM: &str = r#"declarations.begin
(units
  (use std builtin)
  (module create octets
    (function create read (visibility public)
      (parameter create bytes (type Bytes)) (parameter create index (type I64))
      (returns I64) (effect pure)
      (body (call std::bytes-get (local bytes) (local index))))
    (function create collect (visibility private)
      (parameter create bytes (type Bytes)) (parameter create index (type I64))
      (parameter create result (type (list I64)))
      (returns (list I64)) (effect pure)
      (body (if (call std::less (local index) (call std::bytes-length (local bytes)))
        (call collect (local bytes) (call std::add (local index) (i64 1))
          (call std::list-append (types I64) (local result)
            (call read (local bytes) (local index))))
        (local result))))
    (function create all (visibility public)
      (parameter create bytes (type Bytes)) (returns (list I64)) (effect pure)
      (body (call collect (local bytes) (i64 0) (list I64))))
    (test create utf8-is-not-a-character-index (visibility private)
      (actual (call all (call std::bytes-from-text (text "猫"))))
      (expected (list I64 (i64 231) (i64 140) (i64 171))))
    (component create console (visibility private)
      (port create read (type (function (Bytes I64) I64)) (function read))
      (port create all (type (function (Bytes) (list I64))) (function all))))
  (target create read (component octets::console) (runner command) (port octets::console::read))
  (target create all (component octets::console) (runner command) (port octets::console::all)))
declarations.end
"#;

fn byte_value(bytes: &[u8]) -> Value {
    json!({"$bytes": base64::engine::general_purpose::STANDARD.encode(bytes)})
}

fn author() -> Native {
    let public = Native::template("command");
    let input = public.input(
        "bytes.lkjc",
        &format!("request base={}\n{PROGRAM}", public.revision()),
    );
    let plan = public.plan(&input, true);
    public.apply(&input, &plan, true);
    public.cli(&["check"], true);
    public
}

#[test]
fn native_byte_program_authors_checks_drafts_and_runs_without_its_source() {
    let public = author();
    let owner = public.cli(&["query", "find", "module", "octets"], true);
    let draft = public.root.path().join("draft.lkjc");
    public.cli(
        &[
            "change",
            "draft",
            "--owner",
            compact_field(compact_record(&owner, "owner"), "id"),
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
    let artifact = public.root.path().join("bytes.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let descriptor = public.input("bytes.deployment.json", &json!({
        "artifact":"bytes.lkja", "target":"all", "listen":null,
        "http":null, "session":null, "worker":null,
        "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,"maximum_total_bytes":1048576,"maximum_live_streams":1024},
        "grants":[], "secrets":[], "configuration":{}
    }).to_string());
    let bytes = (0..=255).collect::<Vec<u8>>();
    let cases = [vec![], bytes, vec![255, 0, 128, 13, 10, 231, 140, 171]];
    for (index, bytes) in cases.iter().enumerate() {
        let input = public.input(
            &format!("{index}.json"),
            &json!([byte_value(bytes)]).to_string(),
        );
        let result = public.root.path().join(format!("{index}.project.json"));
        public.cli(
            &[
                "run",
                "all",
                "--arguments-file",
                path(&input),
                "--result-file",
                path(&result),
            ],
            true,
        );
        assert_eq!(
            serde_json::from_slice::<Vec<u8>>(&std::fs::read(result).unwrap()).unwrap(),
            *bytes
        );
    }
    std::fs::remove_dir_all(&public.project).unwrap();
    std::fs::remove_file(draft).unwrap();
    std::fs::remove_file(public.root.path().join("bytes.lkjc")).unwrap();
    for (index, bytes) in cases.iter().enumerate() {
        let result = public.root.path().join(format!("{index}.detached.json"));
        let input = public.root.path().join(format!("{index}.json"));
        public.cli(
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
            serde_json::from_slice::<Vec<u8>>(&std::fs::read(result).unwrap()).unwrap(),
            *bytes
        );
    }
}

#[test]
fn native_byte_bounds_and_input_rejection_publish_no_result() {
    let public = author();
    let artifact = public.root.path().join("bytes.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let descriptor = public.input("read.deployment.json", &json!({
        "artifact":"bytes.lkja", "target":"read", "listen":null,
        "http":null, "session":null, "worker":null,
        "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,"maximum_total_bytes":1048576,"maximum_live_streams":1024},
        "grants":[], "secrets":[], "configuration":{}
    }).to_string());
    let before = public.revision();
    let mut cases = Vec::new();
    for bytes in [vec![], vec![0, 255, 128]] {
        for index in [i64::MIN, -1, bytes.len() as i64, i64::MAX] {
            cases.push((json!([byte_value(&bytes), index]), true));
        }
    }
    cases.extend([
        (json!([[0, 255], 0]), false),
        (json!([{"$bytes":"/x=="}, 0]), false),
        (json!([{"$bytes":"AA==", "extra":0}, 0]), false),
        (json!([byte_value(&[0]), true]), false),
    ]);
    for (index, (input, bound)) in cases.iter().enumerate() {
        let input = public.input(&format!("bad-{index}.json"), &input.to_string());
        for detached in [false, true] {
            let result = public
                .root
                .path()
                .join(format!("bad-{index}-{detached}.json"));
            let mut args = if detached {
                vec!["run", "--deployment", path(&descriptor)]
            } else {
                vec!["run", "read"]
            };
            args.extend([
                "--arguments-file",
                path(&input),
                "--result-file",
                path(&result),
            ]);
            let failed = public.cli(&args, false);
            if *bound {
                assert_eq!(
                    compact_field(compact_record(&failed, "diagnostic"), "code"),
                    "normalized_bytes_index"
                );
            }
            assert!(!result.exists());
        }
    }
    assert_eq!(public.revision(), before);
}

#[test]
fn malformed_byte_intrinsic_declarations_reject_before_acceptance() {
    let public = Native::new();
    let before = public.revision();
    for (index, parameters, result) in [
        (
            0,
            "(parameter create bytes (type Text)) (parameter create index (type I64))",
            "I64",
        ),
        (
            1,
            "(parameter create bytes (type Bytes)) (parameter create index (type Bool))",
            "I64",
        ),
        (
            2,
            "(parameter create bytes (type Bytes)) (parameter create index (type I64))",
            "Bool",
        ),
        (3, "(parameter create bytes (type Bytes))", "I64"),
    ] {
        let input = public.input(
            &format!("signature-{index}.lkjc"),
            &format!(
                r#"request base={before}
declarations.begin
(units (module create wrong
  (external create get (visibility public) (implementation core.bytes.get)
    {parameters} (returns {result}))))
declarations.end
"#
            ),
        );
        let rejected = public.plan(&input, false);
        assert_eq!(
            compact_field(compact_record(&rejected, "diagnostic"), "code"),
            "intrinsic_signature"
        );
        assert_eq!(public.revision(), before);
    }
}
