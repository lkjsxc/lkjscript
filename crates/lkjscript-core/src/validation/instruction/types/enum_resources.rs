fn project_resource_result(
    state: &mut State,
    actual: Kind,
    variant: crate::VariantId,
    field: crate::VariantFieldId,
) -> Kind {
    let projected = match actual {
        Kind::ResourceResult { kind, owner }
            if variant.bytes() == crate::RESULT_OK_ID
                && field.bytes() == crate::RESULT_OK_VALUE_ID =>
        {
            Kind::Resource { kind, owner }
        }
        _ => Kind::Any,
    };
    if let Kind::ResourceResult { owner, .. } = actual {
        for local in &mut state.locals {
            if matches!(local, Some(Kind::ResourceResult { owner: actual, .. }) if *actual == owner)
            {
                *local = None;
            }
        }
    }
    projected
}
