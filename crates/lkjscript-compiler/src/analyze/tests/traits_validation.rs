use super::*;

#[test]
fn impl_coherence_unknown_names_and_core_auto_trait_assertions_are_rejected() {
    let main = main_source("unit", "unit");
    let overlap = format!(
        "{}{}{}{}{}",
        marker_trait("marked"),
        POINT_PRODUCT,
        marker_impl("marked", "point"),
        marker_impl("marked", "point"),
        main
    );
    assert!(analysis_error(&overlap).contains("LKJ-DECL-DUPLICATE"));
    let unknown_trait = format!("{}{}{main}", POINT_PRODUCT, marker_impl("missing", "point"));
    assert!(analysis_error(&unknown_trait).contains("unknown trait"));
    let unknown_product = format!(
        "{}{}{main}",
        marker_trait("marked"),
        marker_impl("marked", "missing")
    );
    assert!(analysis_error(&unknown_product).contains("unknown product"));
    for core in ["copy", "clone", "drop", "send", "sync"] {
        let source = format!("{}{}{main}", POINT_PRODUCT, marker_impl(core, "point"));
        assert!(analysis_error(&source).contains("cannot be explicitly implemented"));
    }
    let declaration_after_impl = format!(
        "{}{}{}{}",
        marker_impl("marked", "point"),
        marker_trait("marked"),
        POINT_PRODUCT,
        main_source("unit", "unit")
    );
    assert!(analyze_one(&declaration_after_impl).is_ok());
}

#[test]
fn bounds_require_declared_parameters_known_traits_and_satisfied_facts() {
    let main = main_source("unit", "unit");
    let undeclared = bounded_identity("bad", "marked").replace("bound/\nt\n", "bound/\nu\n");
    let source = format!("{}{}{main}", marker_trait("marked"), undeclared);
    assert!(analysis_error(&source).contains("not declared by forall"));
    let unknown = format!("{}{main}", bounded_identity("bad", "missing"));
    assert!(analysis_error(&unknown).contains("unknown trait"));
    let duplicate =
        bounded_identity("bad", "copy").replace("/bounds", "bound/\nt\ncopy\n/bound\n/bounds");
    let source = format!("{duplicate}{main}");
    assert!(analysis_error(&source).contains("duplicate bound"));
    for unavailable in ["clone", "drop"] {
        let source = format!("{}{main}", bounded_identity("bad", unavailable));
        assert!(
            analysis_error(&source).contains("requires methods"),
            "accepted unavailable core bound {unavailable}"
        );
    }

    let satisfied = format!(
        "{}{}",
        bounded_identity("copy-value", "copy"),
        main_source("i64", "copy-value/\n7\n/copy-value")
    );
    let program = analyze_one(&satisfied).expect("Copy bound is structurally satisfied");
    let ExprKind::Call {
        instantiation: Some(instantiation),
        ..
    } = &program.main.body.kind
    else {
        panic!("expected Copy instantiation");
    };
    assert_eq!(instantiation.witnesses[0].kind, TraitWitnessKind::AutoTrait);

    let unsatisfied = format!(
        "{}{}",
        bounded_identity("copy-value", "copy"),
        main_source(
            "byte-vector",
            "copy-value/\nnew-byte-vector/\n1\n/new-byte-vector\n/copy-value"
        )
    );
    assert!(analysis_error(&unsatisfied)
        .contains("ownership/reference generic instantiation is unavailable"));

    let wrong_arity = format!(
        "{}{}",
        bounded_identity("copy-value", "copy"),
        main_source("i64", "copy-value/\n/copy-value")
    );
    assert!(analysis_error(&wrong_arity).contains("copy-value: expected 1 args, got 0"));

    let first_class = format!(
        "{}{}",
        bounded_identity("copy-value", "copy"),
        main_source("i64", "let/\nbind/\nf\ncopy-value\n/bind\nf/\n7\n/f\n/let")
    );
    assert!(analysis_error(&first_class).contains("not a first-class value"));

    let forwarding = bounded_identity("forward", "copy")
        .replace("\nvalue\n/fn", "\ncopy-value/\nvalue\n/copy-value\n/fn");
    let source = format!(
        "{}{}{}",
        bounded_identity("copy-value", "copy"),
        forwarding,
        main_source("unit", "unit")
    );
    assert!(analysis_error(&source).contains("current transport route"));
}
