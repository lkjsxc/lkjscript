//! Independent fixture families retain deterministic evidence ordinals while sharing eight workers.
use super::*;
use std::collections::VecDeque;
use std::ops::Range;
use std::sync::Mutex;

const WORKERS: usize = 8;
const CHAPTERS: [(&str, usize, usize, usize); 9] = [
    ("nominal", 62, 4, 2),
    ("recursive", 113, 7, 3),
    ("effects", 146, 7, 2),
    ("named", 40, 2, 0),
    ("finite", 41, 2, 1),
    ("requirements", 190, 5, 0),
    ("f64", 47, 3, 0),
    ("parameter-type", 77, 4, 0),
    ("maps", 42, 4, 0),
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct Plan {
    index: usize,
    name: &'static str,
    commands: Range<usize>,
    inventories: Range<usize>,
    runners: Range<usize>,
}

fn plans(
    mut commands: usize,
    mut inventories: usize,
    mut runners: usize,
) -> Result<Vec<Plan>, DevError> {
    CHAPTERS
        .iter()
        .enumerate()
        .map(|(index, &(name, calls, exports, probes))| {
            let command_end = commands
                .checked_add(calls)
                .ok_or_else(|| DevError::infrastructure("offline command ordinal overflow"))?;
            let inventory_end = inventories
                .checked_add(exports)
                .ok_or_else(|| DevError::infrastructure("offline inventory ordinal overflow"))?;
            let runner_end = runners
                .checked_add(probes)
                .ok_or_else(|| DevError::infrastructure("offline runner ordinal overflow"))?;
            let plan = Plan {
                index,
                name,
                commands: commands..command_end,
                inventories: inventories..inventory_end,
                runners: runners..runner_end,
            };
            commands = command_end;
            inventories = inventory_end;
            runners = runner_end;
            Ok(plan)
        })
        .collect()
}

fn child(parent: &Context, plan: &Plan) -> Context {
    let mut receipt = parent.receipt.clone();
    receipt.command_base = plan.commands.start;
    receipt.inventory_base = plan.inventories.start;
    receipt.commands.clear();
    receipt.inventories.clear();
    receipt.producer_inventories.clear();
    receipt.transport_digests.clear();
    receipt.runners.clear();
    receipt.observations.clear();
    receipt.files.clear();
    receipt.nominal = Default::default();
    receipt.recursive = Default::default();
    receipt.effects = Default::default();
    Context {
        root: parent.root.clone(),
        evidence: parent.evidence.clone(),
        binary: parent.binary.clone(),
        receipt,
        command_end: plan.commands.end,
        inventory_end: plan.inventories.end,
        runner_base: plan.runners.start,
        runner_end: plan.runners.end,
    }
}

fn run(plan: &Plan, context: &mut Context, standard: &mut Package) -> Result<(), DevError> {
    match plan.index {
        0 => nominal::workflow(context, standard),
        1 => recursive::workflow(context, standard),
        2 => effects::workflow(context, standard),
        3 => named::workflow(context, standard),
        4 => finite::workflow(context, standard),
        5 => requirements::workflow(context, standard),
        6 => f64::workflow(context, standard),
        7 => parameter_type::workflow(context, standard),
        8 => maps::workflow(context, standard),
        _ => Err(DevError::corrupt("unknown offline fixture family")),
    }?;
    require(
        context.command_index() == plan.commands.end
            && context.inventory_index() == plan.inventories.end
            && context.runner_index() == plan.runners.end
            && context.receipt.producer_inventories.len() == plan.inventories.len()
            && context.receipt.transport_digests.len() == plan.inventories.len(),
        &format!(
            "offline {} inventory changed: commands {}..{}, inventories {}..{}",
            plan.name,
            plan.commands.start,
            context.command_index(),
            plan.inventories.start,
            context.inventory_index()
        ),
    )
}

pub(super) fn workflow(context: &mut Context, standard: &mut Package) -> Result<(), DevError> {
    require(
        context.command_index() == 104
            && context.inventory_index() == 10
            && context.runner_index() == 1,
        "offline core inventory changed before independent fixture families",
    )?;
    // The core's detached HTTP witness has removed every source transport. Acquire
    // one exact read-only standard transport before any worker can stage it.
    standard.container = context.root.join("parallel-standard.lkjp");
    let exported = context.cli(
        None,
        &[
            "package",
            "builtin",
            "export",
            "--kind",
            "transport",
            "--output",
            &standard.container.display().to_string(),
        ],
        true,
    )?;
    require(
        field(&exported, "package", "transport")? == standard.transport,
        "parallel fixture standard acquisition changed its exact dependency",
    )?;
    // These existing public discoveries feed both nominal and recursive sessions.
    // Complete them once before immutable symbol maps are cloned into workers.
    nominal::discover(context, standard)?;
    nominal_session::discover(context, standard)?;
    require(
        context.command_index() == 129 && context.inventory_index() == 10,
        "offline shared discovery inventory changed",
    )?;
    let plans = plans(
        context.command_index(),
        context.inventory_index(),
        context.runner_index(),
    )?;
    let queue = Mutex::new(
        plans
            .iter()
            .map(|plan| (plan.clone(), child(context, plan), standard.clone()))
            .collect::<VecDeque<_>>(),
    );
    let mut completed = Vec::new();
    let mut failure = None;
    std::thread::scope(|scope| {
        let workers = (0..WORKERS)
            .map(|_| {
                scope.spawn(|| {
                    let mut results = Vec::new();
                    loop {
                        let job = queue
                            .lock()
                            .map_err(|_| {
                                DevError::infrastructure("offline fixture queue poisoned")
                            })?
                            .pop_front();
                        let Some((plan, mut context, mut standard)) = job else {
                            return Ok::<_, DevError>(results);
                        };
                        let outcome = run(&plan, &mut context, &mut standard);
                        results.push((plan, context, outcome));
                    }
                })
            })
            .collect::<Vec<_>>();
        // Join every worker, including after a failed family or a panicked worker.
        for worker in workers {
            match worker.join() {
                Ok(Ok(results)) => completed.extend(results),
                Ok(Err(error)) => {
                    failure.get_or_insert(error);
                }
                Err(_) => {
                    failure.get_or_insert_with(|| {
                        DevError::infrastructure("offline fixture worker panicked")
                    });
                }
            }
        }
    });
    completed.sort_by_key(|(plan, _, _)| plan.index);
    if completed.len() != CHAPTERS.len() {
        failure.get_or_insert_with(|| {
            DevError::corrupt("offline fixture family omitted after worker join")
        });
    }
    for (plan, child, outcome) in completed {
        if let Err(error) = outcome {
            failure.get_or_insert(error);
        }
        let mut observed = child.receipt;
        context.receipt.commands.append(&mut observed.commands);
        context
            .receipt
            .inventories
            .append(&mut observed.inventories);
        context
            .receipt
            .producer_inventories
            .append(&mut observed.producer_inventories);
        context
            .receipt
            .transport_digests
            .append(&mut observed.transport_digests);
        context.receipt.runners.append(&mut observed.runners);
        for (key, value) in observed.observations {
            // The existing rejection summary retains the last authority commitment
            // for each diagnostic. Every invocation still has its own command proof.
            let rejection_summary = key.starts_with("rejection-");
            if context.receipt.observations.insert(key, value).is_some() && !rejection_summary {
                failure.get_or_insert_with(|| {
                    DevError::corrupt("offline fixture families repeated an observation identity")
                });
            }
        }
        match plan.index {
            0 => context.receipt.nominal = observed.nominal,
            1 => context.receipt.recursive = observed.recursive,
            2 => context.receipt.effects = observed.effects,
            _ => {}
        }
    }
    match failure {
        Some(error) => Err(error),
        None => require(
            context.command_index() == 887
                && context.inventory_index() == 48
                && context.runner_index() == 9,
            "joined offline evidence is incomplete",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_ranges_are_disjoint_and_cannot_wrap_into_another_family() {
        let plans = plans(129, 10, 1).unwrap();
        assert_eq!(plans.len(), 9);
        assert!(
            plans
                .windows(2)
                .all(|pair| pair[0].commands.end == pair[1].commands.start
                    && pair[0].inventories.end == pair[1].inventories.start
                    && pair[0].runners.end == pair[1].runners.start)
        );
        assert_eq!(plans.last().unwrap().commands.end, 887);
        assert_eq!(plans.last().unwrap().inventories.end, 48);
        assert_eq!(plans.last().unwrap().runners.end, 9);
        assert!(super::plans(usize::MAX, 0, 0).is_err());
        assert!(super::plans(0, usize::MAX, 0).is_err());
        assert!(super::plans(0, 0, usize::MAX).is_err());
    }
}
