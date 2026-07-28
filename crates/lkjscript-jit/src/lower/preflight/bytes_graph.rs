use super::*;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) enum BytesNode {
    Value(FunctionId, ValueId),
    Result(FunctionId),
}

pub(super) struct BytesSets {
    parent: Vec<usize>,
    mode: Vec<Option<BytesMode>>,
}

impl BytesSets {
    fn new(len: usize) -> Self {
        Self {
            parent: (0..len).collect(),
            mode: vec![None; len],
        }
    }

    fn root(&mut self, index: usize) -> usize {
        let parent = self.parent[index];
        if parent != index {
            self.parent[index] = self.root(parent);
        }
        self.parent[index]
    }

    pub(super) fn assign(&mut self, index: usize, mode: BytesMode) -> Result<(), ()> {
        let root = self.root(index);
        match self.mode[root] {
            Some(current) if current != mode => Err(()),
            _ => {
                self.mode[root] = Some(mode);
                Ok(())
            }
        }
    }

    pub(super) fn union(&mut self, left: usize, right: usize) -> Result<(), ()> {
        let left = self.root(left);
        let right = self.root(right);
        if left == right {
            return Ok(());
        }
        match (self.mode[left], self.mode[right]) {
            (Some(a), Some(b)) if a != b => return Err(()),
            (None, Some(mode)) => self.mode[left] = Some(mode),
            _ => {}
        }
        self.parent[right] = left;
        Ok(())
    }

    fn mode(&mut self, index: usize) -> Option<BytesMode> {
        let root = self.root(index);
        self.mode[root]
    }
}

pub(super) fn analyze_bytes_modes(
    program: &lkjscript_ir::Program,
    functions: &[FunctionId],
) -> Result<BytesModes, LoweringError> {
    let mut nodes = Vec::new();
    for id in functions {
        let function = source_function(program, *id)?;
        if function.signature.result.as_ref() == &SsaType::Bytes {
            nodes.push(BytesNode::Result(*id));
        }
        for block in &function.blocks {
            nodes.extend(
                block
                    .parameters
                    .iter()
                    .filter(|value| value.ty == SsaType::Bytes)
                    .map(|value| BytesNode::Value(*id, value.id)),
            );
            nodes.extend(
                block
                    .instructions
                    .iter()
                    .filter(|value| value.ty == SsaType::Bytes)
                    .map(|value| BytesNode::Value(*id, value.id)),
            );
        }
    }
    let indexes: HashMap<_, _> = nodes
        .iter()
        .copied()
        .enumerate()
        .map(|(index, node)| (node, index))
        .collect();
    let mut sets = BytesSets::new(nodes.len());
    for id in functions {
        analyze_function(program, source_function(program, *id)?, &indexes, &mut sets)?;
    }
    let mut modes = HashMap::new();
    let mut results = HashMap::new();
    for node in nodes {
        let mode = sets.mode(indexes[&node]).ok_or_else(|| {
            bytes_mode_error(match node {
                BytesNode::Value(id, _) | BytesNode::Result(id) => id,
            })
        })?;
        match node {
            BytesNode::Value(function, value) => {
                modes.insert((function, value), mode);
            }
            BytesNode::Result(function) => {
                results.insert(function, mode);
            }
        }
    }
    Ok(BytesModes::new(modes, results))
}

fn analyze_function(
    program: &lkjscript_ir::Program,
    function: &Function,
    indexes: &HashMap<BytesNode, usize>,
    sets: &mut BytesSets,
) -> Result<(), LoweringError> {
    for block in &function.blocks {
        for instruction in &block.instructions {
            let output = BytesNode::Value(function.id, instruction.id);
            match &instruction.kind {
                InstructionKind::Constant(Constant::StaticBytes(_)) => {
                    assign_bytes(function.id, output, BytesMode::Static, indexes, sets)?;
                }
                InstructionKind::Copy(value) if instruction.ty == SsaType::Bytes => {
                    connect_bytes(
                        function.id,
                        output,
                        BytesNode::Value(function.id, *value),
                        indexes,
                        sets,
                    )?;
                    assign_bytes(function.id, output, BytesMode::Static, indexes, sets)?;
                }
                InstructionKind::Move { value, .. } if instruction.ty == SsaType::Bytes => {
                    connect_bytes(
                        function.id,
                        output,
                        BytesNode::Value(function.id, *value),
                        indexes,
                        sets,
                    )?;
                    assign_bytes(function.id, output, BytesMode::Owner, indexes, sets)?;
                }
                InstructionKind::Borrow { value, .. } if instruction.ty == SsaType::Bytes => {
                    assign_bytes(function.id, output, BytesMode::Loan, indexes, sets)?;
                    assign_bytes(
                        function.id,
                        BytesNode::Value(function.id, *value),
                        BytesMode::Owner,
                        indexes,
                        sets,
                    )?;
                }
                InstructionKind::Runtime { operation, .. }
                    if instruction.ty == SsaType::Bytes
                        && matches!(
                            operation,
                            RuntimeOp::CopyBytesSlice
                                | RuntimeOp::CloneBytes
                                | RuntimeOp::FreezeByteVector
                        ) =>
                {
                    assign_bytes(function.id, output, BytesMode::Owner, indexes, sets)?;
                }
                InstructionKind::Call {
                    target: CallTarget::Direct(callee),
                    arguments,
                    ..
                } => connect_bytes_call(
                    program, function, *callee, arguments, output, indexes, sets,
                )?,
                InstructionKind::PlaceInit { value, .. } | InstructionKind::Drop { value, .. } => {
                    assign_if_bytes(function.id, *value, BytesMode::Owner, indexes, sets)?;
                }
                InstructionKind::EndBorrow { value, .. } => {
                    assign_if_bytes(function.id, *value, BytesMode::Loan, indexes, sets)?;
                }
                _ => {}
            }
        }
        connect_bytes_terminator(program, function, &block.terminator, indexes, sets)?;
    }
    Ok(())
}
