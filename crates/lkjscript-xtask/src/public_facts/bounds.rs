use std::collections::{BTreeMap, BTreeSet};

use super::model::LocatedFact;

pub const MAX_MEMBERS: usize = 32;
const MAX_TEXT: usize = 1_024;

pub fn acyclic(facts: &BTreeMap<String, LocatedFact>) -> Result<(), String> {
    let mut pending: BTreeMap<_, BTreeSet<_>> = facts
        .iter()
        .map(|(id, item)| {
            (
                id.as_str(),
                item.fact
                    .dependencies
                    .iter()
                    .chain(&item.fact.invalidated_by)
                    .map(String::as_str)
                    .collect(),
            )
        })
        .collect();
    loop {
        let ready: Vec<_> = pending
            .iter()
            .filter(|(_, dependencies)| dependencies.is_empty())
            .map(|(id, _)| *id)
            .collect();
        if ready.is_empty() {
            break;
        }
        for id in &ready {
            pending.remove(id);
        }
        for dependencies in pending.values_mut() {
            dependencies.retain(|id| !ready.contains(id));
        }
    }
    pending.keys().next().map_or(Ok(()), |id| {
        Err(format!("public-fact dependency cycle includes: {id}"))
    })
}

pub fn collection(values: &[String], id: &str) -> Result<(), String> {
    if values.len() > MAX_MEMBERS || values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(format!(
            "public-fact collection is not unique, sorted, or bounded: {id}"
        ));
    }
    for value in values {
        text(value, "public-fact value")?;
    }
    Ok(())
}

pub fn stable_id(value: &str, kind: &str) -> Result<(), String> {
    if value.is_empty()
        || value.len() > 96
        || !value.as_bytes()[0].is_ascii_lowercase()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        || value.ends_with('-')
        || value.contains("--")
        || value.as_bytes().last().is_some_and(u8::is_ascii_digit)
    {
        return Err(format!(
            "{kind} ID is not a stable unnumbered name: {value}"
        ));
    }
    Ok(())
}

pub fn text(value: &str, kind: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > MAX_TEXT {
        Err(format!("{kind} is empty or excessive"))
    } else {
        Ok(())
    }
}

pub fn sha256(value: &str, id: &str) -> Result<(), String> {
    if value.len() != 64 || !lower_hex(value) {
        Err(format!("invalid contract digest: {id}"))
    } else {
        Ok(())
    }
}

fn lower_hex(value: &str) -> bool {
    value
        .bytes()
        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

pub fn commit(value: &str, id: &str) -> Result<(), String> {
    if value.len() != 40 || !lower_hex(value) {
        Err(format!("invalid evidence commit: {id}"))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn digests_and_commits_require_canonical_lowercase() {
        assert!(super::sha256(&"a".repeat(64), "x").is_ok());
        assert!(super::sha256(&"A".repeat(64), "x").is_err());
        assert!(super::commit(&"F".repeat(40), "x").is_err());
        assert!(super::stable_id("1fact", "fact").is_err());
        assert!(super::stable_id("fact--name", "fact").is_err());
    }
}
