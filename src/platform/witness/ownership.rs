//! Full derivation of package namespaces, exact ownership, and test dependencies.

use super::entry::{
    BindingContainerRole, ExpressionRootRole, NamespaceKey, OwnershipEntry, OwnershipParent,
    OwnershipRole, TestDependency,
};
use super::witness_error;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    DeclarationPayload, ExactOwnerKey, ExpressionOperation, KernelSnapshot, OwnerKey, OwnerKind,
    OwnerRecord, ParameterParent, PortImplementation, PropagationClass, RelationEdge,
    RelationEndpoint, owner_namespace,
};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn derive_namespaces(
    snapshot: &KernelSnapshot,
) -> Result<BTreeMap<NamespaceKey, OwnerKey>, Diagnostic> {
    let mut namespaces = BTreeMap::new();
    for (owner, record) in &snapshot.owners {
        let Some(entry) = owner_namespace(record) else {
            continue;
        };
        let key = NamespaceKey {
            parent: entry.parent,
            class: entry.class,
            name: entry.name.clone(),
        };
        if let Some(previous) = namespaces.insert(key, *owner) {
            return Err(witness_error(
                DiagnosticClass::Semantic,
                "witness_namespace_duplicate",
                format!("owners {previous:?} and {owner:?} share one canonical namespace key"),
            ));
        }
    }
    Ok(namespaces)
}

pub(super) fn derive_ownership(
    snapshot: &KernelSnapshot,
) -> Result<BTreeMap<OwnerKey, OwnershipEntry>, Diagnostic> {
    let mut ownership = BTreeMap::new();
    for record in snapshot.owners.values() {
        for (owner, entry) in ownership_contributions(record)? {
            insert_ownership(&mut ownership, owner, entry)?;
        }
    }

    if ownership.len() != snapshot.owners.len() {
        let missing = snapshot
            .owners
            .keys()
            .filter(|owner| !ownership.contains_key(owner))
            .copied()
            .collect::<Vec<_>>();
        return Err(witness_error(
            DiagnosticClass::Corrupt,
            "witness_ownership_incomplete",
            format!("ownership derivation omitted owners {missing:?}"),
        ));
    }
    Ok(ownership)
}

