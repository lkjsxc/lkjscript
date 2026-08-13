use super::*;

pub(super) fn verify_witness_groups(plan: &HirMemoryPlan) -> Result<()> {
    let mut witnesses_by_id = HashMap::new();
    for witness in &plan.witnesses {
        if witnesses_by_id.insert(witness.id, witness).is_some() {
            return Err(Error::msg("HIR memory witness identity is duplicated"));
        }
    }
    let mut covered = BTreeSet::new();
    let groups = plan
        .witness_groups
        .iter()
        .map(|group| {
            let members = group
                .members
                .iter()
                .map(|member| {
                    let witness = witnesses_by_id
                        .get(&member.witness)
                        .copied()
                        .ok_or_else(|| Error::msg("HIR memory witness group member is missing"))?;
                    let semantic_identity =
                        lkjscript_contracts::semantic_type_closure_hash(&witness.facts.semantic)
                            .map_err(|error| Error::msg(error.to_string()))?;
                    if !covered.insert(member.witness)
                        || witness.group != Some(group.id)
                        || witness.ordinal != Some(member.ordinal)
                        || member.semantic_identity != semantic_identity
                    {
                        return Err(Error::msg(
                            "HIR memory witness group partition is inconsistent",
                        ));
                    }
                    Ok(lkjscript_contracts::ExecutableMemoryWitnessGroupMember {
                        id: witness.id.as_bytes(),
                        ordinal: member.ordinal,
                        semantic_identity: member.semantic_identity,
                        facts: executable_facts(&witness.facts)?,
                        dependencies: witness.facts.dependencies.clone(),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(lkjscript_contracts::ExecutableMemoryWitnessGroup {
                id: group.id.as_bytes(),
                recursive: group.recursive,
                members,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if covered.len() != plan.witnesses.len() {
        return Err(Error::msg(
            "HIR memory witness group partition has missing or extra members",
        ));
    }
    lkjscript_contracts::validate_executable_memory_witness_groups(&groups)
        .map_err(|error| Error::msg(error.to_string()))?;
    for witness in &plan.witnesses {
        if witness.recompute_id()? != witness.id {
            return Err(Error::msg(
                "HIR memory witness member identity is noncanonical",
            ));
        }
    }
    Ok(())
}

fn verified_group_declaration_key(ty: &Type) -> Option<VerifiedDeclarationKey> {
    match ty {
        Type::Product(id) => Some(VerifiedDeclarationKey::Product(*id)),
        Type::Enum { id, .. } => Some(VerifiedDeclarationKey::Enum(id.bytes())),
        _ => None,
    }
}

impl VerifiedTypes<'_> {
    pub(super) fn close_recursive_members(&mut self) -> Result<()> {
        let mut demanded = self
            .memo
            .keys()
            .cloned()
            .map(|ty| {
                let semantic = self.verified_semantic_descriptor(&ty)?;
                let identity = lkjscript_contracts::semantic_type_closure_hash(&semantic)
                    .map_err(|error| Error::msg(error.to_string()))?;
                Ok((identity, ty))
            })
            .collect::<Result<Vec<_>>>()?;
        demanded.sort_by_key(|(identity, _)| *identity);
        for (_, ty) in demanded {
            let Some(root) = verified_group_declaration_key(&ty) else {
                continue;
            };
            if !self.graph.is_recursive(&root) {
                continue;
            }
            let component = self
                .graph
                .component(&root)
                .ok_or_else(|| Error::msg("memory verifier lost recursive component"))?;
            let arguments = match &ty {
                Type::Enum { arguments, .. } => arguments.clone(),
                _ => Vec::new(),
            };
            let keys: Vec<_> = self
                .graph
                .keys
                .iter()
                .filter(|key| self.graph.component(key) == Some(component))
                .cloned()
                .collect();
            for key in keys {
                let member_arguments = if key == root {
                    arguments.clone()
                } else {
                    Vec::new()
                };
                let member = match key {
                    VerifiedDeclarationKey::Product(id) => Type::Product(id),
                    VerifiedDeclarationKey::Enum(id) => {
                        let item = self
                            .program
                            .enums
                            .iter()
                            .find(|item| item.id.bytes() == id)
                            .ok_or_else(|| Error::msg("memory verifier lost recursive enum"))?;
                        Type::Enum {
                            id: item.id,
                            arguments: member_arguments,
                        }
                    }
                };
                self.intern(&member)?;
            }
        }
        Ok(())
    }
}
