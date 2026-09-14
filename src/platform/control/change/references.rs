//! Closed reference records and memoized typed selector paths. No text substitution occurs here.

use super::*;

#[cfg(test)]
mod tests;
use crate::platform::change::{
    AuthoredOwnerReferenceSelection, AuthoredOwnerReferenceSelector, AuthoredReference,
    AuthoredReferenceBindings, AuthoredReferenceOrigin, AuthoredReferencePackage,
};

#[derive(Clone)]
enum Raw {
    Package(AuthoredReferencePackage),
    Owner {
        package: Option<String>,
        parent: Option<String>,
        class: NamespaceClass,
        selection: AuthoredOwnerReferenceSelection,
    },
}

#[derive(Clone, Copy)]
enum Alias {
    Package(u32),
    Owner(AuthoredReference),
}

#[derive(Default)]
pub(super) struct ReferenceLookup {
    pub(super) bindings: AuthoredReferenceBindings,
    aliases: BTreeMap<String, Alias>,
}

impl ReferenceLookup {
    pub(super) fn decode(records: &[CompactRecord]) -> Result<Self, Diagnostic> {
        let mut raw = BTreeMap::new();
        let mut packages = BTreeSet::new();
        for record in records {
            let value = match record.operation.as_str() {
                "reference.package" => {
                    check_fields(record, &["as", "source", "package", "package-revision"])?;
                    let package = match (
                        optional(record, "source"),
                        optional(record, "package"),
                        optional(record, "package-revision"),
                    ) {
                        (Some("builtin"), None, None) => {
                            let builtin =
                                crate::platform::builtin_standard::BuiltinStandard::load()?;
                            AuthoredReferencePackage::Exact {
                                package: builtin.package,
                                package_revision: builtin.package_revision,
                                semantic_revision: Some(builtin.semantic_revision),
                            }
                        }
                        (None, Some(_), Some(_)) => AuthoredReferencePackage::Exact {
                            package: parse_field(record, "package")?,
                            package_revision: parse_field(record, "package-revision")?,
                            semantic_revision: None,
                        },
                        _ => {
                            return Err(record_error(
                                record,
                                "change_reference_package_form",
                                "reference.package requires either source=builtin or exact package and package-revision",
                            ));
                        }
                    };
                    packages.insert(package.clone());
                    Raw::Package(package)
                }
                "reference.owner" => {
                    check_fields(
                        record,
                        &["as", "package", "class", "name", "parent", "owner"],
                    )?;
                    let package = match required(record, "package")? {
                        "local" => {
                            packages.insert(AuthoredReferencePackage::Local);
                            None
                        }
                        value => {
                            validate_local_label(record, "package", value, '$')?;
                            Some(value.to_owned())
                        }
                    };
                    let class_name = required(record, "class")?;
                    let class = COMPACT_NAMESPACE_CLASSES
                        .iter()
                        .find_map(|(name, class)| (*name == class_name).then_some(*class))
                        .ok_or_else(|| {
                            field_error(
                                record,
                                "class",
                                "change_reference_class",
                                "unknown reference namespace class",
                            )
                        })?;
                    let parent = optional(record, "parent")
                        .map(|value| {
                            validate_local_label(record, "parent", value, '$')?;
                            Ok::<_, Diagnostic>(value.to_owned())
                        })
                        .transpose()?;
                    let selection = match (optional(record, "name"), optional(record, "owner")) {
                        (Some(_), None) => {
                            AuthoredOwnerReferenceSelection::Name(parse_name(record, "name")?)
                        }
                        (None, Some(_))
                            if class == NamespaceClass::Declaration
                                && package.is_some()
                                && parent.is_none() =>
                        {
                            AuthoredOwnerReferenceSelection::Declaration(parse_field(
                                record, "owner",
                            )?)
                        }
                        _ => {
                            return Err(record_error(
                                record,
                                "change_reference_owner_form",
                                "reference.owner requires a name; only a foreign declaration may instead select an exact owner without name/parent",
                            ));
                        }
                    };
                    Raw::Owner {
                        package,
                        parent,
                        class,
                        selection,
                    }
                }
                _ => continue,
            };
            let alias = symbol(record, "as")?;
            if raw.insert(alias.clone(), (value, record)).is_some() {
                return Err(field_error(
                    record,
                    "as",
                    "change_reference_duplicate",
                    format!("reference alias {alias} is defined more than once"),
                ));
            }
        }
        let packages = packages.into_iter().collect::<Vec<_>>();
        let package_index = |package: &AuthoredReferencePackage| -> Result<u32, Diagnostic> {
            packages
                .binary_search(package)
                .ok()
                .and_then(|index| u32::try_from(index).ok())
                .ok_or_else(|| {
                    Diagnostic::new(
                        DiagnosticClass::Infrastructure,
                        "change_reference_inventory",
                        "normalized package selector is absent",
                    )
                })
        };
        let mut aliases = BTreeMap::new();
        for (alias, (value, _)) in &raw {
            if let Raw::Package(package) = value {
                aliases.insert(alias.clone(), Alias::Package(package_index(package)?));
            }
        }
        // Iterative DFS admits at most one node per raw declaration and rejects cycles before
        // class checking. Shared parents are visited once; there is no recursive alias expansion.
        let mut states = BTreeMap::new();
        let mut order = Vec::new();
        for alias in raw.keys() {
            let mut stack = vec![(alias.as_str(), false)];
            while let Some((alias, exiting)) = stack.pop() {
                let (value, record) = &raw[alias];
                if exiting {
                    states.insert(alias, 2);
                    order.push(alias);
                    continue;
                }
                match states.get(alias) {
                    Some(2) => continue,
                    Some(1) => {
                        return Err(record_error(
                            record,
                            "change_reference_cycle",
                            format!("reference alias {alias} participates in a parent cycle"),
                        ));
                    }
                    _ => {}
                }
                states.insert(alias, 1);
                stack.push((alias, true));
                if let Raw::Owner {
                    parent: Some(parent),
                    ..
                } = value
                {
                    if !raw.contains_key(parent) {
                        return Err(field_error(
                            record,
                            "parent",
                            "change_reference_missing_alias",
                            format!("parent alias {parent} has no definition"),
                        ));
                    }
                    stack.push((parent, false));
                }
            }
        }
        let mut temporary: Vec<(usize, AuthoredOwnerReferenceSelector)> = Vec::new();
        for alias in order {
            let (value, record) = &raw[alias];
            let Raw::Owner {
                package,
                parent,
                class,
                selection,
            } = value
            else {
                continue;
            };
            let package = match package {
                None => package_index(&AuthoredReferencePackage::Local)?,
                Some(alias) => match aliases.get(alias) {
                    Some(Alias::Package(package)) => *package,
                    _ => {
                        return Err(field_error(
                            record,
                            "package",
                            "change_reference_package_alias",
                            format!("package alias {alias} must denote a declared exact package"),
                        ));
                    }
                },
            };
            let parent = match parent {
                None => None,
                Some(alias) => match aliases.get(alias) {
                    Some(Alias::Owner(owner)) => Some(*owner),
                    _ => {
                        return Err(field_error(
                            record,
                            "parent",
                            "change_reference_parent_alias",
                            format!("parent alias {alias} must denote an existing owner"),
                        ));
                    }
                },
            };
            let parent_node = parent.map(|parent| &temporary[parent.0 as usize]);
            let local = packages[package as usize] == AuthoredReferencePackage::Local;
            let parent_class = parent_node.map(|(_, selector)| selector.class);
            let valid = match class {
                NamespaceClass::Module | NamespaceClass::Target => local && parent.is_none(),
                NamespaceClass::Declaration if local => {
                    parent_class == Some(NamespaceClass::Module)
                }
                NamespaceClass::Declaration => parent.is_none(),
                NamespaceClass::Parameter => matches!(
                    parent_class,
                    Some(NamespaceClass::Declaration | NamespaceClass::Operation)
                ),
                _ => parent_class == Some(NamespaceClass::Declaration),
            };
            if !valid {
                return Err(record_error(
                    record,
                    "change_reference_parent_class",
                    format!(
                        "alias {alias}: class {} has an invalid parent or foreign root; dependency interfaces expose no modules or targets",
                        class.name()
                    ),
                ));
            }
            if let Some((_, parent_node)) = parent_node
                && !same_package(
                    &packages[package as usize],
                    &packages[parent_node.package as usize],
                )
            {
                return Err(field_error(
                    record,
                    "parent",
                    "change_reference_parent_package",
                    format!(
                        "alias {alias}: parent must belong to the same exact package selection"
                    ),
                ));
            }
            let depth = parent_node.map_or(0, |(depth, _)| depth + 1);
            let reference = AuthoredReference(index(temporary.len())?);
            temporary.push((
                depth,
                AuthoredOwnerReferenceSelector {
                    package,
                    parent,
                    class: *class,
                    selection: selection.clone(),
                },
            ));
            aliases.insert(alias.to_owned(), Alias::Owner(reference));
        }
        let mut remap = vec![AuthoredReference(0); temporary.len()];
        let mut owners = Vec::new();
        let maximum_depth = temporary.iter().map(|(depth, _)| *depth).max().unwrap_or(0);
        for depth in 0..=maximum_depth {
            let mut layer: BTreeMap<AuthoredOwnerReferenceSelector, Vec<usize>> = BTreeMap::new();
            for (old, (observed_depth, selector)) in temporary.iter().enumerate() {
                if *observed_depth == depth {
                    let mut selector = selector.clone();
                    selector.parent = selector.parent.map(|parent| remap[parent.0 as usize]);
                    layer.entry(selector).or_default().push(old);
                }
            }
            for (selector, previous) in layer {
                let reference = AuthoredReference(index(owners.len())?);
                for previous in previous {
                    remap[previous] = reference;
                }
                owners.push(selector);
            }
        }
        let mut origins = Vec::new();
        for (alias, value) in &mut aliases {
            let (owner, package) = match value {
                Alias::Owner(reference) => {
                    *reference = remap[reference.0 as usize];
                    (Some(*reference), owners[reference.0 as usize].package)
                }
                Alias::Package(package) => (None, *package),
            };
            origins.push(AuthoredReferenceOrigin {
                alias: alias.clone(),
                owner,
                package,
                location: raw[alias].1.location.clone(),
            });
        }
        Ok(Self {
            bindings: AuthoredReferenceBindings {
                packages,
                owners,
                origins,
                raw_records: raw.len() as u64,
            },
            aliases,
        })
    }

