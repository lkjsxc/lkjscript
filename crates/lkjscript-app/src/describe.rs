use std::process::ExitCode;

use lkjscript_contracts::{current_contracts, ContractSet};

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
const EXECUTION_PATH: &str = "baseline-native-with-vm-fallback";
const UNSUPPORTED: &[&str] = &[
    "collector-free production runtime",
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

fn print_human(contracts: &ContractSet) {
    println!("compiler: lkjscript");
    println!("target: linux-x86-64");
    println!("contracts:");
    for contract in contracts.iter() {
        println!(
            "  {} {}",
            contract.descriptor().name.as_str(),
            contract.digest()
        );
    }
    println!("language-forms: {}", LANGUAGE_FORMS.join(", "));
    println!("execution-path: {EXECUTION_PATH}");
    println!("package-capabilities: local-content-addressed");
    println!("unsupported: {}", UNSUPPORTED.join(", "));
}

fn json_description(contracts: &ContractSet) -> String {
    let digest = description_digest(contracts);
    let mut output = String::from("{\"schema\":\"lkjscript.describe\",");
    push_string(&mut output, "contract_digest", &digest.to_hex());
    output.push(',');
    push_string(&mut output, "compiler", "lkjscript");
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
    push_array(&mut output, "language_forms", LANGUAGE_FORMS);
    output.push(',');
    push_string(&mut output, "execution_path", EXECUTION_PATH);
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

fn description_digest(contracts: &ContractSet) -> lkjscript_contracts::ContractDigest {
    let mut bytes = b"lkjscript.describe\0contract-digest\0".to_vec();
    for contract in contracts.iter() {
        bytes.extend_from_slice(contract.descriptor().name.as_str().as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&contract.digest().as_bytes());
    }
    lkjscript_contracts::ContractDigest::from_bytes(lkjscript_contracts::sha256(&bytes))
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
        assert!(!first.contains("platform_revision"));
        assert!(first.contains("\"contract_digest\":\""));
        assert!(contracts
            .iter()
            .all(|contract| first.contains(&contract.digest().to_hex())));
    }
}
