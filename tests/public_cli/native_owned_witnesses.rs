//! Literal native authoring and exact static witness transport without source or grants.
use super::native_byte_buffer::{author, dependency, export, stage};
use super::*;
use serde_json::json;
const LIBRARY: &str = include_str!("../fixtures/owned-witness-library.lkjc");
const CELL: &str = include_str!("../fixtures/owned-witness-cell.lkjc");
const BUFFER: &str = include_str!("../fixtures/owned-witness-buffer.lkjc");
const CONSUMER: &str = include_str!("../fixtures/owned-witness-packages-consumer.lkjc");

fn unchanged(public: &Native, module: &str) -> String {
    let draft = public.root.path().join(format!("{module}-draft.lkjc"));
    public.cli(
        &[
            "change",
            "draft",
            "--module",
            module,
            "--output",
            path(&draft),
        ],
        true,
    );
    let text = std::fs::read_to_string(&draft).unwrap();
    let plan = public.plan(&draft, true);
    assert_eq!(
        compact_field(compact_record(&plan, "result"), "outcome"),
        "unchanged"
    );
    text
}

#[test]
fn native_owned_witnesses_three_packages_and_source_free_exact_selection() {
    let library = Native::new();
    author(&library, LIBRARY);
    let draft = unchanged(&library, "abstraction");
    assert!(draft.contains("(constraint owned)"));
    for forbidden in [
        "ByteBuffer",
        "OwnedI64Cell",
        "builtin",
        "owned-implementation",
    ] {
        assert!(!draft.contains(forbidden), "{forbidden}");
    }
    assert_eq!(
        compact_field(
            compact_record(&library.cli(&["status"], true), "summary"),
            "dependencies"
        ),
        "0"
    );
    let generic = export(&library);
    let implementations = Native::new();
    stage(&implementations, &generic);
    author(
        &implementations,
        &format!(
            "{}declarations.begin\n(units (use abstraction {} {}))\ndeclarations.end\n{CELL}\n{BUFFER}",
            dependency(&generic),
            generic.package,
            generic.revision
        ),
    );
    for module in ["cell", "buffer"] {
        unchanged(&implementations, module);
    }
    let concrete = export(&implementations);
    let consumer = Native::new();
    stage(&consumer, &generic);
    stage(&consumer, &concrete);
    author(
        &consumer,
        &format!(
            "{}{}{}",
            dependency(&generic),
            dependency(&concrete),
            CONSUMER
                .replace("GENERIC_PACKAGE_REVISION", &generic.revision)
                .replace("GENERIC_PACKAGE", &generic.package)
                .replace("IMPLEMENTATION_PACKAGE_REVISION", &concrete.revision)
                .replace("IMPLEMENTATION_PACKAGE", &concrete.package)
        ),
    );
    unchanged(&consumer, "application");
    let artifact = consumer.root.path().join("owned.lkja");
    consumer.cli(&["build", "--output", path(&artifact)], true);
    let deployment = consumer.input(
        "owned.deployment.json",
        &json!({
            "artifact": "owned.lkja", "target": "owned", "listen": null,
            "http": null, "session": null, "worker": null,
            "streams": {"maximum_chunk_bytes": 65536, "maximum_buffered_chunks": 8,
                "maximum_total_bytes": 1048576, "maximum_live_streams": 1024},
            "grants": [], "secrets": [], "configuration": {}
        })
        .to_string(),
    );
    for detached in [false, true] {
        if detached {
            for project in [
                &library.project,
                &implementations.project,
                &consumer.project,
            ] {
                std::fs::remove_dir_all(project).unwrap();
            }
            for pack in [&generic.path, &concrete.path] {
                std::fs::remove_file(pack).unwrap();
            }
        }
        for n in [128, i64::MIN, i64::MAX] {
            let arguments = consumer.input("arguments.json", &json!([n]).to_string());
            let output = consumer
                .root
                .path()
                .join(format!("result-{detached}-{n}.json"));
            let mut args = if detached {
                vec!["run", "--deployment", path(&deployment)]
            } else {
                vec!["run", "owned"]
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
                json!({"length": 3, "scalar": n, "alternate": 99})
            );
        }
    }
}

