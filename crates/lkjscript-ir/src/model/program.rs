use super::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub id: BlockId,
    pub parameters: Vec<BlockParameter>,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
    pub metadata: BlockMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceMetadata {
    pub id: PlaceId,
    pub binding: BindingId,
    pub ty: SsaType,
    /// `Some` is one exact owned whole-place cleanup obligation. `None` is a
    /// borrowed resource parameter or a non-affine lexical place.
    pub drop_glue: Option<DropGlueIdentity>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub id: FunctionId,
    pub name: String,
    pub signature: Signature,
    pub places: Vec<PlaceMetadata>,
    pub failure_cleanups: Vec<FailureCleanupPlan>,
    pub effects: EffectSet,
    pub entry: BlockId,
    pub blocks: Vec<Block>,
    pub origin: Origin,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub prepared_identity: lkjscript_contracts::PreparedProgramIdentity,
    pub memory: StructuralMemoryMetadata,
    pub region_products: Vec<RegionProductMetadata>,
    pub sources: Vec<SourceMetadata>,
    pub products: Vec<ProductMetadata>,
    pub enums: Vec<EnumMetadata>,
    pub traits: Vec<TraitMetadata>,
    pub implementations: Vec<ImplMetadata>,
    pub functions: Vec<Function>,
    pub main: FunctionId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytecodeInstructionLink {
    pub value: ValueId,
    pub offset: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytecodeBlockLink {
    pub block: BlockId,
    pub offset: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionBytecodeLink {
    pub function: FunctionId,
    pub prototype: Option<u32>,
    pub is_main: bool,
    pub blocks: Vec<BytecodeBlockLink>,
    pub instructions: Vec<BytecodeInstructionLink>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytecodeLinkMetadata {
    pub main: FunctionId,
    pub functions: Vec<FunctionBytecodeLink>,
}
