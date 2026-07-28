use std::process::ExitCode;

use lkjscript_contracts::{
    current_contracts, memory_obligations, ContractDigest, MemoryObligation, MEMORY_OBLIGATIONS,
};

mod traced;

pub fn command(args: &[String]) -> Result<ExitCode, String> {
    let contracts = current_contracts().map_err(|error| error.to_string())?;
    let contract = contracts
        .get(MEMORY_OBLIGATIONS)
        .ok_or_else(|| "memory-obligations contract is not registered".to_string())?
        .digest();
    let records = memory_obligations();
    match args {
        [_, operation] if operation == "inventory" => print_inventory(contract, &records, false),
        [_, operation, flag] if operation == "inventory" && flag == "--json" => {
            print_inventory(contract, &records, true)
        }
        [_, operation, identity] if operation == "explain" => {
            let record = records
                .iter()
                .find(|record| record.identity == identity)
                .ok_or_else(|| format!("unknown memory identity: {identity}"))?;
            print_record(record);
        }
        [_, operation] if operation == "traced" => traced::print(contract, false),
        [_, operation, flag] if operation == "traced" && flag == "--json" => {
            traced::print(contract, true);
        }
        _ => {
            return Err(concat!(
                "memory command is exactly: memory inventory [--json], ",
                "memory explain <identity>, or memory traced [--json]"
            )
            .to_string());
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn print_inventory(contract: ContractDigest, records: &[MemoryObligation], json: bool) {
    if json {
        println!("{}", json_inventory(contract, records));
        return;
    }
    println!(
        "schema={MEMORY_OBLIGATIONS} contract={contract} entries={}",
        records.len()
    );
    for record in records {
        print_record(record);
    }
}

fn print_record(record: &MemoryObligation) {
    println!("memory-identity={}", record.identity);
    for (name, value) in fields(record).into_iter().skip(1) {
        println!("  {name}: {value}");
    }
}

fn json_inventory(contract: ContractDigest, records: &[MemoryObligation]) -> String {
    let mut output = String::from("{");
    push_pair(&mut output, "schema", MEMORY_OBLIGATIONS);
    output.push(',');
    push_pair(&mut output, "contract", &contract.to_hex());
    output.push_str(",\"entries\":[");
    for (index, record) in records.iter().enumerate() {
        if index != 0 {
            output.push(',');
        }
        output.push('{');
        for (field_index, (name, value)) in fields(record).into_iter().enumerate() {
            if field_index != 0 {
                output.push(',');
            }
            push_pair(&mut output, name, value);
        }
        output.push('}');
    }
    output.push_str("]}");
    output
}

fn fields(record: &MemoryObligation) -> [(&'static str, &'static str); 28] {
    [
        ("identity", record.identity),
        ("authority", record.authority),
        ("semantic_type", record.semantic_type),
        ("runtime_layout", record.runtime_layout),
        ("value_semantics", record.value_semantics),
        ("mutability", record.mutability),
        ("possible_aliases", record.possible_aliases),
        ("copyability", record.copyability),
        ("current_ownership", record.current_ownership),
        ("escape_behavior", record.escape_behavior),
        ("lifetime", record.lifetime),
        ("strong_cycles", record.strong_cycles),
        ("weak_links", record.weak_links),
        ("destructor", record.destructor),
        ("external_resources", record.external_resources),
        ("portability", record.portability),
        ("contention", record.contention),
        ("allocation_frequency", record.allocation_frequency),
        ("size_class", record.size_class),
        ("current_trace_fields", record.current_trace_fields),
        ("current_exact_roots", record.current_exact_roots),
        ("object_identity", record.object_identity),
        ("current_placement", record.current_placement),
        ("candidate_placements", record.candidate_placements),
        ("reclamation_plan", record.reclamation_plan),
        ("producers", record.producers),
        ("tests", record.tests),
        ("status", record.status),
    ]
}

fn push_pair(output: &mut String, name: &str, value: &str) {
    push_string(output, name);
    output.push(':');
    push_string(output, value);
}

fn push_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            value if value.is_control() => {
                use std::fmt::Write as _;
                let _ = write!(output, "\\u{:04x}", u32::from(value));
            }
            value => output.push(value),
        }
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_json_is_deterministic_complete_and_truthful() {
        let contracts = current_contracts().unwrap_or_default();
        let digest = contracts
            .get(MEMORY_OBLIGATIONS)
            .map(|contract| contract.digest())
            .unwrap_or(ContractDigest::from_bytes([0; 32]));
        let records = memory_obligations();
        let first = json_inventory(digest, &records);
        assert_eq!(first, json_inventory(digest, &records));
        assert!(first.contains("\"identity\":\"gc-heap\""));
        assert!(first.contains("\"current_trace_fields\":\"HeapObj::trace from exact roots\""));
        assert!(first.contains("verified static image data or execution-owned unique store"));
        assert!(first.contains(&digest.to_hex()));
    }
}
