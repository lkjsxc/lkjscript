//! Review inventory codec. Labels are absent; apply independently resolves authored selectors.

use super::*;
use crate::platform::change::{
    AuthoredOwnerReferenceSelection, AuthoredOwnerReferenceSelector, AuthoredReference,
    AuthoredReferencePackage, ResolvedOwnerReference, ResolvedReferenceBindings,
    ResolvedReferencePackage,
};
use crate::platform::kernel::{NamespaceClass, PackageInterfaceDigest};

pub(super) fn encode<F>(
    bindings: &ResolvedReferenceBindings,
    encoder: &mut PlanEncoder<'_, F>,
) -> Result<(), Diagnostic>
where
    F: FnMut(&[u8]) -> Result<(), Diagnostic>,
{
    for (index, package) in bindings.packages.iter().enumerate() {
        encoder.append(
            "logical-plan.reference-package",
            &[
                ("index", index.to_string()),
                (
                    "scope",
                    if matches!(package.selector, AuthoredReferencePackage::Local) {
                        "local"
                    } else {
                        "exact"
                    }
                    .to_owned(),
                ),
                ("package", package.package.to_string()),
                ("semantic-revision", package.semantic_revision.to_string()),
                (
                    "package-revision",
                    package
                        .package_revision
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                ),
                (
                    "interface",
                    package
                        .interface
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                ),
                (
                    "supplier-bound",
                    matches!(
                        package.selector,
                        AuthoredReferencePackage::Exact {
                            semantic_revision: Some(_),
                            ..
                        }
                    )
                    .to_string(),
                ),
            ],
        )?;
    }
    for (index, owner) in bindings.owners.iter().enumerate() {
        let (selection, name, exact) = match &owner.selector.selection {
            AuthoredOwnerReferenceSelection::Name(name) => {
                ("name", name.to_string(), String::new())
            }
            AuthoredOwnerReferenceSelection::Declaration(owner) => {
                ("declaration", String::new(), owner.to_string())
            }
        };
        encoder.append(
            "logical-plan.reference-owner",
            &[
                ("index", index.to_string()),
                ("package-index", owner.selector.package.to_string()),
                (
                    "parent",
                    owner
                        .selector
                        .parent
                        .map(|parent| parent.0.to_string())
                        .unwrap_or_else(|| "package-root".to_owned()),
                ),
                ("class", owner.selector.class.name().to_owned()),
                ("selection", selection.to_owned()),
                ("name", name),
                ("declaration", exact),
                ("package", owner.package.to_string()),
                ("owner", owner.owner.to_string()),
                ("kind", owner.kind.name().to_owned()),
                (
                    "parent-owner",
                    owner
                        .parent
                        .map(|parent| parent.to_string())
                        .unwrap_or_else(|| "package-root".to_owned()),
                ),
            ],
        )?;
    }
    if !bindings.packages.is_empty() {
        encoder.append(
            "logical-plan.reference-counts",
            &[
                ("packages", bindings.packages.len().to_string()),
                ("owners", bindings.owners.len().to_string()),
            ],
        )?;
    }
    Ok(())
}

#[derive(Default)]
pub(super) struct Decoder {
    bindings: ResolvedReferenceBindings,
    counts: bool,
}

