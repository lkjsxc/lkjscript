use lkjscript_compiler::CompileMetrics;
use lkjscript_core::ResourceCategory;

use crate::metrics_json::string;

pub(super) fn render(metrics: &CompileMetrics) -> String {
    let profile = metrics.profile;
    let lowered = profile
        .host_lowered_ceilings_sha256
        .map_or_else(|| "null".to_string(), |digest| string(&hex(digest)));
    let used = ResourceCategory::ALL
        .iter()
        .map(|category| {
            format!(
                "{}:{}",
                string(category.as_str()),
                metrics.resources.used(*category)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        concat!(
            "{{\"profile\":{{\"schema\":{},\"contract\":{},\"name\":{},",
            "\"resource_categories\":{},\"implementation_maxima_sha256\":{},",
            "\"ceilings_sha256\":{},\"host_lowered_ceilings_sha256\":{}}},",
            "\"used\":{{{}}}}}"
        ),
        string(profile.schema),
        string(&profile.contract.to_string()),
        string(profile.name.as_str()),
        string(&profile.resource_categories.to_string()),
        string(&hex(profile.implementation_maxima_sha256)),
        string(&hex(profile.ceilings_sha256)),
        lowered,
        used,
    )
}

fn hex(bytes: [u8; 32]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in bytes {
        output.push(char::from(DIGITS[usize::from(byte >> 4)]));
        output.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    output
}
