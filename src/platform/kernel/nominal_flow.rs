//! Finite instantiation is a syntactic parameter-flow property. Nominal member expansion,
//! stored-value properties, and executable recursion deliberately have different relations.

use super::*;

const MAXIMUM_ANALYSIS_BYTES: usize = 64 * 1_048_576;
const MAXIMUM_WITNESS_EDGES: usize = 64;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Slot {
    declaration: DeclarationReference,
    parameter: TypeParameterId,
}

struct Edge {
    from: usize,
    to: usize,
    expanding: bool,
    member: OwnerKey,
    application_path: Vec<usize>,
    argument: usize,
    occurrence_path: Vec<usize>,
}

#[derive(Default)]
struct Flow {
    slots: Vec<Slot>,
    indexes: BTreeMap<Slot, usize>,
    edges: Vec<Edge>,
    bytes: usize,
}

impl Flow {
    fn reserve<T>(&mut self, count: usize) -> Result<(), Diagnostic> {
        self.bytes = count
            .checked_mul(std::mem::size_of::<T>() + 4 * std::mem::size_of::<usize>())
            .and_then(|bytes| self.bytes.checked_add(bytes))
            .filter(|bytes| *bytes <= MAXIMUM_ANALYSIS_BYTES)
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticClass::Resource,
                    "kernel_type_nominal_storage",
                    "finite-instantiation analysis exceeds its metadata storage limit",
                )
            })?;
        Ok(())
    }

    fn slot(&mut self, slot: Slot) -> Result<usize, Diagnostic> {
        if let Some(index) = self.indexes.get(&slot) {
            return Ok(*index);
        }
        self.reserve::<(Slot, usize)>(1)?;
        self.reserve::<Slot>(1)?;
        let index = self.slots.len();
        self.slots.push(slot);
        self.indexes.insert(slot, index);
        Ok(index)
    }

    fn path(&mut self, prefix: &[usize], child: usize) -> Result<Vec<usize>, Diagnostic> {
        self.reserve::<usize>(prefix.len().checked_add(1).ok_or_else(|| {
            type_error(
                "kernel_type_nominal_depth",
                "nominal argument path overflow",
            )
        })?)?;
        let mut path = prefix.to_vec();
        path.push(child);
        Ok(path)
    }

    // Iterative Kosaraju, followed by one BFS inside the offending component. Every graph
    // step calls the owning validator's admission/cancellation checkpoint, including SCC work.
    fn expanding_cycle(
        &mut self,
        tick: &mut impl FnMut() -> Result<(), Diagnostic>,
    ) -> Result<Option<Vec<usize>>, Diagnostic> {
        let count = self.slots.len();
        self.reserve::<Vec<usize>>(count.checked_mul(2).ok_or_else(|| {
            type_error("kernel_type_nominal_depth", "nominal slot count overflow")
        })?)?;
        self.reserve::<usize>(self.edges.len().checked_mul(2).ok_or_else(|| {
            type_error("kernel_type_nominal_depth", "nominal edge count overflow")
        })?)?;
        self.reserve::<usize>(count.checked_mul(8).ok_or_else(|| {
            type_error(
                "kernel_type_nominal_depth",
                "nominal traversal count overflow",
            )
        })?)?;
        let mut outgoing = vec![Vec::new(); count];
        let mut incoming = vec![Vec::new(); count];
        for (index, edge) in self.edges.iter().enumerate() {
            tick()?;
            outgoing[edge.from].push(index);
            incoming[edge.to].push(index);
        }
        let mut seen = vec![false; count];
        let mut finish = Vec::new();
        for root in 0..count {
            tick()?;
            if seen[root] {
                continue;
            }
            seen[root] = true;
            let mut stack = vec![(root, 0)];
            while let Some((node, next)) = stack.last_mut() {
                tick()?;
                if let Some(index) = outgoing[*node].get(*next) {
                    *next += 1;
                    let target = self.edges[*index].to;
                    if !seen[target] {
                        seen[target] = true;
                        stack.push((target, 0));
                    }
                } else {
                    finish.push(*node);
                    stack.pop();
                }
            }
        }
        let mut component = vec![usize::MAX; count];
        for root in finish.into_iter().rev() {
            tick()?;
            if component[root] != usize::MAX {
                continue;
            }
            component[root] = root;
            let mut pending = vec![root];
            while let Some(node) = pending.pop() {
                tick()?;
                for edge in &incoming[node] {
                    tick()?;
                    let source = self.edges[*edge].from;
                    if component[source] == usize::MAX {
                        component[source] = root;
                        pending.push(source);
                    }
                }
            }
        }
        for (index, edge) in self.edges.iter().enumerate() {
            tick()?;
            if !edge.expanding || component[edge.from] != component[edge.to] {
                continue;
            }
            let mut predecessor = vec![None; count];
            let mut queue = std::collections::VecDeque::from([edge.to]);
            predecessor[edge.to] = Some(index);
            while let Some(node) = queue.pop_front() {
                tick()?;
                if node == edge.from {
                    break;
                }
                for next in &outgoing[node] {
                    tick()?;
                    let target = self.edges[*next].to;
                    if component[target] == component[edge.from] && predecessor[target].is_none() {
                        predecessor[target] = Some(*next);
                        queue.push_back(target);
                    }
                }
            }
            let mut result = Vec::new();
            let mut node = edge.from;
            while node != edge.to {
                tick()?;
                if result.len() >= MAXIMUM_WITNESS_EDGES - 1 {
                    return Err(Diagnostic::new(
                        DiagnosticClass::Resource,
                        "kernel_type_nominal_witness",
                        "expanding-cycle certificate exceeds the bounded diagnostic witness; reduce the declaration closure",
                    ));
                }
                let previous = predecessor[node].ok_or_else(|| {
                    type_error(
                        "kernel_type_nominal_analysis",
                        "nominal SCC certificate is incomplete",
                    )
                })?;
                result.push(previous);
                node = self.edges[previous].from;
            }
            result.reverse();
            result.insert(0, index);
            return Ok(Some(result));
        }
        Ok(None)
    }

    fn diagnostic(&self, cycle: &[usize]) -> Diagnostic {
        let mut message = String::from("expanding nominal parameter cycle: ");
        for (position, index) in cycle.iter().enumerate() {
            let edge = &self.edges[*index];
            let from = self.slots[edge.from];
            let to = self.slots[edge.to];
            if position != 0 {
                message.push_str("; ");
            }
            message.push_str(&format!(
                "{}::{} slot {} -> {}::{} slot {} ({}, member {}, application path {:?}, argument {}, occurrence path {:?})",
                from.declaration.package, from.declaration.declaration, from.parameter,
                to.declaration.package, to.declaration.declaration, to.parameter,
                if edge.expanding { "expanding" } else { "plain" }, edge.member,
                edge.application_path, edge.argument, edge.occurrence_path));
        }
        message.push_str("; forward parameters without nesting on this cycle, or replace the recursive argument with a closed type");
        type_error("kernel_type_nominal_expansion", message)
    }
}

