use crate::SsaType;

pub(crate) fn substitute(
    ty: &SsaType,
    parameters: &[String],
    arguments: &[SsaType],
) -> Result<SsaType, String> {
    match ty {
        SsaType::TypeParameter(name) => parameters
            .iter()
            .position(|parameter| parameter == name)
            .and_then(|index| arguments.get(index))
            .cloned()
            .ok_or_else(|| "unknown evaluator enum type parameter".into()),
        SsaType::List(inner) => Ok(SsaType::List(Box::new(substitute(
            inner, parameters, arguments,
        )?))),
        SsaType::StructuralDestination(_) => {
            Err("private destination cannot be substituted into evaluator closure".into())
        }
        SsaType::Enum {
            id,
            arguments: nested,
        } => Ok(SsaType::Enum {
            id: *id,
            arguments: nested
                .iter()
                .map(|ty| substitute(ty, parameters, arguments))
                .collect::<Result<_, _>>()?,
        }),
        other => Ok(other.clone()),
    }
}
