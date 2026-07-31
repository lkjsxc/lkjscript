use lkjscript_core::{StructuralFieldPath, StructuralProjection};

use crate::eval::{map_structural_error, EvalValue, Evaluator, Flow};

impl Evaluator<'_> {
    pub(super) fn copy_projected(
        &mut self,
        owner: lkjscript_core::StructuralValueKey,
        root_type: lkjscript_core::StructuralType,
        field: u16,
        expected: lkjscript_core::StructuralType,
    ) -> Result<EvalValue, Flow> {
        let semantic = self.projected_semantic(owner, root_type, usize::from(field), expected)?;
        self.semantic_to_eval(semantic)
    }

    pub(crate) fn projected_semantic(
        &mut self,
        owner: lkjscript_core::StructuralValueKey,
        root_type: lkjscript_core::StructuralType,
        field: usize,
        expected: lkjscript_core::StructuralType,
    ) -> Result<lkjscript_core::SemanticValue, Flow> {
        let field = u16::try_from(field).map_err(|_| Flow::Resource("structural fields".into()))?;
        let view = self
            .structural
            .runtime
            .borrow_projected(
                owner,
                root_type,
                StructuralProjection::Field {
                    path: StructuralFieldPath::new(vec![field]),
                    expected,
                },
                false,
            )
            .map_err(map_structural_error)?;
        let result = self
            .structural
            .runtime
            .projected(view)
            .map_err(map_structural_error);
        let ended = self
            .structural
            .runtime
            .end_view(view)
            .map_err(map_structural_error);
        match (result, ended) {
            (Ok(value), Ok(())) => Ok(value),
            (Err(primary), _) => Err(primary),
            (Ok(_), Err(cleanup)) => Err(cleanup),
        }
    }

    pub(crate) fn abort_structural_destination(
        &mut self,
        destination: lkjscript_core::StructuralDestinationKey,
    ) {
        if let Err(error) = self.structural.runtime.abort_destination(destination) {
            self.note_structural_cleanup_failure(error.to_string());
        }
    }
}
