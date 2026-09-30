//! Fresh native generic packages and transport/artifact roundtrips with no memory grants.
use super::*;
use serde_json::json;
const PRODUCER: &str = include_str!("../fixtures/owned-buffer-producer.lkjc");
const TRANSFORMER: &str = include_str!("../fixtures/owned-buffer-transformer.lkjc");
const CONSUMER: &str = include_str!("../fixtures/owned-buffer-consumer.lkjc");
fn author(public: &Native, body: &str) {
    let input = public.input(
        "native.lkjc",
        &format!("request base={}\n{body}", public.revision()),
    );
    let plan = public.plan(&input, true);
    public.apply(&input, &plan, true);
    public.cli(&["check"], true);
}
struct Export {
    path: PathBuf,
    package: String,
    semantic: String,
    revision: String,
    transport: String,
}
fn export(public: &Native) -> Export {
    let output = public.root.path().join("package.lkjp");
    let records = public.cli(
        &[
            "package",
            "current",
            "export",
            "--kind",
            "transport",
            "--output",
            path(&output),
        ],
        true,
    );
    let record = compact_record(&records, "package");
    Export {
        path: output,
        package: compact_field(record, "id").into(),
        semantic: compact_field(record, "revision").into(),
        revision: compact_field(record, "package-revision").into(),
        transport: compact_field(record, "transport").into(),
    }
}
fn stage(public: &Native, package: &Export) {
    public.cli(
        &[
            "package",
            "dependency",
            "stage",
            "--transport",
            &package.transport,
            "--input-file",
            path(&package.path),
        ],
        true,
    );
}
fn dependency(e: &Export) -> String {
    format!(
        "add.dependency package={} semantic-revision={} package-revision={}\n",
        e.package, e.semantic, e.revision
    )
}
#[test]
fn native_byte_buffer_generic_three_packages_roundtrip_draft_and_detached_bytes() {
    let producer = Native::template("command");
    let discovery = producer.cli(&["capabilities", "--section", "type"], true);
    assert!(
        discovery
            .iter()
            .any(|r| r.operation == "type.form" && compact_field(r, "name") == "byte-buffer")
    );
    for name in [
        "buffer-empty",
        "buffer-push",
        "buffer-get",
        "buffer-length",
        "buffer-freeze",
        "buffer-discard",
    ] {
        let owners = producer.cli(
            &["package", "builtin", "query", "owners", "--name", name],
            true,
        );
        assert_eq!(
            owners.iter().filter(|r| r.operation == "owner").count(),
            1,
            "{name}"
        );
    }
    author(&producer, PRODUCER);
    let p = export(&producer);
    let transformer = Native::template("command");
    stage(&transformer, &p);
    author(
        &transformer,
        &format!(
            "{}{}",
            dependency(&p),
            TRANSFORMER
                .replace("PRODUCER_PACKAGE_REVISION", &p.revision)
                .replace("PRODUCER_PACKAGE", &p.package)
        ),
    );
    let t = export(&transformer);
    let consumer = Native::template("command");
    stage(&consumer, &p);
    stage(&consumer, &t);
    author(
        &consumer,
        &format!(
            "{}{}{}",
            dependency(&p),
            dependency(&t),
            CONSUMER
                .replace("PRODUCER_PACKAGE_REVISION", &p.revision)
                .replace("PRODUCER_PACKAGE", &p.package)
                .replace("TRANSFORMER_PACKAGE_REVISION", &t.revision)
                .replace("TRANSFORMER_PACKAGE", &t.package)
        ),
    );
    let module = consumer.cli(&["query", "find", "module", "destination"], true);
    let draft = consumer.root.path().join("draft.lkjc");
    consumer.cli(
        &[
            "change",
            "draft",
            "--owner",
            compact_field(compact_record(&module, "owner"), "id"),
            "--output",
            path(&draft),
        ],
        true,
    );
    assert!(
        std::fs::read_to_string(&draft)
            .unwrap()
            .contains("ByteBuffer")
    );
    let plan = consumer.plan(&draft, true);
    assert_eq!(
        compact_field(compact_record(&plan, "result"), "outcome"),
        "unchanged"
    );
    let artifact = consumer.root.path().join("buffer.lkja");
    consumer.cli(&["build", "--output", path(&artifact)], true);
    let deployment = consumer.input(
        "memory.deployment.json",
        &json!({
            "artifact": "buffer.lkja", "target": "memory", "listen": null,
            "http": null, "session": null, "worker": null,
            "streams": {"maximum_chunk_bytes": 65536, "maximum_buffered_chunks": 8,
                "maximum_total_bytes": 1048576, "maximum_live_streams": 1024},
            "grants": [], "secrets": [], "configuration": {}
        })
        .to_string(),
    );
    let arguments = consumer.input("arguments.json", "[]");
    for detached in [false, true] {
        if detached {
            for project in [&producer.project, &transformer.project, &consumer.project] {
                std::fs::remove_dir_all(project).unwrap();
            }
        }
        let output = consumer.root.path().join(format!("result-{detached}.json"));
        let mut args = if detached {
            vec!["run", "--deployment", path(&deployment)]
        } else {
            vec!["run", "memory"]
        };
        args.extend([
            "--arguments-file",
            path(&arguments),
            "--result-file",
            path(&output),
        ]);
        consumer.cli(&args, true);
        assert_eq!(
            serde_json::from_slice::<Value>(&std::fs::read(output).unwrap()).unwrap(),
            json!({"$bytes":"AP+A"})
        );
    }
}
#[test]
fn native_byte_buffer_rejects_closed_modes_and_unrestricted_phantom_without_publication() {
    let public = Native::new();
    let before = public.revision();
    for (index, body) in [
        "(external create get (visibility public) (implementation core.buffer.get) (parameter create index (type I64)) (parameter create b (type ByteBuffer) (use consume)) (returns I64))",
        "(function create unused (visibility private) (type-parameter create T) (returns Unit) (effect pure) (body (unit))) (function create bad (visibility private) (returns Unit) (effect pure) (body (call unused (types ByteBuffer))))",
    ].iter().enumerate() {
        let input = public.input(&format!("bad-{index}.lkjc"),
            &format!("request base={before}\ndeclarations.begin\n(units (module create bad {body}))\ndeclarations.end"));
        let failure = public.plan(&input, false);
        let code = compact_field(compact_record(&failure, "diagnostic"), "code");
        assert!(matches!(code, "intrinsic_signature" | "kernel_buffer_generic" | "kernel_buffer_ownership"), "{code}");
        assert_eq!(public.revision(), before);
    }
}
