use std::collections::{BTreeMap, BTreeSet};

use lkjscript_resource::{CacheDomain, CpuSet, MAX_CPUS};

use super::error::{HostResult, LinuxHostError};
use super::parse::{cpu_list, indexed_name, number, size_bytes};
use super::root::AnchoredRoot;
use super::types::{
    CacheKind, Evidence, LinuxCacheObservation, LinuxCpuObservation, LinuxFactSource,
};

const MAX_CACHES: usize = MAX_CPUS * 8;

type CacheKey = (u8, CacheKind, u32);

pub(crate) fn discover_caches(
    root: &AnchoredRoot,
    cpus: &mut [LinuxCpuObservation],
) -> HostResult<(Vec<LinuxCacheObservation>, Vec<CacheDomain>)> {
    let known: BTreeSet<u32> = cpus.iter().map(|cpu| cpu.cpu).collect();
    let mut caches: BTreeMap<CacheKey, LinuxCacheObservation> = BTreeMap::new();
    for cpu in cpus.iter() {
        let directory = format!("sys/devices/system/cpu/cpu{}/cache", cpu.cpu);
        if !root.exists(&directory)? {
            continue;
        }
        for name in root.entries(&directory)? {
            if indexed_name(&name, "index").is_none() {
                continue;
            }
            let cache = read_cache(root, &format!("{directory}/{name}"))?;
            if cache
                .shared_cpus
                .as_slice()
                .iter()
                .any(|id| !known.contains(id))
            {
                return Err(LinuxHostError::new("cache-membership", name));
            }
            let key = (cache.level, cache.kind, cache.id);
            if let Some(previous) = caches.get(&key) {
                if previous != &cache {
                    return Err(LinuxHostError::new("cache-conflict", format!("{key:?}")));
                }
            } else {
                if caches.len() == MAX_CACHES {
                    return Err(LinuxHostError::new("cache-count", "cache cap exceeded"));
                }
                caches.insert(key, cache);
            }
        }
    }
    let details: Vec<_> = caches.into_values().collect();
    let max_level = details
        .iter()
        .filter(|cache| matches!(cache.kind, CacheKind::Data | CacheKind::Unified))
        .map(|cache| cache.level)
        .max();
    let mut domains: Vec<(u32, CpuSet)> = Vec::new();
    for cache in details.iter().filter(|cache| {
        Some(cache.level) == max_level && matches!(cache.kind, CacheKind::Data | CacheKind::Unified)
    }) {
        if let Some((id, _)) = domains.iter().find(|(_, set)| set == &cache.shared_cpus) {
            if *id != cache.id {
                return Err(LinuxHostError::new(
                    "llc-id-conflict",
                    "one LLC set has two IDs",
                ));
            }
        } else {
            domains.push((cache.id, cache.shared_cpus.clone()));
        }
    }
    domains.sort_by_key(|(id, set)| (*id, set.as_slice()[0]));
    let mut assigned = BTreeSet::new();
    for (id, set) in &domains {
        for cpu in set.as_slice() {
            if !assigned.insert(*cpu) {
                return Err(LinuxHostError::new(
                    "llc-sharing-conflict",
                    format!("CPU {cpu}"),
                ));
            }
            if let Some(detail) = cpus.iter_mut().find(|detail| detail.cpu == *cpu) {
                detail.llc_domain = Some(*id);
            }
        }
    }
    let resource = domains
        .into_iter()
        .enumerate()
        .map(|(index, (_, cpus))| CacheDomain {
            id: index as u32,
            level: max_level.unwrap_or(1),
            cpus,
        })
        .collect();
    Ok((details, resource))
}

fn read_cache(root: &AnchoredRoot, base: &str) -> HostResult<LinuxCacheObservation> {
    let level = number::<u8>(&root.read(&format!("{base}/level"))?, "cache-level")?;
    if level == 0 {
        return Err(LinuxHostError::new("cache-level", "zero cache level"));
    }
    let id = number::<u32>(&root.read(&format!("{base}/id"))?, "cache-id")?;
    let kind = match root.read(&format!("{base}/type"))?.as_str() {
        "Data" => CacheKind::Data,
        "Instruction" => CacheKind::Instruction,
        "Unified" => CacheKind::Unified,
        _ => CacheKind::Other,
    };
    let shared_cpus = if let Some(list) = root.read_optional(&format!("{base}/shared_cpu_list"))? {
        cpu_list(&list, "cache-shared")?
    } else {
        let map = root.read(&format!("{base}/shared_cpu_map"))?;
        CpuSet::parse_map(&map)?
    };
    Ok(LinuxCacheObservation {
        id,
        level,
        kind,
        size_bytes: optional(root, &format!("{base}/size"), size_bytes)?,
        line_bytes: optional(root, &format!("{base}/coherency_line_size"), |text| {
            number(text, "cache-line")
        })?,
        associativity: optional(root, &format!("{base}/ways_of_associativity"), |text| {
            number(text, "cache-assoc")
        })?,
        shared_cpus,
    })
}

fn optional<T>(
    root: &AnchoredRoot,
    path: &str,
    parse: impl FnOnce(&str) -> HostResult<T>,
) -> HostResult<Evidence<Option<T>>> {
    match root.read_optional(path)? {
        Some(text) => Ok(Evidence::reported(
            Some(parse(&text)?),
            LinuxFactSource::Sysfs,
        )),
        None => Ok(Evidence::unknown(None)),
    }
}
