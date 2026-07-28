use lkjscript_contracts::SEMANTIC_RESOURCE_PLANE_DIGEST;
use lkjscript_resource::{CpuSet, PlacementMode};
use lkjscript_sys::{ConfigValue, LinuxFactSource, LinuxHostSnapshot, SchedExtState};

pub(super) fn header(schema: &str, snapshot: &LinuxHostSnapshot) -> String {
    format!(
        "{{\"schema\":{},\"contract\":\"{}\",\"snapshot\":\"{}\"",
        quote(schema),
        SEMANTIC_RESOURCE_PLANE_DIGEST,
        hex(&snapshot.digest),
    )
}

pub(super) fn cpu_set(set: &CpuSet) -> String {
    format!(
        "[{}]",
        set.as_slice()
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}

pub(super) fn quote(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '\"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            value if value.is_control() => output.push_str("\\uFFFD"),
            value => output.push(value),
        }
    }
    output.push('\"');
    output
}

pub(super) fn optional_string(value: Option<&str>) -> String {
    value.map_or_else(|| "null".into(), quote)
}
pub(super) fn optional_u32(value: Option<u32>) -> String {
    value.map_or_else(|| "null".into(), |value| value.to_string())
}
pub(super) fn optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "null".into(), |value| value.to_string())
}
pub(super) fn comma(output: &mut String, index: usize) {
    if index != 0 {
        output.push(',');
    }
}
pub(super) fn config(value: ConfigValue) -> &'static str {
    match value {
        ConfigValue::Enabled => "enabled",
        ConfigValue::Disabled => "disabled",
        ConfigValue::Unknown => "unknown",
    }
}
pub(super) fn sched_ext(value: SchedExtState) -> &'static str {
    match value {
        SchedExtState::Enabled => "enabled",
        SchedExtState::Disabled => "disabled",
        SchedExtState::Unknown => "unknown",
    }
}
pub(super) fn source(value: LinuxFactSource) -> &'static str {
    match value {
        LinuxFactSource::Unknown => "unknown",
        LinuxFactSource::Sysfs => "sysfs",
        LinuxFactSource::Procfs => "procfs",
        LinuxFactSource::Cgroup => "cgroup",
        LinuxFactSource::BootConfig => "boot-config",
        LinuxFactSource::KernelAbi => "kernel-abi",
    }
}
pub(super) fn placement(value: PlacementMode) -> &'static str {
    match value {
        PlacementMode::KernelManaged => "kernel-managed",
        PlacementMode::CpuPinned => "cpu-pinned",
        PlacementMode::LlcDomainMasked => "llc-domain-masked",
    }
}
fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