impl Decoder {
    pub(super) fn accept(&mut self, record: &CompactRecord, kind: usize) -> Result<(), Diagnostic> {
        if self.counts {
            return Err(invalid(
                "reference inventory follows its counts or duplicates them",
            ));
        }
        let count = if kind == 0 {
            self.bindings.packages.len()
        } else {
            self.bindings.owners.len()
        };
        if kind < 2
            && (count >= crate::platform::change::MAXIMUM_AUTHORED_CHANGES
                || parse_u64(field(record, 0), "reference index")? != count as u64)
        {
            return Err(invalid(
                "reference inventory index is not consecutive or exceeds admission",
            ));
        }
        match kind {
            0 => {
                let package = field(record, 2).parse::<PackageId>()?;
                let semantic_revision = field(record, 3).parse::<RevisionId>()?;
                let package_revision = optional_parse::<PackageRevisionDigest>(field(record, 4))?;
                let interface = optional_parse::<PackageInterfaceDigest>(field(record, 5))?;
                let supplier = parse_bool(field(record, 6), "supplier expectation")?;
                let selector = match (field(record, 1), package_revision, interface, supplier) {
                    ("local", None, None, false) => AuthoredReferencePackage::Local,
                    ("exact", Some(package_revision), Some(_), _) => {
                        AuthoredReferencePackage::Exact {
                            package,
                            package_revision,
                            semantic_revision: supplier.then_some(semantic_revision),
                        }
                    }
                    _ => {
                        return Err(invalid(
                            "reference package scope disagrees with exact revision/interface expectations",
                        ));
                    }
                };
                self.bindings.packages.push(ResolvedReferencePackage {
                    selector,
                    package,
                    semantic_revision,
                    package_revision,
                    interface,
                });
            }
            1 => {
                let package = field(record, 7).parse::<PackageId>()?;
                let owner = field(record, 8).parse::<OwnerKey>()?;
                let kind = OwnerKind::parse(field(record, 9))?;
                let parent = if field(record, 10) == "package-root" {
                    None
                } else {
                    Some(field(record, 10).parse::<OwnerKey>()?)
                };
                let selector = AuthoredOwnerReferenceSelector {
                    package: parse_index(field(record, 1))?,
                    parent: if field(record, 2) == "package-root" {
                        None
                    } else {
                        Some(AuthoredReference(parse_index(field(record, 2))?))
                    },
                    class: NamespaceClass::parse(field(record, 3))?,
                    selection: match (field(record, 4), field(record, 5), field(record, 6)) {
                        ("name", name, "") => {
                            AuthoredOwnerReferenceSelection::Name(Name::new(name)?)
                        }
                        ("declaration", "", owner) => AuthoredOwnerReferenceSelection::Declaration(
                            owner.parse::<DeclarationId>()?,
                        ),
                        _ => {
                            return Err(invalid(
                                "reference owner selection has inconsistent name/exact fields",
                            ));
                        }
                    },
                };
                self.bindings.owners.push(ResolvedOwnerReference {
                    selector,
                    package,
                    owner,
                    kind,
                    parent,
                });
            }
            2 => {
                if self.bindings.packages.is_empty()
                    || parse_u64(field(record, 0), "reference packages")?
                        != self.bindings.packages.len() as u64
                    || parse_u64(field(record, 1), "reference owners")?
                        != self.bindings.owners.len() as u64
                {
                    return Err(invalid(
                        "reference counts disagree with the complete inventory",
                    ));
                }
                validate(&self.bindings)?;
                self.counts = true;
            }
            _ => return Err(invalid("unknown reference inventory record")),
        }
        Ok(())
    }

    pub(super) fn finish(&self) -> Result<(), Diagnostic> {
        if !self.bindings.packages.is_empty() && !self.counts {
            return Err(invalid("reference inventory has no complete counts record"));
        }
        Ok(())
    }
}

fn optional_parse<T: FromStr<Err = Diagnostic>>(value: &str) -> Result<Option<T>, Diagnostic> {
    if value.is_empty() {
        Ok(None)
    } else {
        value.parse::<T>().map(Some)
    }
}

fn parse_index(value: &str) -> Result<u32, Diagnostic> {
    u32::try_from(parse_u64(value, "reference index")?)
        .map_err(|_| invalid("reference index overflowed"))
}

pub(super) fn validate(bindings: &ResolvedReferenceBindings) -> Result<(), Diagnostic> {
    if bindings
        .packages
        .windows(2)
        .any(|pair| pair[0].selector >= pair[1].selector)
    {
        return Err(invalid(
            "reference packages are not in unique canonical selector order",
        ));
    }
    let mut previous = None;
    let mut depths = Vec::new();
    for (index, owner) in bindings.owners.iter().enumerate() {
        let package = bindings
            .packages
            .get(owner.selector.package as usize)
            .ok_or_else(|| invalid("reference owner has no selected package"))?;
        let parent = match owner.selector.parent {
            Some(parent) if (parent.0 as usize) < index => {
                Some(&bindings.owners[parent.0 as usize])
            }
            Some(_) => return Err(invalid("reference parent is not a preceding typed path")),
            None => None,
        };
        let depth = owner
            .selector
            .parent
            .map_or(0usize, |parent| depths[parent.0 as usize] + 1);
        if depth > 3 || previous.is_some_and(|prior| prior >= (depth, &owner.selector)) {
            return Err(invalid(
                "reference owner paths are not in unique canonical order",
            ));
        }
        depths.push(depth);
        previous = Some((depth, &owner.selector));
        if package.package != owner.package
            || parent.is_some_and(|parent| parent.package != owner.package)
            || parent.map(|parent| parent.owner) != owner.parent
            || !owner.kind.accepts_owner(owner.owner)
        {
            return Err(invalid(
                "reference resolution disagrees with its exact package, parent or owner kind",
            ));
        }
        if let AuthoredOwnerReferenceSelection::Declaration(declaration) = owner.selector.selection
            && (matches!(package.selector, AuthoredReferencePackage::Local)
                || owner.selector.class != NamespaceClass::Declaration
                || owner.parent.is_some()
                || owner.owner != OwnerKey::Declaration(declaration))
        {
            return Err(invalid(
                "exact declaration escape hatch disagrees with its resolved public owner",
            ));
        }
    }
    Ok(())
}

fn invalid(message: &str) -> Diagnostic {
    plan_source_error("change_plan_reference_inventory", message)
}
