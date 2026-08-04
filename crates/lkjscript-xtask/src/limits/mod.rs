mod catalog;
mod model;

use std::path::Path;

use model::LimitInventory;

pub fn run(root: &Path, args: &[String]) -> i32 {
    if args != ["--json"] {
        eprintln!("usage: lkjscript-xtask limits --json");
        return 2;
    }
    let policy = match std::fs::read(root.join("meta/structure/policy.json"))
        .map_err(|error| error.to_string())
        .and_then(|bytes| {
            serde_json::from_slice::<crate::model::Policy>(&bytes)
                .map_err(|error| error.to_string())
        }) {
        Ok(policy) => policy,
        Err(error) => {
            eprintln!("load limit authority: {error}");
            return 1;
        }
    };
    if policy.contract != lkjscript_contracts::REPOSITORY_GRAPH_DIGEST.to_hex() {
        eprintln!("limit authority repository-graph contract is stale");
        return 1;
    }
    let contract = match lkjscript_contracts::current_contracts()
        .ok()
        .and_then(|contracts| {
            contracts
                .get(lkjscript_contracts::LIMIT_INVENTORY)
                .map(lkjscript_contracts::RegisteredContract::digest)
        }) {
        Some(contract) => contract.to_hex(),
        None => {
            eprintln!("limit inventory contract is unavailable");
            return 1;
        }
    };
    let inventory = LimitInventory {
        schema: "lkjscript.limit-inventory",
        contract,
        records: catalog::records(&policy),
    };
    match serde_json::to_string_pretty(&inventory) {
        Ok(json) => {
            println!("{json}");
            0
        }
        Err(error) => {
            eprintln!("serialize limit inventory: {error}");
            1
        }
    }
}
