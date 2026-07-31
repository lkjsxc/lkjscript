use super::*;

#[test]
fn nominal_products_remain_resolved_and_state_threadable() {
    let source = format!(concat!(
        "{POINT_PRODUCT}",
        "def/\nname/\nmove-x\n/name\nfn/\nsig/\ninputs/\nproduct/\npoint\n/product\ni64\n/inputs\n",
        "output/\nproduct/\npoint\n/product\n/output\n/sig\nparams/\npoint\nproduct/\npoint\n/product\n",
        "x\ni64\n/params\nwith-field/\npoint\nx\nx\n/with-field\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nvar/\nname/\npoint\n/name\n",
        "type/\nproduct\npoint\n/type\nproduct-value/\npoint\nfield/\nx\n1\n/field\n",
        "field/\ny\n2\n/field\n/product-value\ndo/\nset/\npoint\nmove-x/\npoint\n7\n/move-x\n/set\n",
        "field/\npoint\nx\n/field\n/do\n/var\n/main\n",
    ), POINT_PRODUCT = POINT_PRODUCT);
    let program = analyze_one(&source).expect("analyze product state threading");
    assert_eq!(program.products.len(), 1);
    assert_eq!(program.products[0].name, "point");
    assert!(!program.global_layout.iter().any(|binding| program
        .binding(*binding)
        .is_some_and(|binding| binding.name == "point")));
    let chunk = compile_hir(&program).expect("lower product state threading through SSA");
    let validated =
        lkjscript_core::validate_chunk(chunk.clone(), &lkjscript_core::ValidationLimits::default())
            .expect("product state-threading bytecode validates");
    let instructions = validated.main_instructions();
    assert!(instructions
        .iter()
        .all(|instruction| instruction.op() != Op::MakeProduct));
    for expected in [
        Op::StructuralDestinationCreate,
        Op::StructuralAggregateFieldCopy,
        Op::StoreStructuralLocal,
    ] {
        assert!(instructions
            .iter()
            .any(|instruction| instruction.op() == expected));
    }
    assert!(!chunk.structural_aggregate_fields.is_empty());
}

#[test]
fn product_field_boundaries_and_import_origins_remain_stable() {
    let mut fifteen = String::from("product/\nname/\nwide\n/name\nfields/\n");
    for index in 0..15 {
        fifteen.push_str(&format!(
            "field/\nname/\nf{index}\n/name\ntype/\ni64\n/type\n/field\n"
        ));
    }
    fifteen.push_str(concat!(
        "/fields\n/product\nmain/\nsig/\ninputs/\n/inputs\n",
        "output/\nunit\n/output\n/sig\nunit\n/main\n",
    ));
    assert_eq!(
        analyze_one(&fifteen).expect("15 fields").products[0]
            .fields
            .len(),
        15
    );
    let sixteen = fifteen.replacen(
        "/fields\n/product\n",
        "field/\nname/\nf15\n/name\ntype/\ni64\n/type\n/field\n/fields\n/product\n",
        1,
    );
    assert!(analysis_error(&sixteen).contains("too many fields"));

    let dependency = concat!(
        "def/\nname/\nanswer\n/name\nfn/\nsig/\ninputs/\n/inputs\n",
        "output/\ni64\n/output\n/sig\nparams/\n/params\n42\n/fn\n/def\n",
    );
    let root =
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nanswer/\n/answer\n/main\n";
    let program = analyze_program(
        &parsed_program(&[
            ("imports/dependency.lkjscript", dependency),
            ("app/main.lkjscript", root),
        ])
        .expect("parse source files"),
    )
    .expect("analyze imports");
    let binding = program
        .bindings
        .iter()
        .find(|binding| binding.name == "answer")
        .expect("answer binding");
    let Origin::Source(source_id) = binding.origin else {
        panic!("answer must have source origin");
    };
    assert_eq!(
        source_id
            .index()
            .and_then(|index| program.sources.get(index))
            .map(|source| &source.path),
        Some(&PathBuf::from("imports/dependency.lkjscript"))
    );
}
