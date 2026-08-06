use super::*;

pub(super) fn validate(proto: &FunctionProto, category: &str) -> Result<()> {
    if category == "main" && proto.return_region_product.is_some() {
        return Err(Error::msg(
            "bytecode main invocation-region product cannot cross the process boundary",
        ));
    }
    if !proto.parameter_region_products.is_empty()
        && proto.parameter_region_products.len() != proto.arity
    {
        return Err(Error::msg(format!(
            "bytecode {category} {} region-product parameters do not match arity",
            proto.name
        )));
    }
    for (index, product) in proto.parameter_region_products.iter().enumerate() {
        if product.is_none() {
            continue;
        }
        let overlaps = proto
            .parameter_structurals
            .get(index)
            .is_some_and(Option::is_some)
            || proto
                .parameter_resources
                .get(index)
                .is_some_and(Option::is_some)
            || proto
                .parameter_uniques
                .get(index)
                .is_some_and(Option::is_some)
            || proto
                .parameter_type_variables
                .get(index)
                .is_some_and(Option::is_some)
            || proto
                .parameter_copy_kinds
                .get(index)
                .is_some_and(Option::is_some);
        if overlaps {
            return Err(Error::msg(format!(
                "bytecode {category} {} region-product parameter overlaps metadata",
                proto.name
            )));
        }
    }
    if proto.return_region_product.is_some()
        && (proto.return_structural.is_some()
            || proto.return_type_variable.is_some()
            || proto.return_resource.is_some()
            || proto.return_unique.is_some()
            || proto.return_copy_kind.is_some())
    {
        return Err(Error::msg(format!(
            "bytecode {category} {} region-product return overlaps metadata",
            proto.name
        )));
    }
    Ok(())
}
