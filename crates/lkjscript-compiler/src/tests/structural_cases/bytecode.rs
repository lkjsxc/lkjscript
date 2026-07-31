use super::*;
use lkjscript_core::Op;
use lkjscript_ir::{SsaType, StructuralValueCategory};

fn product(field_type: &str, field_value: &str) -> String {
    format!(
        concat!(
            "product/\nname/\nbox\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\n{}\n/type\n/field\n/fields\n/product\n",
            "main/\nsig/\ninputs/\n/inputs\noutput/\nproduct/\nbox\n/product\n/output\n/sig\n",
            "product-value/\nbox\nfield/\nvalue\n{}\n/field\n/product-value\n/main\n",
        ),
        field_type, field_value,
    )
}

#[test]
fn deterministic_hir_authority_is_not_published_as_inert_bytecode() {
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
    assert_eq!(compiled.bytecode().main().memory_plan, None);
    assert!(!compiled.bytecode().has_structural_execution());
}

#[test]
fn legacy_closed_product_and_enum_retain_heap_route() {
    let product = compile_source(
        &product("list/\ni64\n/list", "empty-list/\ni64\n/empty-list"),
        "legacy-product.lkjscript",
        &Limits::default(),
    )
    .expect("compile legacy-closed product");
    assert!(product
        .ssa()
        .program()
        .memory
        .types
        .iter()
        .all(|item| !matches!(item.ty, SsaType::Product(_))));
    assert!(product
        .bytecode()
        .main_instructions()
        .iter()
        .any(|instruction| instruction.op() == Op::MakeProduct));
    assert!(!product.bytecode().has_structural_execution());

    let source = concat!(
        "enum/\nname/\nboxed\n/name\nvariants/\nvariant/\nname/\nvalue\n/name\nfields/\n",
        "variant-field/\nname/\nitems\n/name\ntype/\nlist/\ni64\n/list\n/type\n/variant-field\n",
        "/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\nboxed/\n/boxed\n/output\n/sig\n",
        "variant-value/\ntype/\nboxed/\n/boxed\n/type\nvariant/\nvalue\n/variant\nfields/\n",
        "variant-field/\nname/\nitems\n/name\nempty-list/\ni64\n/empty-list\n/variant-field\n/fields\n/variant-value\n/main\n",
    );
    let enum_program = compile_source(source, "legacy-enum.lkjscript", &Limits::default())
        .expect("compile legacy-closed enum");
    assert!(enum_program
        .ssa()
        .program()
        .memory
        .types
        .iter()
        .all(|item| !matches!(item.ty, SsaType::Enum { .. })));
    assert!(enum_program
        .bytecode()
        .main_instructions()
        .iter()
        .any(|instruction| instruction.op() == Op::MakeEnum));
    assert!(!enum_program.bytecode().has_structural_execution());
}
