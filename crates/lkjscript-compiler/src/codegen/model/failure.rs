use super::*;

pub(in crate::codegen) struct FailureCodegenIndex<'a> {
    types: HashMap<ValueId, &'a SsaType>,
    definitions: HashMap<ValueId, &'a Instruction>,
    moved: HashSet<ValueId>,
}

impl<'a> FailureCodegenIndex<'a> {
    pub(super) fn new(function: &'a Function) -> Result<Self> {
        let value_count = function.blocks.iter().try_fold(0_usize, |total, block| {
            total
                .checked_add(block.parameters.len())
                .and_then(|total| total.checked_add(block.instructions.len()))
                .ok_or_else(|| Error::msg("SSA value index size overflow"))
        })?;
        let mut types = HashMap::new();
        let mut definitions = HashMap::new();
        let mut moved = HashSet::new();
        types
            .try_reserve(value_count)
            .map_err(|_| Error::host("failure cleanup type index reservation failed"))?;
        definitions
            .try_reserve(value_count)
            .map_err(|_| Error::host("failure cleanup definition index reservation failed"))?;
        moved
            .try_reserve(value_count)
            .map_err(|_| Error::host("moved-value index reservation failed"))?;
        for block in &function.blocks {
            for parameter in &block.parameters {
                types.insert(parameter.id, &parameter.ty);
            }
            for instruction in &block.instructions {
                types.insert(instruction.id, &instruction.ty);
                definitions.insert(instruction.id, instruction);
                if matches!(instruction.kind, InstructionKind::Move { .. }) {
                    moved.insert(instruction.id);
                }
            }
        }
        Ok(Self {
            types,
            definitions,
            moved,
        })
    }

    fn value_type(&self, value: ValueId) -> Result<&'a SsaType> {
        self.types
            .get(&value)
            .copied()
            .ok_or_else(|| Error::msg("failure cleanup references missing SSA value type"))
    }

    fn definition(&self, value: ValueId) -> Result<&'a Instruction> {
        self.definitions
            .get(&value)
            .copied()
            .ok_or_else(|| Error::msg("failure cleanup destination has no defining instruction"))
    }

    pub(super) fn moved(&self, value: ValueId) -> bool {
        self.moved.contains(&value)
    }
}

pub(super) fn compile_failure_cleanups(
    function: &Function,
    slots: &HashMap<ValueId, usize>,
    chunk: &Chunk,
    index: &FailureCodegenIndex<'_>,
) -> Result<(
    BytecodeFailureCleanupInterner,
    Vec<BytecodeFailureCleanupId>,
)> {
    let mut interner = BytecodeFailureCleanupInterner::default();
    let mut mapping = Vec::with_capacity(function.failure_cleanups.len());
    for node in &function.failure_cleanups {
        let next = node
            .next
            .map(|next| {
                mapping
                    .get(next.index().unwrap_or(usize::MAX))
                    .copied()
                    .ok_or_else(|| Error::msg("SSA cleanup chain has a forward link"))
            })
            .transpose()?;
        let action = compile_failure_action(function, slots, chunk, index, &node.action)?;
        mapping.push(interner.intern(action, next)?);
    }
    Ok((interner, mapping))
}

pub(super) fn compile_unentered_cleanup_action(
    value: ValueId,
    slots: &HashMap<ValueId, usize>,
    chunk: &Chunk,
    index: &FailureCodegenIndex<'_>,
) -> Result<BytecodeFailureCleanupAction> {
    let local = slots
        .get(&value)
        .copied()
        .ok_or_else(|| Error::msg("unentered cleanup lost SSA local slot"))?;
    match index.value_type(value)? {
        SsaType::ByteVector | SsaType::Bytes => Ok(BytecodeFailureCleanupAction::DropUnique {
            local,
            place: None,
            kind: unique_value_kind(index.value_type(value)?)
                .ok_or_else(|| Error::msg("unentered cleanup owner has non-unique type"))?,
        }),
        SsaType::Resource(kind) => Ok(BytecodeFailureCleanupAction::DropResource {
            local,
            place: None,
            kind: *kind,
        }),
        ty @ (SsaType::Str | SsaType::Path | SsaType::Product(_) | SsaType::Enum { .. }) => {
            Ok(BytecodeFailureCleanupAction::DropStructural {
                local,
                place: None,
                representation: structural_owner_representation(chunk, ty).ok_or_else(|| {
                    Error::msg("unentered structural owner has no representation")
                })?,
            })
        }
        SsaType::StructuralDestination(_) => {
            Ok(BytecodeFailureCleanupAction::AbortStructuralDestination {
                local,
                destination: structural_destination_for_value(chunk, index, value)?,
            })
        }
        SsaType::Unit
        | SsaType::Bool
        | SsaType::I64
        | SsaType::F64
        | SsaType::Symbol
        | SsaType::ByteSlice
        | SsaType::ByteSliceMut
        | SsaType::Capability(_)
        | SsaType::List(_)
        | SsaType::Function(_)
        | SsaType::TypeParameter(_) => Err(Error::msg(
            "unentered cleanup argument is not an owned value",
        )),
    }
}

include!("failure/action.rs");
