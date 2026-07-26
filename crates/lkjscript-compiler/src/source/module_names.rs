pub(super) fn internal_name(module: &str, name: &str) -> String {
    let digest = lkjscript_contracts::ContractDigest::from_bytes(lkjscript_contracts::sha256(
        module.as_bytes(),
    ));
    if name.starts_with(|character: char| character.is_ascii_uppercase()) {
        format!("Module{}-{name}", digest.to_hex())
    } else {
        format!("__module_{}:{name}", digest.to_hex())
    }
}
