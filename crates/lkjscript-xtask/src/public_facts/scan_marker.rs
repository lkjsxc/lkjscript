use std::collections::BTreeMap;

use super::model::Status;

const PREFIX: &str = "<!-- LKJ-F ";
const SUFFIX: &str = " -->";

#[derive(Clone, Debug)]
pub struct Claim {
    pub status: Status,
    pub digest: String,
}

pub type Claims = BTreeMap<(String, String), Claim>;

pub fn scan_file(relative: &str, content: &str, result: &mut Claims) -> usize {
    let mut failures = 0;
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
        if line.starts_with("<!-- LKJ-STATUS") || line.starts_with("<!-- LKJ-FACT") {
            eprintln!("obsolete status marker {relative}:{}", index + 1);
            failures += 1;
            continue;
        }
        if !line.starts_with(PREFIX) {
            continue;
        }
        match parse(line) {
            Ok((id, claim)) if in_status => {
                let key = (relative.to_string(), id);
                if result.insert(key, claim).is_some() {
                    eprintln!("duplicate public-fact claim {relative}:{}", index + 1);
                    failures += 1;
                }
            }
            Ok(_) => {
                eprintln!(
                    "public-fact claim outside Status section {relative}:{}",
                    index + 1
                );
                failures += 1;
            }
            Err(error) => {
                eprintln!(
                    "malformed public-fact claim {relative}:{}: {error}",
                    index + 1
                );
                failures += 1;
            }
        }
    }
    failures
}

fn parse(line: &str) -> Result<(String, Claim), &'static str> {
    let Some(body) = line
        .strip_prefix(PREFIX)
        .and_then(|line| line.strip_suffix(SUFFIX))
    else {
        return Err("expected exact fact, status, and digest fields");
    };
    let fields: Vec<_> = body.split_whitespace().collect();
    let [id, status, digest] = fields.as_slice() else {
        return Err("expected exact fact, status, and digest fields");
    };
    if digest.len() != 43
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err("invalid public-fact marker field");
    }
    let status = match *status {
        "current" => Status::Current,
        "accepted-target" => Status::AcceptedTarget,
        "accepted-contract" => Status::AcceptedContract,
        "accepted-selection" => Status::AcceptedSelection,
        "experimental" => Status::Experimental,
        "deferred" => Status::Deferred,
        "rejected" => Status::Rejected,
        "superseded" => Status::Superseded,
        "historical" => Status::Historical,
        _ => return Err("unknown public-fact status"),
    };
    Ok((
        (*id).to_string(),
        Claim {
            status,
            digest: (*digest).to_string(),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn exact_marker_requires_digest() {
        let digest = "a".repeat(43);
        let line = format!("<!-- LKJ-F test-fact current {digest} -->");
        assert!(parse(&line).is_ok());
        assert!(parse("<!-- LKJ-STATUS id=test-fact status=current -->").is_err());
    }
}
