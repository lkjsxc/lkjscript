use super::*;

#[derive(Debug)]
pub struct FunctionBuilder {
    pub(super) function: FunctionId,
    pub(super) signature: Signature,
    pub(super) source_function: SourceFunctionId,
    pub(super) signatures: Vec<(FunctionId, Signature)>,
    pub(super) blocks: Vec<Block>,
    pub(super) entry: Option<BlockId>,
    pub(super) values: Vec<ValueFact>,
    pub(super) locals: Vec<LocalFact>,
}
