use std::collections::HashMap;

use crate::optimize::*;
use crate::{BlockId, Instruction, RuntimeOp, Signature, SsaType, ValueId};

#[derive(Clone, Copy)]
pub(crate) struct CheckerDefinition<'a> {
    pub(crate) block: BlockId,
    pub(crate) instruction_index: Option<usize>,
    pub(crate) ty: &'a SsaType,
    pub(crate) instruction: Option<&'a Instruction>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) struct CheckerPosition {
    pub(crate) block: BlockId,
    pub(crate) instruction_index: usize,
    pub(crate) value: ValueId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(crate) struct CheckerExpressionKey<'a> {
    pub(crate) operation: RuntimeOp,
    pub(crate) arguments: &'a [ValueId],
    pub(crate) signature: &'a Signature,
    pub(crate) result_type: &'a SsaType,
}

pub(crate) struct CheckerFunctionIndexes<'a> {
    pub(crate) definitions: Vec<Option<CheckerDefinition<'a>>>,
    pub(crate) constants: Vec<Option<i64>>,
    pub(crate) expressions: HashMap<CheckerExpressionKey<'a>, Vec<CheckerPosition>>,
    pub(crate) dominance: CheckerDominance,
}

pub(crate) struct CheckerIndexes<'a> {
    pub(crate) functions: Vec<CheckerFunctionIndexes<'a>>,
}
