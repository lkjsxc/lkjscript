use crate::optimize::*;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ProgramShape {
    pub(crate) functions: u64,
    pub(crate) blocks: u64,
    pub(crate) parameters: u64,
    pub(crate) instructions: u64,
    pub(crate) operands: u64,
    pub(crate) frame_facts: u64,
    pub(crate) type_nodes: u64,
    pub(crate) metadata_items: u64,
    pub(crate) string_and_metadata_bytes: u64,
}

impl ProgramShape {
    pub(crate) fn allocation_units(self) -> u64 {
        self.functions
            .saturating_add(self.blocks)
            .saturating_add(self.parameters)
            .saturating_add(self.instructions)
            .saturating_add(self.operands)
            .saturating_add(self.frame_facts)
            .saturating_add(self.type_nodes)
            .saturating_add(self.metadata_items)
            .saturating_add(self.string_and_metadata_bytes)
    }

    pub(crate) fn comparison_units(self) -> u64 {
        self.allocation_units()
    }

    pub(crate) fn validation_units(self) -> u64 {
        let words = self.blocks.saturating_add(63) / 64;
        self.allocation_units()
            .saturating_add(self.blocks.saturating_mul(words))
            .saturating_add(self.operands)
    }
}

pub(crate) struct ShapeCounter<'a> {
    pub(crate) shape: ProgramShape,
    pub(crate) limits: OptimizationLimits,
    pub(crate) budget: &'a mut Budget,
}

#[derive(Clone, Copy)]
pub(crate) enum ShapeField {
    Functions,
    Blocks,
    Parameters,
    Instructions,
    Operands,
    FrameFacts,
    TypeNodes,
    MetadataItems,
    StringAndMetadataBytes,
}
