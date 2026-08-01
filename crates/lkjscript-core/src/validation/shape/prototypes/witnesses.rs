use crate::{Error, FunctionProto, Result};

pub(super) fn validate(proto: &FunctionProto, category: &str) -> Result<()> {
    if proto.memory_witness_parameters.len() > crate::MAX_MEMORY_WITNESS_PARAMETERS
        || proto.call_witnesses.len() > crate::MAX_CALL_WITNESS_SITES
    {
        return Err(Error::msg(format!(
            "bytecode {category} {} memory witness metadata exceeds bounds",
            proto.name
        )));
    }
    let mut prior_parameter = None;
    for requirement in &proto.memory_witness_parameters {
        let used = proto
            .parameter_type_variables
            .contains(&Some(requirement.parameter))
            || proto.return_type_variable == Some(requirement.parameter);
        if !used
            || prior_parameter.is_some_and(|prior| prior >= requirement.parameter)
            || requirement.operations.is_empty()
            || requirement
                .operations
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        {
            return Err(Error::msg(format!(
                "bytecode {category} {} has noncanonical witness requirements",
                proto.name
            )));
        }
        prior_parameter = Some(requirement.parameter);
    }
    let mut prior_offset = None;
    for site in &proto.call_witnesses {
        let offset = usize::try_from(site.offset)
            .map_err(|_| Error::msg("bytecode call witness offset exceeds usize"))?;
        if prior_offset.is_some_and(|prior| prior >= site.offset)
            || proto.code.get(offset).copied() != Some(crate::Op::Call as u8)
            || site.bindings.len() > crate::MAX_MEMORY_WITNESS_PARAMETERS
        {
            return Err(Error::msg(format!(
                "bytecode {category} {} has noncanonical call witness sites",
                proto.name
            )));
        }
        prior_offset = Some(site.offset);
        let mut prior_binding = None;
        for binding in &site.bindings {
            if prior_binding.is_some_and(|prior| prior >= binding.parameter) {
                return Err(Error::msg("bytecode call witness bindings are not ordered"));
            }
            prior_binding = Some(binding.parameter);
        }
    }
    Ok(())
}
