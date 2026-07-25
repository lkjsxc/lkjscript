use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::util::walk;

use super::Status;

const PREFIX: &str = "<!-- LKJ-STATUS id=";
const SUFFIX: &str = " -->";

type Claims = BTreeMap<(String, String), Status>;

pub(super) fn claims(root: &Path) -> (Claims, usize) {
    let mut paths = Vec::new();
    if let Ok(entries) = fs::read_dir(root) {
        paths.extend(entries.flatten().map(|entry| entry.path()).filter(|path| {
            path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("md")
        }));
    }
    let mut docs = Vec::new();
    let mut failures = usize::from(walk(&root.join("docs"), &mut docs).is_err());
    paths.extend(
        docs.into_iter()
            .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("md")),
    );
    paths.sort();
    let mut result = BTreeMap::new();
    for path in paths {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let relative = path.strip_prefix(root).unwrap_or(&path).to_string_lossy();
        let mut in_status = false;
        let mut in_fence = false;
        for (index, line) in content.lines().enumerate() {
            if line.trim_start().starts_with("```") {
                in_fence = !in_fence;
                continue;
            }
            if in_fence {
                continue;
            }
            if line.starts_with("## ") {
                in_status = line == "## Status";
            }
            if !line.starts_with("<!-- LKJ-STATUS") {
                continue;
            }
            match parse(line) {
                Ok((id, status)) if in_status => {
                    let key = (relative.to_string(), id);
                    if result.insert(key, status).is_some() {
                        eprintln!("duplicate status claim {relative}:{}", index + 1);
                        failures += 1;
                    }
                }
                Ok(_) => {
                    eprintln!(
                        "status claim outside Status section {relative}:{}",
                        index + 1
                    );
                    failures += 1;
                }
                Err(error) => {
                    eprintln!("malformed status claim {relative}:{}: {error}", index + 1);
                    failures += 1;
                }
            }
        }
    }
    (result, failures)
}

fn parse(line: &str) -> Result<(String, Status), &'static str> {
    let body = line
        .strip_prefix(PREFIX)
        .and_then(|line| line.strip_suffix(SUFFIX));
    let Some((id, status)) = body.and_then(|body| body.split_once(" status=")) else {
        return Err("expected exact id and status fields");
    };
    if id.is_empty() || id.contains(char::is_whitespace) || status.contains(char::is_whitespace) {
        return Err("invalid field spelling");
    }
    let status = match status {
        "current" => Status::Current,
        "accepted-target" => Status::AcceptedTarget,
        "accepted-contract" => Status::AcceptedContract,
        "accepted-selection" => Status::AcceptedSelection,
        "experimental" => Status::Experimental,
        "deferred" => Status::Deferred,
        "rejected" => Status::Rejected,
        "superseded" => Status::Superseded,
        "historical" => Status::Historical,
        _ => return Err("unknown status"),
    };
    Ok((id.to_string(), status))
}

#[cfg(test)]
mod tests {
    use super::{parse, Status};

    #[test]
    fn exact_claim_parses() {
        assert_eq!(
            parse("<!-- LKJ-STATUS id=agent-foundation/1 status=current -->"),
            Ok(("agent-foundation/1".to_string(), Status::Current))
        );
    }

    #[test]
    fn all_statuses_are_closed() {
        for name in [
            "accepted-target",
            "accepted-contract",
            "accepted-selection",
            "experimental",
            "deferred",
            "rejected",
            "superseded",
            "historical",
        ] {
            let line = format!("<!-- LKJ-STATUS id=test/1 status={name} -->");
            assert!(parse(&line).is_ok());
        }
    }

    #[test]
    fn malformed_claims_fail() {
        for line in [
            "LKJ-STATUS id=test/1 status=current",
            "<!-- LKJ-STATUS id=test/1 -->",
            "<!-- LKJ-STATUS id=test/1 status=unknown -->",
            "<!-- LKJ-STATUS id=test 1 status=current -->",
        ] {
            assert!(parse(line).is_err());
        }
    }
}
