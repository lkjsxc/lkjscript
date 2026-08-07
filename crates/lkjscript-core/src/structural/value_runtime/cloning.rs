use super::super::StructuralValueKey;
use super::{
    StructuralEventKind, StructuralObject, StructuralType, StructuralValueError,
    StructuralValueRuntime,
};

impl StructuralValueRuntime {
    pub fn clone_owned(
        &mut self,
        key: StructuralValueKey,
        expected: StructuralType,
    ) -> Result<StructuralValueKey, StructuralValueError> {
        let root = self.resolve_root(key, expected)?;
        let loan = self.roots.borrow_shared(key)?;
        let cloned = match self.objects.get(root) {
            Ok(StructuralObject::Owned { image, facts }) => {
                image.try_clone_flat().map(|copy| (copy, *facts))
            }
            Ok(StructuralObject::Sealed { .. }) => Err(StructuralValueError::WrongOwnership),
            Ok(StructuralObject::Static(_)) => Err(StructuralValueError::WrongPayloadKind),
            Err(error) => Err(error),
        };
        self.roots.end_borrow(loan)?;
        let (cloned, facts) = cloned?;
        match self.publish_image(cloned, facts) {
            Ok(copy) => {
                self.metrics.clones = self.metrics.clones.saturating_add(1);
                self.metrics.clone_nodes = self.metrics.clone_nodes.saturating_add(facts.nodes);
                self.metrics.string_bytes_cloned = self
                    .metrics
                    .string_bytes_cloned
                    .saturating_add(facts.string_bytes);
                self.metrics.path_bytes_cloned = self
                    .metrics
                    .path_bytes_cloned
                    .saturating_add(facts.path_bytes);
                self.record(StructuralEventKind::Clone, key.get(), facts.nodes);
                Ok(copy)
            }
            Err(failure) => {
                self.release_image(failure.1, facts);
                Err(failure.0)
            }
        }
    }
}
