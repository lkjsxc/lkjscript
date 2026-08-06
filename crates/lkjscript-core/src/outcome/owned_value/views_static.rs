impl OwnedValue {
    pub fn as_function(&self) -> Option<u64> {
        if let Some(value) = self.as_structural() {
            return match &value.payload {
                SemanticPayload::Static(crate::StaticStructuralLeaf::Function(function)) => {
                    Some(*function)
                }
                _ => None,
            };
        }
        if let Some(value) = self.as_semantic_dag() {
            return match &value.root_node().payload {
                crate::SemanticDagPayload::Static(crate::StaticStructuralLeaf::Function(
                    function,
                )) => Some(*function),
                _ => None,
            };
        }
        self.root.as_function()
    }
}
