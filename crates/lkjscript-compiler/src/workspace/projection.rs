use std::collections::HashMap;
use std::sync::Arc;

use super::{
    CompletenessBlocker, EntityId, EntityKind, HoleId, MatchPatternKindView, NodeHeader, NodeId,
    NodeKind, ProgramState, ReferenceEdge, SemanticOwner, SemanticType, UnresolvedValueReferenceId,
    WorkspaceError, WorkspaceSnapshot,
};

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct ProjectionMeasurement {
    pub snapshot_nodes_inspected: usize,
    pub nodes_emitted: usize,
    pub reference_edges_inspected: usize,
    pub references_emitted: usize,
    pub visible_entities_inspected: usize,
    pub visible_entities_emitted: usize,
}

#[cfg(test)]
thread_local! {
    static PROJECTION_MEASUREMENT: std::cell::RefCell<ProjectionMeasurement> =
        const { std::cell::RefCell::new(ProjectionMeasurement {
            snapshot_nodes_inspected: 0,
            nodes_emitted: 0,
            reference_edges_inspected: 0,
            references_emitted: 0,
            visible_entities_inspected: 0,
            visible_entities_emitted: 0,
        }) };
}

#[cfg(test)]
pub(super) fn reset_projection_measurement() {
    PROJECTION_MEASUREMENT.with(|measurement| {
        *measurement.borrow_mut() = ProjectionMeasurement::default();
    });
}

