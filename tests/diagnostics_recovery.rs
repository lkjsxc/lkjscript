use lkjscript::{
    compiler,
    diagnostics::{self, SemanticReport},
    source::{self, SourceBundle, SourceInput},
};

fn analyze(name: &str) -> SemanticReport {
    let path = format!("tests/diagnostics/recovery/{name}.rs");
    let input = SourceBundle {
        root: path.clone(),
        library: None,
        files: vec![SourceInput {
            text: std::fs::read_to_string(&path).unwrap(),
            path,
        }],
    };
    let parsed = source::parse_bundle_report(&input);
    let report = diagnostics::semantic_report(&parsed);
    assert!(
        compiler::compile(&input).is_err(),
        "diagnostic recovery authorized {name}"
    );
    report
}
fn observed(report: &SemanticReport, function: &str, code: &str) -> bool {
    report.observations.iter().any(|observation| {
        observation.function.as_deref() == Some(function) && observation.problem.code == code
    })
}
fn method(report: &SemanticReport, receiver: &str, name: &str) -> bool {
    report.observations.iter().any(|observation| {
        observation.receiver.as_deref() == Some(receiver)
            && observation.member.as_deref() == Some(name)
    })
}
fn blocked(report: &SemanticReport, function: &str, phase: &str) -> bool {
    report.blocked.iter().any(|observation| {
        observation.function.as_deref() == Some(function) && observation.phase == phase
    })
}
#[test]
fn unresolved_signature_keeps_independent_types_methods_and_ownership_visible() {
    let report = analyze("independent_body");
    assert!(observed(&report, "wrong_type", "E_TYPE"));
    assert!(method(&report, "i64", "absent_method"));
    assert!(
        report
            .observations
            .iter()
            .any(|observation| observation.problem.code == "E_BORROW"
                && observation.problem.column > 1
                && observation.problem.message.contains("values")
                && observation.problem.message.contains("created at 7:16"))
    );
    assert!(blocked(&report, "unavailable", "source types"));
}
#[test]
fn unavailable_callee_is_not_assigned_an_invented_result_type() {
    let report = analyze("signature_dependency");
    assert!(blocked(&report, "unavailable", "source types"));
    assert!(blocked(&report, "caller", "source types"));
    assert!(!observed(&report, "caller", "E_TYPE"));
    assert!(method(&report, "i64", "absent_method"));
}
#[test]
fn uncertainty_is_indexed_by_method_name_and_recovered_owner() {
    let report = analyze("uncertain_method_domain");
    assert!(blocked(&report, "uncertain", "method lookup"));
    assert!(!method(&report, "i64", "maybe"));
    assert!(method(&report, "i64", "unrelated"));
    let report = analyze("failed_method_owner");
    assert!(blocked(&report, "affected", "method lookup"));
    assert!(!method(&report, "Opaque", "unknown"));
    assert!(observed(&report, "independent", "E_TYPE"));
}
#[test]
fn import_uncertainty_preserves_scope_strong_names_and_generic_parameters() {
    let report = analyze("uncertain_glob_scope");
    assert!(blocked(&report, "uncertain::unavailable", "source types"));
    assert!(method(&report, "i64", "unrelated"));
    let report = analyze("generic_shadow");
    assert!(observed(&report, "generic", "E_TYPE"));
    assert!(observed(&report, "nominal", "E_TYPE"));
    assert!(method(&report, "i64", "absent"));
}
#[test]
fn known_fields_are_analyzed_without_completing_an_unavailable_layout() {
    let report = analyze("known_field");
    assert!(observed(&report, "type_error", "E_TYPE"));
    assert!(method(&report, "i64", "absent"));
    assert!(
        report
            .blocked
            .iter()
            .any(
                |observation| observation.function.as_deref() == Some("method_error")
                    && observation.phase == "layout/ownership"
                    && observation.problem.message.contains("Rc")
            )
    );
    let report = analyze("layout_dependency");
    assert!(method(&report, "Vec", "unrelated"));
    assert!(blocked(&report, "dependent", "layout/ownership"));
}
#[test]
fn named_imports_do_not_hide_inherent_or_complete_absence_observations() {
    let report = analyze("import_candidate_domain");
    assert!(observed(&report, "inherent", "E_TYPE"));
    assert!(method(&report, "Known", "unrelated"));
    assert!(blocked(&report, "uncertain", "method lookup"));
    assert!(!method(&report, "i64", "supplied"));
}
#[test]
fn complete_recursive_groups_run_normal_ownership_checks() {
    let report = analyze("recursive_group");
    assert_eq!(
        report
            .coverage
            .get("structural ownership bodies constructed"),
        Some(&2)
    );
    assert!(blocked(&report, "unavailable", "source types"));
    assert!(method(&report, "i64", "unrelated"));
}
#[test]
fn cyclic_aliases_and_failed_function_items_block_only_their_dependencies() {
    let report = analyze("alias_cycles");
    assert!(blocked(&report, "unavailable", "source types"));
    assert!(method(&report, "i64", "absent"));
    let report = analyze("failed_function");
    assert!(blocked(&report, "dependent", "source types"));
    assert!(observed(&report, "independent", "E_TYPE"));
    assert!(!observed(&report, "dependent", "E_NAME"));
    assert!(
        !report
            .observations
            .iter()
            .any(|observation| { observation.problem.message == "unresolved name Item" })
    );
    assert!(report.blocked.iter().any(|observation| {
        observation
            .problem
            .message
            .contains("lookup lies in skipped input")
    }));
    assert!(report.observations.iter().any(|observation| {
        observation.problem.code == "E_NAME"
            && observation.problem.message == "unresolved name BoundaryMissing"
    }));
}
#[test]
fn unavailable_imports_respect_namespaces_and_inner_type_shadowing() {
    let report = analyze("import_namespace");
    assert!(blocked(&report, "uncertain::dependent", "source types"));
    assert!(blocked(&report, "generic", "source types"));
    assert!(method(&report, "i64", "absent"));
}
#[test]
fn unloaded_module_blocks_candidate_absence_but_not_known_type_constraints() {
    let report = analyze("missing_module");
    assert!(observed(&report, "wrong_type", "E_TYPE"));
    assert!(blocked(&report, "method", "method lookup"));
    assert!(blocked(&report, "name", "source types"));
    assert!(!method(&report, "i64", "absent"));
    assert!(!observed(&report, "name", "E_NAME"));
}
#[test]
fn recovery_does_not_evaluate_unchecked_constants_or_erase_named_lifetimes() {
    let report = analyze("unavailable_contracts");
    assert!(blocked(&report, "uses_constant", "ownership construction"));
    for function in ["uses_lifetime", "alias_lifetime", "local_lifetime"] {
        assert!(blocked(&report, function, "layout/ownership"));
    }
    assert!(observed(&report, "independent", "E_TYPE"));
}
#[test]
fn recovered_names_propagate_through_resolved_globs_without_beating_strong_bindings() {
    let report = analyze("recovered_glob");
    for function in ["first::dependent", "second::dependent"] {
        assert!(blocked(&report, function, "source types"));
        assert!(!observed(&report, function, "E_NAME"));
    }
    assert!(observed(&report, "shadow::independent", "E_TYPE"));
}

#[test]
fn uncertain_trait_visibility_cannot_select_a_contextual_header() {
    let report = analyze("contextual_trait_import");
    assert!(observed(&report, "independent", "E_TYPE"));
    assert!(!observed(&report, "dependent", "E_TYPE"));
    assert!(blocked(&report, "dependent", "method lookup"));
}