impl<R: ExpressionRead> ExpressionValidator<'_, '_, R> {
    pub(super) fn validate_nominal_schema(
        &mut self,
        root: DeclarationReference,
    ) -> Result<(), Diagnostic> {
        self.consume_work()?;
        if self.admitted_nominals.contains(&root) {
            return Ok(());
        }
        let mut flow = Flow::default();
        flow.reserve::<DeclarationReference>(1)?;
        let mut pending = BTreeSet::from([root]);
        let mut declarations = BTreeSet::new();
        while let Some(declaration) = pending.pop_first() {
            self.consume_work()?;
            if declarations.contains(&declaration) {
                continue;
            }
            flow.reserve::<DeclarationReference>(1)?;
            declarations.insert(declaration);
            let parameters = self.nominal_parameters(declaration)?;
            flow.reserve::<TypeParameterId>(parameters.len())?;
            let members = self.nominal_members(declaration)?;
            flow.reserve::<(OwnerKey, TypeObjectDigest)>(members.len())?;
            for (member, ty) in members {
                flow.reserve::<(TypeObjectDigest, Vec<usize>)>(1)?;
                let mut syntax = vec![(ty, Vec::new())];
                let mut seen = BTreeSet::new();
                while let Some((ty, path)) = syntax.pop() {
                    self.consume_work()?;
                    if path.len() > MAXIMUM_TYPE_DEPTH {
                        return Err(type_error(
                            "kernel_type_nominal_depth",
                            "nominal syntax exceeds the structural type-depth limit",
                        ));
                    }
                    if seen.contains(&ty) {
                        continue;
                    }
                    flow.reserve::<TypeObjectDigest>(1)?;
                    seen.insert(ty);
                    let object = self.type_object(ty)?;
                    if let TypeForm::TypeParameter { parameter } = &object.form
                        && !parameters.contains(parameter)
                    {
                        return Err(type_error(
                            "kernel_type_parameter_scope",
                            format!(
                                "nominal member {member:?} uses parameter {parameter} outside declaration {}",
                                declaration.declaration
                            ),
                        ));
                    }
                    if let Some((target, arguments)) = nominal_parts(&object.form) {
                        let target_parameters = self.nominal_parameters(target)?;
                        flow.reserve::<TypeParameterId>(target_parameters.len())?;
                        if target_parameters.len() != arguments.len() {
                            return Err(type_error(
                                "kernel_type_nominal_arity",
                                "nominal application requires all ordered parameter arguments",
                            ));
                        }
                        if !declarations.contains(&target) && !pending.contains(&target) {
                            flow.reserve::<DeclarationReference>(1)?;
                            pending.insert(target);
                        }
                        for (argument, (target_parameter, ty)) in
                            target_parameters.iter().zip(arguments).enumerate()
                        {
                            self.consume_work()?;
                            flow.reserve::<(TypeObjectDigest, Vec<usize>)>(1)?;
                            let mut occurrences = vec![(*ty, Vec::new())];
                            let mut visited = BTreeSet::new();
                            while let Some((ty, occurrence_path)) = occurrences.pop() {
                                self.consume_work()?;
                                if occurrence_path.len() > MAXIMUM_TYPE_DEPTH {
                                    return Err(type_error(
                                        "kernel_type_nominal_depth",
                                        "nominal argument exceeds the structural type-depth limit",
                                    ));
                                }
                                if visited.contains(&ty) {
                                    continue;
                                }
                                flow.reserve::<TypeObjectDigest>(1)?;
                                visited.insert(ty);
                                let object = self.type_object(ty)?;
                                if let TypeForm::TypeParameter { parameter } = object.form {
                                    if !parameters.contains(&parameter) {
                                        return Err(type_error(
                                            "kernel_type_parameter_scope",
                                            "nominal application contains a foreign source parameter",
                                        ));
                                    }
                                    let from = flow.slot(Slot {
                                        declaration,
                                        parameter,
                                    })?;
                                    let to = flow.slot(Slot {
                                        declaration: target,
                                        parameter: *target_parameter,
                                    })?;
                                    flow.reserve::<Edge>(1)?;
                                    flow.reserve::<usize>(path.len())?;
                                    flow.edges.push(Edge {
                                        from,
                                        to,
                                        expanding: !occurrence_path.is_empty(),
                                        member,
                                        application_path: path.clone(),
                                        argument,
                                        occurrence_path,
                                    });
                                } else {
                                    for (index, child) in
                                        object.child_types().into_iter().enumerate()
                                    {
                                        self.consume_work()?;
                                        let child_path = flow.path(&occurrence_path, index)?;
                                        flow.reserve::<(TypeObjectDigest, Vec<usize>)>(1)?;
                                        occurrences.push((child, child_path));
                                    }
                                }
                            }
                        }
                    }
                    for (index, child) in object.child_types().into_iter().enumerate() {
                        self.consume_work()?;
                        let child_path = flow.path(&path, index)?;
                        flow.reserve::<(TypeObjectDigest, Vec<usize>)>(1)?;
                        syntax.push((child, child_path));
                    }
                }
            }
        }
        if let Some(cycle) = flow.expanding_cycle(&mut || self.consume_work())? {
            return Err(flow.diagnostic(&cycle));
        }
        // Scope of reuse is this exact validator/read candidate. No success is installed while
        // a sibling, dependency, SCC certificate, admission, or cancellation remains unresolved.
        flow.reserve::<DeclarationReference>(declarations.len())?;
        self.admitted_nominals.extend(declarations);
        Ok(())
    }

