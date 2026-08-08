use super::storage::{LocalMetadata, LocalStorageClass};
use crate::codegen::*;

pub(super) fn color_locals(
    function: &Function,
    value_metadata: &HashMap<ValueId, LocalMetadata>,
    interference: Vec<HashSet<ValueId>>,
) -> Result<HashMap<ValueId, usize>> {
    let entry = function
        .blocks
        .iter()
        .find(|block| block.id == function.entry)
        .ok_or_else(|| Error::msg("SSA function entry block is missing"))?;
    let value_count = value_metadata.len();
    let mut colors: Vec<Option<usize>> = Vec::new();
    colors
        .try_reserve_exact(value_count)
        .map_err(|_| Error::host("SSA local color allocation failed"))?;
    colors.resize(value_count, None);
    let mut color_types: Vec<(SsaType, LocalStorageClass)> = Vec::new();
    color_types
        .try_reserve(value_count)
        .map_err(|_| Error::host("SSA local color-type allocation failed"))?;
    for (slot, parameter) in entry.parameters.iter().enumerate() {
        let index = parameter
            .id
            .index()
            .ok_or_else(|| Error::msg("SSA entry parameter ValueId exceeds usize"))?;
        let Some(color) = colors.get_mut(index) else {
            return Err(Error::msg("SSA entry parameter ValueId is out of range"));
        };
        let metadata = value_metadata
            .get(&parameter.id)
            .ok_or_else(|| Error::msg("SSA entry parameter lost local metadata"))?;
        *color = Some(slot);
        color_types.push((metadata.ty.clone(), metadata.storage));
    }

    let mut order = Vec::new();
    order
        .try_reserve_exact(value_count)
        .map_err(|_| Error::host("SSA local coloring-order allocation failed"))?;
    order.extend(value_metadata.keys().copied());
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
        let metadata = value_metadata
            .get(&value)
            .ok_or_else(|| Error::msg("SSA local allocation lost value metadata"))?;
        let neighbors = interference
            .get(index)
            .ok_or_else(|| Error::msg("SSA local interference metadata is inconsistent"))?;
        let color = color_types
            .iter()
            .enumerate()
            .find(|(candidate, candidate_type)| {
                candidate_type.0 == metadata.ty
                    && candidate_type.1 == metadata.storage
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
                color_types.push((metadata.ty.clone(), metadata.storage));
                color_types.len().saturating_sub(1)
            });
        let Some(destination) = colors.get_mut(index) else {
            return Err(Error::msg("SSA local color destination is out of range"));
        };
        *destination = Some(color);
    }

    let mut slots = HashMap::new();
    slots
        .try_reserve(value_count)
        .map_err(|_| Error::host("SSA local slot-map allocation failed"))?;
    for (raw, color) in colors.into_iter().enumerate() {
        let value = ValueId::new(
            u64::try_from(raw).map_err(|_| Error::msg("SSA local ValueId exceeds u64"))?,
        );
        let color = color.ok_or_else(|| Error::msg("SSA value did not receive a local color"))?;
        slots.insert(value, color);
    }
    Ok(slots)
}