#[test]
fn native_owned_witness_inspection_and_semantic_rejections_preserve_head() {
    let public = Native::new();
    let before = public.revision();
    const GOOD: &str = include_str!("../fixtures/owned-witness-cell-consumer.lkjc");
    let contract = &LIBRARY[form_span(LIBRARY, "(owned-contract create Storage")];
    let other = format!(
        "declarations.begin\n(units (module create foreign {}))\ndeclarations.end",
        contract.replace("create Storage", "create Different")
    );
    let cases = [
        (
            "missing",
            GOOD.replacen(
                "(implementations concrete@cell::Scalar)",
                "(implementations)",
                1,
            ),
            CELL.to_owned(),
        ),
        (
            "wrong-self",
            GOOD.replacen("(types OwnedI64Cell)", "(types ByteBuffer)", 1),
            CELL.to_owned(),
        ),
        (
            "wrong-contract",
            GOOD.to_owned(),
            CELL.replacen(
                "(contract abstraction::Storage)",
                "(contract foreign::Different)",
                1,
            ),
        ),
        (
            "missing-method",
            GOOD.to_owned(),
            CELL.replacen(
                "(method method_10000000000000000000000000000002 update)",
                "",
                1,
            ),
        ),
        (
            "unused-wrong-map",
            GOOD.to_owned(),
            CELL.replacen(
                "(method method_10000000000000000000000000000002 update)",
                "(method method_10000000000000000000000000000002 read)",
                1,
            ),
        ),
        (
            "wrong-scope",
            GOOD.replacen(
                "concrete@cell::Scalar",
                "parameter@abstraction::produce@implparam_10000000000000000000000000000001",
                1,
            ),
            CELL.to_owned(),
        ),
        (
            "missing-witness-id",
            GOOD.replacen(
                "concrete@cell::Scalar",
                "parameter@main@implparam_99999999999999999999999999999999",
                1,
            ),
            CELL.to_owned(),
        ),
        (
            "task-method",
            GOOD.to_owned(),
            CELL.replacen(
                "(returns I64) (effect pure) (body (i64 99))",
                "(returns I64) (effect (task)) (body (i64 99))",
                1,
            ),
        ),
    ];
    for (name, consumer, implementation) in cases {
        let input = public.input(
            &format!("invalid-{name}.lkjc"),
            &format!("request base={before}\n{LIBRARY}\n{other}\n{implementation}\n{consumer}"),
        );
        let failed = public.plan(&input, false);
        let code = compact_field(compact_record(&failed, "diagnostic"), "code");
        assert!(
            code.starts_with("kernel_owned")
                || code.starts_with("kernel_buffer")
                || code == "kernel_type_pure_task_call",
            "{name} must fail semantic admission, not parsing: {code}"
        );
        assert_eq!(public.revision(), before);
    }
    author(&public, &format!("{LIBRARY}\n{CELL}\n{GOOD}"));
    for (name, kind, record, count) in [
        ("Storage", "owned_contract", "owned.method", 3),
        (
            "Alternate",
            "owned_implementation",
            "owned.method-implementation",
            3,
        ),
        (
            "produce",
            "pure_function",
            "owned.implementation-parameter",
            1,
        ),
    ] {
        let module = public.cli(
            &[
                "query",
                "find",
                "module",
                if name == "Alternate" {
                    "cell"
                } else {
                    "abstraction"
                },
            ],
            true,
        );
        let found = public.cli(
            &[
                "query",
                "find",
                "declaration",
                name,
                "--parent",
                compact_field(compact_record(&module, "owner"), "id"),
            ],
            true,
        );
        let id = compact_field(compact_record(&found, "owner"), "id");
        let records = public.cli(&["inspect", "owner", kind, id], true);
        assert_eq!(
            records.iter().filter(|r| r.operation == record).count(),
            count,
            "{name}"
        );
        if name == "produce" {
            let detail = public.cli(
                &[
                    "inspect",
                    "owner",
                    kind,
                    id,
                    "--detail",
                    "definition",
                    "--limit",
                    "100",
                ],
                true,
            );
            assert_eq!(
                detail
                    .iter()
                    .filter(|r| r.operation == "definition.implementation-parameter")
                    .count(),
                1
            );
        }
    }
}

#[test]
fn native_owned_exports_require_visible_method_and_contract_signatures() {
    for (from, to) in [
        (
            "(function create read (visibility public)",
            "(function create read (visibility private)",
        ),
        (
            "(owned-contract create Storage (visibility public)",
            "(owned-contract create Storage (visibility private)",
        ),
    ] {
        let public = Native::new();
        let source = format!("{LIBRARY}\n{CELL}").replace(from, to);
        assert!(source.contains(to));
        let input = public.input(
            "private-signature.lkjc",
            &format!("request base={}\n{source}", public.revision()),
        );
        let plan = public.plan(&input, true);
        public.apply(&input, &plan, true);
        let head = public.revision();
        let output = public.root.path().join("private-signature.lkjp");
        let failure = public.cli(
            &[
                "package",
                "current",
                "export",
                "--kind",
                "transport",
                "--output",
                path(&output),
            ],
            false,
        );
        assert_eq!(
            compact_field(compact_record(&failure, "diagnostic"), "code"),
            "package_transport_interface_owner_missing",
            "{failure:?}"
        );
        assert!(!output.exists());
        assert_eq!(public.revision(), head);
    }
}

#[test]
fn native_owned_recursive_signatures_and_witness_composite_closure_execute() {
    let public = Native::new();
    author(&public, &[
        LIBRARY, CELL,
        include_str!("../fixtures/owned-witness-composite.lkjc"),
        include_str!("../fixtures/owned-witness-recursive-data.lkjc"),
        r#"declarations.begin
(units (module create closure-commands
  (component create commands (visibility private)
    (port create composite (type (function () I64)) (function composite::composite-main))
    (port create recursive (type (function () I64)) (function recursive-data::recursive-data-main))))
  (target create composite (component closure-commands::commands) (runner command) (port closure-commands::commands::composite))
  (target create recursive (component closure-commands::commands) (runner command) (port closure-commands::commands::recursive)))
declarations.end"#,
    ].join("\n"));
    unchanged(&public, "recursive-data");
    unchanged(&public, "composite");
    for (target, expected) in [("composite", 7), ("recursive", 128)] {
        let result = public.root.path().join(format!("{target}.json"));
        public.cli(&["run", target, "--result-file", path(&result)], true);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&std::fs::read(result).unwrap()).unwrap(),
            json!(expected)
        );
    }
}
