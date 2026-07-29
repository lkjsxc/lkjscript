use std::path::Path;

use lkjscript_resource::CpuSet;

use super::error::{HostResult, LinuxHostError};
use super::parse::{cpu_list, number};
use super::root::AnchoredRoot;
use super::types::{ConfigValue, Evidence, LinuxFactSource};

pub(crate) fn discover_cgroup(root: &AnchoredRoot) -> HostResult<(Option<CpuSet>, Option<usize>)> {
    let Some(text) = root.read_optional("proc/self/cgroup")? else {
        return Ok((None, None));
    };
    let path = text
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .unwrap_or("/")
        .trim_start_matches('/');
    if Path::new(path)
        .components()
        .any(|part| !matches!(part, std::path::Component::Normal(_)))
        && !path.is_empty()
    {
        return Err(LinuxHostError::new("cgroup-path", path));
    }
    let base = if path.is_empty() {
        "sys/fs/cgroup".to_owned()
    } else {
        format!("sys/fs/cgroup/{path}")
    };
    let cpuset = root
        .read_optional(&format!("{base}/cpuset.cpus.effective"))?
        .map(|text| cpu_list(&text, "cpuset-effective"))
        .transpose()?;
    let quota = root
        .read_optional(&format!("{base}/cpu.max"))?
        .map(|text| quota_workers(&text))
        .transpose()?
        .flatten();
    Ok((cpuset, quota))
}

fn quota_workers(text: &str) -> HostResult<Option<usize>> {
    let mut fields = text.split_ascii_whitespace();
    let quota = fields
        .next()
        .ok_or_else(|| LinuxHostError::new("cpu-max", "missing quota"))?;
    let period = fields
        .next()
        .ok_or_else(|| LinuxHostError::new("cpu-max", "missing period"))?;
    if fields.next().is_some() || quota == "max" {
        return if quota == "max" {
            Ok(None)
        } else {
            Err(LinuxHostError::new("cpu-max", text))
        };
    }
    let quota = number::<u64>(quota, "cpu-max")?;
    let period = number::<u64>(period, "cpu-max")?;
    if quota == 0 || period == 0 {
        return Err(LinuxHostError::new("cpu-max", "zero quota or period"));
    }
    let workers = quota.saturating_add(period - 1) / period;
    usize::try_from(workers)
        .map(Some)
        .map_err(|_| LinuxHostError::new("cpu-max", "worker quota overflow"))
}

pub(crate) fn config_sched_cache(
    root: &AnchoredRoot,
    release: Option<&str>,
) -> HostResult<Evidence<ConfigValue>> {
    let Some(release) = release else {
        return Ok(Evidence::unknown(ConfigValue::Unknown));
    };
    if release.is_empty()
        || !release
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-+".contains(&byte))
    {
        return Err(LinuxHostError::new("kernel-release", release));
    }
    let Some(config) = root.read_optional(&format!("boot/config-{release}"))? else {
        return if root.is_readable("proc/config.gz")? {
            Ok(Evidence {
                value: ConfigValue::Unknown,
                source: LinuxFactSource::Procfs,
                certain: false,
            })
        } else {
            Ok(Evidence::unknown(ConfigValue::Unknown))
        };
    };
    let value = config
        .lines()
        .find_map(|line| line.strip_prefix("CONFIG_SCHED_CACHE="));
    match value {
        Some("y") => Ok(Evidence::reported(
            ConfigValue::Enabled,
            LinuxFactSource::BootConfig,
        )),
        Some("n") => Ok(Evidence::reported(
            ConfigValue::Disabled,
            LinuxFactSource::BootConfig,
        )),
        Some(_) => Err(LinuxHostError::new("sched-cache-config", "expected y or n")),
        None => Ok(Evidence {
            value: ConfigValue::Unknown,
            source: LinuxFactSource::BootConfig,
            certain: false,
        }),
    }
}
