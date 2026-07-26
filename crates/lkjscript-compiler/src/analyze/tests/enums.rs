use super::*;
use crate::hir::{EnumId, VariantFieldId, VariantId, ENUM_RECURSION_MAX_DEPTH};

pub(super) fn canonical_source(body: &str) -> String {
    body.to_string()
}

pub(super) fn maybe_declaration(name: &str) -> String {
    format!(
        "enum/\nname/\n{name}\n/name\nforall/\nT\n/forall\nvariants/\nvariant/\nname/\nNone\n/name\nfields/\n/fields\n/variant\nvariant/\nname/\nSome\n/name\nfields/\nvariant-field/\nname/\nvalue\n/name\ntype/\nT\n/type\n/variant-field\n/fields\n/variant\n/variants\n/enum\n"
    )
}

fn unit_main_source() -> String {
    main_source("Unit", "unit")
}

#[test]
fn generic_enum_metadata_has_stable_nominal_member_identities_and_order() {
    let source = canonical_source(&format!(
        "{}{}",
        maybe_declaration("Maybe"),
        unit_main_source()
    ));
    let first = analyze_one(&source).expect("generic enum declaration");
    let second = analyze_one(&source).expect("stable reanalysis");
    let definition = &first.enums[0];
    assert_eq!(definition.id, second.enums[0].id);
    assert_ne!(definition.id, EnumId::UNRESOLVED);
    assert_eq!(definition.type_parameters, ["T"]);
    assert_eq!(definition.variants.len(), 2);
    assert_eq!(definition.variants[0].source_order, 0);
    assert_eq!(definition.variants[1].source_order, 1);
    assert_ne!(definition.variants[0].id, definition.variants[1].id);
    assert_ne!(definition.variants[0].id, VariantId::new([0; 32]));
    let field = &definition.variants[1].fields[0];
    assert_eq!(field.source_order, 0);
    assert_eq!(field.ty, Type::Param("T".into()));
    assert_ne!(field.id, VariantFieldId::new([0; 32]));
}

#[test]
fn same_shaped_enums_are_nominally_unequal_and_instantiation_is_invariant() {
    let declarations = format!(
        "{}{}",
        maybe_declaration("Left"),
        maybe_declaration("Right")
    );
    let source = canonical_source(&format!("{declarations}{}", unit_main_source()));
    let program = analyze_one(&source).expect("same-shaped nominal declarations");
    assert_ne!(program.enums[0].id, program.enums[1].id);
    let left_i64 = Type::Enum {
        id: program.enums[0].id,
        name: "Left".into(),
        arguments: vec![Type::I64],
    };
    let left_bool = Type::Enum {
        id: program.enums[0].id,
        name: "Left".into(),
        arguments: vec![Type::Bool],
    };
    assert!(!Type::unify_assignable(&left_i64, &left_bool));
}

#[test]
fn explicit_instantiation_resolves_in_signatures_and_missing_arguments_reject() {
    let valid_function = function_source(
        "identity",
        &[],
        "Maybe/\nI64\n/Maybe\n->\nMaybe/\nI64\n/Maybe",
        "value\nMaybe/\nI64\n/Maybe",
        "value",
    );
    let valid = canonical_source(&format!(
        "{}{}{}",
        maybe_declaration("Maybe"),
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
        "Maybe/\n/Maybe\n->\nUnit",
        "value\nMaybe/\n/Maybe",
        "unit",
    );
    let invalid = canonical_source(&format!(
        "{}{}{}",
        maybe_declaration("Maybe"),
        invalid_function,
        unit_main_source()
    ));
    assert!(analysis_error(&invalid).contains("requires 1 explicit invariant arguments"));
}

#[test]
fn duplicates_empty_variants_and_nested_ownership_reject_exactly() {
    let duplicate_declaration = canonical_source(&format!(
        "{}{}{}",
        maybe_declaration("Maybe"),
        maybe_declaration("Maybe"),
        unit_main_source()
    ));
    assert!(analysis_error(&duplicate_declaration).contains("duplicate enum declaration Maybe"));

    let duplicate_parameter =
        maybe_declaration("Maybe").replace("forall/\nT\n/forall", "forall/\nT\nT\n/forall");
    let source = canonical_source(&format!("{duplicate_parameter}{}", unit_main_source()));
    assert!(analysis_error(&source).contains("duplicate forall parameter T"));

    let duplicate_variant = maybe_declaration("Maybe").replace("\nNone\n/name", "\nSome\n/name");
    let source = canonical_source(&format!("{duplicate_variant}{}", unit_main_source()));
    assert!(analysis_error(&source).contains("duplicate variant Some"));

    let duplicate_field = maybe_declaration("Maybe").replace(
        "/variant-field\n/fields", "/variant-field\nvariant-field/\nname/\nvalue\n/name\ntype/\nT\n/type\n/variant-field\n/fields");
    let source = canonical_source(&format!("{duplicate_field}{}", unit_main_source()));
    assert!(analysis_error(&source).contains("duplicate field value"));

    let ownership = maybe_declaration("Maybe").replace(
        "type/\nT\n/type",
        "type/\nList/\nOwned/\nBuf\n/Owned\n/List\n/type",
    );
    let source = canonical_source(&format!("{ownership}{}", unit_main_source()));
    assert!(analysis_error(&source).contains("ownership/reference types cannot be stored"));
}

fn recursive_chain(edges: usize) -> String {
    let mut source = String::new();
    for index in 0..=edges {
        let next = if index < edges {
            format!(
                "variant-field/\nname/\nnext\n/name\ntype/\nE{}/\n/E{}\n/type\n/variant-field\n",
                index + 1,
                index + 1
            )
        } else {
            String::new()
        };
        source.push_str(&format!(
            "enum/\nname/\nE{index}\n/name\nvariants/\nvariant/\nname/\nNode\n/name\nfields/\n{next}/fields\n/variant\n/variants\n/enum\n"
        ));
    }
    source
}

#[test]
fn recursion_depth_exact_bound_succeeds_and_plus_one_rejects() {
    let self_cycle = recursive_chain(0).replace(
        "fields/\n/fields",
        "fields/\nvariant-field/\nname/\nnext\n/name\ntype/\nE0/\n/E0\n/type\n/variant-field\n/fields",
    );
    analyze_one(&canonical_source(&format!(
        "{self_cycle}{}",
        unit_main_source()
    )))
    .expect("bounded self recursion");
    let exact = canonical_source(&format!(
        "{}{}",
        recursive_chain(ENUM_RECURSION_MAX_DEPTH),
        unit_main_source()
    ));
    analyze_one(&exact).expect("exact recursion depth");
    let plus_one = canonical_source(&format!(
        "{}{}",
        recursive_chain(ENUM_RECURSION_MAX_DEPTH + 1),
        unit_main_source()
    ));
    assert!(analysis_error(&plus_one).contains("enum recursion depth exceeds"));
}
