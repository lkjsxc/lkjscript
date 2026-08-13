use crate::analyze::*;

pub(in crate::analyze) struct ParsedEnum<'a> {
    pub name: String,
    pub parameters: Vec<String>,
    pub variants: &'a [AstExpr],
}

pub(in crate::analyze) fn enum_declaration(
    args: &[AstExpr],
) -> std::result::Result<ParsedEnum<'_>, String> {
    let Some((name_form, tail)) = args.split_first() else {
        return Err("enum expects name/ first".into());
    };
    let name = declared_name_form(name_form, "enum")?;
    let (parameters, variants_form) = match tail {
        [variants] => (Vec::new(), variants),
        [AstExpr::Call { name, args }, variants] if name == "forall" && !args.is_empty() => {
            let parameters = args
                .iter()
                .map(symbolic_name)
                .collect::<std::result::Result<Vec<_>, _>>()?;
            (parameters, variants)
        }
        _ => return Err("enum expects optional nonempty forall/ then variants/".into()),
    };
    let variants = match variants_form {
        AstExpr::Call { name, args } if name == "variants" && !args.is_empty() => args.as_slice(),
        _ => return Err("enum variants/ must contain at least one variant".into()),
    };
    Ok(ParsedEnum {
        name,
        parameters,
        variants,
    })
}

pub(in crate::analyze) fn parse_variant(
    expression: &AstExpr,
) -> std::result::Result<(String, &[AstExpr]), String> {
    let AstExpr::Call { name, args } = expression else {
        return Err("variants/ may contain only variant/ forms".into());
    };
    let [name_form, fields_form] = args.as_slice() else {
        return Err("variant expects exactly name/ and fields/".into());
    };
    if name != "variant" {
        return Err("variants/ may contain only variant/ forms".into());
    }
    let variant_name = declared_name_form(name_form, "variant")?;
    let fields = match fields_form {
        AstExpr::Call { name, args } if name == "fields" => args.as_slice(),
        _ => return Err("variant expects fields/ second".into()),
    };
    Ok((variant_name, fields))
}

pub(in crate::analyze) fn parse_variant_field(
    analyzer: &Analyzer,
    expression: &AstExpr,
) -> std::result::Result<(String, Type), String> {
    let AstExpr::Call { name, args } = expression else {
        return Err("fields/ may contain only variant-field/ forms".into());
    };
    let [name_form, type_form] = args.as_slice() else {
        return Err("variant-field expects exactly name/ and type/".into());
    };
    if name != "variant-field" {
        return Err("fields/ may contain only variant-field/ forms".into());
    }
    let field_name = declared_name_form(name_form, "variant-field")?;
    let ty = match type_form {
        AstExpr::Call { name, args } if name == "type" => parse_type_form(analyzer, args)?,
        _ => return Err("variant-field expects type/ second".into()),
    };
    Ok((field_name, ty))
}
