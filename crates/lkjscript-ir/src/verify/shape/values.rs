use std::collections::{HashMap, HashSet};

use crate::verify::*;
use crate::{Function, IrError, Program, SsaType, ValueId};

pub(crate) fn collect_values(
    program: &Program,
    function: &Function,
    type_parameters: &[&str],
) -> crate::Result<(Vec<SsaType>, HashMap<ValueId, Definition>)> {
    let mut values: HashMap<ValueId, (SsaType, Definition)> = HashMap::new();
    for block in &function.blocks {
        let mut owner_places = HashSet::new();
        for parameter in &block.parameters {
            verify_type(program, &parameter.ty, type_parameters)?;
            if let Some(place) = parameter.owner_place {
                let declared = place_by_id(function, place)?;
                if !is_owned_value(&parameter.ty)
                    || declared.ty != parameter.ty
                    || declared.drop_glue.is_none()
                    || !owner_places.insert(place)
                {
                    return fail(
                        "SSA block owner parameters require unique matching affine places",
                    );
                }
            }
            let definition = Definition {
                block: block.id,
                instruction: None,
            };
            if values
                .insert(parameter.id, (parameter.ty.clone(), definition))
                .is_some()
            {
                return fail(format!(
                    "SSA function {} has duplicate ValueId {}",
                    function.name,
                    parameter.id.raw()
                ));
            }
        }
        for (instruction_index, instruction) in block.instructions.iter().enumerate() {
            verify_type(program, &instruction.ty, type_parameters)?;
            let definition = Definition {
                block: block.id,
                instruction: Some(instruction_index),
            };
            if values
                .insert(instruction.id, (instruction.ty.clone(), definition))
                .is_some()
            {
                return fail(format!(
                    "SSA function {} has duplicate ValueId {}",
                    function.name,
                    instruction.id.raw()
                ));
            }
        }
    }
    let mut types = vec![SsaType::Unit; values.len()];
    let mut definitions = HashMap::with_capacity(values.len());
    for raw in 0..values.len() {
        let raw = u32::try_from(raw).map_err(|_| IrError::new("SSA ValueId count exceeds u32"))?;
        let id = ValueId::new(raw);
        let Some((ty, definition)) = values.remove(&id) else {
            return fail(format!(
                "SSA function {} has missing ValueId {}",
                function.name, raw
            ));
        };
        let Some(slot) = types.get_mut(usize::try_from(raw).unwrap_or(usize::MAX)) else {
            return fail("SSA ValueId indexing failed");
        };
        *slot = ty;
        definitions.insert(id, definition);
    }
    Ok((types, definitions))
}
