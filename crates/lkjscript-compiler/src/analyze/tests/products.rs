use super::*;

#[test]
fn nominal_products_remain_resolved_and_state_threadable() {
    let source = format!(
        "{POINT_PRODUCT}def/\nname/\nmove-x\n/name\nfn/\nsig/\nProduct\nPoint\nI64\n->\nProduct\nPoint\n/sig\nparams/\npoint\nProduct/\nPoint\n/Product\nx\nI64\n/params\nwith-field/\npoint\nx\nx\n/with-field\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nvar/\nname/\npoint\n/name\ntype/\nProduct\nPoint\n/type\nproduct-value/\nPoint\nfield/\nx\n1\n/field\nfield/\ny\n2\n/field\n/product-value\ndo/\nset/\npoint\nmove-x/\npoint\n7\n/move-x\n/set\nfield/\npoint\nx\n/field\n/do\n/var\n/main\n"
    );
    let program = analyze_one(&source).expect("analyze product state threading");
    assert_eq!(program.products.len(), 1);
    assert_eq!(program.products[0].name, "Point");
    assert!(!program.global_layout.iter().any(|binding| program
        .binding(*binding)
        .is_some_and(|binding| binding.name == "Point")));
    let chunk = compile_hir(&program).expect("lower product state threading through SSA");
    assert!(chunk.main.code.contains(&(Op::MakeProduct as u8)));
    assert!(chunk.main.code.contains(&(Op::StoreLocal as u8)));
    assert!(!chunk.product_fields.is_empty());
}

#[test]
fn product_field_boundaries_and_import_origins_remain_stable() {
    let mut fifteen = String::from("product/\nname/\nWide\n/name\nfields/\n");
    for index in 0..15 {
        fifteen.push_str(&format!(
            "field/\nname/\nf{index}\n/name\ntype/\nI64\n/type\n/field\n"
        ));
    }
    fifteen.push_str("/fields\n/product\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n");
    assert_eq!(
        analyze_one(&fifteen).expect("15 fields").products[0]
            .fields
            .len(),
        15
    );
    let sixteen = fifteen.replacen(
        "/fields\n/product\n",
        "field/\nname/\nf15\n/name\ntype/\nI64\n/type\n/field\n/fields\n/product\n",
        1,
    );
    assert!(analysis_error(&sixteen).contains("too many fields"));

    let dependency =
        "def/\nname/\nanswer\n/name\nfn/\nsig/\n->\nI64\n/sig\nparams/\n/params\n42\n/fn\n/def\n";
    let root = "main/\nsig/\n->\nI64\n/sig\nanswer/\n/answer\n/main\n";
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
