use super::*;

pub(super) fn validate(
    proto: &FunctionProto,
    category: &str,
) -> Result<()> {
    if !proto.parameter_type_variables.is_empty()
        && proto.parameter_type_variables.len() != proto.arity
    {
        return Err(Error::msg(format!(
            "bytecode {category} {} type-variable parameter metadata does not match arity",
            proto.name
        )));
    }
    for index in 0..proto.arity {
        let variable = proto
            .parameter_type_variables
            .get(index)
            .copied()
            .flatten();
        if variable.is_none() {
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
                .is_some_and(Option::is_some);
        if overlaps {
            return Err(Error::msg(format!(
                "bytecode {category} {} type-variable parameter overlaps exact metadata",
                proto.name
            )));
        }
    }
    if proto.return_type_variable.is_some()
        && (proto.return_structural.is_some()
            || proto.return_resource.is_some()
            || proto.return_unique.is_some())
    {
        return Err(Error::msg(format!(
            "bytecode {category} {} type-variable return overlaps exact metadata",
            proto.name
        )));
    }
    Ok(())
}
