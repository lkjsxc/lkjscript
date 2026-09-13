//! Typed, request-only addressing over one accepted base and explicit dependency selection.

use super::*;
use crate::platform::diagnostic::SourceLocation;
use crate::platform::kernel::PackageInterfaceDigest;

/// An index into the canonical, topologically ordered selector inventory, never an allocation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AuthoredReference(pub u32);

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AuthoredReferencePackage {
    Local,
    Exact {
        package: PackageId,
        package_revision: PackageRevisionDigest,
        /// The embedded shorthand supplies this additional exact expectation at normalization.
        semantic_revision: Option<RevisionId>,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AuthoredOwnerReferenceSelector {
    pub package: u32,
    pub parent: Option<AuthoredReference>,
    pub class: NamespaceClass,
    pub selection: AuthoredOwnerReferenceSelection,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AuthoredOwnerReferenceSelection {
    Name(Name),
    Declaration(DeclarationId),
}

/// Presentation and admission input. Neither labels nor physical input positions encode meaning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthoredReferenceOrigin {
    pub alias: String,
    pub owner: Option<AuthoredReference>,
    pub package: u32,
    pub location: SourceLocation,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AuthoredReferenceBindings {
    pub packages: Vec<AuthoredReferencePackage>,
    pub owners: Vec<AuthoredOwnerReferenceSelector>,
    pub origins: Vec<AuthoredReferenceOrigin>,
    pub raw_records: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedReferencePackage {
    pub selector: AuthoredReferencePackage,
    pub package: PackageId,
    pub semantic_revision: RevisionId,
    /// Local addressing is bound directly to the accepted base, without deriving an export.
    pub package_revision: Option<PackageRevisionDigest>,
    pub interface: Option<PackageInterfaceDigest>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedOwnerReference {
    pub selector: AuthoredOwnerReferenceSelector,
    pub package: PackageId,
    pub owner: OwnerKey,
    pub kind: OwnerKind,
    pub parent: Option<OwnerKey>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ResolvedReferenceBindings {
    pub packages: Vec<ResolvedReferencePackage>,
    pub owners: Vec<ResolvedOwnerReference>,
}

/// Checks the normalized representation as well as raw admission, including direct Rust callers.
pub(super) fn inventory(
    request: &AuthoredChangeSet,
) -> Result<Option<&AuthoredReferenceBindings>, Diagnostic> {
    let mut result = None;
    for (index, change) in request.changes.iter().enumerate() {
        if let AuthoredChange::ReferenceBindings { bindings } = change {
            if index != 0 || result.is_some() || request.changes.len() < 2 {
                return Err(reference_error(
                    "change_reference_inventory",
                    "reference bindings require one canonical prelude and an actual authored change",
                ));
            }
            if bindings.packages.is_empty()
                || bindings.raw_records != bindings.origins.len() as u64
                || bindings.raw_records == 0
                || bindings.raw_records > MAXIMUM_AUTHORED_CHANGES as u64
                || bindings.owners.len() > bindings.origins.len()
                || bindings.packages.len() > bindings.origins.len() + 1
                || bindings.packages.windows(2).any(|pair| pair[0] >= pair[1])
            {
                return Err(reference_error(
                    "change_reference_inventory",
                    "reference selector inventory is not canonical or exceeds raw admission",
                ));
            }
            let mut depths = Vec::new();
            let mut previous = None;
            for (index, selector) in bindings.owners.iter().enumerate() {
                if selector.package as usize >= bindings.packages.len()
                    || selector
                        .parent
                        .is_some_and(|parent| parent.0 as usize >= index)
                {
                    return Err(reference_error(
                        "change_reference_inventory",
                        "reference selector has an invalid package or non-topological parent",
                    ));
                }
                let depth = selector
                    .parent
                    .map_or(0usize, |parent| depths[parent.0 as usize] + 1);
                if depth > 3
                    || previous.is_some_and(|(prior_depth, prior)| {
                        (prior_depth, prior) >= (depth, selector)
                    })
                {
                    return Err(reference_error(
                        "change_reference_inventory",
                        "reference selectors must be unique and ordered by depth and typed path",
                    ));
                }
                previous = Some((depth, selector));
                depths.push(depth);
            }
            let mut names = BTreeSet::new();
            let mut declared_owners = BTreeSet::new();
            let mut declared_packages = BTreeSet::new();
            for origin in &bindings.origins {
                validate_symbol(&origin.alias)?;
                if !names.insert(&origin.alias)
                    || origin.package as usize >= bindings.packages.len()
                {
                    return Err(reference_error(
                        "change_reference_duplicate",
                        "reference alias is duplicated or has an invalid package",
                    ));
                }
                if let Some(owner) = origin.owner {
                    if bindings
                        .owners
                        .get(owner.0 as usize)
                        .is_none_or(|selector| selector.package != origin.package)
                    {
                        return Err(reference_error(
                            "change_reference_inventory",
                            "reference origin disagrees with its typed selector",
                        ));
                    }
                    declared_owners.insert(owner);
                } else {
                    declared_packages.insert(origin.package);
                }
            }
            if declared_owners.len() != bindings.owners.len()
                || bindings
                    .packages
                    .iter()
                    .enumerate()
                    .any(|(index, package)| {
                        *package != AuthoredReferencePackage::Local
                            && !declared_packages.contains(&(index as u32))
                    })
            {
                return Err(reference_error(
                    "change_reference_inventory",
                    "selector inventory contains undeclared paths",
                ));
            }
            result = Some(bindings);
        }
    }
    Ok(result)
}

pub(super) fn admitted_operations(request: &AuthoredChangeSet) -> Result<usize, Diagnostic> {
    let bindings = inventory(request)?;
    request
        .changes
        .len()
        .checked_add(bindings.map_or(0, |bindings| bindings.raw_records as usize - 1))
        .ok_or_else(|| {
            reference_error(
                "change_reference_count",
                "authored operation admission overflowed",
            )
        })
}

fn reference_error(code: &'static str, message: impl Into<String>) -> Diagnostic {
    request_error(DiagnosticClass::Semantic, code, message)
}

pub(super) fn wrong_domain() -> Diagnostic {
    request_error(
        DiagnosticClass::Corrupt,
        "change_reference_owner_domain",
        "resolved typed selector disagrees with its owner identity domain",
    )
}

fn located(
    mut diagnostic: Diagnostic,
    bindings: &AuthoredReferenceBindings,
    owner: Option<AuthoredReference>,
    package: u32,
) -> Diagnostic {
    if let Some(origin) = bindings
        .origins
        .iter()
        .find(|origin| origin.owner == owner && origin.package == package)
    {
        diagnostic.location = Some(origin.location.clone());
        diagnostic
            .notes
            .push(format!("reference alias {}", origin.alias));
    }
    diagnostic
}

struct InterfaceIndex {
    owners: BTreeMap<OwnerKey, crate::platform::kernel::PackageInterfaceRecord>,
    names: BTreeMap<NamespaceKey, Option<OwnerKey>>,
    interface: PackageInterfaceDigest,
}

impl<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized> AuthoredLowerer<'_, B, W> {
    pub(super) fn lower_dependency_changes(
        &mut self,
        changes: &[AuthoredChange],
        unique: bool,
    ) -> Result<(), Diagnostic> {
        let mut selected = BTreeSet::new();
        for change in changes {
            let (package, record) = match change {
                AuthoredChange::AddDependency {
                    package,
                    semantic_revision,
                    package_revision,
                }
                | AuthoredChange::ReplaceDependency {
                    package,
                    semantic_revision,
                    package_revision,
                } => (
                    *package,
                    Some(DependencyRecord {
                        graph_contract_version:
                            crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
                        package: *package,
                        semantic_revision: *semantic_revision,
                        package_revision: *package_revision,
                    }),
                ),
                AuthoredChange::DeleteDependency { package } => (*package, None),
                _ => continue,
            };
            if unique && !selected.insert(package) {
                return Err(reference_error(
                    "change_reference_dependency_edits",
                    format!(
                        "package {package} has more than one dependency edit; select one explicit final binding"
                    ),
                ));
            }
            match (change, record) {
                (AuthoredChange::AddDependency { .. }, Some(record)) => {
                    self.add_dependency(record)?
                }
                (_, Some(record)) => self.replace_dependency(record)?,
                (_, None) => self.delete_dependency(package)?,
            }
            self.check_budget("dependency lowering")?;
        }
        Ok(())
    }

    pub(super) fn resolve_bindings(
        &mut self,
        bindings: &AuthoredReferenceBindings,
    ) -> Result<(), Diagnostic> {
        let mut interfaces = BTreeMap::<PackageId, InterfaceIndex>::new();
        for (index, selector) in bindings.packages.iter().enumerate() {
            let resolved = (|| {
                Ok(match selector {
                    AuthoredReferencePackage::Local => ResolvedReferencePackage {
                        selector: selector.clone(), package: self.base.package_id(),
                        semantic_revision: self.base.exact_revision().ok_or_else(|| reference_error("change_reference_base", "local reference has no exact accepted base"))?,
                        package_revision: None, interface: None,
                    },
                    AuthoredReferencePackage::Exact { package, package_revision, semantic_revision } => {
                        self.load_dependency(*package)?;
                        let dependency = self.dependencies.get(package).and_then(|working| working.record.clone())
                            .ok_or_else(|| reference_error("change_reference_dependency_missing", format!("package {package} at {package_revision} is not selected by the candidate dependency closure; reference.package does not add a dependency")))?;
                        if dependency.package_revision != *package_revision || semantic_revision.is_some_and(|revision| revision != dependency.semantic_revision) {
                            return Err(reference_error(if semantic_revision.is_some() { "change_reference_supplier_mismatch" } else { "change_reference_dependency_revision" }, format!("selected package {package} is {} / {}, but reference requires {package_revision} / {semantic_revision:?}; re-plan with the selected supplier or use its explicit exact package binding", dependency.package_revision, dependency.semantic_revision)));
                        }
                        if !interfaces.contains_key(package) {
                            let read = self.base.read_reference_interface(&dependency)?;
                            self.work.canonical.add(read.work);
                            self.check_budget("reference interface admission")?;
                            read.value.revision.matches_dependency(*package_revision, &dependency)?;
                            let mut names = BTreeMap::new();
                            for (owner, record) in &read.value.owners {
                                self.work.ownership_steps = self.work.ownership_steps.checked_add(1).ok_or_else(|| reference_error("change_reference_work", "reference namespace work overflowed"))?;
                                self.check_budget("reference namespace indexing")?;
                                let key = interface_namespace(record);
                                names.entry(key).and_modify(|value| *value = None).or_insert(Some(*owner));
                            }
                            interfaces.insert(*package, InterfaceIndex { owners: read.value.owners, names, interface: read.value.revision.interface });
                        }
                        ResolvedReferencePackage { selector: selector.clone(), package: *package,
                            semantic_revision: dependency.semantic_revision, package_revision: Some(*package_revision),
                            interface: Some(interfaces[package].interface),
                        }
                    }
                })
            })().map_err(|error| located(error, bindings, None, index as u32))?;
            self.resolutions.packages.push(resolved);
        }
        for (index, selector) in bindings.owners.iter().enumerate() {
            let reference = AuthoredReference(index as u32);
            let resolved = self
                .resolve_binding(selector, &interfaces)
                .map_err(|mut error| {
                    let package = &self.resolutions.packages[selector.package as usize];
                    let selection = match &selector.selection {
                        AuthoredOwnerReferenceSelection::Name(name) => format!("name={name}"),
                        AuthoredOwnerReferenceSelection::Declaration(owner) => {
                            format!("owner={owner}")
                        }
                    };
                    let parent = selector.parent.map_or_else(
                        || "package-root".to_owned(),
                        |parent| self.resolutions.owners[parent.0 as usize].owner.to_string(),
                    );
                    error.notes.push(format!(
                        "class={} {selection} parent={parent} package={} semantic-revision={} package-revision={} interface={}",
                        selector.class.name(),
                        package.package,
                        package.semantic_revision,
                        package.package_revision.map_or_else(|| "local-base".to_owned(), |revision| revision.to_string()),
                        package.interface.map_or_else(|| "local-base".to_owned(), |interface| interface.to_string()),
                    ));
                    located(error, bindings, Some(reference), selector.package)
                })?;
            self.resolutions.owners.push(resolved);
            self.work.ownership_steps =
                self.work.ownership_steps.checked_add(1).ok_or_else(|| {
                    reference_error(
                        "change_reference_work",
                        "reference selector work overflowed",
                    )
                })?;
            self.check_budget("reference selector resolution")?;
        }
        Ok(())
    }

    fn resolve_binding(
        &mut self,
        selector: &AuthoredOwnerReferenceSelector,
        interfaces: &BTreeMap<PackageId, InterfaceIndex>,
    ) -> Result<ResolvedOwnerReference, Diagnostic> {
        let package = self.resolutions.packages[selector.package as usize].package;
        let local = matches!(
            self.resolutions.packages[selector.package as usize].selector,
            AuthoredReferencePackage::Local
        );
        let parent = selector
            .parent
            .map(|reference| &self.resolutions.owners[reference.0 as usize]);
        if parent.is_some_and(|parent| parent.package != package) {
            return Err(reference_error(
                "change_reference_parent_package",
                "parent belongs to a different exact package selection",
            ));
        }
        let parent_kind = parent.map(|parent| parent.kind);
        let parent_owner = parent.map(|parent| parent.owner);
        let valid = match selector.class {
            NamespaceClass::Module | NamespaceClass::Target => local && parent.is_none(),
            NamespaceClass::Declaration => {
                if local {
                    parent_kind == Some(OwnerKind::Module)
                } else {
                    parent.is_none()
                }
            }
            NamespaceClass::Field => parent_kind == Some(OwnerKind::Record),
            NamespaceClass::Case => parent_kind == Some(OwnerKind::Variant),
            NamespaceClass::Operation => parent_kind == Some(OwnerKind::Interface),
            NamespaceClass::Requirement | NamespaceClass::Port => {
                parent_kind == Some(OwnerKind::Component)
            }
            NamespaceClass::Parameter => matches!(
                parent_kind,
                Some(
                    OwnerKind::PureFunction
                        | OwnerKind::TaskFunction
                        | OwnerKind::External
                        | OwnerKind::Operation
                )
            ),
            NamespaceClass::TypeParameter => matches!(
                parent_kind,
                Some(
                    OwnerKind::Record
                        | OwnerKind::Variant
                        | OwnerKind::PureFunction
                        | OwnerKind::TaskFunction
                        | OwnerKind::External
                )
            ),
            NamespaceClass::EffectParameter => matches!(
                parent_kind,
                Some(OwnerKind::PureFunction | OwnerKind::TaskFunction)
            ),
        };
        if !valid {
            return Err(reference_error(
                "change_reference_parent_kind",
                "selected parent kind does not own this namespace; dependency interfaces expose no modules or targets",
            ));
        }
        let owner = match &selector.selection {
            AuthoredOwnerReferenceSelection::Declaration(owner)
                if !local && selector.class == NamespaceClass::Declaration && parent.is_none() =>
            {
                OwnerKey::Declaration(*owner)
            }
            AuthoredOwnerReferenceSelection::Declaration(_) => {
                return Err(reference_error(
                    "change_reference_exact_class",
                    "only a foreign declaration may use the exact declaration escape hatch",
                ));
            }
            AuthoredOwnerReferenceSelection::Name(name) => {
                let key = NamespaceKey {
                    parent: parent_owner,
                    class: selector.class,
                    name: name.clone(),
                };
                if local {
                    self.namespace_owner(key)?
                } else {
                    match interfaces[&package].names.get(&key) {
                        Some(Some(owner)) => *owner,
                        Some(None) => {
                            return Err(reference_error(
                                "change_reference_ambiguous",
                                "multiple exported declarations have this exact name; select the intended declaration with owner=DECLARATION_ID",
                            ));
                        }
                        None => {
                            return Err(reference_error(
                                "change_reference_not_exposed",
                                "no legally exposed owner has this name under the exact interface parent; private, removed and unexposed namespaces cannot be bound",
                            ));
                        }
                    }
                }
            }
        };
        let kind = if local {
            self.require_owner(owner)?;
            let record = &self.owners[&owner].record;
            let namespace = crate::platform::kernel::owner_namespace(record).ok_or_else(|| {
                reference_error(
                    "change_reference_namespace",
                    "owner has no canonical named namespace",
                )
            })?;
            if namespace.class != selector.class || namespace.parent != parent_owner {
                return Err(reference_error(
                    "change_reference_namespace",
                    "selected owner disagrees with exact parent and namespace class",
                ));
            }
            record.kind()
        } else {
            let record = interfaces[&package].owners.get(&owner).ok_or_else(|| {
                reference_error(
                    "change_reference_not_exposed",
                    "the selected exact declaration is absent from the admitted public interface",
                )
            })?;
            let namespace = interface_namespace(record);
            if namespace.class != selector.class || namespace.parent != parent_owner {
                return Err(reference_error(
                    "change_reference_namespace",
                    "interface owner disagrees with exact parent and namespace class",
                ));
            }
            record.header().kind
        };
        Ok(ResolvedOwnerReference {
            selector: selector.clone(),
            package,
            owner,
            kind,
            parent: parent_owner,
        })
    }

    pub(super) fn selected(
        &self,
        reference: AuthoredReference,
        class: Option<NamespaceClass>,
        local: bool,
    ) -> Result<&ResolvedOwnerReference, Diagnostic> {
        let selected = self
            .resolutions
            .owners
            .get(reference.0 as usize)
            .ok_or_else(|| {
                reference_error(
                    "change_reference_index",
                    "typed reference is absent from the resolved inventory",
                )
            })?;
        if class.is_some_and(|class| class != selected.selector.class) {
            return Err(reference_error(
                "change_reference_use_kind",
                "reference alias has the wrong namespace class for this typed position",
            ));
        }
        if local && selected.package != self.base.package_id() {
            return Err(reference_error(
                "change_reference_foreign_mutation",
                "a foreign owner cannot be used as a local mutation or lexical selector",
            ));
        }
        Ok(selected)
    }
}

fn interface_namespace(record: &crate::platform::kernel::PackageInterfaceRecord) -> NamespaceKey {
    use crate::platform::kernel::{PackageInterfaceRecord as R, ParameterParent};
    let (parent, class, name) = match record {
        R::Declaration(record) => (None, NamespaceClass::Declaration, &record.name),
        R::TypeParameter(record) => (
            Some(OwnerKey::Declaration(record.declaration)),
            NamespaceClass::TypeParameter,
            &record.name,
        ),
        R::EffectParameter(record) => (
            Some(OwnerKey::Declaration(record.declaration)),
            NamespaceClass::EffectParameter,
            &record.name,
        ),
        R::Field(record) => (
            Some(OwnerKey::Declaration(record.declaration)),
            NamespaceClass::Field,
            &record.name,
        ),
        R::Case(record) => (
            Some(OwnerKey::Declaration(record.declaration)),
            NamespaceClass::Case,
            &record.name,
        ),
        R::Operation(record) => (
            Some(OwnerKey::Declaration(record.declaration)),
            NamespaceClass::Operation,
            &record.name,
        ),
        R::Requirement(record) => (
            Some(OwnerKey::Declaration(record.declaration)),
            NamespaceClass::Requirement,
            &record.name,
        ),
        R::Port(record) => (
            Some(OwnerKey::Declaration(record.declaration)),
            NamespaceClass::Port,
            &record.name,
        ),
        R::Parameter(record) => (
            Some(match record.parent {
                ParameterParent::Function(owner) => OwnerKey::Declaration(owner),
                ParameterParent::Operation(owner) => OwnerKey::Operation(owner),
            }),
            NamespaceClass::Parameter,
            &record.name,
        ),
    };
    NamespaceKey {
        parent,
        class,
        name: name.clone(),
    }
}
