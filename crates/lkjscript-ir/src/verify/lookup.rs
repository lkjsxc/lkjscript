use crate::{
    Block, BlockId, Function, FunctionId, ImplId, IrError, ProductId, Program, SsaType, Terminator,
    TraitId, ValueId,
};

pub(crate) fn successors(terminator: &Terminator) -> Vec<BlockId> {
    match terminator {
        Terminator::Branch { target, .. } => vec![*target],
        Terminator::ConditionalBranch {
            true_target,
            false_target,
            ..
        } => vec![*true_target, *false_target],
        _ => Vec::new(),
    }
}

pub(crate) fn function(program: &Program, id: FunctionId) -> crate::Result<&Function> {
    function_by_id(program, id)
}

pub(crate) fn function_by_id(program: &Program, id: FunctionId) -> crate::Result<&Function> {
    id.index()
        .and_then(|index| program.functions.get(index))
        .filter(|function| function.id == id)
        .ok_or_else(|| IrError::new(format!("missing SSA FunctionId {}", id.raw())))
}

pub(crate) fn trait_by_id(program: &Program, id: TraitId) -> crate::Result<&crate::TraitMetadata> {
    id.index()
        .and_then(|index| program.traits.get(index))
        .filter(|trait_metadata| trait_metadata.id == id)
        .ok_or_else(|| IrError::new(format!("missing SSA TraitId {}", id.raw())))
}

pub(crate) fn impl_by_id(program: &Program, id: ImplId) -> crate::Result<&crate::ImplMetadata> {
    id.index()
        .and_then(|index| program.implementations.get(index))
        .filter(|implementation| implementation.id == id)
        .ok_or_else(|| IrError::new(format!("missing SSA ImplId {}", id.raw())))
}

pub(crate) fn product_by_id(
    program: &Program,
    id: ProductId,
) -> crate::Result<&crate::ProductMetadata> {
    id.index()
        .and_then(|index| program.products.get(index))
        .filter(|product| product.id == id)
        .ok_or_else(|| IrError::new(format!("missing SSA ProductId {}", id.raw())))
}

pub(crate) fn place_by_id(
    function: &Function,
    id: crate::PlaceId,
) -> crate::Result<&crate::PlaceMetadata> {
    id.index()
        .and_then(|index| function.places.get(index))
        .filter(|place| place.id == id)
        .ok_or_else(|| IrError::new(format!("missing SSA PlaceId {}", id.raw())))
}

pub(crate) fn block(function: &Function, id: BlockId) -> crate::Result<&Block> {
    block_by_id(function, id)
}

pub(crate) fn block_by_id(function: &Function, id: BlockId) -> crate::Result<&Block> {
    id.index()
        .and_then(|index| function.blocks.get(index))
        .filter(|block| block.id == id)
        .ok_or_else(|| IrError::new(format!("missing SSA BlockId {}", id.raw())))
}

pub(crate) fn value_type(types: &[SsaType], id: ValueId) -> crate::Result<&SsaType> {
    id.index()
        .and_then(|index| types.get(index))
        .ok_or_else(|| IrError::new(format!("missing SSA ValueId {}", id.raw())))
}

pub(crate) fn fail<T>(message: impl Into<String>) -> crate::Result<T> {
    Err(IrError::new(message))
}
