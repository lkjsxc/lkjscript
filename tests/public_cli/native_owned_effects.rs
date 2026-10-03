//! Literal owned generic tasks with caller-bound effects, requirements and exact witnesses.
use super::native_byte_buffer::{Export, author, dependency, export, stage};
use super::*;
use serde_json::json;

const LIBRARY: &str = include_str!("../../examples/owned-effects/library.lkjc");
const CARRIERS: &str = include_str!("../../examples/owned-effects/carriers.lkjc");
const APPLICATION: &str = include_str!("../../examples/owned-effects/application.lkjc");

struct Packages {
    library: Native,
    carriers: Native,
    consumer: Native,
    generic: Export,
    concrete: Export,
}

impl Packages {
    fn stage() -> Self {
        let library = Native::template("command");
        author(&library, LIBRARY);
        let draft = unchanged(&library, "owned-effects");
        for absent in ["OwnedI64Cell", "ByteBuffer", "owned-implementation"] {
            assert!(!draft.contains(absent), "independent library: {absent}");
        }
        assert!(draft.contains("(constraint owned)"));
        assert!(draft.contains("(effects (row (parameter"));
        assert!(draft.contains("(requirements "));
        // The library is admitted and exported before concrete packages exist.
        let generic = export(&library);
        let carriers = Native::template("command");
        stage(&carriers, &generic);
        author(
            &carriers,
            &format!(
                "{}declarations.begin\n(units (use owned-effects {} {}))\ndeclarations.end\n{CARRIERS}",
                dependency(&generic),
                generic.package,
                generic.revision,
            ),
        );
        unchanged(&carriers, "owned-effect-carriers");
        let concrete = export(&carriers);
        let consumer = Native::template("command");
        stage(&consumer, &generic);
        stage(&consumer, &concrete);
        Self {
            library,
            carriers,
            consumer,
            generic,
            concrete,
        }
    }

    fn source(&self, body: &str) -> String {
        format!(
            "{}{}declarations.begin\n(units (use owned-effects {} {}) (use owned-effect-carriers {} {}))\ndeclarations.end\n{body}",
            dependency(&self.generic),
            dependency(&self.concrete),
            self.generic.package,
            self.generic.revision,
            self.concrete.package,
            self.concrete.revision,
        )
    }

    fn accept(&self) {
        author(&self.consumer, &self.source(APPLICATION));
        unchanged(&self.consumer, "owned-effects-app");
    }

    fn detach(&self) {
        for project in [
            &self.library.project,
            &self.carriers.project,
            &self.consumer.project,
        ] {
            std::fs::remove_dir_all(project).unwrap();
        }
        for package in [&self.generic.path, &self.concrete.path] {
            std::fs::remove_file(package).unwrap();
        }
    }
}

fn unchanged(public: &Native, module: &str) -> String {
    let directory = tempfile::Builder::new()
        .prefix(&format!("{module}-{}-draft-", public.revision()))
        .tempdir_in(public.root.path())
        .unwrap()
        .keep();
    let draft = directory.join("draft.lkjc");
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
    let plan = public.plan(&draft, true);
    assert_eq!(
        compact_field(compact_record(&plan, "result"), "outcome"),
        "unchanged"
    );
    std::fs::read_to_string(draft).unwrap()
}

fn descriptor() -> Value {
    serde_json::from_str(include_str!(
        "../../examples/owned-effects/owned-effects.deployment.json"
    ))
    .unwrap()
}

#[test]
fn native_owned_effects_three_packages_keep_exact_authority_and_source_free_results() {
    let packages = Packages::stage();
    packages.accept();
    let public = &packages.consumer;
    let artifact = public.root.path().join("owned-effects.lkja");
    public.cli(&["build", "--output", path(&artifact)], true);
    let descriptor = descriptor();
    let deployment = public.input("owned-effects.deployment.json", &descriptor.to_string());
    let mut rejected = Vec::new();
    for requirement in ["clock-a", "clock-b"] {
        let mut missing = descriptor.clone();
        missing["grants"]
            .as_array_mut()
            .unwrap()
            .retain(|grant| grant["requirement"] != requirement);
        rejected.push((
            format!("missing-{requirement}"),
            missing,
            "deployment_grant_missing",
        ));
    }
    let mut wrong = descriptor.clone();
    wrong["grants"][1]["adapter"] = json!({"kind":"secure_random"});
    rejected.push((
        "wrong-adapter".into(),
        wrong,
        "normalized_deployment_adapter_interface",
    ));
    for detached in [false, true] {
        if detached {
            packages.detach();
        }
        for (name, invalid, code) in &rejected {
            let denied = public.input(&format!("{name}-{detached}.json"), &invalid.to_string());
            let output = public
                .root
                .path()
                .join(format!("denied-{name}-{detached}.json"));
            let records = public.cli(
                &[
                    "run",
                    "--deployment",
                    path(&denied),
                    "--result-file",
                    path(&output),
                ],
                false,
            );
            assert!(
                records
                    .iter()
                    .any(|r| r.operation == "diagnostic" && compact_field(r, "code") == *code),
                "{name}: {records:?}"
            );
            assert!(!output.exists());
            assert!(!records.iter().any(|r| r.operation == "execution"));
        }
        if !detached {
            continue;
        }
        let output = public.root.path().join(format!("result-{detached}.json"));
        let records = public.cli(
            &[
                "run",
                "--deployment",
                path(&deployment),
                "--result-file",
                path(&output),
            ],
            true,
        );
        let observation: Value = serde_json::from_str(compact_field(
            compact_record(&records, "execution"),
            "production-observation",
        ))
        .unwrap();
        assert_eq!(observation["capability_calls"], json!(8));
        assert_eq!(
            serde_json::from_slice::<Value>(&std::fs::read(output).unwrap()).unwrap(),
            json!({"scalar":43,"rebound":43,"alternate":100,"bytes":{"$bytes":"SQI="}})
        );
    }
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        for public in [packages.library, packages.carriers, packages.consumer] {
            println!(
                "retained owned-effects public evidence: {}",
                public.root.keep().display()
            );
        }
    }
}

