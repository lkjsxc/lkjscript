mod capsules;
mod provenance;

pub use capsules::capsules;
#[cfg(test)]
pub(crate) use capsules::cycle;
pub use provenance::provenance;

use crate::model::Finding;

pub fn metric(findings: &mut Vec<Finding>, rule: &str, path: &str, observed: u64, limit: u64) {
    if observed > limit {
        findings.push(Finding {
            severity: "error".into(),
            rule: rule.into(),
            path: path.into(),
            observed: Some(observed),
            limit: Some(limit),
            message: format!("observed {observed}, limit {limit}"),
            provenance: None,
            sort_key: format!("error:{rule}:{path}:{observed:020}"),
        });
    }
}

pub fn warning(
    findings: &mut Vec<Finding>,
    rule: &str,
    path: &str,
    observed: u64,
    limit: u64,
    message: &str,
) {
    findings.push(Finding {
        severity: "warning".into(),
        rule: rule.into(),
        path: path.into(),
        observed: Some(observed),
        limit: Some(limit),
        message: message.into(),
        provenance: None,
        sort_key: format!("warning:{rule}:{path}:{observed:020}"),
    });
}

pub fn simple(severity: &str, rule: &str, path: &str, message: &str) -> Finding {
    Finding {
        severity: severity.into(),
        rule: rule.into(),
        path: path.into(),
        observed: None,
        limit: None,
        message: message.into(),
        provenance: None,
        sort_key: format!("{severity}:{rule}:{path}"),
    }
}