#[cfg(test)]
pub(super) fn take_projection_measurement() -> ProjectionMeasurement {
    PROJECTION_MEASUREMENT.with(|measurement| std::mem::take(&mut *measurement.borrow_mut()))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
fn record_projection_measurement(
    snapshot_nodes_inspected: usize,
    nodes_emitted: usize,
    reference_edges_inspected: usize,
    references_emitted: usize,
    visible_entities_inspected: usize,
    visible_entities_emitted: usize,
) {
    PROJECTION_MEASUREMENT.with(|measurement| {
        let mut measurement = measurement.borrow_mut();
        measurement.snapshot_nodes_inspected = measurement
            .snapshot_nodes_inspected
            .checked_add(snapshot_nodes_inspected)
            .expect("projection node-inspection measurement overflow");
        measurement.nodes_emitted = measurement
            .nodes_emitted
            .checked_add(nodes_emitted)
            .expect("projection node-emission measurement overflow");
        measurement.reference_edges_inspected = measurement
            .reference_edges_inspected
            .checked_add(reference_edges_inspected)
            .expect("projection reference-inspection measurement overflow");
        measurement.references_emitted = measurement
            .references_emitted
            .checked_add(references_emitted)
            .expect("projection reference-emission measurement overflow");
        measurement.visible_entities_inspected = measurement
            .visible_entities_inspected
            .checked_add(visible_entities_inspected)
            .expect("projection visible-entity inspection measurement overflow");
        measurement.visible_entities_emitted = measurement
            .visible_entities_emitted
            .checked_add(visible_entities_emitted)
            .expect("projection visible-entity emission measurement overflow");
    });
}

/// One concise human-readable view selected from a workspace snapshot.
///
/// Projection labels are review-local spellings of stable identities. They are
/// never parsed or used to construct semantic identity.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[non_exhaustive]
pub enum ProjectionSlice {
    Entity(EntityId),
    Body(EntityId),
    Type(NodeId),
    References(EntityId),
    Call(NodeId),
    Match(NodeId),
    Hole(HoleId),
    UnresolvedValueReference(UnresolvedValueReferenceId),
}

impl WorkspaceSnapshot {
    /// Renders selected semantic headers without consulting source attachments.
    ///
    /// Rendering uses explicit iteration for body depth, reports allocation
    /// failure, and preserves the caller's slice order.
    pub fn project(&self, slices: &[ProjectionSlice]) -> Result<String, WorkspaceError> {
        let mut output = ProjectionOutput::new();
        output.push("workspace revision=")?;
        output.decimal(self.revision.sequence())?;
        output.push(" state=")?;
        output.push(match self.state {
            ProgramState::Complete => "complete",
            ProgramState::Incomplete => "incomplete",
        })?;
        output.push("\n")?;
        for blocker in self.completeness_blockers() {
            output.push("blocker ")?;
            match blocker {
                CompletenessBlocker::MissingEntryPoint => {
                    output.push("missing-entry-point\n")?;
                }
                CompletenessBlocker::MissingBody {
                    declaration,
                    hole,
                    expected_type,
                } => {
                    output.push("missing-body declaration=")?;
                    output.entity_id(*declaration)?;
                    output.push(" hole=")?;
                    output.node_id(hole.node())?;
                    output.push(" expected=")?;
                    project_semantic_type(expected_type, &mut output)?;
                    output.push("\n")?;
                }
                CompletenessBlocker::TypedHole {
                    hole,
                    expected_type,
                    owner,
                    context,
                } => {
                    output.push("typed-hole hole=")?;
                    output.node_id(hole.node())?;
                    output.push(" expected=")?;
                    project_semantic_type(expected_type, &mut output)?;
                    output.push(" owner=")?;
                    output.entity_id(*owner)?;
                    output.push(" context=")?;
                    output.node_id(*context)?;
                    output.push("\n")?;
                }
                CompletenessBlocker::UnresolvedValueReference {
                    reference,
                    requested_name,
                    expected_type,
                    owner,
                    context,
                } => {
                    output.push("unresolved-value-reference reference=")?;
                    output.node_id(reference.node())?;
                    output.push(" requested=")?;
                    output.quoted(requested_name)?;
                    output.push(" expected=")?;
                    project_semantic_type(expected_type, &mut output)?;
                    output.push(" owner=")?;
                    output.entity_id(*owner)?;
                    output.push(" context=")?;
                    output.node_id(*context)?;
                    output.push("\n")?;
                }
            }
        }

        for slice in slices {
            match *slice {
                ProjectionSlice::Entity(entity) => self.project_entity(entity, &mut output)?,
                ProjectionSlice::Body(entity) => self.project_body(entity, &mut output)?,
                ProjectionSlice::Type(node) => self.project_type(node, &mut output)?,
                ProjectionSlice::References(entity) => {
                    self.project_references(entity, &mut output)?;
                }
                ProjectionSlice::Call(node) => self.project_call(node, &mut output)?,
                ProjectionSlice::Match(node) => self.project_match(node, &mut output)?,
                ProjectionSlice::Hole(hole) => self.project_hole(hole, &mut output)?,
                ProjectionSlice::UnresolvedValueReference(reference) => {
                    self.project_unresolved_value_reference(reference, &mut output)?;
                }
            }
        }
        Ok(output.finish())
    }

    fn project_entity(
        &self,
        entity: EntityId,
        output: &mut ProjectionOutput,
    ) -> Result<(), WorkspaceError> {
        let header = self.workspace_entity(entity)?;
        output.push("entity ")?;
        output.entity_id(header.id)?;
        output.push(" kind=")?;
        output.push(entity_kind(header.kind))?;
        output.push(" name=")?;
        output.quoted(&header.name)?;
        output.push(" owner=")?;
        match header.owner {
            Some(owner) => output.entity_id(owner)?,
            None => output.push("-")?,
        }
        let type_facts = self.entity_type(self.revision, entity)?;
        output.push(" type=")?;
        match &type_facts.declared {
            Some(ty) => project_semantic_type(ty, output)?,
            None => output.push("-")?,
        }
        output.push("\n")?;
        if header.kind == EntityKind::Function {
            let signature = self.function_signature(self.revision, entity)?;
            for parameter in &signature.type_parameters {
                output.push("  type-parameter ")?;
                output.entity_id(parameter.id)?;
                output.push(" name=")?;
                output.quoted(&parameter.name)?;
                output.push("\n")?;
                for bound in &parameter.bounds {
                    output.push("    bound trait=")?;
                    project_trait(bound.trait_identity, output)?;
                    output.push("\n")?;
                }
            }
            for parameter in &signature.parameters {
                output.push("  value-parameter ")?;
                output.entity_id(parameter.entity)?;
                output.push(" name=")?;
                output.quoted(&parameter.name)?;
                output.push(" type=")?;
                project_semantic_type(&parameter.ty, output)?;
                output.push("\n")?;
            }
            output.push("  result type=")?;
            project_semantic_type(&signature.result, output)?;
            output.push("\n")?;
        }
        Ok(())
    }

    fn project_body(
        &self,
        entity: EntityId,
        output: &mut ProjectionOutput,
    ) -> Result<(), WorkspaceError> {
        let entity = self.workspace_entity(entity)?;
        output.push("body ")?;
        output.entity_id(entity.id)?;
        output.push(" name=")?;
        output.quoted(&entity.name)?;
        output.push("\n")?;

        let mut depths: HashMap<NodeId, usize> = HashMap::new();
        depths
            .try_reserve(self.indexes.nodes.len())
            .map_err(|_| host("projection body work-map allocation failed"))?;
        #[cfg(test)]
        let mut emitted = 0_usize;
        for node in &self.indexes.nodes {
            let depth = match node.owner {
                SemanticOwner::Entity(owner) if owner == entity.id => Some(0_usize),
                SemanticOwner::Entity(_) => None,
                SemanticOwner::Node(owner) => depths
                    .get(&owner)
                    .copied()
                    .map(|depth| {
                        depth
                            .checked_add(1)
                            .ok_or_else(|| host("projection body depth overflow"))
                    })
                    .transpose()?,
            };
            let Some(depth) = depth else {
                continue;
            };
            #[cfg(test)]
            {
                emitted = emitted
                    .checked_add(1)
                    .ok_or_else(|| host("projection emitted-node count overflow"))?;
            }
            depths.insert(node.id, depth);
            output.spaces(
                depth
                    .checked_add(1)
                    .and_then(|value| value.checked_mul(2))
                    .ok_or_else(|| host("projection indentation overflow"))?,
            )?;
            let facts = self.node_semantics(self.revision, node.id)?;
            project_node_header(node, &facts, output)?;
        }
        #[cfg(test)]
        record_projection_measurement(self.indexes.nodes.len(), emitted, 0, 0, 0, 0);
        Ok(())
    }

    fn project_type(
        &self,
        node: NodeId,
        output: &mut ProjectionOutput,
    ) -> Result<(), WorkspaceError> {
        let header = self.workspace_node(node)?;
        let facts = self.node_semantics(self.revision, node)?;
        output.push("type ")?;
        output.node_id(header.id)?;
        output.push(" actual=")?;
        project_semantic_type(&facts.actual, output)?;
        output.push(" expected=")?;
        match &facts.expected {
            Some(expected) => project_semantic_type(expected, output)?,
            None => output.push("-")?,
        }
        output.push(" operation=")?;
        project_operation(facts.operation, output)?;
        output.push(" effects=")?;
        project_effects(facts.effects, output)?;
        project_incomplete_marker(header.kind, output)?;
        output.push("\n")
    }

    fn project_references(
        &self,
        entity: EntityId,
        output: &mut ProjectionOutput,
    ) -> Result<(), WorkspaceError> {
        let target = self.workspace_entity(entity)?;
        let mut references = Vec::new();
        references
            .try_reserve(self.indexes.references.len())
            .map_err(|_| host("projection reference allocation failed"))?;
        references.extend(
            self.indexes
                .references
                .iter()
                .filter(|edge| edge.target == target.id)
                .copied(),
        );
        references.sort_by_key(|edge| (edge.site, edge.target));
        #[cfg(test)]
        record_projection_measurement(0, 0, self.indexes.references.len(), references.len(), 0, 0);

        output.push("references ")?;
        output.entity_id(target.id)?;
        output.push(" name=")?;
        output.quoted(&target.name)?;
        output.push(" count=")?;
        output.decimal(
            u64::try_from(references.len())
                .map_err(|_| host("projection reference count exceeds u64"))?,
        )?;
        output.push("\n")?;
        for ReferenceEdge { site, target } in references {
            output.push("  reference site=")?;
            output.node_id(site)?;
            output.push(" target=")?;
            output.entity_id(target)?;
            project_incomplete_marker(self.workspace_node(site)?.kind, output)?;
            output.push("\n")?;
        }
        Ok(())
    }

    fn project_call(
        &self,
        node: NodeId,
        output: &mut ProjectionOutput,
    ) -> Result<(), WorkspaceError> {
        let call = self.call_instantiation(self.revision, node)?;
        let callee = self.workspace_entity(call.callee)?;
        let signature = self.function_signature(self.revision, call.callee)?;
        output.push("call ")?;
        output.node_id(call.site)?;
        output.push(" callee=")?;
        output.entity_id(call.callee)?;
        output.push(" name=")?;
        output.quoted(&callee.name)?;
        output.push(" generic=")?;
        output.push(if call.type_arguments.is_empty() {
            "false"
        } else {
            "true"
        })?;
        output.push(" result=")?;
        project_semantic_type(&call.result, output)?;
        output.push(" effects=")?;
        project_effects(call.effects, output)?;
        output.push("\n")?;
        for argument in &call.type_arguments {
            let parameter = signature
                .type_parameters
                .iter()
                .find(|parameter| parameter.id == argument.parameter)
                .ok_or_else(|| WorkspaceError::StaleIdentity(Arc::from("call type parameter")))?;
            output.push("  type-argument parameter=")?;
            output.entity_id(argument.parameter)?;
            output.push(" name=")?;
            output.quoted(&parameter.name)?;
            output.push(" type=")?;
            project_semantic_type(&argument.argument, output)?;
            output.push("\n")?;
            for bound in &parameter.bounds {
                output.push("    bound trait=")?;
                project_trait(bound.trait_identity, output)?;
                output.push("\n")?;
            }
        }
        for witness in &call.witnesses {
            output.push("  witness parameter=")?;
            output.entity_id(witness.parameter)?;
            output.push(" trait=")?;
            project_trait(witness.trait_identity, output)?;
            output.push(" type=")?;
            project_semantic_type(&witness.ty, output)?;
            match witness.kind {
                super::TraitWitnessKindView::AutoTrait => output.push(" kind=auto")?,
                super::TraitWitnessKindView::Explicit(implementation) => {
                    output.push(" kind=explicit implementation=")?;
                    output.entity_id(implementation)?;
                }
            }
            output.push("\n")?;
        }
        Ok(())
    }

    fn project_match(
        &self,
        node: NodeId,
        output: &mut ProjectionOutput,
    ) -> Result<(), WorkspaceError> {
        let view = self.match_view(self.revision, node)?;
        output.push("match ")?;
        output.node_id(view.site)?;
        output.push(" scrutinee=")?;
        output.node_id(view.scrutinee)?;
        output.push(" result=")?;
        project_semantic_type(&view.result, output)?;
        output.push(" exhaustive=")?;
        output.push(if view.exhaustive { "true" } else { "false" })?;
        output.push("\n")?;
        for (ordinal, arm) in view.arms.into_iter().enumerate() {
            output.push("  arm ")?;
            output.decimal(
                u64::try_from(ordinal)
                    .map_err(|_| host("match arm projection ordinal exceeds u64"))?,
            )?;
            output.push(" pattern=p")?;
            output.decimal(arm.pattern_root.projection_ordinal())?;
            output.push(" body=")?;
            output.node_id(arm.body)?;
            output.push(" result=")?;
            project_semantic_type(&arm.result, output)?;
            output.push("\n")?;
            for pattern in arm.patterns {
                output.push("    pattern p")?;
                output.decimal(pattern.label.projection_ordinal())?;
                output.push(" type=")?;
                project_semantic_type(&pattern.ty, output)?;
                match pattern.kind {
                    MatchPatternKindView::Wildcard => output.push(" kind=wildcard")?,
                    MatchPatternKindView::Binding { binding } => {
                        let binding_header = self.workspace_entity(binding)?;
                        output.push(" kind=binding binding=")?;
                        output.entity_id(binding)?;
                        output.push(" name=")?;
                        output.quoted(&binding_header.name)?;
                    }
                    MatchPatternKindView::Bool(value) => {
                        output.push(" kind=bool value=")?;
                        output.push(if value { "true" } else { "false" })?;
                    }
                    MatchPatternKindView::I64(value) => {
                        output.push(" kind=i64 value=")?;
                        output.signed_decimal(value)?;
                    }
                    MatchPatternKindView::EnumVariant {
                        enumeration,
                        variant,
                        fields,
                    } => {
                        output.push(" kind=enum-variant enum=")?;
                        project_optional_entity(enumeration, output)?;
                        output.push(" variant=")?;
                        project_optional_entity(variant, output)?;
                        project_pattern_fields(&fields, output)?;
                    }
                    MatchPatternKindView::Product { product, fields } => {
                        output.push(" kind=product product=")?;
                        output.entity_id(product)?;
                        project_pattern_fields(&fields, output)?;
                    }
                }
                output.push("\n")?;
            }
        }
        Ok(())
    }

    fn project_hole(
        &self,
        hole: HoleId,
        output: &mut ProjectionOutput,
    ) -> Result<(), WorkspaceError> {
        let state = self.hole_context(self.revision, hole)?;
        self.workspace_node(state.context)?;
        output.push("hole ")?;
        output.node_id(state.id.node())?;
        output.push(" [HOLE] expected=")?;
        project_semantic_type(&state.expected_type, output)?;
        output.push(" owner=")?;
        output.entity_id(state.owner)?;
        output.push(" context=")?;
        output.node_id(state.context)?;
        output.push(" goal=")?;
        output.quoted(&state.goal)?;
        output.push(" visible=[")?;
        let mut visible = Vec::new();
        visible
            .try_reserve(state.visible_entities.len())
            .map_err(|_| host("projection hole visibility allocation failed"))?;
        visible.extend(state.visible_entities.iter().copied());
        visible.sort();
        #[cfg(test)]
        record_projection_measurement(0, 0, 0, 0, state.visible_entities.len(), visible.len());
        for (index, entity) in visible.into_iter().enumerate() {
            if index != 0 {
                output.push(",")?;
            }
            output.entity_id(entity)?;
        }
        output.push("]\n")
    }

    fn project_unresolved_value_reference(
        &self,
        reference: UnresolvedValueReferenceId,
        output: &mut ProjectionOutput,
    ) -> Result<(), WorkspaceError> {
        let state = self.unresolved_value_reference(self.revision, reference)?;
        output.push("unresolved-value-reference ")?;
        output.node_id(state.id.node())?;
        output.push(" [UNRESOLVED] intent=copy-load requested=")?;
        output.quoted(&state.requested_name)?;
        output.push(" expected=")?;
        project_semantic_type(&state.expected_type, output)?;
        output.push(" owner=")?;
        output.entity_id(state.owner)?;
        output.push(" context=")?;
        output.node_id(state.context)?;
        output.push(" visible-count=")?;
        output.decimal(
            u64::try_from(state.visible_entities.len())
                .map_err(|_| host("unresolved visibility count exceeds u64"))?,
        )?;
        output.push("\n")
    }
}

fn project_pattern_fields(
    fields: &[super::MatchPatternFieldView],
    output: &mut ProjectionOutput,
) -> Result<(), WorkspaceError> {
    output.push(" fields=[")?;
    for (index, field) in fields.iter().enumerate() {
        if index != 0 {
            output.push(",")?;
        }
        project_optional_entity(field.field, output)?;
        output.push(":p")?;
        output.decimal(field.pattern.projection_ordinal())?;
    }
    output.push("]")
}

fn project_optional_entity(
    entity: Option<EntityId>,
    output: &mut ProjectionOutput,
) -> Result<(), WorkspaceError> {
    match entity {
        Some(entity) => output.entity_id(entity),
        None => output.push("builtin"),
    }
}

fn project_trait(
    identity: super::SemanticTrait,
    output: &mut ProjectionOutput,
) -> Result<(), WorkspaceError> {
    match identity {
        super::SemanticTrait::Entity(entity) => output.entity_id(entity),
        super::SemanticTrait::Builtin(kind) => output.push(match kind {
            super::BuiltinTrait::Copy => "builtin:copy",
            super::BuiltinTrait::Clone => "builtin:clone",
            super::BuiltinTrait::Drop => "builtin:drop",
            super::BuiltinTrait::Send => "builtin:send",
            super::BuiltinTrait::Sync => "builtin:sync",
        }),
    }
}

fn project_effects(
    effects: super::EffectSummary,
    output: &mut ProjectionOutput,
) -> Result<(), WorkspaceError> {
    output.push("[")?;
    if effects.is_pure() {
        output.push("pure")?;
    } else {
        let named = [
            (super::EffectSummary::ALLOCATES, "allocates"),
            (super::EffectSummary::READS_MEMORY, "reads-memory"),
            (super::EffectSummary::WRITES_MEMORY, "writes-memory"),
            (super::EffectSummary::MUTATES_LOCAL, "mutates-local"),
            (super::EffectSummary::HOST_IO, "host-io"),
            (super::EffectSummary::MAY_TRAP, "may-trap"),
            (super::EffectSummary::MAY_EXIT, "may-exit"),
            (super::EffectSummary::MAY_DIVERGE, "may-diverge"),
            (super::EffectSummary::UNKNOWN, "unknown"),
        ];
        let mut first = true;
        for (effect, name) in named {
            if effects.contains(effect) {
                if !first {
                    output.push(",")?;
                }
                output.push(name)?;
                first = false;
            }
        }
    }
    output.push("]")
}

fn project_semantic_type(
    ty: &SemanticType,
    output: &mut ProjectionOutput,
) -> Result<(), WorkspaceError> {
    enum Work<'a> {
        Type(&'a SemanticType),
        Text(&'static str),
        Entity(EntityId),
    }
    let mut pending = Vec::new();
    pending
        .try_reserve(1)
        .map_err(|_| host("semantic type projection allocation failed"))?;
    pending.push(Work::Type(ty));
    output.push("\"")?;
    while let Some(item) = pending.pop() {
        match item {
            Work::Text(text) => output.push(text)?,
            Work::Entity(entity) => {
                output.decimal(entity.slot())?;
                output.push(":")?;
                output.decimal(entity.generation())?;
            }
            Work::Type(ty) => match ty {
                SemanticType::Never => output.push("never")?,
                SemanticType::Unit => output.push("unit")?,
                SemanticType::Bool => output.push("bool")?,
                SemanticType::I64 => output.push("i64")?,
                SemanticType::F64 => output.push("f64")?,
                SemanticType::String => output.push("string")?,
                SemanticType::Bytes => output.push("bytes")?,
                SemanticType::ByteVector => output.push("byte-vector")?,
                SemanticType::ByteSlice => output.push("byte-slice")?,
                SemanticType::ByteSliceMut => output.push("byte-slice-mut")?,
                SemanticType::Path => output.push("path")?,
                SemanticType::Capability(kind) => {
                    output.push("capability ")?;
                    output.push(kind.as_str())?;
                }
                SemanticType::Symbol => output.push("symbol")?,
                SemanticType::Resource(kind) => output.push(kind.as_str())?,
                SemanticType::Product(entity) => {
                    output.push("product(")?;
                    pending
                        .try_reserve(2)
                        .map_err(|_| host("semantic type projection allocation failed"))?;
                    pending.push(Work::Text(")"));
                    pending.push(Work::Entity(*entity));
                }
                SemanticType::Enum {
                    constructor,
                    arguments,
                } => {
                    match constructor {
                        super::SemanticEnum::Entity(entity) => {
                            output.push("enum(")?;
                            output.decimal(entity.slot())?;
                            output.push(":")?;
                            output.decimal(entity.generation())?;
                            output.push(")")?;
                        }
                        super::SemanticEnum::Builtin(kind) => output.push(match kind {
                            super::BuiltinEnum::Option => "option",
                            super::BuiltinEnum::Result => "result",
                            super::BuiltinEnum::NumericError => "numeric-error",
                            super::BuiltinEnum::Utf8Error => "utf8-error",
                            super::BuiltinEnum::SystemError => "system-error",
                        })?,
                    }
                    let additional = arguments
                        .len()
                        .checked_mul(2)
                        .ok_or_else(|| host("semantic type projection size overflow"))?;
                    pending
                        .try_reserve(additional)
                        .map_err(|_| host("semantic type projection allocation failed"))?;
                    for argument in arguments.iter().rev() {
                        pending.push(Work::Type(argument));
                        pending.push(Work::Text(" "));
                    }
                }
                SemanticType::TypeParameter(entity) => {
                    output.push("type-parameter(")?;
                    pending
                        .try_reserve(2)
                        .map_err(|_| host("semantic type projection allocation failed"))?;
                    pending.push(Work::Text(")"));
                    pending.push(Work::Entity(*entity));
                }
                SemanticType::List(inner) => {
                    output.push("list ")?;
                    pending
                        .try_reserve(1)
                        .map_err(|_| host("semantic type projection allocation failed"))?;
                    pending.push(Work::Type(inner));
                }
                SemanticType::Function { parameters, result } => {
                    output.push("fn inputs")?;
                    let additional = parameters
                        .len()
                        .checked_mul(2)
                        .and_then(|value| value.checked_add(2))
                        .ok_or_else(|| host("semantic type projection size overflow"))?;
                    pending
                        .try_reserve(additional)
                        .map_err(|_| host("semantic type projection allocation failed"))?;
                    pending.push(Work::Type(result));
                    pending.push(Work::Text(" output "));
                    for parameter in parameters.iter().rev() {
                        pending.push(Work::Type(parameter));
                        pending.push(Work::Text(" "));
                    }
                }
                SemanticType::Forall { parameters, body } => {
                    output.push("forall")?;
                    let additional = parameters
                        .len()
                        .checked_mul(2)
                        .and_then(|value| value.checked_add(2))
                        .ok_or_else(|| host("semantic type projection size overflow"))?;
                    pending
                        .try_reserve(additional)
                        .map_err(|_| host("semantic type projection allocation failed"))?;
                    pending.push(Work::Type(body));
                    pending.push(Work::Text(" "));
                    for parameter in parameters.iter().rev() {
                        pending.push(Work::Entity(*parameter));
                        pending.push(Work::Text(" "));
                    }
                }
            },
        }
    }
    output.push("\"")
}

fn project_node_header(
    node: &NodeHeader,
    facts: &super::NodeSemanticFacts,
    output: &mut ProjectionOutput,
) -> Result<(), WorkspaceError> {
    output.push("node ")?;
    output.node_id(node.id)?;
    output.push(" kind=")?;
    output.push(node_kind(node.kind))?;
    output.push(" type=")?;
    project_semantic_type(&facts.actual, output)?;
    output.push(" expected=")?;
    match &facts.expected {
        Some(expected) => project_semantic_type(expected, output)?,
        None => output.push("-")?,
    }
    output.push(" operation=")?;
    project_operation(facts.operation, output)?;
    output.push(" effects=")?;
    project_effects(facts.effects, output)?;
    project_incomplete_marker(node.kind, output)?;
    output.push("\n")
}

fn project_incomplete_marker(
    kind: NodeKind,
    output: &mut ProjectionOutput,
) -> Result<(), WorkspaceError> {
    match kind {
        NodeKind::Hole => output.push(" [HOLE]"),
        NodeKind::UnresolvedValueReference => output.push(" [UNRESOLVED]"),
        _ => Ok(()),
    }
}

fn project_operation(
    operation: Option<crate::operation::Operation>,
    output: &mut ProjectionOutput,
) -> Result<(), WorkspaceError> {
    match operation {
        Some(operation) => output.push(operation.name()),
        None => output.push("-"),
    }
}

fn entity_kind(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Main => "main",
        EntityKind::Parameter => "parameter",
        EntityKind::ImmutableLocal => "immutable-local",
        EntityKind::StaticBytesLocal => "static-bytes-local",
        EntityKind::MutableLocal => "mutable-local",
        EntityKind::Function => "function",
        EntityKind::TypeParameter => "type-parameter",
        EntityKind::BuiltinOperation => "builtin-operation",
        EntityKind::Product => "product",
        EntityKind::ProductField => "product-field",
        EntityKind::Enum => "enum",
        EntityKind::EnumVariant => "enum-variant",
        EntityKind::EnumField => "enum-field",
        EntityKind::Trait => "trait",
        EntityKind::Implementation => "implementation",
    }
}

fn node_kind(kind: NodeKind) -> &'static str {
    match kind {
        NodeKind::Literal => "literal",
        NodeKind::Load => "load",
        NodeKind::UnresolvedValueReference => "unresolved-value-reference",
        NodeKind::Move => "move",
        NodeKind::Borrow => "borrow",
        NodeKind::Call => "call",
        NodeKind::Operation => "operation",
        NodeKind::Conversion => "conversion",
        NodeKind::Sequence => "sequence",
        NodeKind::Conditional => "conditional",
        NodeKind::While => "while",
        NodeKind::Loop => "loop",
        NodeKind::Return => "return",
        NodeKind::Break => "break",
        NodeKind::Continue => "continue",
        NodeKind::Trap => "trap",
        NodeKind::Exit => "exit",
        NodeKind::Let => "let",
        NodeKind::MutableLocal => "mutable-local",
        NodeKind::SetLocal => "set-local",
        NodeKind::Product => "product",
        NodeKind::Enum => "enum",
        NodeKind::Match => "match",
        NodeKind::MatchUnreachable => "match-unreachable",
        NodeKind::Symbol => "symbol",
        NodeKind::Hole => "hole",
    }
}

