use std::collections::BTreeSet;

use crate::{ResourceError, ResourceResult};

pub const MAX_CPU: u32 = 1_048_575;
pub const MAX_CPUS: usize = 4_096;
pub const MAX_RANGES: usize = 256;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CpuSet(Vec<u32>);

impl CpuSet {
    pub fn new(cpus: impl IntoIterator<Item = u32>) -> ResourceResult<Self> {
        let mut values: Vec<u32> = cpus.into_iter().collect();
        if values.is_empty() {
            return Err(ResourceError::new("cpu-empty", "CPU set is required"));
        }
        if values.len() > MAX_CPUS || values.iter().any(|cpu| *cpu > MAX_CPU) {
            return Err(ResourceError::new("cpu-limit", "CPU set exceeds its bound"));
        }
        values.sort_unstable();
        if values.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ResourceError::new(
                "cpu-duplicate",
                "CPU occurs more than once",
            ));
        }
        Ok(Self(values))
    }

    pub fn parse_list(input: &str) -> ResourceResult<Self> {
        if input.is_empty() || input.bytes().any(|byte| byte.is_ascii_whitespace()) {
            return Err(ResourceError::new(
                "cpu-list",
                "empty or whitespace CPU list",
            ));
        }
        let parts: Vec<&str> = input.split(',').collect();
        if parts.len() > MAX_RANGES || parts.iter().any(|part| part.is_empty()) {
            return Err(ResourceError::new(
                "cpu-ranges",
                "malformed or excessive ranges",
            ));
        }
        let mut cpus = BTreeSet::new();
        for part in parts {
            let bounds: Vec<&str> = part.split('-').collect();
            let (start, end) = match bounds.as_slice() {
                [one] => (parse_cpu(one)?, parse_cpu(one)?),
                [left, right] if !left.is_empty() && !right.is_empty() => {
                    (parse_cpu(left)?, parse_cpu(right)?)
                }
                _ => return Err(ResourceError::new("cpu-range", "malformed CPU range")),
            };
            if start > end {
                return Err(ResourceError::new("cpu-descending", "descending CPU range"));
            }
            let width = usize::try_from(end - start)
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or_else(|| ResourceError::new("cpu-overflow", "CPU range overflow"))?;
            if cpus.len().saturating_add(width) > MAX_CPUS {
                return Err(ResourceError::new("cpu-limit", "too many CPUs"));
            }
            for cpu in start..=end {
                if !cpus.insert(cpu) {
                    return Err(ResourceError::new("cpu-duplicate", "overlapping CPU range"));
                }
            }
        }
        Self::new(cpus)
    }

    pub fn parse_map(input: &str) -> ResourceResult<Self> {
        if input.is_empty() || input.bytes().any(|byte| byte.is_ascii_whitespace()) {
            return Err(ResourceError::new("cpu-map", "empty or whitespace CPU map"));
        }
        let groups: Vec<&str> = input.split(',').collect();
        if groups.len() > (MAX_CPU as usize / 32) + 1
            || groups
                .iter()
                .any(|group| group.is_empty() || group.len() > 8)
        {
            return Err(ResourceError::new("cpu-map", "malformed CPU map groups"));
        }
        let mut cpus = Vec::new();
        for (group_index, group) in groups.iter().rev().enumerate() {
            if !group.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(ResourceError::new("cpu-map", "non-hex CPU map"));
            }
            let bits = u32::from_str_radix(group, 16)
                .map_err(|_| ResourceError::new("cpu-map", "CPU map overflow"))?;
            for bit in 0..32_usize {
                if bits & (1_u32 << bit) != 0 {
                    let cpu = group_index
                        .checked_mul(32)
                        .and_then(|base| base.checked_add(bit))
                        .and_then(|value| u32::try_from(value).ok())
                        .ok_or_else(|| ResourceError::new("cpu-overflow", "CPU map overflow"))?;
                    cpus.push(cpu);
                }
            }
        }
        Self::new(cpus)
    }

    pub fn as_slice(&self) -> &[u32] {
        &self.0
    }
    pub fn contains(&self, cpu: u32) -> bool {
        self.0.binary_search(&cpu).is_ok()
    }
    pub fn len(&self) -> usize {
        self.0.len()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn parse_cpu(value: &str) -> ResourceResult<u32> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(ResourceError::new("cpu-number", "malformed CPU number"));
    }
    let cpu = value
        .parse::<u32>()
        .map_err(|_| ResourceError::new("cpu-overflow", "CPU number overflow"))?;
    if cpu > MAX_CPU {
        return Err(ResourceError::new("cpu-limit", "CPU number exceeds limit"));
    }
    Ok(cpu)
}
