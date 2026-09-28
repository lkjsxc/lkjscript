//! Deployment names are conveniences; authority always resolves to an exact reference.
use super::{DeploymentGrant, deployment_error, validate_name};
use crate::platform::diagnostic::Diagnostic;
use crate::platform::kernel::RequirementReference;
use std::collections::BTreeMap;

enum Selector<'a> {
    Name(&'a str),
    Exact(RequirementReference),
}

pub(super) fn exact(reference: RequirementReference) -> String {
    format!("{}/{}", reference.package, reference.requirement)
}

fn parse(value: &str) -> Result<Selector<'_>, Diagnostic> {
    let Some((package, requirement)) = value.split_once('/') else {
        validate_name(value, "requirement")?;
        return Ok(Selector::Name(value));
    };
    let invalid = || {
        deployment_error(
            "deployment_grant_selector",
            "exact requirement selector must be pkg_<32 lowercase hex>/req_<32 lowercase hex>",
        )
    };
    Ok(Selector::Exact(RequirementReference {
        package: package.parse().map_err(|_| invalid())?,
        requirement: requirement.parse().map_err(|_| invalid())?,
    }))
}

pub(super) fn validate(value: &str) -> Result<(), Diagnostic> {
    parse(value).map(|_| ())
}

/// Resolve against the complete selected component, never against unmatched names.
/// Missing table entries stay visible to admission rather than being filtered out.
pub(super) fn resolve<'a, 'b>(
    grants: &'a [DeploymentGrant],
    required: impl IntoIterator<Item = Option<(RequirementReference, &'b str)>>,
) -> Result<BTreeMap<RequirementReference, &'a DeploymentGrant>, Diagnostic> {
    let mut references = BTreeMap::new();
    let mut names = BTreeMap::new();
    for required in required {
        let (reference, name) = required.ok_or_else(|| {
            deployment_error(
                "deployment_requirement_missing",
                "component requirement escaped the exact artifact table",
            )
        })?;
        if references.insert(reference, name).is_some() {
            return Err(deployment_error(
                "deployment_requirement_duplicate",
                "selected component repeats an exact requirement",
            ));
        }
        names
            .entry(name)
            .and_modify(|entry| *entry = None)
            .or_insert(Some(reference));
    }
    let mut selected = BTreeMap::new();
    for grant in grants {
        let foreign = || {
            deployment_error(
                "deployment_grant_foreign",
                format!(
                    "deployment grants undeclared component requirement '{}'",
                    grant.requirement
                ),
            )
        };
        let reference = match parse(&grant.requirement)? {
            Selector::Exact(reference) => {
                if !references.contains_key(&reference) {
                    return Err(foreign());
                }
                reference
            }
            Selector::Name(name) => match names.get(name) {
                Some(Some(reference)) => *reference,
                Some(None) => {
                    return Err(deployment_error(
                        "deployment_grant_ambiguous",
                        format!(
                            "component requirement name '{name}' is ambiguous; use an exact package/requirement selector"
                        ),
                    ));
                }
                None => return Err(foreign()),
            },
        };
        if selected.insert(reference, grant).is_some() {
            return Err(deployment_error(
                "deployment_grant_duplicate",
                format!(
                    "exact component requirement '{}' is granted twice",
                    exact(reference)
                ),
            ));
        }
    }
    if let Some((reference, name)) = references
        .iter()
        .find(|(reference, _)| !selected.contains_key(*reference))
    {
        return Err(deployment_error(
            "deployment_grant_missing",
            format!(
                "component requirement '{name}' ({}) has no deployment grant",
                exact(*reference)
            ),
        ));
    }
    Ok(selected)
}

#[cfg(test)]
#[path = "grant_selection_tests.rs"]
mod tests;
