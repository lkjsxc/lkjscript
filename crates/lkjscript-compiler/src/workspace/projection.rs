use std::collections::HashMap;
use std::sync::Arc;

use super::{
    EntityId, EntityKind, HoleId, NodeHeader, NodeId, NodeKind, ProgramState, ReferenceEdge,
    SemanticOwner, WorkspaceError, WorkspaceSnapshot,
};

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
    Hole(HoleId),
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

        for slice in slices {
            match *slice {
                ProjectionSlice::Entity(entity) => self.project_entity(entity, &mut output)?,
                ProjectionSlice::Body(entity) => self.project_body(entity, &mut output)?,
                ProjectionSlice::Type(node) => self.project_type(node, &mut output)?,
                ProjectionSlice::References(entity) => {
                    self.project_references(entity, &mut output)?;
                }
                ProjectionSlice::Hole(hole) => self.project_hole(hole, &mut output)?,
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
        output.push("\n")
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
            depths.insert(node.id, depth);
            output.spaces(
                depth
                    .checked_add(1)
                    .and_then(|value| value.checked_mul(2))
                    .ok_or_else(|| host("projection indentation overflow"))?,
            )?;
            project_node_header(node, self.is_hole(node.id), output)?;
        }
        Ok(())
    }

    fn project_type(
        &self,
        node: NodeId,
        output: &mut ProjectionOutput,
    ) -> Result<(), WorkspaceError> {
        let header = self.workspace_node(node)?;
        output.push("type ")?;
        output.node_id(header.id)?;
        output.push(" actual=")?;
        output.quoted(&header.actual_type)?;
        output.push(" expected=")?;
        match &header.expected_type {
            Some(expected) => output.quoted(expected)?,
            None => output.push("-")?,
        }
        if self.is_hole(node) {
            output.push(" [HOLE]")?;
        }
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
            if self.is_hole(site) {
                output.push(" [HOLE]")?;
            }
            output.push("\n")?;
        }
        Ok(())
    }

    fn project_hole(
        &self,
        hole: HoleId,
        output: &mut ProjectionOutput,
    ) -> Result<(), WorkspaceError> {
        let state = self.hole_context(self.revision, hole)?;
        let context = self.workspace_node(state.context)?;
        output.push("hole ")?;
        output.node_id(state.id.node())?;
        output.push(" [HOLE] expected=")?;
        output.quoted(
            context
                .expected_type
                .as_deref()
                .unwrap_or(context.actual_type.as_ref()),
        )?;
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
        for (index, entity) in visible.into_iter().enumerate() {
            if index != 0 {
                output.push(",")?;
            }
            output.entity_id(entity)?;
        }
        output.push("]\n")
    }

    fn is_hole(&self, node: NodeId) -> bool {
        self.holes
            .iter()
            .any(|overlay| overlay.state.id.node() == node)
    }
}

fn project_node_header(
    node: &NodeHeader,
    hole: bool,
    output: &mut ProjectionOutput,
) -> Result<(), WorkspaceError> {
    output.push("node ")?;
    output.node_id(node.id)?;
    output.push(" kind=")?;
    output.push(node_kind(node.kind))?;
    output.push(" type=")?;
    output.quoted(&node.actual_type)?;
    output.push(" expected=")?;
    match &node.expected_type {
        Some(expected) => output.quoted(expected)?,
        None => output.push("-")?,
    }
    if hole {
        output.push(" [HOLE]")?;
    }
    output.push("\n")
}

fn entity_kind(kind: EntityKind) -> &'static str {
    match kind {
        EntityKind::Main => "main",
        EntityKind::Parameter => "parameter",
        EntityKind::ImmutableLocal => "immutable-local",
        EntityKind::StaticBytesLocal => "static-bytes-local",
        EntityKind::MutableLocal => "mutable-local",
        EntityKind::Function => "function",
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