    pub(super) fn owner(
        &self,
        record: &CompactRecord,
        field: &str,
        class: Option<NamespaceClass>,
        local: bool,
    ) -> Result<Option<AuthoredReference>, Diagnostic> {
        let alias = required(record, field)?;
        let Some(value) = self.aliases.get(alias) else {
            return Ok(None);
        };
        let Alias::Owner(reference) = value else {
            return Err(field_error(
                record,
                field,
                "change_reference_use_kind",
                format!("package alias {alias} is not an owner reference"),
            ));
        };
        let selector = &self.bindings.owners[reference.0 as usize];
        if class.is_some_and(|expected| expected != selector.class) {
            return Err(field_error(
                record,
                field,
                "change_reference_use_kind",
                format!(
                    "alias {alias} selects {}, which is not accepted in {field}",
                    selector.class.name()
                ),
            ));
        }
        if local
            && self.bindings.packages[selector.package as usize] != AuthoredReferencePackage::Local
        {
            return Err(field_error(
                record,
                field,
                "change_reference_foreign_local",
                format!("foreign alias {alias} cannot select a local owner or acquire local scope"),
            ));
        }
        Ok(Some(*reference))
    }
}

impl Decoder {
    pub(super) fn decode_precondition(
        &self,
        record: &CompactRecord,
    ) -> Result<AuthoredPrecondition, Diagnostic> {
        use crate::platform::change::{
            AuthoredExistingOwner as O, AuthoredSelectedPrecondition as P,
        };
        if !record.fields.iter().any(|field| {
            matches!(field.name.as_str(), "owner" | "parent")
                && self.references.aliases.contains_key(&field.value)
        }) {
            return decode_precondition(record);
        }
        check_precondition_fields(record, COMPACT_CHANGE_PRECONDITION_FIELDS)?;
        let owner = |field| -> Result<O, Diagnostic> {
            match self.references.owner(record, field, None, true)? {
                Some(reference) => Ok(O::Selected(reference)),
                None => Ok(O::Exact(parse_field(record, field)?)),
            }
        };
        let parent = || -> Result<Option<O>, Diagnostic> {
            match optional(record, "parent") {
                None | Some("package") => Ok(None),
                Some(_) => owner("parent").map(Some),
            }
        };
        let condition = match record.operation.as_str() {
            "precondition.owner-exists" => P::OwnerExists {
                owner: owner("owner")?,
            },
            "precondition.owner-absent" => P::OwnerAbsent {
                owner: owner("owner")?,
            },
            "precondition.owner-name" => P::OwnerName {
                owner: owner("owner")?,
                equals: parse_name(record, "name")?,
            },
            "precondition.owner-parent" => P::OwnerParent {
                owner: owner("owner")?,
                equals: parent()?,
            },
            "precondition.namespace-absent" => P::NamespaceAbsent {
                parent: parent()?,
                class: parse_namespace_class(record, "class")?,
                name: parse_name(record, "name")?,
            },
            "precondition.namespace-points-to" => P::NamespacePointsTo {
                parent: parent()?,
                class: parse_namespace_class(record, "class")?,
                name: parse_name(record, "name")?,
                owner: owner("owner")?,
            },
            _ => return decode_precondition(record),
        };
        Ok(AuthoredPrecondition::Selected { condition })
    }

