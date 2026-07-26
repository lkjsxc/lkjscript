use lkjscript_core::{BudgetAuthority, BudgetLedger, Error, ResourceCategory, Result};

use crate::source::{Expr, ValidatedSourceTree};

#[derive(Default)]
struct MatchShape {
    patterns: u64,
    arms: u64,
    rows: u64,
    columns: u64,
    work: u64,
    plans: u64,
    witness_bytes: u64,
}

pub(super) fn reserve_matches(tree: &ValidatedSourceTree, ledger: &mut BudgetLedger) -> Result<()> {
    let shape = match_shape_counts(tree)?;
    for (category, amount) in [
        (ResourceCategory::Patterns, shape.patterns),
        (ResourceCategory::MatchArms, shape.arms),
        (ResourceCategory::UsefulnessRows, shape.rows),
        (ResourceCategory::UsefulnessColumns, shape.columns),
        (ResourceCategory::UsefulnessSpecializationWork, shape.work),
        (ResourceCategory::MatchPlans, shape.plans),
        (
            ResourceCategory::ExhaustivenessWitnessBytes,
            shape.witness_bytes,
        ),
    ] {
        super::reserve(ledger, BudgetAuthority::PatternUsefulness, category, amount)?;
    }
    Ok(())
}

fn match_shape_counts(tree: &ValidatedSourceTree) -> Result<MatchShape> {
    let mut shape = MatchShape::default();
    for file in tree.files() {
        for form in &file.forms {
            count_matches(form, &mut shape)?;
        }
    }
    Ok(shape)
}

fn count_matches(expression: &Expr, total: &mut MatchShape) -> Result<()> {
    let Expr::Call { name, args } = expression else {
        return Ok(());
    };
    if name == "match" {
        add_match(args, total)?;
    }
    for child in args {
        count_matches(child, total)?;
    }
    Ok(())
}

fn add_match(args: &[Expr], total: &mut MatchShape) -> Result<()> {
    let arms = args
        .get(1)
        .and_then(|form| match form {
            Expr::Call { name, args } if name == "arms" => Some(args.as_slice()),
            _ => None,
        })
        .unwrap_or_default();
    let arm_count =
        u64::try_from(arms.len()).map_err(|_| Error::msg("match arm count exceeds u64"))?;
    let patterns = arms.iter().try_fold(0_u64, |sum, arm| {
        let count = match arm {
            Expr::Call { name, args } if name == "arm" => args
                .first()
                .map(count_raw_pattern)
                .transpose()?
                .unwrap_or(0),
            _ => 0,
        };
        checked_add(sum, count, "match pattern count overflow")
    })?;
    let rows = arm_count
        .checked_mul(arm_count.saturating_add(1))
        .and_then(|value| value.checked_div(2))
        .and_then(|value| value.checked_add(arm_count.saturating_add(1)))
        .ok_or_else(|| Error::msg("match row reservation overflow"))?;
    let columns = checked_add(patterns, arm_count, "match column reservation overflow")?;
    let work = patterns
        .checked_add(1)
        .and_then(|value| value.checked_mul(arm_count.saturating_add(1)))
        .and_then(|value| value.checked_mul(columns.saturating_add(1)))
        .and_then(|value| value.checked_mul(16))
        .ok_or_else(|| Error::msg("match work reservation overflow"))?;
    let witness = patterns
        .checked_mul(32_768)
        .and_then(|value| value.checked_add(64))
        .ok_or_else(|| Error::msg("match witness reservation overflow"))?;
    total.patterns = checked_add(total.patterns, patterns, "aggregate pattern count overflow")?;
    total.arms = checked_add(total.arms, arm_count, "aggregate match arm count overflow")?;
    total.rows = checked_add(total.rows, rows, "aggregate match row count overflow")?;
    total.columns = checked_add(
        total.columns,
        columns,
        "aggregate match column count overflow",
    )?;
    total.work = checked_add(total.work, work, "aggregate match work overflow")?;
    total.plans = checked_add(total.plans, 1, "match plan count overflow")?;
    total.witness_bytes =
        checked_add(total.witness_bytes, witness, "match witness bytes overflow")?;
    Ok(())
}

fn count_raw_pattern(pattern: &Expr) -> Result<u64> {
    let Expr::Call { name, args } = pattern else {
        return Ok(0);
    };
    let fields = match name.as_str() {
        "variant-pattern" => args.get(2),
        "product-pattern" => args.get(1),
        _ => return Ok(1),
    };
    let Some(Expr::Call {
        name,
        args: members,
    }) = fields
    else {
        return Ok(1);
    };
    if name != "fields" {
        return Ok(1);
    }
    members.iter().try_fold(1_u64, |sum, member| {
        let nested = match member {
            Expr::Call { args, .. } => args.get(1),
            _ => None,
        };
        let count = nested.map(count_raw_pattern).transpose()?.unwrap_or(0);
        checked_add(sum, count, "match pattern count overflow")
    })
}

fn checked_add(left: u64, right: u64, message: &str) -> Result<u64> {
    left.checked_add(right).ok_or_else(|| Error::msg(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_pattern_count_is_exact_and_overflow_fails_before_mutation() {
        let mut pattern = Expr::Call {
            name: "wildcard".into(),
            args: Vec::new(),
        };
        for _ in 0..8 {
            pattern = Expr::Call {
                name: "product-pattern".into(),
                args: vec![
                    Expr::Call {
                        name: "type".into(),
                        args: Vec::new(),
                    },
                    Expr::Call {
                        name: "fields".into(),
                        args: vec![Expr::Call {
                            name: "product-field-pattern".into(),
                            args: vec![Expr::LitI64(0), pattern],
                        }],
                    },
                ],
            };
        }
        assert!(matches!(count_raw_pattern(&pattern), Ok(9)));
        assert!(matches!(count_raw_pattern(&pattern), Ok(9)));

        let mut shape = MatchShape {
            patterns: u64::MAX,
            ..MatchShape::default()
        };
        let arm = Expr::Call {
            name: "arm".into(),
            args: vec![pattern, Expr::LitI64(0)],
        };
        let args = vec![
            Expr::LitBool(true),
            Expr::Call {
                name: "arms".into(),
                args: vec![arm],
            },
        ];
        let result = add_match(&args, &mut shape);
        assert!(matches!(result, Err(ref error)
            if error.to_string().contains("aggregate pattern count overflow")));
        assert_eq!(shape.patterns, u64::MAX);
    }
}
