use std::process::ExitCode;

use lkjscript_contracts::{current_contracts, ContractSet};

const SEMANTIC_OPERATIONS: &[&str] = &[
    "snapshot",
    "read-entity",
    "query-node",
    "hole-context",
    "legal-actions",
    "diagnostics",
    "apply-transaction",
];
const LANGUAGE_FORMS: &[&str] = &[
    "generic-enum",
    "exhaustive-match",
    "never",
    "return",
    "loop",
    "while",
    "break",
    "continue",
    "explicit-numeric-conversions",
    "generic-Option",
    "generic-Result",
    "typed-errors",
    "typed-holes",
];
const ENGINES: &[&str] = &["vm", "auto", "baseline-jit", "optimizing-jit"];
const RESOURCE_PROFILES: &[&str] = &[
    "sandbox",
    "default",
    "build",
    "trusted-local",
    "deterministic",
];
const UNSUPPORTED: &[&str] = &[
    "non-Linux host acceptance",
    "remote package registry",
    "remote artifact cache",
    "direct WebAssembly component execution",
];

pub fn command(args: &[String]) -> Result<ExitCode, String> {
    let json = match args {
        [_] => false,
        [_, flag] if flag == "--json" => true,
        _ => return Err("describe accepts only an optional --json".to_string()),
    };
    let contracts = current_contracts().map_err(|error| error.to_string())?;
    if json {
        println!("{}", json_description(&contracts));
    } else {
        print_human(&contracts);
    }
    Ok(ExitCode::SUCCESS)
}

pub fn semantic() -> Result<ExitCode, String> {
    let contracts = current_contracts().map_err(|error| error.to_string())?;
    let semantic = contracts
        .get(lkjscript_contracts::SEMANTIC_SOURCE)
        .ok_or_else(|| "Semantic Source contract is not registered".to_string())?;
    println!(
        "schema={} contract={} operations={}",
        lkjscript_contracts::SEMANTIC_SOURCE,
        semantic.digest(),
        SEMANTIC_OPERATIONS.join(",")
    );
    Ok(ExitCode::SUCCESS)
}

fn print_human(contracts: &ContractSet) {
    println!("compiler: lkjscript {}", env!("CARGO_PKG_VERSION"));
    println!("target: linux-x86-64");
    println!("contracts:");
    for contract in contracts.iter() {
        println!(
            "  {} {}",
            contract.descriptor().name.as_str(),
            contract.digest()
        );
    }
    println!("semantic-operations: {}", SEMANTIC_OPERATIONS.join(", "));
    println!("language-forms: {}", LANGUAGE_FORMS.join(", "));
    println!("engines: {}", ENGINES.join(", "));
    println!("resource-profiles: {}", RESOURCE_PROFILES.join(", "));
    println!("package-capabilities: local-content-addressed");
    println!("unsupported: {}", UNSUPPORTED.join(", "));
}

fn json_description(contracts: &ContractSet) -> String {
    let mut output = String::from("{\"schema\":\"lkjscript.describe\",");
    push_string(
        &mut output,
        "compiler",
        concat!("lkjscript@", env!("CARGO_PKG_VERSION")),
    );
    output.push(',');
    push_string(&mut output, "target", "linux-x86-64");
    output.push_str(",\"contracts\":[");
    for (index, contract) in contracts.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push('{');
        push_string(&mut output, "name", contract.descriptor().name.as_str());
        output.push(',');
        push_string(&mut output, "digest", &contract.digest().to_hex());
        output.push('}');
    }
    output.push(']');
    push_array(&mut output, "semantic_operations", SEMANTIC_OPERATIONS);
    push_array(&mut output, "language_forms", LANGUAGE_FORMS);
    push_array(&mut output, "engines", ENGINES);
    push_array(&mut output, "resource_profiles", RESOURCE_PROFILES);
    push_array(&mut output, "unsupported", UNSUPPORTED);
    output.push('}');
    output
}

fn push_array(output: &mut String, name: &str, values: &[&str]) {
    output.push(',');
    output.push('"');
    output.push_str(name);
    output.push_str("\":[");
    for (index, value) in values.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push('"');
        output.push_str(value);
        output.push('"');
    }
    output.push(']');
}

fn push_string(output: &mut String, name: &str, value: &str) {
    output.push('"');
    output.push_str(name);
    output.push_str("\":\"");
    output.push_str(value);
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_description_is_deterministic_and_has_full_digests() {
        let result = current_contracts();
        assert!(result.is_ok());
        let contracts = result.unwrap_or_default();
        let first = json_description(&contracts);
        assert_eq!(first, json_description(&contracts));
        assert!(first.contains("\"schema\":\"lkjscript.describe\""));
        assert!(contracts
            .iter()
            .all(|contract| first.contains(&contract.digest().to_hex())));
    }
}
