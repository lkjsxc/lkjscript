use lkjscript_resource::PlacementMode;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum RuntimeCommand {
    Topology { json: bool, explain: Option<String> },
    HostScheduler { json: bool },
    Plan(PlanOptions),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PlanOptions {
    pub(super) json: bool,
    pub(super) parallelism: Option<usize>,
    pub(super) tasks: Option<usize>,
    pub(super) policy: String,
    pub(super) affinity: PlacementMode,
}

pub(super) fn parse(args: &[String]) -> Result<RuntimeCommand, String> {
    match args.get(1).map(String::as_str) {
        Some("topology") => parse_topology(&args[2..]),
        Some("host-scheduler") => Ok(RuntimeCommand::HostScheduler {
            json: only_json(&args[2..])?,
        }),
        Some("plan") => parse_plan(&args[2..]).map(RuntimeCommand::Plan),
        Some(other) => Err(format!("unknown runtime operation: {other}")),
        None => Err("runtime needs topology, host-scheduler, or plan".into()),
    }
}

fn parse_topology(args: &[String]) -> Result<RuntimeCommand, String> {
    if args.first().map(String::as_str) == Some("explain") {
        let identity = args
            .get(1)
            .filter(|_| args.len() == 2)
            .ok_or_else(|| "runtime topology explain needs one identity".to_string())?;
        return Ok(RuntimeCommand::Topology {
            json: false,
            explain: Some(identity.clone()),
        });
    }
    Ok(RuntimeCommand::Topology {
        json: only_json(args)?,
        explain: None,
    })
}

fn parse_plan(args: &[String]) -> Result<PlanOptions, String> {
    let mut options = PlanOptions {
        json: false,
        parallelism: None,
        tasks: None,
        policy: "owner-compute".into(),
        affinity: PlacementMode::KernelManaged,
    };
    let mut index = 0;
    while let Some(argument) = args.get(index).map(String::as_str) {
        match argument {
            "--json" => {
                options.json = true;
                index += 1;
            }
            "--parallelism" => {
                options.parallelism = Some(positive(args, index, argument)?);
                index += 2;
            }
            "--tasks" => {
                options.tasks = Some(positive(args, index, argument)?);
                index += 2;
            }
            "--policy" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--policy needs a registered name".to_string())?;
                if !matches!(
                    value.as_str(),
                    "sequential"
                        | "static-partition"
                        | "global-fifo"
                        | "local-work-stealing"
                        | "hierarchical-locality"
                        | "owner-compute"
                ) {
                    return Err(format!("unknown scheduler policy: {value}"));
                }
                options.policy = value.clone();
                index += 2;
            }
            "--affinity" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--affinity needs a registered mode".to_string())?;
                options.affinity = match value.as_str() {
                    "kernel-managed" => PlacementMode::KernelManaged,
                    "cpu-pinned" => PlacementMode::CpuPinned,
                    "llc-domain-masked" => PlacementMode::LlcDomainMasked,
                    _ => return Err(format!("unknown affinity mode: {value}")),
                };
                index += 2;
            }
            other => return Err(format!("unknown runtime plan option: {other}")),
        }
    }
    Ok(options)
}

fn positive(args: &[String], index: usize, name: &str) -> Result<usize, String> {
    args.get(index + 1)
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{name} needs a positive integer"))
}

fn only_json(args: &[String]) -> Result<bool, String> {
    match args {
        [] => Ok(false),
        [value] if value == "--json" => Ok(true),
        _ => Err("only --json is accepted here".into()),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn runtime_arguments_are_closed_and_typed() {
        assert_eq!(
            parse(&["runtime".into(), "topology".into(), "--json".into()]).expect("topology"),
            RuntimeCommand::Topology {
                json: true,
                explain: None
            }
        );
        let plan = parse(&[
            "runtime".into(),
            "plan".into(),
            "--parallelism".into(),
            "4".into(),
            "--policy".into(),
            "hierarchical-locality".into(),
            "--affinity".into(),
            "llc-domain-masked".into(),
        ])
        .expect("plan");
        let RuntimeCommand::Plan(plan) = plan else {
            panic!("plan command changed kind");
        };
        assert_eq!(plan.parallelism, Some(4));
        assert_eq!(plan.affinity, PlacementMode::LlcDomainMasked);
        assert!(parse(&[
            "runtime".into(),
            "plan".into(),
            "--policy".into(),
            "unknown".into()
        ])
        .is_err());
    }
}
