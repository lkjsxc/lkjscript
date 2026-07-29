use lkjscript_resource::CpuSet;

use super::error::{HostResult, LinuxHostError};

pub(crate) fn cpu_list(text: &str, field: &'static str) -> HostResult<CpuSet> {
    CpuSet::parse_list(text.trim())
        .map_err(|error| LinuxHostError::new(field, format!("{}: {}", error.code, error.detail)))
}

pub(crate) fn number<T: std::str::FromStr>(text: &str, field: &'static str) -> HostResult<T> {
    text.trim()
        .parse::<T>()
        .map_err(|_| LinuxHostError::new(field, format!("malformed value {text:?}")))
}

pub(crate) fn named_cpu_sets(status: &str) -> HostResult<(CpuSet, CpuSet)> {
    let list = status
        .lines()
        .find_map(|line| line.strip_prefix("Cpus_allowed_list:"))
        .ok_or_else(|| LinuxHostError::new("affinity-status", "missing Cpus_allowed_list"))?;
    let map = status
        .lines()
        .find_map(|line| line.strip_prefix("Cpus_allowed:"))
        .ok_or_else(|| LinuxHostError::new("affinity-status", "missing Cpus_allowed"))?;
    let list = cpu_list(list.trim(), "affinity-list")?;
    let map = CpuSet::parse_map(map.trim()).map_err(LinuxHostError::from)?;
    if list != map {
        return Err(LinuxHostError::new(
            "affinity-mismatch",
            format!("list {list:?} differs from map {map:?}"),
        ));
    }
    Ok((list, map))
}

pub(crate) fn intersect(sets: &[&CpuSet]) -> HostResult<CpuSet> {
    let first = sets
        .first()
        .ok_or_else(|| LinuxHostError::new("effective-empty", "no CPU sets"))?;
    CpuSet::new(
        first
            .as_slice()
            .iter()
            .copied()
            .filter(|cpu| sets[1..].iter().all(|set| set.contains(*cpu))),
    )
    .map_err(|_| LinuxHostError::new("effective-empty", "effective CPU set is empty"))
}

pub(crate) fn indexed_name(name: &str, prefix: &str) -> Option<u32> {
    let suffix = name.strip_prefix(prefix)?;
    if suffix.is_empty() || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    suffix.parse().ok()
}

pub(crate) fn size_bytes(text: &str) -> HostResult<u64> {
    let text = text.trim();
    let (digits, multiplier) = match text.as_bytes().last().copied() {
        Some(b'K' | b'k') => (&text[..text.len() - 1], 1_024_u64),
        Some(b'M' | b'm') => (&text[..text.len() - 1], 1_048_576_u64),
        Some(b'G' | b'g') => (&text[..text.len() - 1], 1_073_741_824_u64),
        _ => (text, 1),
    };
    number::<u64>(digits, "cache-size")?
        .checked_mul(multiplier)
        .ok_or_else(|| LinuxHostError::new("cache-size", "size overflow"))
}
