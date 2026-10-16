//! Region cleanup is lexical on every admitted control-flow edge.
use super::*;

pub(super) fn verify(code: &CompiledCode) -> Result<(), Diagnostic> {
    if !code
        .instructions
        .iter()
        .any(|i| matches!(i, CompiledInstruction::Region(_)))
    {
        return Ok(());
    }
    let invalid = || {
        unit_corrupt(
            "compiler_unit_region_scope",
            "control flow crosses a region lifetime or mismatches its cleanup",
        )
    };
    let mut active = Vec::new();
    let mut locals = BTreeSet::new();
    let mut before = Vec::with_capacity(code.instructions.len());
    let mut after = Vec::with_capacity(code.instructions.len());
    for (index, instruction) in code.instructions.iter().enumerate() {
        before.push(active.last().map(|&(index, _)| index));
        match instruction {
            CompiledInstruction::Region(RegionInstruction::Begin { local, .. }) => {
                if !locals.insert(*local) {
                    return Err(invalid());
                }
                active.push((index, *local));
            }
            CompiledInstruction::Region(RegionInstruction::End { local }) => {
                let Some((_, expected)) = active.pop() else {
                    return Err(invalid());
                };
                if expected != *local {
                    return Err(invalid());
                }
                locals.remove(local);
            }
            CompiledInstruction::Return
            | CompiledInstruction::ReturnBorrowed
            | CompiledInstruction::ReturnView
                if !active.is_empty() =>
            {
                return Err(invalid());
            }
            _ => {}
        }
        after.push(active.last().map(|&(index, _)| index));
    }
    if !active.is_empty() {
        return Err(invalid());
    }
    for (index, instruction) in code.instructions.iter().enumerate() {
        let check = |target: u32| {
            if before.get(target as usize) != Some(&after[index]) {
                Err(invalid())
            } else {
                Ok(())
            }
        };
        match instruction {
            CompiledInstruction::Jump(target) | CompiledInstruction::JumpIfFalse(target) => {
                check(*target)?
            }
            CompiledInstruction::MatchOwned { cases, .. } => {
                for case in cases {
                    check(case.target)?;
                }
            }
            CompiledInstruction::MatchBorrowedOwned { cases, .. } => {
                for case in cases {
                    check(case.target)?;
                }
            }
            CompiledInstruction::SwitchVariant(cases) => {
                for case in cases {
                    check(case.target)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn region_protocol_rejects_cleanup_bypass_and_crossed_or_repeated_scopes() {
        let begin = |local| {
            CompiledInstruction::Region(RegionInstruction::Begin {
                local,
                capability_type: 0,
            })
        };
        let end = |local| CompiledInstruction::Region(RegionInstruction::End { local });
        let code = |instructions| CompiledCode {
            parameter_count: 0,
            local_count: 2,
            instructions,
        };
        assert!(
            verify(&code(vec![
                begin(0),
                CompiledInstruction::Unit,
                end(0),
                CompiledInstruction::Return
            ]))
            .is_ok()
        );
        for instructions in [
            vec![
                begin(0),
                CompiledInstruction::Jump(4),
                CompiledInstruction::Unit,
                end(0),
                CompiledInstruction::Return,
            ],
            vec![
                CompiledInstruction::Jump(2),
                begin(0),
                CompiledInstruction::Unit,
                end(0),
                CompiledInstruction::Return,
            ],
            vec![
                begin(0),
                begin(1),
                end(0),
                end(1),
                CompiledInstruction::Return,
            ],
            vec![
                begin(0),
                begin(0),
                end(0),
                end(0),
                CompiledInstruction::Return,
            ],
            vec![
                begin(0),
                CompiledInstruction::Unit,
                CompiledInstruction::Return,
            ],
        ] {
            assert_eq!(
                verify(&code(instructions)).unwrap_err().code,
                "compiler_unit_region_scope"
            );
        }
    }
}