#[test]
fn native_owned_effects_reject_bad_operands_then_preserve_identity_through_reviewed_edit() {
    let packages = Packages::stage();
    let public = &packages.consumer;
    let before = public.revision();
    let first = form_span(APPLICATION, "(implementation-call owned-effects::transform");
    let call = &APPLICATION[first.clone()];
    let bad_branch = call.replacen(
        "concrete@owned-effect-carriers::Scalar",
        "concrete@owned-effect-carriers::Octets",
        1,
    );
    let untaken = APPLICATION.replacen(call, &format!("(if (bool false) {bad_branch} {call})"), 1);
    let foreign = APPLICATION
        .replacen("(component create command (visibility private)",
            "(component create command (visibility private)\n      (requirement create foreign (interface std::WallClock) (operations std::WallClock::utc-milliseconds) (limits (maximum_calls 1 calls)))", 1)
        .replacen("(requirements command::clock-a)", "(requirements command::foreign)", 1);
    for (name, invalid) in [
        (
            "missing-effect",
            APPLICATION.replacen("(effects (row (requirement command::clock-b)))", "", 1),
        ),
        (
            "missing-requirement",
            APPLICATION.replacen("(requirements command::clock-a)", "", 1),
        ),
        (
            "wrong-witness",
            APPLICATION.replacen(
                "concrete@owned-effect-carriers::Scalar",
                "concrete@owned-effect-carriers::Octets",
                1,
            ),
        ),
        (
            "callback-row",
            APPLICATION.replacen(
                "(effects (row (requirement command::clock-b)))",
                "(effects (row (requirement command::clock-a)))",
                1,
            ),
        ),
        (
            "insufficient-row",
            APPLICATION.replacen(
                "(effect (task (requirement command::clock-a) (requirement command::clock-b)))",
                "(effect (task (requirement command::clock-a)))",
                1,
            ),
        ),
        ("foreign-authority", foreign),
        ("untaken-wrong-witness", untaken),
    ] {
        let input = public.input(
            &format!("{name}.lkjc"),
            &format!("request base={before}\n{}", packages.source(&invalid)),
        );
        let rejected = public.plan(&input, false);
        assert!(
            rejected
                .iter()
                .any(|r| r.operation == "diagnostic"
                    && compact_field(r, "code").starts_with("kernel_")),
            "{name} must fail semantic admission: {rejected:?}"
        );
        assert_eq!(public.revision(), before, "{name}");
    }
    packages.accept();
    let accepted = public.revision();
    let original = unchanged(public, "owned-effects-app");
    assert!(original.contains("(i64 42)"));
    let edited = public.input(
        "reviewed-edit.lkjc",
        &original.replacen("(i64 42)", "(i64 41)", 1),
    );
    let plan = public.plan(&edited, true);
    public.apply(&edited, &plan, true);
    public.cli(&["check"], true);
    let after = unchanged(public, "owned-effects-app");
    assert_eq!(
        after,
        original
            .replacen(&accepted, &public.revision(), 1)
            .replacen("(i64 42)", "(i64 41)", 1),
        "reviewed edit must retain every declaration, scheme, witness and binding identity"
    );
    if std::env::var_os("LKJSCRIPT_RETAIN_PRODUCT_EVIDENCE").is_some() {
        for public in [packages.library, packages.carriers, packages.consumer] {
            println!(
                "retained owned-effects reviewed-edit evidence: {}",
                public.root.keep().display()
            );
        }
    }
}
