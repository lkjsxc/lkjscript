use lkjscript_core::StructuralKind;

use crate::eval::{
    map_structural_error, structural_root, take_value, value, EvalValue, Evaluator, Flow,
};
use crate::{ProductId, SsaType, ValueId};

impl Evaluator<'_> {
    pub(crate) fn structural_product(
        &mut self,
        product: ProductId,
        fields: &[ValueId],
        values: &mut [Option<EvalValue>],
    ) -> Result<EvalValue, Flow> {
        let ty = SsaType::Product(product);
        let value_type = self.structural_type(&ty)?;
        let field_types =
            super::product_fields(self.program.program(), product).map_err(Flow::Trap)?;
        if fields.len() != field_types.len() {
            return Err(Flow::Trap("product field arity mismatch".into()));
        }
        self.charge_aggregate()?;
        self.allocate()?;
        let structural_fields = field_types
            .iter()
            .map(|ty| self.structural_type(ty))
            .collect::<Result<Vec<_>, _>>()?;
        let destination = self
            .structural
            .runtime
            .begin_product(value_type, structural_fields.clone())
            .map_err(map_structural_error)?;
        for (index, ((source, ty), expected)) in fields
            .iter()
            .zip(&field_types)
            .zip(structural_fields)
            .enumerate()
        {
            let field = index;
            let transferred = matches!(ty, SsaType::Bytes | SsaType::ByteVector);
            let semantic = if transferred {
                let owned = match take_value(values, *source) {
                    Ok(owned) => owned,
                    Err(flow) => {
                        self.abort_structural_destination(destination);
                        return Err(flow);
                    }
                };
                match self.take_semantic(owned, expected) {
                    Ok(value) => value,
                    Err(error) => {
                        let (flow, restored) = *error;
                        if let Err(restore) = super::restore_slot(values, *source, restored) {
                            self.note_structural_cleanup_failure(restore.detail());
                        }
                        self.abort_structural_destination(destination);
                        return Err(flow);
                    }
                }
            } else {
                match value(values, *source).and_then(|source| self.copy_semantic(source, expected))
                {
                    Ok(value) => value,
                    Err(flow) => {
                        self.abort_structural_destination(destination);
                        return Err(flow);
                    }
                }
            };
            if let Err(failure) =
                self.structural
                    .runtime
                    .initialize_node(destination, field, semantic)
            {
                self.abort_structural_destination(destination);
                if transferred {
                    let restored = self.semantic_to_eval(failure.value)?;
                    super::restore_slot(values, *source, restored)?;
                }
                return Err(map_structural_error(failure.error));
            }
        }
        self.structural
            .runtime
            .finish_destination(destination)
            .map(|key| {
                EvalValue::StructuralOwner(crate::eval::EvalStructuralOwner { key, value_type })
            })
            .map_err(|error| {
                self.abort_structural_destination(destination);
                map_structural_error(error)
            })
    }

    pub(crate) fn structural_product_field(
        &mut self,
        product: ProductId,
        field: u64,
        input: &EvalValue,
    ) -> Result<EvalValue, Flow> {
        let root_type = self.structural_type(&SsaType::Product(product))?;
        let (owner, actual) = structural_root(input, StructuralKind::Product)?;
        if actual != root_type {
            return Err(Flow::Trap("product field identity mismatch".into()));
        }
        let fields = super::product_fields(self.program.program(), product).map_err(Flow::Trap)?;
        let field = usize::try_from(field)
            .map_err(|_| Flow::Trap("product field exceeds host width".into()))?;
        let ty = fields
            .get(field)
            .ok_or_else(|| Flow::Trap("product field out of bounds".into()))?;
        if *ty == SsaType::ByteVector {
            return Err(Flow::Trap(
                "affine byte-vector field observation requires consuming metadata".into(),
            ));
        }
        let expected = self.structural_type(ty)?;
        self.copy_projected(owner, root_type, field, expected)
    }

    pub(crate) fn structural_with_product_field(
        &mut self,
        product: ProductId,
        field: u64,
        input: &EvalValue,
        replacement: &EvalValue,
    ) -> Result<EvalValue, Flow> {
        let root_type = self.structural_type(&SsaType::Product(product))?;
        let (owner, actual) = structural_root(input, StructuralKind::Product)?;
        if actual != root_type {
            return Err(Flow::Trap("product replacement identity mismatch".into()));
        }
        let fields = super::product_fields(self.program.program(), product).map_err(Flow::Trap)?;
        if fields.contains(&SsaType::ByteVector) {
            return Err(Flow::Trap(
                "with-product-field awaits affine aggregate transfer metadata".into(),
            ));
        }
        self.charge_aggregate()?;
        self.allocate()?;
        let structural_fields = fields
            .iter()
            .map(|ty| self.structural_type(ty))
            .collect::<Result<Vec<_>, _>>()?;
        let destination = self
            .structural
            .runtime
            .begin_product(root_type, structural_fields.clone())
            .map_err(map_structural_error)?;
        let replacement_field = usize::try_from(field)
            .map_err(|_| Flow::Trap("product field exceeds host width".into()))?;
        for (field_index, expected) in structural_fields.into_iter().enumerate() {
            let semantic = if field_index == replacement_field {
                self.copy_semantic(replacement, expected)
            } else {
                self.projected_semantic(owner, root_type, field_index, expected)
            };
            let semantic = match semantic {
                Ok(value) => value,
                Err(flow) => {
                    self.abort_structural_destination(destination);
                    return Err(flow);
                }
            };
            if let Err(failure) =
                self.structural
                    .runtime
                    .initialize_node(destination, field_index, semantic)
            {
                self.abort_structural_destination(destination);
                return Err(map_structural_error(failure.error));
            }
        }
        self.structural
            .runtime
            .finish_destination(destination)
            .map(|key| {
                EvalValue::StructuralOwner(crate::eval::EvalStructuralOwner {
                    key,
                    value_type: root_type,
                })
            })
            .map_err(|error| {
                self.abort_structural_destination(destination);
                map_structural_error(error)
            })
    }
}

mod access;