    pub(super) fn parse_module_selector(
        &self,
        record: &CompactRecord,
        field: &str,
    ) -> Result<ModuleSelector, Diagnostic> {
        match self
            .references
            .owner(record, field, Some(NamespaceClass::Module), true)?
        {
            Some(reference) => Ok(ModuleSelector::Selected { reference }),
            None => parse_module_selector(record, field),
        }
    }
    pub(super) fn parse_declaration_selector(
        &self,
        record: &CompactRecord,
        field: &str,
    ) -> Result<DeclarationSelector, Diagnostic> {
        match self
            .references
            .owner(record, field, Some(NamespaceClass::Declaration), true)?
        {
            Some(reference) => Ok(DeclarationSelector::Selected { reference }),
            None => parse_declaration_selector(record, field),
        }
    }
    pub(super) fn parse_owner_selector(
        &self,
        record: &CompactRecord,
        field: &str,
    ) -> Result<OwnerSelector, Diagnostic> {
        match self.references.owner(record, field, None, true)? {
            Some(reference) => Ok(OwnerSelector::Selected { reference }),
            None => parse_owner_selector(record, field),
        }
    }
    pub(super) fn parse_declaration_reference(
        &self,
        record: &CompactRecord,
        field: &str,
    ) -> Result<AuthoredDeclarationReference, Diagnostic> {
        match self
            .references
            .owner(record, field, Some(NamespaceClass::Declaration), false)?
        {
            Some(reference) => Ok(AuthoredDeclarationReference::Selected { reference }),
            None => parse_declaration_reference(record, field),
        }
    }
    pub(super) fn parse_port_reference(
        &self,
        record: &CompactRecord,
        field: &str,
    ) -> Result<AuthoredPortReference, Diagnostic> {
        match self
            .references
            .owner(record, field, Some(NamespaceClass::Port), false)?
        {
            Some(reference) => Ok(AuthoredPortReference::Selected { reference }),
            None => parse_port_reference(record, field),
        }
    }
    pub(super) fn parse_field_reference(
        &self,
        record: &CompactRecord,
        field: &str,
    ) -> Result<AuthoredFieldReference, Diagnostic> {
        match self
            .references
            .owner(record, field, Some(NamespaceClass::Field), false)?
        {
            Some(reference) => Ok(AuthoredFieldReference::Selected { reference }),
            None => parse_field_reference(record, field),
        }
    }
    pub(super) fn parse_case_reference(
        &self,
        record: &CompactRecord,
        field: &str,
    ) -> Result<AuthoredCaseReference, Diagnostic> {
        match self
            .references
            .owner(record, field, Some(NamespaceClass::Case), false)?
        {
            Some(reference) => Ok(AuthoredCaseReference::Selected { reference }),
            None => parse_case_reference(record, field),
        }
    }
    pub(super) fn parse_operation_reference(
        &self,
        record: &CompactRecord,
        field: &str,
    ) -> Result<AuthoredOperationReference, Diagnostic> {
        match self
            .references
            .owner(record, field, Some(NamespaceClass::Operation), false)?
        {
            Some(reference) => Ok(AuthoredOperationReference::Selected { reference }),
            None => parse_operation_reference(record, field),
        }
    }
    pub(super) fn parse_requirement_reference(
        &self,
        record: &CompactRecord,
        field: &str,
    ) -> Result<AuthoredRequirementReference, Diagnostic> {
        if let Some(value) = required(record, field)?.strip_prefix("parameter:") {
            let mut exact = record.clone();
            let target = exact
                .fields
                .iter_mut()
                .find(|f| f.name == field)
                .ok_or_else(|| {
                    record_error(
                        record,
                        "change_requirement_operand",
                        "requirement operand field is absent",
                    )
                })?;
            target.value = value.to_owned();
            if let Some(reference) = self.references.owner(
                &exact,
                field,
                Some(NamespaceClass::RequirementParameter),
                false,
            )? {
                return Ok(AuthoredRequirementReference::ParameterSelected { reference });
            }
            if value.starts_with('$') {
                validate_local_label(&exact, field, value, '$')?;
                return Ok(AuthoredRequirementReference::ParameterSymbol {
                    symbol: value.to_owned(),
                });
            }
            let (package, parameter) = value.split_once('/').ok_or_else(|| field_error(record, field, "change_requirement_operand", "parameter operand requires an exact package/reqparam_ID, named reference or request symbol"))?;
            return Ok(AuthoredRequirementReference::ParameterExact {
                package: package.parse().map_err(|error: Diagnostic| {
                    field_error(record, field, error.code, error.message)
                })?,
                parameter: parameter.parse().map_err(|error: Diagnostic| {
                    field_error(record, field, error.code, error.message)
                })?,
            });
        }
        match self
            .references
            .owner(record, field, Some(NamespaceClass::Requirement), false)?
        {
            Some(reference) => Ok(AuthoredRequirementReference::Selected { reference }),
            None => parse_requirement_reference(record, field),
        }
    }
    pub(super) fn parse_effect_parameter_reference(
        &self,
        record: &CompactRecord,
        field: &str,
    ) -> Result<AuthoredEffectParameterReference, Diagnostic> {
        match self
            .references
            .owner(record, field, Some(NamespaceClass::EffectParameter), false)?
        {
            Some(reference) => Ok(AuthoredEffectParameterReference::Selected { reference }),
            None => parse_effect_parameter_reference(record, field),
        }
    }
    pub(super) fn parse_type_parameter_reference(
        &self,
        record: &CompactRecord,
        field: &str,
    ) -> Result<AuthoredTypeParameterReference, Diagnostic> {
        match self
            .references
            .owner(record, field, Some(NamespaceClass::TypeParameter), false)?
        {
            Some(reference) => Ok(AuthoredTypeParameterReference::Selected { reference }),
            None => parse_type_parameter_reference(record, field),
        }
    }
    pub(super) fn parse_local_reference(
        &self,
        record: &CompactRecord,
        field: &str,
    ) -> Result<AuthoredLocalReference, Diagnostic> {
        match self
            .references
            .owner(record, field, Some(NamespaceClass::Parameter), true)?
        {
            Some(reference) => Ok(AuthoredLocalReference::Selected { reference }),
            None => parse_local_reference(record, field),
        }
    }
    pub(super) fn parse_field_selector(
        &self,
        record: &CompactRecord,
    ) -> Result<AuthoredFieldSelector, Diagnostic> {
        if optional(record, "field").is_some() && optional(record, "name").is_none() {
            Ok(AuthoredFieldSelector::Nominal {
                field: self.parse_field_reference(record, "field")?,
            })
        } else {
            parse_field_selector(record)
        }
    }
}

fn index(value: usize) -> Result<u32, Diagnostic> {
    u32::try_from(value).map_err(|_| {
        Diagnostic::new(
            DiagnosticClass::Resource,
            "change_reference_count",
            "reference inventory exceeds its index bound",
        )
    })
}

fn same_package(left: &AuthoredReferencePackage, right: &AuthoredReferencePackage) -> bool {
    match (left, right) {
        (AuthoredReferencePackage::Local, AuthoredReferencePackage::Local) => true,
        (
            AuthoredReferencePackage::Exact {
                package: a,
                package_revision: ar,
                ..
            },
            AuthoredReferencePackage::Exact {
                package: b,
                package_revision: br,
                ..
            },
        ) => a == b && ar == br,
        _ => false,
    }
}