struct ProjectionOutput {
    value: String,
}

impl ProjectionOutput {
    const fn new() -> Self {
        Self {
            value: String::new(),
        }
    }

    fn finish(self) -> String {
        self.value
    }

    fn push(&mut self, value: &str) -> Result<(), WorkspaceError> {
        self.value
            .try_reserve(value.len())
            .map_err(|_| host("projection output allocation failed"))?;
        self.value.push_str(value);
        Ok(())
    }

    fn spaces(&mut self, count: usize) -> Result<(), WorkspaceError> {
        self.value
            .try_reserve(count)
            .map_err(|_| host("projection indentation allocation failed"))?;
        for _ in 0..count {
            self.value.push(' ');
        }
        Ok(())
    }

    fn decimal(&mut self, mut value: u64) -> Result<(), WorkspaceError> {
        let mut bytes = [0_u8; 20];
        let mut start = bytes.len();
        loop {
            start -= 1;
            bytes[start] = b'0'
                + u8::try_from(value % 10)
                    .map_err(|_| host("projection decimal digit conversion failed"))?;
            value /= 10;
            if value == 0 {
                break;
            }
        }
        let digits = std::str::from_utf8(&bytes[start..])
            .map_err(|_| host("projection decimal encoding failed"))?;
        self.push(digits)
    }