    fn nominal_members(
        &mut self,
        declaration: DeclarationReference,
    ) -> Result<Vec<(OwnerKey, TypeObjectDigest)>, Diagnostic> {
        let kind = if declaration.package == self.read.package_id() {
            self.read
                .owner(OwnerKey::Declaration(declaration.declaration))?
                .map(|owner| owner.kind())
        } else {
            Some(
                self.dependency_owner(
                    declaration.package,
                    OwnerKey::Declaration(declaration.declaration),
                    "nominal definition",
                )?
                .header()
                .kind,
            )
        };
        let mut members = Vec::new();
        match kind {
            Some(OwnerKind::Record) => {
                for field in self.record_fields(declaration)? {
                    self.consume_work()?;
                    let record = self.field_record(declaration.package, field)?;
                    if record.declaration != declaration.declaration {
                        return Err(type_error(
                            "kernel_type_field_owner",
                            "nominal field has a foreign declaration owner",
                        ));
                    }
                    members.push((OwnerKey::Field(field), record.ty));
                }
            }
            Some(OwnerKind::Variant) => {
                for case in self.variant_cases(declaration)? {
                    self.consume_work()?;
                    let record = self.case_record(declaration.package, case)?;
                    if record.declaration != declaration.declaration {
                        return Err(type_error(
                            "kernel_type_case_owner",
                            "nominal case has a foreign declaration owner",
                        ));
                    }
                    if let Some(payload) = record.payload {
                        members.push((OwnerKey::Case(case), payload));
                    }
                }
            }
            _ => {
                return Err(type_error(
                    "kernel_type_nominal_kind",
                    "nominal definition must be an exact record or variant",
                ));
            }
        }
        Ok(members)
    }
}
