use super::*;

#[test]
fn marker_trait_impl_and_bound_resolve_to_dense_canonical_identities() {
    let source = format!(
        "{}{}{}{}{}",
        marker_trait("Marked"),
        POINT_PRODUCT,
        marker_impl("Marked", "Point"),
        bounded_identity("keep-marked", "Marked"),
        main_source(
            "Product\nPoint",
            "keep-marked/\nproduct-value/\nPoint\nfield/\nx\n1\n/field\nfield/\ny\n2\n/field\n/product-value\n/keep-marked"
        )
    );
    let program = analyze_one(&source).expect("analyze marker trait program");
    assert_eq!(program.traits.len(), CoreTrait::ALL.len() + 1);
    for (index, core) in CoreTrait::ALL.iter().enumerate() {
        assert_eq!(program.traits[index].name, core.name());
        assert_eq!(program.traits[index].id.raw(), index as u32);
    }
    let marker = &program.traits[CoreTrait::ALL.len()];
    assert_eq!(marker.name, "Marked");
    assert_eq!(program.implementations.len(), 1);
    assert_eq!(program.implementations[0].trait_id, marker.id);
    assert_eq!(program.functions[0].bounds[0].trait_id, marker.id);
    let ExprKind::Call {
        instantiation: Some(instantiation),
        ..
    } = &program.main.body.kind
    else {
        panic!("expected resolved bounded generic call");
    };
    assert_eq!(
        instantiation.substitutions[0].ty,
        Type::Product("Point".into())
    );
    assert_eq!(instantiation.witnesses[0].trait_id, marker.id);
    assert_eq!(
        instantiation.witnesses[0].kind,
        TraitWitnessKind::Explicit(program.implementations[0].id)
    );

    let ssa = lower_program(&program).expect("lower bounded marker program");
    assert_eq!(ssa.program().traits.len(), CoreTrait::ALL.len() + 1);
    assert_eq!(ssa.program().implementations.len(), 1);
    assert_eq!(ssa.program().functions[0].signature.bounds.len(), 1);
    let call = ssa
        .program()
        .functions
        .last()
        .expect("main")
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .find_map(|instruction| match &instruction.kind {
            lkjscript_ir::InstructionKind::Call { instantiation, .. } => instantiation.as_ref(),
            _ => None,
        })
        .expect("SSA bounded call");
    assert_eq!(call.witnesses.len(), 1);
}

#[test]
fn malformed_marker_declarations_bounds_and_namespace_collisions_are_rejected() {
    let main = main_source("Unit", "unit");
    for malformed in [
        format!("trait/\nname/\nMarked\n/name\nmethod/\nclone\n/method\n/trait\n{main}"),
        format!("trait/\nname/\nlower\n/name\n/trait\n{main}"),
        format!("impl/\ntrait/\nMarked\n/trait\nfor/\nI64\n/for\nvalue/\n1\n/value\n/impl\n{main}"),
    ] {
        assert!(
            analyze_one(&malformed).is_err(),
            "accepted malformed marker declaration"
        );
    }
    let duplicate = format!(
        "{}{}{}",
        marker_trait("Marked"),
        marker_trait("Marked"),
        main
    );
    assert!(analysis_error(&duplicate).contains("duplicate trait"));
    let product_collision = format!("{}{}{}", marker_trait("Point"), POINT_PRODUCT, main);
    assert!(analysis_error(&product_collision).contains("collides with a trait"));
    let function_collision = format!(
        "{}{}{}",
        marker_trait("Marked"),
        function_source("Marked", &[], "->\nUnit", "", "unit"),
        main
    );
    assert!(analysis_error(&function_collision).contains("duplicate module"));
    for reserved in ["Copy", "Clone", "Drop", "Send", "Sync", "I64"] {
        let source = format!("{}{main}", marker_trait(reserved));
        assert!(
            analyze_one(&source).is_err(),
            "accepted reserved trait {reserved}"
        );
    }
}
