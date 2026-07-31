use super::*;
use lkjscript_core::Op;
use lkjscript_ir::{SsaType, StructuralValueCategory};

fn product(field_type: &str, field_value: &str) -> String {
    format!(
        concat!(
            "product/\nname/\nbox\n/name\nfields/\nfield/\nname/\nvalue\n/name\n",
            "type/\n{}\n/type\n/field\n/fields\n/product\n",
            "main/\nsig/\ninputs/\n/inputs\noutput/\nproduct/\nbox\n/product\n/output\n/sig\n",
            "product-value/\nbox\nfield/\nvalue\n{}\n/field\n/product-value\n/main\n",
        ),
        field_type, field_value,
    )
}

#[test]
fn deterministic_copy_product_executes_as_structural_bytecode() {
    let compiled = compile_source(
        &product("bool", "true"),
        "deterministic-product.lkjscript",
        &Limits::default(),
    )
    .expect("compile deterministic product");
    let product = compiled
        .ssa()
        .program()
        .memory
        .types
        .iter()
        .find(|item| matches!(item.ty, SsaType::Product(_)))
        .expect("deterministic product has structural metadata");
    assert!(compiled
        .ssa()
        .program()
        .memory
        .representation(&product.ty, StructuralValueCategory::Owner)
        .is_some());
    assert!(compiled.bytecode().main().memory_plan.is_some());
    assert!(compiled.bytecode().has_structural_execution());
    assert!(compiled
        .bytecode()
        .main_instructions()
        .iter()
        .any(|instruction| instruction.op() == Op::StructuralDestinationCreate));
    assert!(compiled
        .bytecode()
        .main_instructions()
        .iter()
        .all(|instruction| instruction.op() != Op::MakeProduct));
}

#[test]
fn region_product_cannot_escape_the_process_boundary() {
    let error = compile_source(
        &product("list/\ni64\n/list", "empty-list/\ni64\n/empty-list"),
        "escaping-region-product.lkjscript",
        &Limits::default(),
    )
    .expect_err("region product process escape rejects");
    assert!(error
        .to_string()
        .contains("cannot cross the process boundary"));
}

#[test]
fn region_product_uses_exact_route_and_unresolved_enum_rejects() {
    let source = concat!(
        "product/\nname/\nbox\n/name\nfields/\nfield/\nname/\nvalue\n/name\n",
        "type/\nlist/\ni64\n/list\n/type\n/field\n/fields\n/product\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nlist-first/\n",
        "field/\nproduct-value/\nbox\nfield/\nvalue\nlist-prepend/\n7\n",
        "empty-list/\ni64\n/empty-list\n/list-prepend\n/field\n/product-value\nvalue\n/field\n",
        "/list-first\n/main\n",
    );
    let product = compile_source(source, "region-product.lkjscript", &Limits::default())
        .expect("compile region product");
    assert!(product
        .ssa()
        .program()
        .memory
        .types
        .iter()
        .all(|item| !matches!(item.ty, SsaType::Product(_))));
    assert_eq!(product.ssa().program().region_products.len(), 1);
    assert!(product
        .bytecode()
        .main_instructions()
        .iter()
        .any(|instruction| instruction.op() == Op::MakeProduct));
    assert!(product.bytecode().products()[0].region);
    assert!(!product.bytecode().has_structural_execution());

    let source = concat!(
        "enum/\nname/\nboxed\n/name\nvariants/\nvariant/\nname/\nvalue\n/name\nfields/\n",
        "variant-field/\nname/\nitems\n/name\ntype/\nlist/\ni64\n/list\n/type\n/variant-field\n",
        "/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\nboxed/\n/boxed\n/output\n/sig\n",
        "variant-value/\ntype/\nboxed/\n/boxed\n/type\nvariant/\nvalue\n/variant\nfields/\n",
        "variant-field/\nname/\nitems\n/name\nempty-list/\ni64\n/empty-list\n",
        "/variant-field\n/fields\n/variant-value\n/main\n",
    );
    let error = compile_source(source, "unresolved-enum.lkjscript", &Limits::default())
        .expect_err("enum with an unresolved list witness rejects");
    assert!(error.to_string().contains("LKJ-MEM-ENUM-UNRESOLVED"));
}