    fn signed_decimal(&mut self, value: i64) -> Result<(), WorkspaceError> {
        if value < 0 {
            self.push("-")?;
            self.decimal(value.unsigned_abs())
        } else {
            self.decimal(value as u64)
        }
    }

    fn entity_id(&mut self, id: EntityId) -> Result<(), WorkspaceError> {
        self.push("e")?;
        self.decimal(id.slot())?;
        self.push("g")?;
        self.decimal(id.generation())
    }

    fn node_id(&mut self, id: NodeId) -> Result<(), WorkspaceError> {
        self.push("n")?;
        self.decimal(id.slot())?;
        self.push("g")?;
        self.decimal(id.generation())
    }

    fn quoted(&mut self, value: &str) -> Result<(), WorkspaceError> {
        self.push("\"")?;
        for character in value.chars() {
            match character {
                '\\' => self.push("\\\\")?,
                '"' => self.push("\\\"")?,
                '\n' => self.push("\\n")?,
                '\r' => self.push("\\r")?,
                '\t' => self.push("\\t")?,
                character if character.is_control() => self.push("�")?,
                character => {
                    let mut encoded = [0_u8; 4];
                    self.push(character.encode_utf8(&mut encoded))?;
                }
            }
        }
        self.push("\"")
    }
}

fn host(message: &'static str) -> WorkspaceError {
    WorkspaceError::Host(Arc::from(message))
}
