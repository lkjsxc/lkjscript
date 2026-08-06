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
    pub failure_cleanups: Vec<FailureCleanupNode>,
    pub effects: EffectSet,
    pub entry: BlockId,
    pub blocks: Vec<Block>,
    pub origin: Origin,
}

pub struct FailureCleanupActions<'a> {
    nodes: &'a [FailureCleanupNode],
    pending: std::array::IntoIter<Option<FailureCleanupId>, 3>,
    current: Option<FailureCleanupId>,
}

impl<'a> Iterator for FailureCleanupActions<'a> {
    type Item = &'a FailureCleanupAction;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(id) = self.current {
                let node = self.nodes.get(id.index()?)?;
                self.current = node.next;
                return Some(&node.action);
            }
            self.current = self.pending.find_map(|root| root);
            self.current?;
        }
    }
}

impl Function {
    #[must_use]
    pub fn failure_cleanup_actions(
        &self,
        roots: Option<FailureCleanupRoots>,
    ) -> FailureCleanupActions<'_> {
        let roots = roots.map_or([None; 3], |roots| {
            [roots.loans, roots.unplaced, roots.places]
        });
        FailureCleanupActions {
            nodes: &self.failure_cleanups,
            pending: roots.into_iter(),
            current: None,
        }
    }
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
    pub offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytecodeBlockLink {
    pub block: BlockId,
    pub offset: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionBytecodeLink {
    pub function: FunctionId,
    pub prototype: Option<u64>,
    pub is_main: bool,
    pub blocks: Vec<BytecodeBlockLink>,
    pub instructions: Vec<BytecodeInstructionLink>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytecodeLinkMetadata {
    pub main: FunctionId,
    pub functions: Vec<FunctionBytecodeLink>,
}
