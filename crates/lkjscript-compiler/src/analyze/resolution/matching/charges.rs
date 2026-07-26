use crate::analyze::*;

pub(super) fn plan(patterns: &[MatchPattern], arms: usize) -> Result<MatchPlanCharges> {
    let patterns = patterns.iter().try_fold(0_u64, |sum, pattern| {
        count_pattern(pattern).and_then(|value| {
            sum.checked_add(value)
                .ok_or_else(|| Error::msg("pattern count overflow"))
        })
    })?;
    let arms = u64::try_from(arms).map_err(|_| Error::msg("match arm count exceeds u64"))?;
    let rows = arms
        .checked_mul(
            arms.checked_add(1)
                .ok_or_else(|| Error::msg("usefulness row count overflow"))?,
        )
        .and_then(|value| value.checked_div(2))
        .and_then(|value| value.checked_add(arms + 1))
        .ok_or_else(|| Error::msg("usefulness row count overflow"))?;
    let columns = patterns
        .checked_add(arms)
        .ok_or_else(|| Error::msg("usefulness column count overflow"))?;
    let specialization_work = patterns
        .checked_add(1)
        .and_then(|value| value.checked_mul(arms + 1))
        .and_then(|value| value.checked_mul(columns + 1))
        .and_then(|value| value.checked_mul(16))
        .ok_or_else(|| Error::msg("match specialization reservation overflow"))?;
    let witness_bytes = patterns
        .checked_mul(32_768)
        .and_then(|value| value.checked_add(64))
        .ok_or_else(|| Error::msg("match witness reservation overflow"))?;
    Ok(MatchPlanCharges {
        patterns,
        arms,
        rows,
        columns,
        specialization_work,
        witness_bytes,
    })
}

fn count_pattern(pattern: &MatchPattern) -> Result<u64> {
    let fields = match pattern {
        MatchPattern::Variant { fields, .. } | MatchPattern::Product { fields, .. } => fields,
        _ => return Ok(1),
    };
    fields.iter().try_fold(1_u64, |sum, field| {
        count_pattern(&field.pattern).and_then(|value| {
            sum.checked_add(value)
                .ok_or_else(|| Error::msg("pattern count overflow"))
        })
    })
}
