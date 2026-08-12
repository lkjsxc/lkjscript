use lkjscript_contracts::ContractDigest;
use lkjscript_core::{Error, Result};

pub(super) fn expected(name: &str) -> Result<ContractDigest> {
    match name {
        lkjscript_contracts::LANGUAGE => Ok(lkjscript_contracts::LANGUAGE_DIGEST),
        lkjscript_contracts::SOURCE => Ok(lkjscript_contracts::SOURCE_DIGEST),
        lkjscript_contracts::MODULE_INTERFACE => Ok(lkjscript_contracts::MODULE_INTERFACE_DIGEST),
        lkjscript_contracts::PACKAGE_MANIFEST => Ok(lkjscript_contracts::PACKAGE_MANIFEST_DIGEST),
        lkjscript_contracts::PACKAGE_LOCK => Ok(lkjscript_contracts::PACKAGE_LOCK_DIGEST),
        _ => Err(Error::msg(format!("unknown package contract {name}"))),
    }
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

pub(super) fn all() -> std::collections::BTreeMap<String, String> {
    [
        (
            lkjscript_contracts::LANGUAGE,
            lkjscript_contracts::LANGUAGE_DIGEST,
        ),
        (
            lkjscript_contracts::SOURCE,
            lkjscript_contracts::SOURCE_DIGEST,
        ),
        (
            lkjscript_contracts::MODULE_INTERFACE,
            lkjscript_contracts::MODULE_INTERFACE_DIGEST,
        ),
        (
            lkjscript_contracts::PACKAGE_MANIFEST,
            lkjscript_contracts::PACKAGE_MANIFEST_DIGEST,
        ),
        (
            lkjscript_contracts::PACKAGE_LOCK,
            lkjscript_contracts::PACKAGE_LOCK_DIGEST,
        ),
    ]
    .into_iter()
    .map(|(name, digest)| (name.to_string(), digest.to_hex()))
    .collect()
}
