use super::*;

#[test]
fn impl_coherence_unknown_names_and_core_auto_trait_assertions_are_rejected() {
    let main = main_source("Unit", "unit");
    let overlap = format!(
        "{}{}{}{}{}",
        marker_trait("Marked"),
        POINT_PRODUCT,
        marker_impl("Marked", "Point"),
        marker_impl("Marked", "Point"),
        main
    );
    assert!(analysis_error(&overlap).contains("LKJ-DECL-DUPLICATE"));
    let unknown_trait = format!("{}{}{main}", POINT_PRODUCT, marker_impl("Missing", "Point"));
    assert!(analysis_error(&unknown_trait).contains("unknown trait"));
    let unknown_product = format!(
        "{}{}{main}",
        marker_trait("Marked"),
        marker_impl("Marked", "Missing")
    );
    assert!(analysis_error(&unknown_product).contains("unknown product"));
    for core in ["Copy", "Clone", "Drop", "Send", "Sync"] {
        let source = format!("{}{}{main}", POINT_PRODUCT, marker_impl(core, "Point"));
        assert!(analysis_error(&source).contains("cannot be explicitly implemented"));
    }
    let declaration_after_impl = format!(
        "{}{}{}{}",
        marker_impl("Marked", "Point"),
        marker_trait("Marked"),
        POINT_PRODUCT,
        main_source("Unit", "unit")
    );
    assert!(analyze_one(&declaration_after_impl).is_ok());
}

#[test]
fn bounds_require_declared_parameters_known_traits_and_satisfied_facts() {
    let main = main_source("Unit", "unit");
    let undeclared = bounded_identity("bad", "Marked").replace("bound/\nT\n", "bound/\nU\n");
    let source = format!("{}{}{main}", marker_trait("Marked"), undeclared);
    assert!(analysis_error(&source).contains("not declared by forall"));
    let unknown = format!("{}{main}", bounded_identity("bad", "Missing"));
    assert!(analysis_error(&unknown).contains("unknown trait"));
    let duplicate =
        bounded_identity("bad", "Copy").replace("/bounds", "bound/\nT\nCopy\n/bound\n/bounds");
    let source = format!("{duplicate}{main}");
    assert!(analysis_error(&source).contains("duplicate bound"));
    for unavailable in ["Clone", "Drop"] {
        let source = format!("{}{main}", bounded_identity("bad", unavailable));
        assert!(
            analysis_error(&source).contains("requires methods"),
            "accepted unavailable core bound {unavailable}"
        );
    }

    let satisfied = format!(
        "{}{}",
        bounded_identity("copy-value", "Copy"),
        main_source("I64", "copy-value/\n7\n/copy-value")
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
        bounded_identity("copy-value", "Copy"),
        main_source("Buf", "copy-value/\nbuf-new/\n1\n/buf-new\n/copy-value")
    );
    assert!(analysis_error(&unsatisfied).contains("does not satisfy trait Copy"));

    let first_class = format!(
        "{}{}",
        bounded_identity("copy-value", "Copy"),
        main_source("I64", "let/\nbind/\nf\ncopy-value\n/bind\nf/\n7\n/f\n/let")
    );
    assert!(analysis_error(&first_class).contains("not a first-class value"));

    let forwarding = bounded_identity("forward", "Copy")
        .replace("\nvalue\n/fn", "\ncopy-value/\nvalue\n/copy-value\n/fn");
    let source = format!(
        "{}{}{}",
        bounded_identity("copy-value", "Copy"),
        forwarding,
        main_source("Unit", "unit")
    );
    assert!(analysis_error(&source).contains("generic context is unavailable"));
}
