//! Bounded native form encoding workflows; expected bytes do not call the native codec.
use super::super::native_http as http;
use super::*;

type Pairs = Vec<(String, String)>;

fn pairs(values: &[(&str, &str)]) -> Pairs {
    values
        .iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

fn fields(values: &Pairs) -> Value {
    json!(
        values
            .iter()
            .map(|(name, value)| json!({"name":name,"value":value}))
            .collect::<Vec<_>>()
    )
}

fn canonical(values: &Pairs) -> String {
    fn component(value: &str) -> String {
        let mut output = String::new();
        for byte in value.bytes() {
            match byte {
                b' ' => output.push('+'),
                b'*' | b'-' | b'.' | b'_' => output.push(char::from(byte)),
                byte if byte.is_ascii_alphanumeric() => output.push(char::from(byte)),
                byte => output.push_str(&format!("%{byte:02X}")),
            }
        }
        output
    }
    values
        .iter()
        .map(|(name, value)| format!("{}={}", component(name), component(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn add_case(
    cases: &mut Vec<Value>,
    expected: &mut Vec<Value>,
    values: &Pairs,
    maximum_bytes: i64,
    maximum_fields: i64,
) {
    let encoded = canonical(values);
    let error = if maximum_bytes < 0 || maximum_fields < 0 {
        "invalid limits"
    } else if i64::try_from(values.len()).unwrap() > maximum_fields {
        "too many fields"
    } else if i64::try_from(encoded.len()).unwrap() > maximum_bytes {
        "body too large"
    } else {
        ""
    };
    cases.push(json!({"fields":fields(values),"maximum-bytes":maximum_bytes,"maximum-fields":maximum_fields}));
    expected.push(json!({"valid":error.is_empty(),"body":bytes(if error.is_empty() { encoded.as_bytes() } else { b"" }),"error":error}));
}

fn command(public: &Native, target: &str, arguments: Value, detached: bool) -> Value {
    let input = public.input("encoding-arguments.json", &arguments.to_string());
    let result = public.root.path().join("encoding-result.json");
    let deployment = public.input("encoding.deployment.json", &json!({
        "artifact":"forms.lkja","target":target,"listen":null,"http":null,"session":null,"worker":null,
        "streams":{"maximum_chunk_bytes":65536,"maximum_buffered_chunks":8,"maximum_total_bytes":1048576,"maximum_live_streams":1024},
        "grants":[],"secrets":[],"configuration":{}
    }).to_string());
    let mut route = if detached {
        vec!["run", "--deployment", path(&deployment)]
    } else {
        vec!["run", target]
    };
    route.extend([
        "--arguments-file",
        path(&input),
        "--result-file",
        path(&result),
    ]);
    let records = public.cli(&route, true);
    let execution = compact_record(&records, "execution");
    if detached {
        let cleanup: Value = serde_json::from_str(compact_field(execution, "cleanup")).unwrap();
        assert_eq!(cleanup["remaining_tasks"], 0);
        assert_eq!(cleanup["cleanup_failures"], json!([]));
    } else {
        assert_eq!(compact_field(execution, "differential"), "equal");
    }
    let value = serde_json::from_slice(&std::fs::read(&result).unwrap()).unwrap();
    std::fs::remove_file(result).unwrap();
    value
}

#[test]
fn native_forms_encoding_matches_independent_bytes_limits_and_detached_roundtrips() {
    let library = author_library();
    let public = import_consumer(&library);
    public.cli(
        &[
            "build",
            "--output",
            path(&public.root.path().join("forms.lkja")),
        ],
        true,
    );
    let mut samples: Vec<Pairs> = (0..=127u8)
        .map(|byte| {
            vec![(
                char::from(byte).to_string(),
                format!(" +&{}", char::from(byte)),
            )]
        })
        .collect();
    for scalar in [
        0x80, 0x7ff, 0x800, 0xd7ff, 0xe000, 0xfeff, 0xffff, 0x10000, 0x10ffff,
    ] {
        let text = char::from_u32(scalar).unwrap().to_string();
        samples.push(vec![(text.clone(), text)]);
    }
    let texts = [
        "",
        "a b",
        "&=+%",
        "猫",
        "😀",
        "\0\r\n",
        "e\u{301}",
        "\u{feff}name",
    ];
    for name in texts {
        for value in texts {
            samples.push(pairs(&[(name, value)]));
        }
    }
    samples.extend([
        vec![],
        pairs(&[("", ""), ("", "")]),
        pairs(&[
            ("a", "first"),
            ("a", "last"),
            ("", ""),
            ("name", "日本語 +%&=\n"),
        ]),
        pairs(&[("_charset_", "unchanged"), ("line", "\n\r\r\n")]),
    ]);
    let mut cases = Vec::new();
    let mut expected = Vec::new();
    for values in &samples {
        let length = i64::try_from(canonical(values).len()).unwrap();
        let count = i64::try_from(values.len()).unwrap();
        if length > 0 {
            add_case(&mut cases, &mut expected, values, length - 1, count);
        }
        add_case(&mut cases, &mut expected, values, length, count);
        add_case(&mut cases, &mut expected, values, i64::MAX, i64::MAX);
    }
    let duplicate = pairs(&[("a", "first"), ("a", "last")]);
    for maximum_bytes in [i64::MIN, -1, 0, 13, 14, i64::MAX] {
        for maximum_fields in [i64::MIN, -1, 0, 1, 2, i64::MAX] {
            add_case(
                &mut cases,
                &mut expected,
                &duplicate,
                maximum_bytes,
                maximum_fields,
            );
        }
    }
    for count in [0, 1, 64, 65] {
        let values = vec![(String::new(), String::new()); count];
        for limit in [0, 127, 129] {
            add_case(&mut cases, &mut expected, &values, limit, 64);
        }
    }
    for value in [
        "x".repeat(8190),
        "~".repeat(2730),
        "猫".repeat(910),
        "x".repeat(500_000),
    ] {
        let values = vec![("a".to_owned(), value)];
        for limit in [0, 8191, 8192] {
            add_case(&mut cases, &mut expected, &values, limit, 1);
        }
    }
    let producer = library.project.clone();
    drop(library);
    assert!(!producer.exists());
    let accepted = public.revision();
    for detached in [false, true] {
        if detached {
            drop_source(&public);
        }
        for (inputs, outputs) in cases.chunks(16).zip(expected.chunks(16)) {
            // Large cases run singly under the unchanged command collection quota.
            // Do not confuse a batch's cumulative resource failure with invalid form data.
            let width = if serde_json::to_vec(inputs).unwrap().len() > 4096 {
                1
            } else {
                16
            };
            for (inputs, outputs) in inputs.chunks(width).zip(outputs.chunks(width)) {
                let actual = command(&public, "encode-batch", json!([inputs]), detached);
                assert_eq!(actual, json!(outputs));
            }
        }
        if !detached {
            assert_eq!(public.revision(), accepted);
        }
    }
    // Exact bytes were checked independently first: mutually wrong codecs cannot pass by roundtrip alone.
    for values in samples.chunks(16) {
        let inputs: Vec<_> = values
            .iter()
            .map(|value| json!({"fields":fields(value),"maximum-bytes":8192,"maximum-fields":64}))
            .collect();
        let encoded = command(&public, "encode-batch", json!([inputs]), true);
        let bodies: Vec<_> = encoded
            .as_array()
            .unwrap()
            .iter()
            .map(|item| {
                assert_eq!(item["valid"], true);
                item["body"].clone()
            })
            .collect();
        let decoded = command(&public, "batch", json!([bodies]), true);
        let reports: Vec<_> = values
            .iter()
            .map(|value| json!({"valid":true,"fields":fields(value),"error":""}))
            .collect();
        assert_eq!(decoded, json!(reports));
    }
    eprintln!(
        "native form encoding: {} independent cases through project and detached paths; {} detached roundtrips",
        cases.len(),
        samples.len()
    );
}

fn drop_source(public: &Native) {
    std::fs::remove_dir_all(&public.project).unwrap();
    std::fs::remove_file(public.root.path().join("consumer.lkjc")).unwrap();
}

fn receiver(consumer: &Native) -> (Native, Value) {
    let transport = consumer.root.path().join("consumer.lkjp");
    let export = consumer.cli(
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
    let package = compact_record(&export, "package");
    let public = Native::template("http");
    public.cli(
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
    let source = include_str!("../../docs/guides/examples/form-http.lkjc")
        .replace("SITE_BASE", &public.revision())
        .replace(
            "CONSUMER_PACKAGE_REVISION",
            compact_field(package, "package-revision"),
        )
        .replace("CONSUMER_REVISION", compact_field(package, "revision"))
        .replace("CONSUMER_PACKAGE", compact_field(package, "id"));
    let input = public.input("receiver.lkjc", &source);
    public.apply(&input, &public.plan(&input, true), true);
    let checked = public.cli(&["check"], true);
    assert_eq!(
        compact_field(compact_record(&checked, "tests"), "passed"),
        "129"
    );
    public.cli(
        &[
            "build",
            "--output",
            path(&public.root.path().join("receiver.lkja")),
        ],
        true,
    );
    let mut descriptor: Value = serde_json::from_slice(
        &std::fs::read(public.project.join("service.deployment.json")).unwrap(),
    )
    .unwrap();
    descriptor["artifact"] = json!("receiver.lkja");
    descriptor["target"] = json!("form-http");
    descriptor["http"]["maximum_request_body_bytes"] = json!(8192);
    std::fs::remove_dir_all(&public.project).unwrap();
    std::fs::remove_file(input).unwrap();
    (public, descriptor)
}

#[test]
fn native_forms_encoding_posts_exact_native_bodies_to_an_independent_detached_receiver() {
    let library = author_library();
    let encoder = import_consumer(&library);
    let (receiver, descriptor) = receiver(&encoder);
    encoder.cli(
        &[
            "build",
            "--output",
            path(&encoder.root.path().join("forms.lkja")),
        ],
        true,
    );
    drop(library);
    drop_source(&encoder);
    let values = pairs(&[
        ("a", "first +%"),
        ("a", "last&=猫"),
        ("", ""),
        ("line", "\n\0\r\n<script>"),
    ]);
    let report = command(&encoder, "encode", json!([fields(&values), 8192, 64]), true);
    assert_eq!(report["valid"], true);
    let body = base64::engine::general_purpose::STANDARD
        .decode(report["body"]["$bytes"].as_str().unwrap())
        .unwrap();
    assert_eq!(body, canonical(&values).as_bytes());
    let body = String::from_utf8(body).unwrap();
    let expected = json!({"valid":true,"fields":fields(&values),"error":""});
    for lifecycle in ["first", "restart"] {
        let server = http::Server::start(&receiver, lifecycle, &descriptor, &[]);
        std::thread::scope(|scope| {
            let mut clients = Vec::new();
            for _ in 0..4 {
                clients.push(scope.spawn(|| {
                    http::send(
                        server.address,
                        "POST",
                        "/decode?ignored=not-the-body",
                        &[
                            ("Host", "localhost"),
                            ("Content-Type", "application/x-www-form-urlencoded"),
                        ],
                        &body,
                    )
                }));
            }
            for client in clients {
                let response = client.join().unwrap();
                assert_eq!(response.status, 200);
                assert_eq!(
                    serde_json::from_str::<Value>(&response.body).unwrap(),
                    expected
                );
            }
        });
        let malformed = http::send(
            server.address,
            "POST",
            "/decode",
            &[
                ("Host", "localhost"),
                ("Content-Type", "application/x-www-form-urlencoded"),
            ],
            "ok=one&bad=%FF",
        );
        assert_eq!(malformed.status, 400);
        assert_eq!(
            serde_json::from_str::<Value>(&malformed.body).unwrap(),
            json!({"valid":false,"fields":[],"error":"invalid UTF-8"})
        );
        let foreign = http::send(
            server.address,
            "POST",
            "/decode",
            &[("Host", "localhost"), ("Content-Type", "application/json")],
            &body,
        );
        assert_eq!(foreign.status, 415);
        server.stop();
    }
}

#[test]
fn native_forms_encoding_preserves_review_binding_and_native_tests_detect_mutation() {
    let public = Native::template("command");
    let before = public.revision();
    let source =
        include_str!("../../docs/guides/examples/form-codec.lkjc").replace("LIBRARY_BASE", &before);
    for (name, original, replacement) in [
        (
            "wrong-payload",
            "(case create valid (payload Bytes))",
            "(case create valid (payload I64))",
        ),
        (
            "wrong-octet",
            "(call unescaped (local octet))",
            "(call unescaped (text \"x\"))",
        ),
    ] {
        assert_eq!(source.matches(original).count(), 1);
        let input = public.input(
            &format!("{name}.lkjc"),
            &source.replace(original, replacement),
        );
        let rejected = public.plan(&input, false);
        assert!(
            compact_field(compact_record(&rejected, "diagnostic"), "code")
                .starts_with("kernel_type_")
        );
        assert_eq!(public.revision(), before);
    }
    let original = public.input("original.lkjc", &source);
    let plan = public.plan(&original, true);
    let mutation = "(if (call std::i64-equal (local octet) (i64 32)) (i64 43) (local octet))";
    assert_eq!(source.matches(mutation).count(), 1);
    let altered = public.input("mutated.lkjc", &source.replace(mutation, "(local octet)"));
    public.apply(&altered, &plan, false);
    assert_eq!(public.revision(), before);
    // An independently reviewed mutation is well typed, but must fail the literal native expectations.
    public.apply(&altered, &public.plan(&altered, true), true);
    let accepted = public.revision();
    let rejected = public.cli(&["check"], false);
    assert_eq!(
        compact_field(compact_record(&rejected, "diagnostic"), "code"),
        "normalized_test_failed"
    );
    assert_eq!(public.revision(), accepted);
}
