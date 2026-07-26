use crate::analyze::*;

pub(in crate::analyze) fn trait_declaration(
    args: &[AstExpr],
) -> std::result::Result<String, String> {
    let [name_form] = args else {
        return Err(concat!(
            "marker trait expects exactly one name/ form; ",
            "methods and associated types are unsupported"
        )
        .into());
    };
    declared_name_form(name_form, "trait")
}

pub(in crate::analyze) fn impl_declaration(
    args: &[AstExpr],
) -> std::result::Result<(String, Type), String> {
    let [trait_form, for_form] = args else {
        return Err(concat!(
            "marker impl expects exactly trait/ and for/ forms; ",
            "methods, associated values, and generics are unsupported"
        )
        .into());
    };
    let trait_name = match trait_form {
        AstExpr::Call { name, args } if name == "trait" => match args.as_slice() {
            [trait_name] => symbolic_name(trait_name)?,
            _ => return Err("impl trait/ must contain exactly one trait name".into()),
        },
        _ => return Err("marker impl expects trait/ first".into()),
    };
    let target = match for_form {
        AstExpr::Call { name, args } if name == "for" => parse_type_form(args)?,
        _ => return Err("marker impl expects for/ second".into()),
    };
    Ok((trait_name, target))
}

pub(in crate::analyze) fn product_declaration(
    args: &[AstExpr],
) -> std::result::Result<(String, &[AstExpr]), String> {
    let [name_form, fields_form] = args else {
        return Err("product expects exactly name/ and fields/ forms".into());
    };
    let name = match name_form {
        AstExpr::Call {
            name,
            args: name_args,
        } if name == "name" => match name_args.as_slice() {
            [AstExpr::LitStr(name)] => name.clone(),
            _ => return Err("product name must be one non-empty name/ text line".into()),
        },
        _ => return Err("product expects name/…/name first".into()),
    };
    let fields = match fields_form {
        AstExpr::Call { name, args } if name == "fields" => args.as_slice(),
        _ => return Err("product expects fields/…/fields second".into()),
    };
    Ok((name, fields))
}

pub(in crate::analyze) fn parse_product_field(
    expression: &AstExpr,
) -> std::result::Result<(String, Type), String> {
    let AstExpr::Call { name, args } = expression else {
        return Err("fields must contain field/…/field forms".into());
    };
    if name != "field" {
        return Err("fields must contain field/…/field forms".into());
    }
    let [name_form, type_form] = args.as_slice() else {
        return Err("field expects exactly name/ and type/ forms".into());
    };
    let field_name = match name_form {
        AstExpr::Call {
            name,
            args: name_args,
        } if name == "name" => match name_args.as_slice() {
            [AstExpr::LitStr(name)] => name.clone(),
            _ => return Err("field name must be one non-empty name/ text line".into()),
        },
        _ => return Err("field expects name/…/name first".into()),
    };
    let ty = match type_form {
        AstExpr::Call { name, args } if name == "type" => parse_type_form(args)?,
        _ => return Err("field expects type/…/type second".into()),
    };
    Ok((field_name, ty))
}

pub(in crate::analyze) fn is_declaration_type_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_uppercase())
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
}

pub(in crate::analyze) fn is_builtin_type_name(name: &str) -> bool {
    matches!(
        name,
        "Unit"
            | "Bool"
            | "I64"
            | "F64"
            | "NumericError"
            | "Utf8Error"
            | "SystemError"
            | "Str"
            | "Buf"
            | "Symbol"
            | "Handle"
            | "List"
            | "Option"
            | "Result"
            | "Product"
            | "Any"
            | "Int"
            | "Float"
    )
}
