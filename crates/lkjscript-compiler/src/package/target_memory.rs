use lkjscript_core::{Error, Result};

use super::analysis::ModuleAnalysis;
use super::model::{
    LockedTargetMemory, LockedWitnessDependency, LockedWitnessDependencyRole, LockedWitnessGroup,
    LockedWitnessGroupMember, Target,
};

pub(super) fn build(
    targets: &[Target],
    modules: &std::collections::BTreeMap<String, ModuleAnalysis>,
) -> Result<Vec<LockedTargetMemory>> {
    targets
        .iter()
        .map(|target| {
            let analysis = modules
                .get(&target.module)
                .ok_or_else(|| Error::msg("target module analysis is absent"))?;
            let has_main = analysis.source.declarations().iter().any(|declaration| {
                declaration.origin().logical_path() == analysis.logical_id
                    && declaration.kind() == crate::source::DeclarationKind::Main
            });
            if !has_main {
                return Err(Error::msg(format!(
                    "package target has no source main: {}",
                    target.module
                )));
            }
            target_record(target, &analysis.memory_plan)
        })
        .collect()
}

pub(super) fn target_record(
    target: &Target,
    plan: &crate::memory_plan::HirMemoryPlan,
) -> Result<LockedTargetMemory> {
    let groups = plan
        .witness_groups
        .iter()
        .map(|group| {
            let members = group
                .members
                .iter()
                .map(|member| LockedWitnessGroupMember {
                    member: member.witness.to_hex(),
                    ordinal: member.ordinal,
                    semantic_identity: hex(member.semantic_identity),
                })
                .collect();
            LockedWitnessGroup {
                group: group.id.to_hex(),
                recursive: group.recursive,
                members,
            }
        })
        .collect();
    let mut dependencies = Vec::new();
    for witness in &plan.witnesses {
        for dependency in &witness.facts.dependencies {
            let lkjscript_contracts::ExecutableMemoryWitnessTarget::ExternalMember {
                group,
                member,
            } = dependency.target
            else {
                continue;
            };
            dependencies.push(LockedWitnessDependency {
                source_member: witness.id.to_hex(),
                role: locked_role(&dependency.role),
                target_group: hex(group),
                target_member: hex(member),
            });
        }
    }
    dependencies.sort_by(|left, right| {
        (
            &left.source_member,
            &left.role,
            &left.target_group,
            &left.target_member,
        )
            .cmp(&(
                &right.source_member,
                &right.role,
                &right.target_group,
                &right.target_member,
            ))
    });
    if !dependencies.windows(2).all(|pair| pair[0] != pair[1]) {
        return Err(Error::msg(
            "target external witness dependency closure is not unique",
        ));
    }
    Ok(LockedTargetMemory {
        name: target.name.clone(),
        module: target.module.clone(),
        memory_plan_id: plan.id.to_hex(),
        witness_groups: groups,
        external_witness_dependencies: dependencies,
        specialization_identity_support: "absent".into(),
    })
}

fn locked_role(
    role: &lkjscript_contracts::ExecutableMemoryWitnessRole,
) -> LockedWitnessDependencyRole {
    use lkjscript_contracts::ExecutableMemoryWitnessRole as Source;
    match role {
        Source::ListElement => LockedWitnessDependencyRole::ListElement,
        Source::ProductField {
            product,
            field,
            source_order,
        } => LockedWitnessDependencyRole::ProductField {
            product: hex(*product),
            field: hex(*field),
            source_order: *source_order,
        },
        Source::EnumVariantField {
            enumeration,
            variant,
            field,
            variant_source_order,
            field_source_order,
        } => LockedWitnessDependencyRole::EnumVariantField {
            enumeration: hex(*enumeration),
            variant: hex(*variant),
            field: hex(*field),
            variant_source_order: *variant_source_order,
            field_source_order: *field_source_order,
        },
        Source::TypeArgument { constructor, index } => LockedWitnessDependencyRole::TypeArgument {
            constructor: hex(*constructor),
            index: *index,
        },
    }
}

fn hex(value: [u8; 32]) -> String {
    lkjscript_contracts::ContractDigest::from_bytes(value).to_hex()
}
