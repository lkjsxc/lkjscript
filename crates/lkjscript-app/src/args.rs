use lkjscript_core::ResourceProfile;
use lkjscript_jit::JitConfig;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Engine {
    Vm,
    Auto,
    BaselineJit,
    OptimizingJit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeCacheMode {
    Disabled,
    Local,
}

pub struct RunOptions {
    pub engine: Engine,
    pub auto_threshold: u64,
    pub auto_enabled: bool,
    pub resource_profile: ResourceProfile,
    pub native_cache: NativeCacheMode,
    pub file: String,
    pub script_args: Vec<String>,
}

pub fn parse_run(args: &[String]) -> Result<RunOptions, String> {
    let mut index = 1_usize;
    let mut engine = Engine::Auto;
    let mut auto_threshold = JitConfig::default().auto_threshold;
    let mut auto_enabled = true;
    let mut resource_profile = ResourceProfile::default();
    let mut native_cache = NativeCacheMode::Disabled;
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
            "--native-cache" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--native-cache needs disabled or local".to_string())?;
                native_cache = match value.as_str() {
                    "disabled" => NativeCacheMode::Disabled,
                    "local" => NativeCacheMode::Local,
                    other => return Err(format!("unknown native cache mode: {other}")),
                };
                index += 2;
            }
            "--resource-profile" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--resource-profile needs a registered name".to_string())?;
                resource_profile =
                    ResourceProfile::named(value).map_err(|error| error.to_string())?;
                index += 2;
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
        resource_profile,
        native_cache,
        file,
        script_args,
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{parse_run, Engine, NativeCacheMode, ResourceProfile};

    #[test]
    fn ordinary_run_defaults_to_auto_and_explicit_vm_remains_available() {
        let default =
            parse_run(&["run".into(), "main.lkjscript".into()]).expect("parse default run");
        assert_eq!(default.engine, Engine::Auto);
        assert_eq!(default.auto_threshold, 64);
        assert_eq!(default.resource_profile, ResourceProfile::default());
        assert_eq!(default.native_cache, NativeCacheMode::Disabled);

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

        let sandbox = parse_run(&[
            "run".into(),
            "--resource-profile".into(),
            "sandbox".into(),
            "main.lkjscript".into(),
        ])
        .expect("parse resource profile");
        assert_eq!(
            sandbox.resource_profile,
            ResourceProfile::named("sandbox").expect("registered profile")
        );
        let cached = parse_run(&[
            "run".into(),
            "--native-cache".into(),
            "local".into(),
            "main.lkjscript".into(),
        ])
        .expect("parse native cache");
        assert_eq!(cached.native_cache, NativeCacheMode::Local);
        assert!(parse_run(&[
            "run".into(),
            "--resource-profile".into(),
            "unknown".into(),
            "main.lkjscript".into(),
        ])
        .is_err());
    }
}
