use super::*;
use crate::hir::{VariantFieldId, VariantId};

pub(super) fn canonical_source(body: &str) -> String {
    body.to_string()
}

pub(super) fn maybe_declaration(name: &str) -> String {
    format!(
        "enum/\nname/\n{name}\n/name\nforall/\nt\n/forall\nvariants/\nvariant/\nname/\nnone\n/name\nfields/\n/fields\n/variant\nvariant/\nname/\nsome\n/name\nfields/\nvariant-field/\nname/\nvalue\n/name\ntype/\nt\n/type\n/variant-field\n/fields\n/variant\n/variants\n/enum\n"
    )
}

fn unit_main_source() -> String {
    main_source("unit", "unit")
}

#[test]
fn generic_enum_metadata_has_stable_nominal_member_identities_and_order() {
    let source = canonical_source(&format!(
        "{}{}",
        maybe_declaration("maybe"),
        unit_main_source()
    ));
    let first = analyze_one(&source).expect("generic enum declaration");
    let second = analyze_one(&source).expect("stable reanalysis");
    let definition = &first.enums[0];
    assert_eq!(definition.id, second.enums[0].id);
    assert_ne!(definition.id.bytes(), [0; 32]);
    assert_eq!(definition.type_parameters, ["t"]);
    assert_eq!(definition.variants.len(), 2);
    assert_eq!(definition.variants[0].source_order, 0);
    assert_eq!(definition.variants[1].source_order, 1);
    assert_ne!(definition.variants[0].id, definition.variants[1].id);
    assert_ne!(definition.variants[0].id, VariantId::new([0; 32]));
    let field = &definition.variants[1].fields[0];
    assert_eq!(field.source_order, 0);
    assert_eq!(field.ty, Type::Param("t".into()));
    assert_ne!(field.id, VariantFieldId::new([0; 32]));
}

#[test]
fn same_shaped_enums_are_nominally_unequal_and_instantiation_is_invariant() {
    let declarations = format!(
        "{}{}",
        maybe_declaration("left"),
        maybe_declaration("right")
    );
    let source = canonical_source(&format!("{declarations}{}", unit_main_source()));
    let program = analyze_one(&source).expect("same-shaped nominal declarations");
    assert_ne!(program.enums[0].id, program.enums[1].id);
    let left_i64 = Type::Enum {
        id: program.enums[0].id,
        arguments: vec![Type::I64],
    };
    let left_bool = Type::Enum {
        id: program.enums[0].id,
        arguments: vec![Type::Bool],
    };
    assert!(!Type::unify_assignable(&left_i64, &left_bool));
}

#[test]
fn explicit_instantiation_resolves_in_signatures_and_missing_arguments_reject() {
    let valid_function = function_source(
        "identity",
        &[],
        "inputs/\nmaybe/\ni64\n/maybe\n/inputs\noutput/\nmaybe/\ni64\n/maybe\n/output",
        "value\nmaybe/\ni64\n/maybe",
        "value",
    );
    let valid = canonical_source(&format!(
        "{}{}{}",
        maybe_declaration("maybe"),
        valid_function,
        unit_main_source()
    ));
    let program = analyze_one(&valid).expect("explicit enum instantiation");
    let Type::Fn { params, ret } = &program
        .bindings
        .iter()
        .find(|binding| binding.name == "identity")
        .expect("identity binding")
        .ty
    else {
        panic!("identity function type")
    };
    assert_eq!(params[0], **ret);
    assert!(matches!(params[0], Type::Enum { arguments: ref args, .. } if args == &[Type::I64]));

    let invalid_function = function_source(
        "bad",
        &[],
        "inputs/\nmaybe/\n/maybe\n/inputs\noutput/\nunit\n/output",
        "value\nmaybe/\n/maybe",
        "unit",
    );
    let invalid = canonical_source(&format!(
        "{}{}{}",
        maybe_declaration("maybe"),
        invalid_function,
        unit_main_source()
    ));
    assert!(analysis_error(&invalid).contains("requires 1 explicit invariant arguments"));
}

#[test]
fn duplicates_empty_variants_and_nested_ownership_reject_exactly() {
    let duplicate_declaration = canonical_source(&format!(
        "{}{}{}",
        maybe_declaration("maybe"),
        maybe_declaration("maybe"),
        unit_main_source()
    ));
    assert!(analysis_error(&duplicate_declaration).contains("duplicate enum declaration maybe"));

    let duplicate_parameter =
        maybe_declaration("maybe").replace("forall/\nt\n/forall", "forall/\nt\nt\n/forall");
    let source = canonical_source(&format!("{duplicate_parameter}{}", unit_main_source()));
    assert!(analysis_error(&source).contains("duplicate forall parameter t"));

    let duplicate_variant = maybe_declaration("maybe").replace("\nnone\n/name", "\nsome\n/name");
    let source = canonical_source(&format!("{duplicate_variant}{}", unit_main_source()));
    assert!(analysis_error(&source).contains("duplicate variant some"));

    let duplicate_field = maybe_declaration("maybe").replace(
        "/variant-field\n/fields", "/variant-field\nvariant-field/\nname/\nvalue\n/name\ntype/\nt\n/type\n/variant-field\n/fields");
    let source = canonical_source(&format!("{duplicate_field}{}", unit_main_source()));
    assert!(analysis_error(&source).contains("duplicate field value"));

    let ownership = maybe_declaration("maybe")
        .replace("type/\nt\n/type", "type/\nlist/\nbyte-vector\n/list\n/type");
    let source = canonical_source(&format!("{ownership}{}", unit_main_source()));
    assert!(analysis_error(&source).contains("ownership/reference types cannot be stored"));
}

fn recursive_chain(edges: usize) -> String {
    let mut source = String::new();
    for index in 0..=edges {
        let next = if index < edges {
            format!(
                "variant-field/\nname/\nnext\n/name\ntype/\ne{}/\n/e{}\n/type\n/variant-field\n",
                index + 1,
                index + 1
            )
        } else {
            String::new()
        };
        source.push_str(&format!(
            "enum/\nname/\ne{index}\n/name\nvariants/\nvariant/\nname/\nnode\n/name\nfields/\n{next}/fields\n/variant\n/variants\n/enum\n"
        ));
    }
    source
}

#[test]
fn recursive_enum_graph_has_no_depth_or_work_admission_quota() {
    let self_cycle = recursive_chain(0).replace(
        "fields/\n/fields",
        "fields/\nvariant-field/\nname/\nnext\n/name\ntype/\ne0/\n/e0\n/type\n/variant-field\n/fields",
    );
    analyze_one(&canonical_source(&format!(
        "{self_cycle}{}",
        unit_main_source()
    )))
    .expect("nominal self recursion");
    let wide = canonical_source(&format!("{}{}", recursive_chain(300), unit_main_source()));
    analyze_one(&wide).expect("enum recursion beyond former depth and work geometry");
}
