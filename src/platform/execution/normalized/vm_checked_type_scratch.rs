//! Preflight for legacy raw generic identities which need not occur in prepared
//! call closure. This reserves every cloned vector/name and both encoding buffers
//! before construction; concrete structured child signatures use borrowed lookup.
use super::*;

pub(super) fn bound(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    control: &ExecutionControl,
) -> Result<u64, ExecutionError> {
    let mut nodes = 0_u64;
    visit(program, ty, control, 0, &mut nodes)
}

fn visit(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    control: &ExecutionControl,
    depth: usize,
    nodes: &mut u64,
) -> Result<u64, ExecutionError> {
    control.check()?;
    *nodes = nodes
        .checked_add(1)
        .filter(|n| *n <= super::super::super::value::MAXIMUM_ADMISSION_ITEMS)
        .ok_or_else(exhausted)?;
    if depth > crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
        return Err(exhausted());
    }
    let object = program
        .types
        .get(&ty)
        .ok_or_else(|| admission_error("raw type substitution has a foreign template"))?;
    let mut nested = 0;
    let mut child = |ty| -> Result<(), ExecutionError> {
        nested = add(nested, visit(program, ty, control, depth + 1, nodes)?)?;
        Ok(())
    };
    let mut names = 0;
    let mut storage = 0;
    let mut atoms = 0;
    let count = match &object.form {
        TypeForm::StructuralRecord { fields } => {
            storage = mul(
                fields.len(),
                std::mem::size_of::<crate::platform::kernel::StructuralTypeField>(),
            )?;
            for field in fields {
                names = add(names, field.name.as_str().len() as u64)?;
                child(field.ty)?;
            }
            fields.len()
        }
        TypeForm::Applied { arguments, .. } => {
            // The nominal codec clones its argument vector a second time.
            storage = mul(arguments.len(), 2 * std::mem::size_of::<TypeObjectDigest>())?;
            for ty in arguments {
                child(*ty)?;
            }
            arguments.len()
        }
        TypeForm::Function { parameters, result } => {
            storage = mul(parameters.len(), std::mem::size_of::<TypeObjectDigest>())?;
            for ty in parameters {
                child(*ty)?;
            }
            child(*result)?;
            parameters.len() + 1
        }
        TypeForm::TaskFunction {
            parameters,
            result,
            effect,
        } => {
            storage = mul(
                parameters.len(),
                2 * std::mem::size_of::<TypeObjectDigest>(),
            )?;
            storage = add(
                storage,
                mul(
                    effect.requirements.len(),
                    4 * std::mem::size_of::<crate::platform::kernel::RequirementOperand>(),
                )?,
            )?;
            storage = add(
                storage,
                mul(
                    effect.parameters.len(),
                    4 * std::mem::size_of::<crate::platform::kernel::EffectParameterReference>(),
                )?,
            )?;
            atoms = effect.requirements.len() + effect.parameters.len();
            for ty in parameters {
                child(*ty)?;
            }
            child(*result)?;
            parameters.len() + 1
        }
        TypeForm::List { item } | TypeForm::Option { item } | TypeForm::Stream { item } => {
            child(*item)?;
            1
        }
        TypeForm::Map { key, value } => {
            child(*key)?;
            child(*value)?;
            2
        }
        TypeForm::Result { ok, error } => {
            child(*ok)?;
            child(*error)?;
            2
        }
        // The legacy identity function borrows all other forms unchanged.
        _ => return Ok(0),
    };
    // Each child/reference occupies at most 96 canonical bytes including tags,
    // lengths and declaration identity. Fixed metadata is below 128 bytes.
    let payload = add(add(128, mul(count, 96)?)?, add(names, mul(atoms, 96)?)?)?;
    add(
        nested,
        add(
            add(storage, names)?,
            add(payload.checked_mul(2).ok_or_else(exhausted)?, 50)?,
        )?,
    )
}

fn add(a: u64, b: u64) -> Result<u64, ExecutionError> {
    a.checked_add(b).ok_or_else(exhausted)
}
fn mul(count: usize, width: usize) -> Result<u64, ExecutionError> {
    (count as u64)
        .checked_mul(width as u64)
        .ok_or_else(exhausted)
}
fn exhausted() -> ExecutionError {
    resource_error(
        "normalized_type_substitution",
        "raw type substitution exceeds finite scratch or work",
    )
}
