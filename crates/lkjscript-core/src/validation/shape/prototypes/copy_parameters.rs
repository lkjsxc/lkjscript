use super::*;

pub(super) fn validate(proto: &FunctionProto, category: &str) -> Result<()> {
    if !proto.parameter_copy_kinds.is_empty()
        && proto.parameter_copy_kinds.len() != usize::from(proto.arity)
    {
        return Err(Error::msg(format!(
            "bytecode {category} {} copy parameter metadata does not match arity",
            proto.name
        )));
    }
    for (index, kind) in proto.parameter_copy_kinds.iter().copied().enumerate() {
        let Some(kind) = kind else {
            continue;
        };
        if !copy_kind(kind) {
            return Err(Error::msg(format!(
                "bytecode {category} {} copy parameter kind is not scalar",
                proto.name
            )));
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
                .is_some_and(Option::is_some);
        if overlaps {
            return Err(Error::msg(format!(
                "bytecode {category} {} copy parameter metadata overlaps ownership metadata",
                proto.name
            )));
        }
    }
    if proto.return_copy_kind.is_some_and(|kind| !copy_kind(kind)) {
        return Err(Error::msg(format!(
            "bytecode {category} {} copy return kind is not scalar",
            proto.name
        )));
    }
    if proto.return_copy_kind.is_some()
        && (proto.return_structural.is_some()
            || proto.return_type_variable.is_some()
            || proto.return_resource.is_some()
            || proto.return_unique.is_some())
    {
        return Err(Error::msg(format!(
            "bytecode {category} {} copy return metadata overlaps ownership metadata",
            proto.name
        )));
    }
    Ok(())
}

fn copy_kind(kind: crate::StructuralKind) -> bool {
    matches!(
        kind,
        crate::StructuralKind::Unit
            | crate::StructuralKind::Bool
            | crate::StructuralKind::I64
            | crate::StructuralKind::F64
            | crate::StructuralKind::Static
    )
}
