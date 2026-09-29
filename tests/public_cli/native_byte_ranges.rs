//! Literal package authoring and source-free byte-range consumption.
use super::*;
use base64::Engine;
use serde_json::json;

const LIBRARY: &str = include_str!("../fixtures/byte-ranges-library.lkjc");
const CONSUMER: &str = include_str!("../fixtures/byte-ranges-consumer.lkjc");

fn bytes(value: &[u8]) -> Value {
    json!({"$bytes": base64::engine::general_purpose::STANDARD.encode(value)})
}

fn author_pair() -> (Native, Native) {
    let library = Native::template("command");
    let input = library.input(
        "library.lkjc",
        &format!("request base={}\n{LIBRARY}", library.revision()),
    );
    let plan = library.plan(&input, true);
    library.apply(&input, &plan, true);
    library.cli(&["check"], true);
    let transport = library.root.path().join("ranges.lkjp");
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
    let id = compact_field(package, "id");
    let revision = compact_field(package, "revision");
    let package_revision = compact_field(package, "package-revision");
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
    let body = CONSUMER
        .replace("LIBRARY_PACKAGE_REVISION", package_revision)
        .replace("LIBRARY_PACKAGE", id);
    let input = consumer.input("consumer.lkjc", &format!(
        "request base={}\nadd.dependency package={id} semantic-revision={revision} package-revision={package_revision}\n{body}", consumer.revision()));
    let plan = consumer.plan(&input, true);
    consumer.apply(&input, &plan, true);
    consumer.cli(&["check"], true);
    (library, consumer)
}

fn deployment(public: &Native, target: &str) -> PathBuf {
    public.input(&format!("{target}.deployment.json"), &json!({
        "artifact":"ranges.lkja", "target":target, "listen":null,
        "http":null, "session":null, "worker":null,
        "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,"maximum_total_bytes":1048576,"maximum_live_streams":1024},
        "grants":[], "secrets":[], "configuration":{}
    }).to_string())
}

fn run(
    public: &Native,
    descriptor: &Path,
    target: &str,
    input: &Value,
    serial: &str,
    detached: bool,
    success: bool,
) -> (Vec<CompactRecord>, PathBuf) {
    let input = public.input(&format!("{serial}.arguments.json"), &input.to_string());
    let result = public.root.path().join(format!("{serial}.result.json"));
    let mut arguments = if detached {
        vec!["run", "--deployment", path(descriptor)]
    } else {
        vec!["run", target]
    };
    arguments.extend([
        "--arguments-file",
        path(&input),
        "--result-file",
        path(&result),
    ]);
    (public.cli(&arguments, success), result)
}

#[test]
fn native_byte_ranges_compose_packages_captures_maps_and_detached_binary_io() {
    let (library, public) = author_pair();
    let draft = public.root.path().join("draft.lkjc");
    let owner = public.cli(&["query", "find", "module", "windows"], true);
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
    public.cli(
        &[
            "build",
            "--output",
            path(&public.root.path().join("ranges.lkja")),
        ],
        true,
    );
    let descriptor = deployment(&public, "range");
    let packet_descriptor = deployment(&public, "packet");
    let cases = [
        (vec![], 0, 0),
        ((0..=255).collect::<Vec<u8>>(), 127, 256),
        (vec![231, 140, 171, 0, 255, 13, 10], 1, 5),
        (vec![0, 255, 128], 0, 3),
        (vec![0, 255, 128], 3, 3),
    ];
    for detached in [false, true] {
        if detached {
            std::fs::remove_dir_all(&library.project).unwrap();
            std::fs::remove_file(library.root.path().join("ranges.lkjp")).unwrap();
            std::fs::remove_file(library.root.path().join("library.lkjc")).unwrap();
            std::fs::remove_dir_all(&public.project).unwrap();
            std::fs::remove_file(public.root.path().join("consumer.lkjc")).unwrap();
            std::fs::remove_file(&draft).unwrap();
        }
        for (index, (original, start, end)) in cases.iter().enumerate() {
            let selected = &original[*start..*end];
            let mut joined = selected.to_vec();
            joined.extend([0, 255]);
            let hex = selected
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            let expected = json!({"captured":bytes(selected), "copied":bytes(selected), "hex":hex,
                "joined":bytes(&joined), "lookup":7, "nested":bytes(selected), "original":bytes(original),
                "roundtrip":bytes(selected), "selected":bytes(selected)});
            let (_, result) = run(
                &public,
                &descriptor,
                "range",
                &json!([bytes(original), start, end]),
                &format!("range-{index}-{detached}"),
                detached,
                true,
            );
            assert_eq!(
                serde_json::from_slice::<Value>(&std::fs::read(result).unwrap()).unwrap(),
                expected
            );
        }
        for (index, (input, expected)) in [
            (vec![3, 0, 255, 128, 42], vec![0, 255, 128]),
            (vec![0, 42], vec![]),
            (vec![1, 255], vec![255]),
        ]
        .iter()
        .enumerate()
        {
            let (_, result) = run(
                &public,
                &packet_descriptor,
                "packet",
                &json!([bytes(input)]),
                &format!("packet-{index}-{detached}"),
                detached,
                true,
            );
            assert_eq!(
                serde_json::from_slice::<Value>(&std::fs::read(result).unwrap()).unwrap(),
                bytes(expected)
            );
        }
    }
}

