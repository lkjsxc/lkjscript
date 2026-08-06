use super::storage::{local_storage_class, LocalStorageClass};
use crate::codegen::*;

pub(super) fn color_locals(
    function: &Function,
    chunk: &Chunk,
    value_types: HashMap<ValueId, SsaType>,
    interference: Vec<HashSet<ValueId>>,
) -> Result<HashMap<ValueId, usize>> {
    let entry = function
        .blocks
        .iter()
        .find(|block| block.id == function.entry)
        .ok_or_else(|| Error::msg("SSA function entry block is missing"))?;
    let value_count = value_types.len();
    let mut colors: Vec<Option<usize>> = vec![None; value_count];
    let mut color_types: Vec<(SsaType, LocalStorageClass)> = Vec::new();
    for (slot, parameter) in entry.parameters.iter().enumerate() {
        let index = parameter
            .id
            .index()
            .ok_or_else(|| Error::msg("SSA entry parameter ValueId exceeds usize"))?;
        let Some(color) = colors.get_mut(index) else {
            return Err(Error::msg("SSA entry parameter ValueId is out of range"));
        };
        *color = Some(slot);
        color_types.push((
            parameter.ty.clone(),
            local_storage_class(function, chunk, parameter.id),
        ));
    }

    let mut order: Vec<ValueId> = value_types.keys().copied().collect();
    order.sort_by(|left, right| {
        let left_degree = left
            .index()
            .and_then(|index| interference.get(index))
            .map_or(0, HashSet::len);
        let right_degree = right
            .index()
            .and_then(|index| interference.get(index))
            .map_or(0, HashSet::len);
        right_degree.cmp(&left_degree).then_with(|| left.cmp(right))
    });
    for value in order {
        let index = value
            .index()
            .ok_or_else(|| Error::msg("SSA ValueId exceeds usize during local allocation"))?;
        if colors.get(index).copied().flatten().is_some() {
            continue;
        }
        let ty = value_types
            .get(&value)
            .ok_or_else(|| Error::msg("SSA local allocation lost a value type"))?;
        let neighbors = interference
            .get(index)
            .ok_or_else(|| Error::msg("SSA local interference metadata is inconsistent"))?;
        let storage = local_storage_class(function, chunk, value);
        let color = color_types
            .iter()
            .enumerate()
            .find(|(candidate, candidate_type)| {
                candidate_type.0 == *ty
                    && candidate_type.1 == storage
                    && neighbors.iter().all(|neighbor| {
                        neighbor
                            .index()
                            .and_then(|index| colors.get(index))
                            .copied()
                            .flatten()
                            != Some(*candidate)
                    })
            })
            .map(|(candidate, _)| candidate)
            .unwrap_or_else(|| {
                color_types.push((ty.clone(), storage));
                color_types.len().saturating_sub(1)
            });
        let Some(destination) = colors.get_mut(index) else {
            return Err(Error::msg("SSA local color destination is out of range"));
        };
        *destination = Some(color);
    }

    let mut slots = HashMap::with_capacity(value_count);
    for (raw, color) in colors.into_iter().enumerate() {
        let value = ValueId::new(
            u64::try_from(raw).map_err(|_| Error::msg("SSA local ValueId exceeds u64"))?,
        );
        let color = color.ok_or_else(|| Error::msg("SSA value did not receive a local color"))?;
        slots.insert(value, color);
    }
    Ok(slots)
}
