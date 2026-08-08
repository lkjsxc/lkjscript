use lkjscript_contracts::{ContractDigest, RegisteredContract};
use lkjscript_core::{Error, Result};

pub(super) fn expected(name: &str) -> Result<ContractDigest> {
    lkjscript_contracts::current_contracts()
        .map_err(|error| Error::msg(format!("build contract registry: {error:?}")))?
        .get(name)
        .map(RegisteredContract::digest)
        .ok_or_else(|| Error::msg(format!("missing registered contract {name}")))
}

pub(super) fn require(value: &str, name: &str) -> Result<ContractDigest> {
    let expected = expected(name)?;
    if ContractDigest::from_hex(value) == Some(expected) {
        Ok(expected)
    } else {
        Err(Error::msg(format!(
            "{name} contract mismatch: expected {}, received {value}",
            expected.to_hex()
        )))
    }
}

pub(super) fn all() -> Result<std::collections::BTreeMap<String, String>> {
    [
        lkjscript_contracts::LANGUAGE,
        lkjscript_contracts::SOURCE,
        lkjscript_contracts::MODULE_INTERFACE,
        lkjscript_contracts::PACKAGE_MANIFEST,
        lkjscript_contracts::PACKAGE_LOCK,
    ]
    .into_iter()
    .map(|name| expected(name).map(|digest| (name.to_string(), digest.to_hex())))
    .collect()
}