#[test]
fn native_byte_ranges_reject_bad_bounds_and_inputs_without_publishing_results() {
    let (_library, public) = author_pair();
    public.cli(
        &[
            "build",
            "--output",
            path(&public.root.path().join("ranges.lkja")),
        ],
        true,
    );
    let descriptor = deployment(&public, "range");
    let packet_descriptor = deployment(&public, "packet");
    let before = public.revision();
    for detached in [false, true] {
        for (index, (start, end)) in [
            (i64::MIN, 0),
            (-1, 2),
            (2, 1),
            (0, 4),
            (4, 4),
            (0, i64::MAX),
            (i64::MAX, i64::MAX),
        ]
        .iter()
        .enumerate()
        {
            let (failure, result) = run(
                &public,
                &descriptor,
                "range",
                &json!([bytes(&[0, 255, 128]), start, end]),
                &format!("bad-range-{index}-{detached}"),
                detached,
                false,
            );
            assert_eq!(
                compact_field(compact_record(&failure, "diagnostic"), "code"),
                "normalized_bytes_range"
            );
            assert!(!result.exists());
        }
        for (index, input) in [
            json!([bytes(&[0]), true, 1]),
            json!([bytes(&[0]), 0]),
            json!([[0, 255], 0, 2]),
        ]
        .iter()
        .enumerate()
        {
            let (_, result) = run(
                &public,
                &descriptor,
                "range",
                input,
                &format!("bad-input-{index}-{detached}"),
                detached,
                false,
            );
            assert!(!result.exists());
        }
        for (index, (input, code)) in [
            (vec![], "normalized_bytes_index"),
            (vec![3, 0], "normalized_bytes_range"),
        ]
        .iter()
        .enumerate()
        {
            let (failure, result) = run(
                &public,
                &packet_descriptor,
                "packet",
                &json!([bytes(input)]),
                &format!("bad-packet-{index}-{detached}"),
                detached,
                false,
            );
            assert_eq!(
                compact_field(compact_record(&failure, "diagnostic"), "code"),
                *code
            );
            assert!(!result.exists());
        }
    }
    assert_eq!(public.revision(), before);
}

#[test]
fn native_byte_ranges_reject_foreign_closed_signatures_before_acceptance() {
    let public = Native::new();
    let before = public.revision();
    for (index, (name, parameters, result)) in [
        ("slice", "(parameter create a (type Text)) (parameter create b (type I64)) (parameter create c (type I64))", "Bytes"),
        ("slice", "(parameter create a (type Bytes)) (parameter create b (type Bool)) (parameter create c (type I64))", "Bytes"),
        ("slice", "(parameter create a (type Bytes)) (parameter create b (type I64))", "Bytes"),
        ("slice", "(parameter create a (type Bytes)) (parameter create b (type I64)) (parameter create c (type I64))", "I64"),
        ("copy", "(parameter create a (type Text))", "Bytes"),
        ("copy", "(parameter create a (type Bytes)) (parameter create b (type I64))", "Bytes"),
        ("copy", "(parameter create a (type Bytes))", "Text"),
    ].iter().enumerate() {
        let input = public.input(&format!("bad-signature-{index}.lkjc"), &format!(
            "request base={before}\ndeclarations.begin\n(units (module create wrong (external create operation (visibility public) (implementation core.bytes.{name}) {parameters} (returns {result}))))\ndeclarations.end\n"));
        let rejected = public.plan(&input, false);
        assert_eq!(compact_field(compact_record(&rejected, "diagnostic"), "code"), "intrinsic_signature");
        assert_eq!(public.revision(), before);
    }
}
