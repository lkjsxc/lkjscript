pub(super) fn internal_name(module: &str, name: &str) -> String {
    let digest = lkjscript_contracts::ContractDigest::from_bytes(lkjscript_contracts::sha256(
        module.as_bytes(),
    ));
    format!("__module_{}:{name}", digest.to_hex())
}

pub(crate) fn is_internal_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("__module_") else {
        return false;
    };
    let Some((digest, source_name)) = rest.split_once(':') else {
        return false;
    };
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        && lkjscript_contracts::is_identifier(source_name)
}
