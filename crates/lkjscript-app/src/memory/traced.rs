use lkjscript_contracts::{ContractDigest, LEGACY_TRACED_FAMILIES};

const SCHEMA: &str = "lkjscript.memory-tracing-ratchet";

pub(super) fn print(contract: ContractDigest, json: bool) {
    if json {
        print_json(contract);
        return;
    }
    println!(
        "schema={SCHEMA} contract={contract} families={}",
        LEGACY_TRACED_FAMILIES.len()
    );
    for family in LEGACY_TRACED_FAMILIES {
        println!(
            "legacy-traced-family={} heap-variant={}",
            family.identity, family.heap_variant
        );
    }
}

fn print_json(contract: ContractDigest) {
    print!(
        "{{\"schema\":\"{SCHEMA}\",\"contract\":\"{}\",\"families\":[",
        contract.to_hex()
    );
    for (index, family) in LEGACY_TRACED_FAMILIES.iter().enumerate() {
        if index != 0 {
            print!(",");
        }
        print!(
            "{{\"identity\":\"{}\",\"heap_variant\":\"{}\"}}",
            family.identity, family.heap_variant
        );
    }
    println!("]}}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_registry_is_exact_and_nonempty() {
        assert_eq!(LEGACY_TRACED_FAMILIES.len(), 11);
        assert_eq!(LEGACY_TRACED_FAMILIES[0].identity, "buf");
        assert_eq!(LEGACY_TRACED_FAMILIES[10].identity, "symbol");
    }
}
