impl Producer<'_> {
    fn finalize_witness_groups(&mut self) -> Result<Vec<MemoryWitnessGroup>> {
        let (groups, remap) = self.type_planner.close_and_finalize_witness_groups()?;
        for call in &mut self.calls {
            for argument in &mut call.witness_arguments {
                argument.witness = *remap.get(&argument.witness)
                    .ok_or_else(|| Error::msg("memory witness remap lost call argument"))?;
            }
        }
        self.finish_type_work()?;
        self.work.witness_groups = u64::try_from(groups.len())
            .map_err(|_| Error::msg("HIR memory-plan witness groups exceed u64"))?;
        self.work.witness_group_edges = self.type_planner.witnesses.iter()
            .try_fold(0u64, |sum, witness| {
                let dependencies = u64::try_from(witness.facts.dependencies.len())
                    .map_err(|_| Error::msg("HIR memory-plan witness dependency count exceeds u64"))?;
                sum.checked_add(dependencies)
                    .ok_or_else(|| Error::msg("HIR memory-plan witness group edge work overflow"))
            })?;
        Ok(groups)
    }
}

impl TypePlanner<'_> {
    fn close_and_finalize_witness_groups(
        &mut self,
    ) -> Result<(Vec<MemoryWitnessGroup>, HashMap<MemoryWitnessId, MemoryWitnessId>)> {
        self.close_recursive_witness_members()?;
        let count = self.witnesses.len();
        let semantic_ids = self.witnesses.iter().map(|item|
            witness_semantic_identity(&item.facts)).collect::<Result<Vec<_>>>()?;
        let mut parent: Vec<usize> = (0..count).collect();
        for index in 0..count {
            let requirements = lkjscript_contracts::semantic_dependency_requirements(
                &self.witnesses[index].facts.semantic)
                .map_err(|error| Error::msg(error.to_string()))?;
            for (dependency, (_, expected)) in self.witnesses[index]
                .facts.dependencies.iter().zip(requirements) {
                if matches!(dependency.target,
                    lkjscript_contracts::ExecutableMemoryWitnessTarget::LocalMember(_)) {
                    let target = exact_semantic_root(&self.witnesses, &expected)?;
                    union(&mut parent, index, target);
                }
            }
        }
        let mut components: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for index in 0..count {
            let root = find(&mut parent, index);
            components.entry(root).or_default().push(index);
        }
        let mut groups: Vec<Vec<usize>> = components.into_values().collect();
        for members in &mut groups {
            members.sort_by_key(|index| semantic_ids[*index]);
        }
        groups.sort_by_key(|members| semantic_ids[members[0]]);
        let mut group_of = vec![0usize; count];
        let mut ordinal = vec![0u64; count];
        for (group, members) in groups.iter().enumerate() {
            for (position, member) in members.iter().enumerate() {
                group_of[*member] = group;
                ordinal[*member] = u64::try_from(position)
                    .map_err(|_| Error::msg("memory witness group ordinal exceeds u64"))?;
            }
        }
        let old_ids: Vec<_> = self.witnesses.iter().map(|item| item.id).collect();
        let mut states = vec![0u8; groups.len()];
        let mut group_ids = vec![MemoryWitnessGroupId::from_bytes([0; 32]); groups.len()];
        for group in 0..groups.len() {
            finish_group(group, &groups, &group_of, &ordinal, &semantic_ids,
                &mut self.witnesses, &mut states, &mut group_ids)?;
        }
        let remap: HashMap<_, _> = old_ids.into_iter().zip(
            self.witnesses.iter().map(|item| item.id)).collect();
        for fact in &mut self.facts {
            fact.witness = *remap.get(&fact.witness)
                .ok_or_else(|| Error::msg("memory witness remap lost type fact"))?;
        }
        for witness in &mut self.witnesses {
            if let Some(list) = &mut witness.facts.list {
                list.element = *remap.get(&list.element)
                    .ok_or_else(|| Error::msg("memory witness remap lost list element"))?;
            }
        }
        let mut output = Vec::with_capacity(groups.len());
        for (index, members) in groups.iter().enumerate() {
            let recursive = members.len() > 1 || self.witnesses[members[0]].facts.dependencies
                .iter().any(|edge| matches!(edge.target,
                    lkjscript_contracts::ExecutableMemoryWitnessTarget::LocalMember(_)));
            output.push(MemoryWitnessGroup { id: group_ids[index], recursive,
                members: members.iter().map(|member| MemoryWitnessGroupMember {
                    witness: self.witnesses[*member].id,
                    ordinal: ordinal[*member],
                    semantic_identity: semantic_ids[*member],
                }).collect() });
        }
        output.sort_by_key(|group| group.id);
        Ok((output, remap))
    }

    fn close_recursive_witness_members(&mut self) -> Result<()> {
        let mut demanded = self.memo.keys().cloned().map(|ty| {
            let semantic = self.producer_semantic_descriptor(&ty)?;
            let identity = lkjscript_contracts::semantic_type_closure_hash(&semantic)
                .map_err(|error| Error::msg(error.to_string()))?;
            Ok((identity, ty))
        }).collect::<Result<Vec<_>>>()?;
        demanded.sort_by_key(|(identity, _)| *identity);
        for (_, ty) in demanded {
            let Some(root) = declaration_key(&ty) else { continue; };
            if !self.graph.is_recursive(&root) { continue; }
            let component = self.graph.component(&root)
                .ok_or_else(|| Error::msg("recursive witness lost declaration component"))?;
            let arguments = match &ty { Type::Enum { arguments, .. } => arguments.as_slice(), _ => &[] };
            let keys: Vec<_> = self.graph.keys.iter().filter(|key|
                self.graph.component(key) == Some(component)).cloned().collect();
            for key in keys {
                let member_arguments = if key == root { arguments } else { &[] };
                let member = self.recursive_root_type(&key, member_arguments)?;
                self.intern(&member)?;
            }
        }
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn finish_group(
    group: usize, groups: &[Vec<usize>], group_of: &[usize], ordinals: &[u64],
    semantic_ids: &[[u8; 32]], witnesses: &mut [MemoryWitness], states: &mut [u8],
    group_ids: &mut [MemoryWitnessGroupId],
) -> Result<()> {
    crate::stack::grow(|| finish_group_inner(
        group, groups, group_of, ordinals, semantic_ids, witnesses, states, group_ids,
    ))
}

#[allow(clippy::too_many_arguments)]
fn finish_group_inner(
    group: usize, groups: &[Vec<usize>], group_of: &[usize], ordinals: &[u64],
    semantic_ids: &[[u8; 32]], witnesses: &mut [MemoryWitness], states: &mut [u8],
    group_ids: &mut [MemoryWitnessGroupId],
) -> Result<()> {
    if states[group] == 2 { return Ok(()); }
    if states[group] == 1 { return Err(Error::msg("memory witness external group cycle")); }
    states[group] = 1;
    for member in &groups[group] {
        let requirements = lkjscript_contracts::semantic_dependency_requirements(
            &witnesses[*member].facts.semantic)
            .map_err(|error| Error::msg(error.to_string()))?;
        let targets = requirements.iter().map(|(_, expected)|
            exact_semantic_root(witnesses, expected)).collect::<Result<Vec<_>>>()?;
        for (edge_index, target) in targets.into_iter().enumerate() {
            let local = matches!(witnesses[*member].facts.dependencies[edge_index].target,
                lkjscript_contracts::ExecutableMemoryWitnessTarget::LocalMember(_));
            let resolved = if local {
                if group_of[target] != group { return Err(Error::msg("local witness edge escaped group")); }
                lkjscript_contracts::ExecutableMemoryWitnessTarget::LocalMember(ordinals[target])
            } else {
                let child_group = group_of[target];
                finish_group(child_group, groups, group_of, ordinals, semantic_ids,
                    witnesses, states, group_ids)?;
                lkjscript_contracts::ExecutableMemoryWitnessTarget::ExternalMember {
                    group: group_ids[child_group].as_bytes(), member: witnesses[target].id.as_bytes() }
            };
            witnesses[*member].facts.dependencies[edge_index].target = resolved;
        }
    }
    let recursive = groups[group].len() > 1 || witnesses[groups[group][0]].facts.dependencies
        .iter().any(|edge| matches!(edge.target,
            lkjscript_contracts::ExecutableMemoryWitnessTarget::LocalMember(_)));
    let members: Vec<_> = groups[group].iter().map(|member| Ok(
        lkjscript_contracts::ExecutableMemoryWitnessGroupMember { id: [0; 32],
            ordinal: ordinals[*member], semantic_identity: semantic_ids[*member],
            facts: executable_facts(&witnesses[*member].facts)?,
            dependencies: witnesses[*member].facts.dependencies.clone() }))
        .collect::<Result<Vec<_>>>()?;
    let id = MemoryWitnessGroupId::from_bytes(
        lkjscript_contracts::executable_memory_witness_group_id(recursive, &members));
    group_ids[group] = id;
    for member in &groups[group] {
        witnesses[*member].group = id; witnesses[*member].ordinal = ordinals[*member];
        witnesses[*member].id = memory_witness_id(id, ordinals[*member],
            semantic_ids[*member]);
    }
    states[group] = 2;
    Ok(())
}

fn witness_semantic_identity(facts: &MemoryWitnessFacts) -> Result<[u8; 32]> {
    lkjscript_contracts::semantic_type_closure_hash(&facts.semantic)
        .map_err(|error| Error::msg(error.to_string()))
}

fn exact_semantic_root(witnesses: &[MemoryWitness], expected: &lkjscript_contracts::SemanticType) -> Result<usize> {
    let mut matches = witnesses.iter().enumerate().filter(|(_, item)| item.facts.semantic.root == *expected);
    let index = matches.next().map(|(index, _)| index)
        .ok_or_else(|| Error::msg("memory witness dependency member is missing"))?;
    if matches.next().is_some() { return Err(Error::msg("memory witness dependency member is ambiguous")); }
    Ok(index)
}

fn find(parent: &mut [usize], value: usize) -> usize {
    if parent[value] != value { parent[value] = find(parent, parent[value]); }
    parent[value]
}
fn union(parent: &mut [usize], left: usize, right: usize) {
    let left = find(parent, left); let right = find(parent, right);
    if left != right { parent[right] = left; }
}
