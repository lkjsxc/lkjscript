use lkjscript_jit::JitConfig;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Engine {
    Vm,
    Auto,
    BaselineJit,
    OptimizingJit,
}

pub struct RunOptions {
    pub engine: Engine,
    pub auto_threshold: u64,
    pub auto_enabled: bool,
    pub file: String,
    pub script_args: Vec<String>,
}

pub fn parse_run(args: &[String]) -> Result<RunOptions, String> {
    let mut index = 1_usize;
    let mut engine = Engine::Auto;
    let mut auto_threshold = JitConfig::default().auto_threshold;
    let mut auto_enabled = true;
    while let Some(argument) = args.get(index).map(String::as_str) {
        match argument {
            "--engine" => {
                let value = args.get(index + 1).ok_or_else(|| {
                    "--engine needs vm, auto, baseline-jit, or optimizing-jit".to_string()
                })?;
                engine = match value.as_str() {
                    "vm" => Engine::Vm,
                    "auto" => Engine::Auto,
                    "baseline-jit" => Engine::BaselineJit,
                    "optimizing-jit" => Engine::OptimizingJit,
                    other => return Err(format!("unknown execution engine: {other}")),
                };
                index += 2;
            }
            "--auto-jit-threshold" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--auto-jit-threshold needs a positive integer".to_string())?;
                auto_threshold = value
                    .parse::<u64>()
                    .ok()
                    .filter(|value| *value > 0)
                    .ok_or_else(|| "--auto-jit-threshold needs a positive integer".to_string())?;
                index += 2;
            }
            "--disable-auto-jit" => {
                auto_enabled = false;
                index += 1;
            }
            unknown if unknown.starts_with("--") => {
                return Err(format!("unknown run option: {unknown}"));
            }
            _ => break,
        }
    }
    let file = args
        .get(index)
        .ok_or_else(|| "run needs a .lkjscript path".to_string())?
        .clone();
    index += 1;
    if args.get(index).map(String::as_str) == Some("--") {
        index += 1;
    }
    let script_args = args.get(index..).unwrap_or_default().to_vec();
    Ok(RunOptions {
        engine,
        auto_threshold,
        auto_enabled,
        file,
        script_args,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{parse_run, Engine};

    #[test]
    fn ordinary_run_defaults_to_auto_and_explicit_vm_remains_available() {
        let default =
            parse_run(&["run".into(), "main.lkjscript".into()]).expect("parse default run");
        assert_eq!(default.engine, Engine::Auto);
        assert_eq!(default.auto_threshold, 64);

        let explicit_vm = parse_run(&[
            "run".into(),
            "--engine".into(),
            "vm".into(),
            "main.lkjscript".into(),
        ])
        .expect("parse explicit VM run");
        assert_eq!(explicit_vm.engine, Engine::Vm);

        let optimizing = parse_run(&[
            "run".into(),
            "--engine".into(),
            "optimizing-jit".into(),
            "main.lkjscript".into(),
        ])
        .expect("parse forced optimizing run");
        assert_eq!(optimizing.engine, Engine::OptimizingJit);

        assert!(parse_run(&[
            "run".into(),
            "--resource-profile".into(),
            "sandbox".into(),
            "main.lkjscript".into(),
        ])
        .is_err());
    }
}
