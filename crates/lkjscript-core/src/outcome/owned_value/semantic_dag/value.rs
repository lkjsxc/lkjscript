use crate::{SemanticDagNode, SemanticDagNodeId};

impl OwnedValue {
    /// Publishes one already validated key-free semantic DAG as an owned return.
    pub fn from_semantic_dag(snapshot: SemanticDagSnapshot) -> Self {
        Self::from_owned_semantic_dag(snapshot)
    }

    /// Validates candidate nodes privately before publishing an owned return.
    pub fn from_semantic_dag_nodes(
        nodes: Vec<SemanticDagNode>,
        root: SemanticDagNodeId,
    ) -> Result<Self> {
        SemanticDagSnapshot::new(nodes, root).map(Self::from_owned_semantic_dag)
    }

    pub fn as_semantic_dag(&self) -> Option<&SemanticDagSnapshot> {
        self.semantic_dag.as_deref()
    }

    pub fn into_semantic_dag(self) -> Option<SemanticDagSnapshot> {
        self.semantic_dag.map(|snapshot| *snapshot)
    }

    pub(super) fn from_owned_semantic_dag(snapshot: SemanticDagSnapshot) -> Self {
        Self {
            root: Value::UNIT,
            lists: Vec::new(),
            unique_byte_vector: None,
            unique_bytes: None,
            symbols: Vec::new(),
            structural: None,
            semantic_dag: Some(Box::new(snapshot)),
        }
    }
}
