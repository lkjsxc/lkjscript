use std::fs;
use std::path::Path;

use lkjscript_contracts::LEGACY_TRACED_FAMILIES;

const RULE: &str = "LKJ-MEMORY-TRACING-RATCHET";

pub fn check(root: &Path) -> usize {
    let path = root.join("crates/lkjscript-core/src/value/heap_object.rs");
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("{RULE} cannot read {}: {error}", path.display());
            return 1;
        }
    };
    let observed = match heap_variants(&source) {
        Ok(variants) => variants,
        Err(error) => {
            eprintln!("{RULE} {}: {error}", path.display());
            return 1;
        }
    };
    let mut expected: Vec<_> = LEGACY_TRACED_FAMILIES
        .iter()
        .map(|family| family.heap_variant.to_owned())
        .collect();
    expected.sort();
    expected.dedup();
    if observed != expected {
        eprintln!(
            "{RULE} traced HeapObj families changed: expected [{}], observed [{}]",
            expected.join(","),
            observed.join(",")
        );
        return 1;
    }
    if !registry_is_canonical() {
        eprintln!("{RULE} registry identities or variants are not sorted unique");
        return 1;
    }
    0
}

fn heap_variants(source: &str) -> Result<Vec<String>, &'static str> {
    let body = source
        .split_once("pub enum HeapObj {")
        .and_then(|(_, tail)| tail.split_once("\n}\n\nimpl HeapObj"))
        .map(|(body, _)| body)
        .ok_or("HeapObj declaration shape is not canonical")?;
    let mut variants = Vec::new();
    for line in body.lines() {
        let Some(rest) = line.strip_prefix("    ") else {
            continue;
        };
        if rest.starts_with(char::is_whitespace) || rest.starts_with('}') {
            continue;
        }
        let name: String = rest
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .collect();
        if !name.is_empty() {
            variants.push(name);
        }
    }
    variants.sort();
    variants.dedup();
    Ok(variants)
}

fn registry_is_canonical() -> bool {
    LEGACY_TRACED_FAMILIES.windows(2).all(|pair| {
        pair[0].identity < pair[1].identity && pair[0].heap_variant != pair[1].heap_variant
    })
}

#[cfg(test)]
mod tests {
    use super::{heap_variants, registry_is_canonical};

    #[test]
    fn parser_retains_only_enum_variants() {
        let source = "pub enum HeapObj {\n    Int(i64),\n    Pair {\n        car: Value,\n    },\n}\n\nimpl HeapObj {}";
        assert_eq!(heap_variants(source), Ok(vec!["Int".into(), "Pair".into()]));
    }

    #[test]
    fn registry_is_sorted_and_unique() {
        assert!(registry_is_canonical());
    }
}
