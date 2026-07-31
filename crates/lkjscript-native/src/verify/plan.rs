use super::*;

pub(crate) fn verify_plan(
    plan: u64,
    declarations: Vec<FunctionDeclaration>,
    static_bytes: Vec<Box<[u8]>>,
    limits: BackendLimits,
) -> Result<VerifiedMachinePlan, crate::NativeError> {
    verify_plan_inner(plan, declarations, static_bytes, limits)
        .map_err(crate::NativeError::Verification)
}

pub(super) fn verify_plan_inner(
    plan: u64,
    declarations: Vec<FunctionDeclaration>,
    static_bytes: Vec<Box<[u8]>>,
    limits: BackendLimits,
) -> Result<VerifiedMachinePlan, VerificationError> {
    if declarations.is_empty() {
        return Err(VerificationError::EmptyPlan);
    }
    if declarations.len() > limits.max_functions() {
        return Err(VerificationError::LimitExceeded("function count"));
    }
    verify_layout_identities(&declarations)?;
    let static_metadata = static_bytes.iter().try_fold(0_u64, |total, bytes| {
        let len = u64::try_from(bytes.len())
            .map_err(|_| VerificationError::LimitExceeded("static bytes"))?;
        total
            .checked_add(8)
            .and_then(|value| value.checked_add(len))
            .ok_or(VerificationError::LimitExceeded("static bytes"))
    })?;
    if static_metadata > limits.max_metadata_bytes() {
        return Err(VerificationError::LimitExceeded("static bytes"));
    }
    let mut source_functions = HashSet::new();
    let signatures: Vec<_> = declarations
        .iter()
        .map(|declaration| (declaration.id, declaration.signature.clone()))
        .collect();
    let mut functions = Vec::with_capacity(declarations.len());
    let mut total_blocks = 0_usize;
    let mut total_values = 0_usize;
    let mut total_work = 0_u64;
    let mut total_root_records = 0_u64;
    let maximum_root_records = limits.max_metadata_bytes() / ROOT_RECORD_METADATA_BYTES;
    let mut root_requirements = Vec::with_capacity(declarations.len());

    for declaration in declarations {
        if !source_functions.insert(declaration.source_function) {
            return Err(VerificationError::DuplicateSourceFunction);
        }
        verify_signature(declaration.id, &declaration.signature)?;
        let function = declaration
            .body
            .ok_or(VerificationError::MissingFunctionBody(declaration.id))?;
        if function.id.plan != plan || function.id != declaration.id {
            return Err(VerificationError::MissingFunctionBody(declaration.id));
        }
        for instruction in function
            .blocks
            .iter()
            .flat_map(|block| block.instructions.iter())
        {
            let static_identity = match instruction.operation {
                Operation::StaticBytesConst(identity)
                | Operation::StaticStringConst(identity, _) => Some(identity),
                _ => None,
            };
            if static_identity
                .is_some_and(|identity| static_bytes.get(identity.index() as usize).is_none())
            {
                return Err(VerificationError::TypeMismatch("static bytes constant"));
            }
        }
        total_blocks = total_blocks
            .checked_add(function.blocks.len())
            .ok_or(VerificationError::LimitExceeded("block count"))?;
        total_values = total_values
            .checked_add(function.values.len())
            .ok_or(VerificationError::LimitExceeded("value count"))?;
        if total_blocks > limits.max_blocks() {
            return Err(VerificationError::LimitExceeded("block count"));
        }
        if total_values > limits.max_values() {
            return Err(VerificationError::LimitExceeded("value count"));
        }
        if function.locals.len() > limits.max_locals_per_function() {
            return Err(VerificationError::LimitExceeded("local count"));
        }
        let function_work = verify_function(&function, &signatures)?;
        total_work = total_work
            .checked_add(function_work)
            .ok_or(VerificationError::LimitExceeded("work units"))?;
        if total_work > limits.max_work_units() {
            return Err(VerificationError::LimitExceeded("work units"));
        }
        let requirements = derive_call_root_requirements(
            &function,
            &mut total_work,
            limits.max_work_units(),
            &mut total_root_records,
            maximum_root_records,
        )?;
        root_requirements.push(requirements);
        functions.push(function);
    }

    Ok(VerifiedMachinePlan {
        functions,
        static_bytes,
        root_requirements,
        limits,
        work_units: total_work,
    })
}
