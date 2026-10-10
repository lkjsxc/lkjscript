//! Ordinary native edits retain exact incremental caches through a copied executable.

use super::*;

fn source(ports: usize, target: bool) -> String {
    let ports = (0..ports)
        .map(|index| format!("(port create p{index} (type (function () I64)) (function one))"))
        .collect::<Vec<_>>()
        .join("\n");
    let target = if target {
        "(target create extra-main (component extra::console) (runner command) \
         (port extra::console::p0))"
    } else {
        ""
    };
    format!(
        "declarations.begin\n(units \
        (module create extra \
          (function create one (visibility public) (returns I64) \
            (effect pure) (body (i64 1))) \
          (component create console (visibility private) {ports})) \
        {target})\ndeclarations.end\n"
    )
}

fn apply_exact(public: &Native, name: &str, source: &str, units: usize) {
    let input = public.input(
        name,
        &format!("request base={}\n{source}", public.revision()),
    );
    let plan = public.plan(&input, true);
    assert_eq!(
        compact_field(compact_record(&plan, "validation"), "compiler-units"),
        units.to_string()
    );
    let applied = public.apply(&input, &plan, true);
    assert_eq!(compact_field(&applied[0], "status"), "accepted");
    assert_eq!(
        compact_field(compact_record(&applied, "derived-cache"), "status"),
        "updated"
    );
    let checked = public.cli(&["check"], true);
    assert_eq!(
        compact_field(compact_record(&checked, "compilation"), "cache"),
        "exact-current"
    );
    assert_eq!(
        compact_field(compact_record(&checked, "tests"), "failed"),
        "0"
    );
}

#[test]
fn new_components_do_not_promote_ports_to_compiler_units() {
    for ports in [1, 2] {
        let public = Native::template("command");
        let baseline = public.cli(&["check"], true);
        apply_exact(&public, "new-component.lkjc", &source(ports, false), 2);
        let checked = public.cli(&["check"], true);
        assert_eq!(
            compact_field(compact_record(&checked, "tests"), "passed"),
            compact_field(compact_record(&baseline, "tests"), "passed")
        );
    }
}

#[test]
fn new_command_targets_keep_exact_incremental_cache_and_detached_behavior() {
    for ports in [1, 2] {
        let public = Native::template("command");
        public.cli(&["check"], true);
        apply_exact(&public, "new-command.lkjc", &source(ports, true), 3);
        let attached = public.cli(&["run", "extra-main"], true);
        assert_eq!(
            compact_field(compact_record(&attached, "execution"), "value"),
            "1"
        );
        assert_eq!(
            compact_field(compact_record(&attached, "execution"), "differential"),
            "equal"
        );
        let bundle = public.root.path().join("bundle");
        std::fs::create_dir(&bundle).unwrap();
        public.cli(
            &["build", "--output", path(&bundle.join("command.lkja"))],
            true,
        );
        let mut descriptor: Value = serde_json::from_slice(
            &std::fs::read(public.project.join("command.deployment.json")).unwrap(),
        )
        .unwrap();
        descriptor["artifact"] = serde_json::json!("command.lkja");
        descriptor["target"] = serde_json::json!("extra-main");
        let deployment = bundle.join("command.deployment.json");
        std::fs::write(&deployment, serde_json::to_vec(&descriptor).unwrap()).unwrap();
        std::fs::remove_dir_all(&public.project).unwrap();
        let detached = public.cli(&["run", "--deployment", path(&deployment)], true);
        assert_eq!(
            compact_field(compact_record(&detached, "execution"), "value"),
            "1"
        );
    }
}

#[test]
fn new_port_type_failure_preserves_authority_and_cache_then_recovers() {
    let public = Native::template("command");
    public.cli(&["check"], true);
    let before = content_inventory(&public.project);
    let invalid = public.input(
        "invalid-port.lkjc",
        &format!(
            "request base={}\n{}",
            public.revision(),
            source(1, false).replace("(function () I64)", "(function () Bool)"),
        ),
    );
    let rejected = public.plan(&invalid, false);
    assert!(
        rejected
            .iter()
            .any(|record| record.operation == "diagnostic"
                && super::super::compact_field(record, "code")
                    == Some("kernel_type_port_function")),
        "{rejected:?}"
    );
    assert_eq!(content_inventory(&public.project), before);
    apply_exact(&public, "valid-port.lkjc", &source(1, false), 2);
}
