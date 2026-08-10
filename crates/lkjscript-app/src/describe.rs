use std::process::ExitCode;

use lkjscript_contracts::{current_contracts, ContractSet};
use serde::Serialize;

pub fn command(args: &[String]) -> Result<ExitCode, String> {
    let json = match args {
        [_] => false,
        [_, flag] if flag == "--json" => true,
        _ => return Err("describe accepts only an optional --json".to_string()),
    };
    let contracts = current_contracts().map_err(|error| error.to_string())?;
    if json {
        println!("{}", json_description(&contracts)?);
    } else {
        print_human(&contracts);
    }
    Ok(ExitCode::SUCCESS)
}

fn print_human(contracts: &ContractSet) {
    println!("compiler: lkjscript");
    println!("contract-set: {}", description_digest(contracts).to_hex());
    println!("contracts:");
    for contract in contracts.iter() {
        println!(
            "  {} {}",
            contract.descriptor().name.as_str(),
            contract.digest()
        );
    }
}

fn json_description(contracts: &ContractSet) -> Result<String, String> {
    let contract_digest = description_digest(contracts).to_hex();
    let contracts = contracts
        .iter()
        .map(|contract| ContractDescription {
            name: contract.descriptor().name.as_str(),
            digest: contract.digest().to_hex(),
        })
        .collect();
    serde_json::to_string(&Description {
        schema: "lkjscript.describe",
        compiler: "lkjscript",
        contract_digest,
        contracts,
    })
    .map_err(|error| format!("encode lkjscript.describe result: {error}"))
}

fn description_digest(contracts: &ContractSet) -> lkjscript_contracts::ContractDigest {
    let mut bytes = b"lkjscript.describe\0contract-digest\0".to_vec();
    for contract in contracts.iter() {
        bytes.extend_from_slice(contract.descriptor().name.as_str().as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&contract.digest().as_bytes());
    }
    lkjscript_contracts::ContractDigest::from_bytes(lkjscript_contracts::sha256(&bytes))
}

#[derive(Serialize)]
struct Description<'a> {
    schema: &'static str,
    compiler: &'static str,
    contract_digest: String,
    contracts: Vec<ContractDescription<'a>>,
}

#[derive(Serialize)]
struct ContractDescription<'a> {
    name: &'a str,
    digest: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_description_is_deterministic_and_contains_only_derived_contract_facts() {
        let result = current_contracts();
        assert!(result.is_ok());
        let contracts = result.unwrap_or_default();
        let first = json_description(&contracts);
        assert!(first.is_ok());
        let first = first.unwrap_or_default();
        assert_eq!(Ok(first.clone()), json_description(&contracts));
        let decoded: serde_json::Value = serde_json::from_str(&first).unwrap_or_default();
        assert_eq!(decoded["schema"].as_str(), Some("lkjscript.describe"));
        assert_eq!(decoded["compiler"].as_str(), Some("lkjscript"));
        assert_eq!(
            decoded["contracts"].as_array().map(Vec::len),
            Some(contracts.iter().count())
        );
        for removed in [
            "target",
            "language_forms",
            "execution_path",
            "unsupported",
            "package_capabilities",
            "platform_revision",
        ] {
            assert!(
                decoded.get(removed).is_none(),
                "stale field remains: {removed}"
            );
        }
    }
}
