//! Release tags are exact, opaque product identifiers, not compatibility ranges.
//! Every numeric component has the same role; no component denotes stability.

use super::ContainerError;

pub const MAXIMUM_TAG_BYTES: usize = 64;

/// Validate the bounded canonical spelling without allocating on success or parsing integers.
/// Numeric size/order has no role in selection or compatibility.
pub fn validate_tag(tag: &str) -> Result<(), ContainerError> {
    if tag.len() > MAXIMUM_TAG_BYTES {
        return Err(ContainerError::corrupt(
            "release tag exceeds the 64-byte identifier limit",
        ));
    }
    let Some(version) = tag.strip_prefix('v') else {
        return Err(ContainerError::corrupt("release tag must start with 'v'"));
    };
    let mut parts = version.split('.');
    let canonical = (0..3).all(|_| {
        parts.next().is_some_and(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part.len() == 1 || !part.starts_with('0'))
        })
    }) && parts.next().is_none();
    if !canonical {
        return Err(ContainerError::corrupt(
            "release tag must be exact vA.B.C with three canonical decimal identifiers",
        ));
    }
    Ok(())
}

/// Match one tag to one product selection. Sharing any prefix implies nothing.
pub fn validate_strict_tag(tag: &str, product_version: &str) -> Result<(), ContainerError> {
    validate_tag(tag)?;
    if tag.strip_prefix('v') != Some(product_version) {
        return Err(ContainerError::corrupt(format!(
            "release tag '{tag}' does not equal product version tag 'v{product_version}'"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_three_components_are_opaque_decimal_identifiers() {
        for a in ["0", "1", "7", "12345"] {
            for b in ["0", "1", "7", "12345"] {
                for c in ["0", "1", "7", "12345"] {
                    let version = format!("{a}.{b}.{c}");
                    let tag = format!("v{version}");
                    assert!(validate_tag(&tag).is_ok(), "{tag}");
                    assert!(validate_strict_tag(&tag, &version).is_ok(), "{tag}");
                }
            }
        }
    }

    #[test]
    fn exact_identity_never_falls_back_to_a_shared_component_or_range() {
        for version in ["0.0.0", "0.1.61", "0.2.0", "1.0.0", "1.1.0", "9.8.7"] {
            let tag = format!("v{version}");
            for selected in ["0.0.0", "0.1.61", "0.2.0", "1.0.0", "1.1.0", "9.8.7"] {
                assert_eq!(
                    validate_strict_tag(&tag, selected).is_ok(),
                    version == selected,
                    "{tag} versus {selected}"
                );
            }
            for selected in ["*", "^1", "~1.0", ">=0.1", "1.*", "latest", "v1.0.0"] {
                assert!(validate_strict_tag(&tag, selected).is_err());
            }
        }
    }

    #[test]
    fn malformed_components_are_rejected_in_every_position() {
        for invalid in [
            "", "00", "01", "-1", "+1", " 1", "1 ", "１", "a", "*", "1\n",
        ] {
            for position in 0..3 {
                let mut parts = ["1", "2", "3"];
                parts[position] = invalid;
                let tag = format!("v{}", parts.join("."));
                assert!(validate_tag(&tag).is_err(), "{tag:?}");
            }
        }
        for tag in [
            "",
            "v",
            "1.2.3",
            "V1.2.3",
            "vv1.2.3",
            "v1",
            "v1.2",
            "v1.2.3.4",
            "v1.2.3-rc.1",
            "v1.2.3+build",
            "v1.2.3/child",
            "v1.2.3\0",
            " v1.2.3",
            "latest",
            "^1.2.3",
            "~1.2.3",
            ">=1.2.3",
            "v1.2.*",
        ] {
            assert!(validate_tag(tag).is_err(), "{tag:?}");
        }
    }

    #[test]
    fn byte_bound_is_exact_without_integer_overflow_or_component_rank() {
        for position in 0..3 {
            let accepted = "9".repeat(59);
            let rejected = "9".repeat(60);
            for (component, expected) in [(&accepted, true), (&rejected, false)] {
                let mut parts = ["0", "0", "0"];
                parts[position] = component;
                let version = parts.join(".");
                let tag = format!("v{version}");
                assert_eq!(tag.len(), if expected { 64 } else { 65 });
                assert_eq!(validate_tag(&tag).is_ok(), expected, "{tag}");
                assert_eq!(validate_strict_tag(&tag, &version).is_ok(), expected);
            }
        }
    }
}