/// Returns every ownership fact contributed by one canonical owner record. An expression's
/// parent fact is contributed by the record that contains it, so a local record edit can derive
/// exact removals and insertions without scanning unrelated owners.
pub(crate) fn ownership_contributions(
    record: &OwnerRecord,
) -> Result<BTreeMap<OwnerKey, OwnershipEntry>, Diagnostic> {
    let mut ownership = BTreeMap::new();
    let self_entry = match record {
        OwnerRecord::Module(_) => Some(OwnershipEntry::new(
            OwnershipParent::Package,
            OwnershipRole::PackageModule,
        )),
        OwnerRecord::Declaration(record) => Some(OwnershipEntry::new(
            OwnershipParent::Owner(OwnerKey::Module(record.module)),
            OwnershipRole::ModuleDeclaration,
        )),
        OwnerRecord::TypeParameter(record) => Some(OwnershipEntry::new(
            OwnershipParent::Owner(OwnerKey::Declaration(record.declaration)),
            OwnershipRole::DeclarationTypeParameter,
        )),
        OwnerRecord::Field(record) => Some(OwnershipEntry::new(
            OwnershipParent::Owner(OwnerKey::Declaration(record.declaration)),
            OwnershipRole::DeclarationField,
        )),
        OwnerRecord::Case(record) => Some(OwnershipEntry::new(
            OwnershipParent::Owner(OwnerKey::Declaration(record.declaration)),
            OwnershipRole::DeclarationCase,
        )),
        OwnerRecord::Operation(record) => Some(OwnershipEntry::new(
            OwnershipParent::Owner(OwnerKey::Declaration(record.declaration)),
            OwnershipRole::DeclarationOperation,
        )),
        OwnerRecord::Parameter(record) => Some(match record.parent {
            ParameterParent::Function(declaration) => OwnershipEntry::new(
                OwnershipParent::Owner(OwnerKey::Declaration(declaration)),
                OwnershipRole::DeclarationParameter,
            ),
            ParameterParent::Operation(operation) => OwnershipEntry::new(
                OwnershipParent::Owner(OwnerKey::Operation(operation)),
                OwnershipRole::OperationParameter,
            ),
        }),
        OwnerRecord::Requirement(record) => Some(OwnershipEntry::new(
            OwnershipParent::Owner(OwnerKey::Declaration(record.declaration)),
            OwnershipRole::DeclarationRequirement,
        )),
        OwnerRecord::Port(record) => Some(OwnershipEntry::new(
            OwnershipParent::Owner(OwnerKey::Declaration(record.declaration)),
            OwnershipRole::DeclarationPort,
        )),
        OwnerRecord::Target(_) => Some(OwnershipEntry::new(
            OwnershipParent::Package,
            OwnershipRole::PackageTarget,
        )),
        OwnerRecord::HttpRoute(record) => Some(OwnershipEntry::new(
            OwnershipParent::Owner(OwnerKey::Target(record.target)),
            OwnershipRole::TargetHttpRoute,
        )),
        OwnerRecord::Documentation(record) => Some(OwnershipEntry::new(
            OwnershipParent::Owner(record.owner),
            OwnershipRole::Documentation,
        )),
        OwnerRecord::Annotation(record) => Some(OwnershipEntry::new(
            OwnershipParent::Owner(record.owner),
            OwnershipRole::Annotation,
        )),
        OwnerRecord::Binding(_) | OwnerRecord::Expression(_) => None,
    };
    if let Some(entry) = self_entry {
        insert_ownership(&mut ownership, record.owner(), entry)?;
    }

    match record {
        OwnerRecord::Declaration(record) => match &record.payload {
            DeclarationPayload::Function(function) => insert_expression_root(
                &mut ownership,
                function.body,
                record.header.owner,
                ExpressionRootRole::FunctionBody,
            )?,
            DeclarationPayload::Constant { value, .. } => insert_expression_root(
                &mut ownership,
                *value,
                record.header.owner,
                ExpressionRootRole::ConstantValue,
            )?,
            DeclarationPayload::Test {
                actual, expected, ..
            } => {
                insert_expression_root(
                    &mut ownership,
                    *actual,
                    record.header.owner,
                    ExpressionRootRole::TestActual,
                )?;
                insert_expression_root(
                    &mut ownership,
                    *expected,
                    record.header.owner,
                    ExpressionRootRole::TestExpected,
                )?;
            }
            DeclarationPayload::Record { .. }
            | DeclarationPayload::Variant { .. }
            | DeclarationPayload::Interface { .. }
            | DeclarationPayload::External(_)
            | DeclarationPayload::Component { .. } => {}
        },
        OwnerRecord::Binding(record) => {
            if let Some(value) = record.value {
                insert_expression_root(
                    &mut ownership,
                    value,
                    record.header.owner,
                    ExpressionRootRole::BindingValue,
                )?;
            }
        }
        OwnerRecord::Port(record) => {
            if let PortImplementation::Expression(expression) = record.implementation {
                insert_expression_root(
                    &mut ownership,
                    expression,
                    record.header.owner,
                    ExpressionRootRole::PortImplementation,
                )?;
            }
        }
        OwnerRecord::Expression(record) => {
            for child in record.children() {
                insert_ownership(
                    &mut ownership,
                    OwnerKey::Expression(child.expression),
                    OwnershipEntry::new(
                        OwnershipParent::Owner(OwnerKey::Expression(record.id)),
                        OwnershipRole::ExpressionChild {
                            role: child.role,
                            ordinal: child.ordinal,
                        },
                    ),
                )?;
            }
            match &record.operation {
                ExpressionOperation::Let { bindings, .. } => {
                    for (ordinal, binding) in bindings.iter().enumerate() {
                        insert_binding_parent(
                            &mut ownership,
                            *binding,
                            record.id,
                            BindingContainerRole::Let,
                            ordinal,
                        )?;
                    }
                }
                ExpressionOperation::Match { arms, .. } => {
                    for (ordinal, arm) in arms.iter().enumerate() {
                        if let Some(binding) = arm.payload_binding {
                            insert_binding_parent(
                                &mut ownership,
                                binding,
                                record.id,
                                BindingContainerRole::MatchPayload,
                                ordinal,
                            )?;
                        }
                    }
                }
                ExpressionOperation::Transaction { binding, .. } => insert_binding_parent(
                    &mut ownership,
                    *binding,
                    record.id,
                    BindingContainerRole::Transaction,
                    0,
                )?,
                _ => {}
            }
        }
        _ => {}
    }
    Ok(ownership)
}

