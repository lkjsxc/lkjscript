//! Public-only generic outcomes, exact witnesses, canonical editing and source-free use.
use super::*;

#[test]
fn native_owned_choices_four_packages_exact_witnesses_and_detached_execution() {
    let library = Native::new();
    author(
        &library,
        &[
            include_str!("../fixtures/owned-witness-library.lkjc"),
            include_str!("../fixtures/owned-choices.lkjc"),
            include_str!("../fixtures/owned-choices-outcome.lkjc"),
            include_str!("../fixtures/owned-choices-witness.lkjc"),
        ]
        .join("\n"),
    );
    for module in ["abstraction", "choices", "outcomes", "witness"] {
        unchanged(&library, module);
    }
    library.cli(&["check"], true);
    let p = export(&library);
    let carrier = Native::new();
    stage(&carrier, &p);
    author(
        &carrier,
        &format!(
            "{}declarations.begin\n(units (use abstraction {} {}))\ndeclarations.end\n{}",
            dependency(&p),
            p.package,
            p.revision,
            include_str!("../fixtures/owned-witness-cell.lkjc"),
        ),
    );
    let c = export(&carrier);
    let octets = Native::new();
    stage(&octets, &p);
    author(
        &octets,
        &format!(
            "{}declarations.begin\n(units (use abstraction {} {}))\ndeclarations.end\n{}",
            dependency(&p),
            p.package,
            p.revision,
            include_str!("../fixtures/owned-witness-buffer.lkjc"),
        ),
    );
    let b = export(&octets);
    let consumer = Native::new();
    stage(&consumer, &p);
    stage(&consumer, &c);
    stage(&consumer, &b);
    let input = format!(
        "{}{}{}declarations.begin\n(units (use abstraction {} {}) (use choices {} {}) (use outcomes {} {}) (use witness {} {}) (use cell {} {}) (use buffer {} {}))\ndeclarations.end\n{}",
        dependency(&p),
        dependency(&c),
        dependency(&b),
        p.package,
        p.revision,
        p.package,
        p.revision,
        p.package,
        p.revision,
        p.package,
        p.revision,
        c.package,
        c.revision,
        b.package,
        b.revision,
        include_str!("../fixtures/owned-choices-consumer.lkjc"),
    );
    let before = consumer.revision();
    let bad = consumer.input(
        "wrong-choice-witness.lkjc",
        &format!(
            "request base={before}\n{}",
            input.replacen(
                "(implementations concrete@cell::Scalar)",
                "(implementations concrete@buffer::Octets)",
                1
            ),
        ),
    );
    let rejected = consumer.plan(&bad, false);
    assert!(
        rejected
            .iter()
            .any(|r| r.operation == "diagnostic"
                && compact_field(r, "code") == "kernel_owned_contract"),
        "{rejected:?}"
    );
    assert_eq!(consumer.revision(), before);
    author(&consumer, &input);
    consumer.cli(&["check"], true);
    let draft = unchanged(&consumer, "choice-consumer");
    let original = std::fs::read_to_string(&draft).unwrap();
    assert!(original.contains("(choose-owned"));
    assert!(original.contains("(match-owned"));
    let before = consumer.revision();
    let edited = consumer.input(
        "choice-literal-edit.lkjc",
        &original.replace("(i64 41)", "(i64 42)"),
    );
    let plan = consumer.plan(&edited, true);
    consumer.apply(&edited, &plan, true);
    assert_ne!(consumer.revision(), before);
    let after = unchanged(&consumer, "choice-consumer");
    assert_eq!(
        std::fs::read_to_string(after).unwrap(),
        original
            .replacen(&before, &consumer.revision(), 1)
            .replace("(i64 41)", "(i64 42)"),
        "a literal edit must retain all declaration, parameter and branch-local identities"
    );
    consumer.cli(&["check"], true);
    let artifact = consumer.root.path().join("choices.lkja");
    consumer.cli(&["build", "--output", path(&artifact)], true);
    let deployment = consumer.input(
        "choices.deployment.json",
        &json!({
            "artifact": "choices.lkja", "target": "owned-outcomes", "listen": null,
            "http": null, "session": null, "worker": null,
            "streams": {"maximum_chunk_bytes":65536, "maximum_buffered_chunks":8,
                "maximum_total_bytes":1048576, "maximum_live_streams":1024},
            "grants": [], "secrets": [], "configuration": {}
        })
        .to_string(),
    );
    for detached in [false, true] {
        if detached {
            for project in [
                &library.project,
                &carrier.project,
                &octets.project,
                &consumer.project,
            ] {
                std::fs::remove_dir_all(project).unwrap();
            }
            for package in [&p.path, &c.path, &b.path] {
                std::fs::remove_file(package).unwrap();
            }
        }
        for n in [i64::MIN, -257, i64::MAX] {
            for accepted in [false, true] {
                let arguments = consumer.input(
                    &format!("choice-{detached}-{n}-{accepted}.json"),
                    &json!([n, accepted]).to_string(),
                );
                let output = consumer
                    .root
                    .path()
                    .join(format!("choice-result-{detached}-{n}-{accepted}.json"));
                let mut args = if detached {
                    vec!["run", "--deployment", path(&deployment)]
                } else {
                    vec!["run", "owned-outcomes"]
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
                    json!({"value":n, "alternate":if accepted {99} else {n},
                        "bytes":1, "nested":n, "marker":42,
                        "witness":{"scalar":n, "alternate":99, "skipped":7}})
                );
            }
        }
    }
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        for public in [library, carrier, octets, consumer] {
            println!(
                "retained owned-choice public evidence: {}",
                public.root.keep().display()
            );
        }
    }
}