pub(super) fn derive_test_dependencies(
    snapshot: &KernelSnapshot,
    ownership: &BTreeMap<OwnerKey, OwnershipEntry>,
    relations: &[RelationEdge],
) -> Result<BTreeSet<TestDependency>, Diagnostic> {
    let tests = snapshot
        .owners
        .iter()
        .filter_map(|(owner, record)| (record.kind() == OwnerKind::Test).then_some(*owner))
        .collect::<BTreeSet<_>>();
    let mut dependencies = BTreeSet::new();
    for edge in relations {
        let RelationEndpoint::Owner(source) = edge.source else {
            continue;
        };
        if source.package != snapshot.root.package_id
            || matches!(
                edge.kind.propagation(),
                PropagationClass::Ownership
                    | PropagationClass::Presentation
                    | PropagationClass::Test
            )
        {
            continue;
        }
        let Some(test) = owning_declaration(source.owner, ownership)? else {
            continue;
        };
        if !tests.contains(&test) {
            continue;
        }
        if let RelationEndpoint::Owner(target) = edge.target
            && target.package == snapshot.root.package_id
            && owning_declaration(target.owner, ownership)? == Some(test)
        {
            continue;
        }
        dependencies.insert(TestDependency {
            test,
            kind: edge.kind,
            target: edge.target,
        });
    }
    Ok(dependencies)
}

fn insert_expression_root(
    ownership: &mut BTreeMap<OwnerKey, OwnershipEntry>,
    expression: crate::platform::semantic_id::ExpressionId,
    parent: OwnerKey,
    role: ExpressionRootRole,
) -> Result<(), Diagnostic> {
    insert_ownership(
        ownership,
        OwnerKey::Expression(expression),
        OwnershipEntry::new(
            OwnershipParent::Owner(parent),
            OwnershipRole::ExpressionRoot(role),
        ),
    )
}

fn insert_binding_parent(
    ownership: &mut BTreeMap<OwnerKey, OwnershipEntry>,
    binding: crate::platform::semantic_id::BindingId,
    expression: crate::platform::semantic_id::ExpressionId,
    role: BindingContainerRole,
    ordinal: usize,
) -> Result<(), Diagnostic> {
    let ordinal = u32::try_from(ordinal).map_err(|_| {
        witness_error(
            DiagnosticClass::Resource,
            "witness_ownership_ordinal",
            "binding ordinal cannot be represented",
        )
    })?;
    insert_ownership(
        ownership,
        OwnerKey::Binding(binding),
        OwnershipEntry::new(
            OwnershipParent::Owner(OwnerKey::Expression(expression)),
            OwnershipRole::ExpressionBinding { role, ordinal },
        ),
    )
}

fn insert_ownership(
    ownership: &mut BTreeMap<OwnerKey, OwnershipEntry>,
    owner: OwnerKey,
    entry: OwnershipEntry,
) -> Result<(), Diagnostic> {
    if let Some(previous) = ownership.insert(owner, entry) {
        return Err(witness_error(
            DiagnosticClass::Corrupt,
            "witness_ownership_multiple",
            format!("owner {owner:?} has derived parents {previous:?} and {entry:?}"),
        ));
    }
    Ok(())
}

fn owning_declaration(
    owner: OwnerKey,
    ownership: &BTreeMap<OwnerKey, OwnershipEntry>,
) -> Result<Option<OwnerKey>, Diagnostic> {
    let mut current = owner;
    let mut work = 0;
    loop {
        if matches!(current, OwnerKey::Declaration(_)) {
            return Ok(Some(current));
        }
        work += 1;
        if work > ownership.len() {
            return Err(witness_error(
                DiagnosticClass::Corrupt,
                "witness_owner_ancestor_cycle",
                "ownership ancestry is cyclic",
            ));
        }
        let Some(entry) = ownership.get(&current) else {
            return Ok(None);
        };
        match entry.parent {
            OwnershipParent::Package => return Ok(None),
            OwnershipParent::Owner(parent) => current = parent,
        }
    }
}
