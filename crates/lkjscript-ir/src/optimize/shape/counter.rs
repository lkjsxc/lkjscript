use crate::optimize::*;
use crate::{Constant, Instruction, InstructionKind, Signature, SsaType};

impl<'a> ShapeCounter<'a> {
    pub(crate) fn new(budget: &'a mut Budget) -> Self {
        Self {
            shape: ProgramShape::default(),
            limits: budget.limits,
            budget,
        }
    }

    pub(crate) fn add_bounded(
        &mut self,
        field: ShapeField,
        amount: u64,
    ) -> Result<(), OptimizationError> {
        let (slot, limit) = match field {
            ShapeField::Functions => (&mut self.shape.functions, self.limits.max_functions),
            ShapeField::Blocks => (&mut self.shape.blocks, self.limits.max_blocks),
            ShapeField::Parameters => (&mut self.shape.parameters, self.limits.max_parameters),
            ShapeField::Instructions => {
                (&mut self.shape.instructions, self.limits.max_instructions)
            }
            ShapeField::Operands => (&mut self.shape.operands, self.limits.max_operands),
            ShapeField::FrameFacts => (&mut self.shape.frame_facts, self.limits.max_frame_facts),
            ShapeField::TypeNodes => (&mut self.shape.type_nodes, self.limits.max_type_nodes),
            ShapeField::MetadataItems => (
                &mut self.shape.metadata_items,
                self.limits.max_metadata_items,
            ),
            ShapeField::StringAndMetadataBytes => (
                &mut self.shape.string_and_metadata_bytes,
                self.limits.max_string_and_metadata_bytes,
            ),
        };
        *slot = slot.checked_add(amount).ok_or_else(budget_error)?;
        if *slot > limit {
            return Err(budget_error());
        }
        self.budget.charge(amount)
    }

    pub(crate) fn add_string(&mut self, value: &str) -> Result<(), OptimizationError> {
        self.add_bounded(
            ShapeField::StringAndMetadataBytes,
            u64::try_from(value.len()).map_err(|_| budget_error())?,
        )
    }

    pub(crate) fn add_metadata(&mut self) -> Result<(), OptimizationError> {
        self.add_bounded(ShapeField::MetadataItems, 1)?;
        self.add_bounded(ShapeField::StringAndMetadataBytes, 8)
    }

    pub(crate) fn add_signature(&mut self, signature: &Signature) -> Result<(), OptimizationError> {
        self.add_metadata()?;
        self.add_bounded(
            ShapeField::Parameters,
            u64::try_from(signature.parameters.len()).map_err(|_| budget_error())?,
        )?;
        self.add_bounded(
            ShapeField::MetadataItems,
            u64::try_from(signature.type_parameters.len()).map_err(|_| budget_error())?,
        )?;
        for name in &signature.type_parameters {
            self.add_string(name)?;
        }
        for bound in &signature.bounds {
            self.add_string(&bound.parameter)?;
            self.add_metadata()?;
        }
        for ty in &signature.parameters {
            self.add_type(ty)?;
        }
        self.add_type(&signature.result)
    }

    pub(crate) fn add_type(&mut self, root: &SsaType) -> Result<(), OptimizationError> {
        let mut pending = vec![root];
        while let Some(ty) = pending.pop() {
            self.add_bounded(ShapeField::TypeNodes, 1)?;
            self.add_bounded(ShapeField::StringAndMetadataBytes, 1)?;
            match ty {
                SsaType::Owned(inner)
                | SsaType::Ref(inner)
                | SsaType::RefMut(inner)
                | SsaType::List(inner) => pending.push(inner),
                SsaType::Enum { arguments, .. } => {
                    pending.extend(arguments);
                }
                SsaType::Function(signature) => {
                    self.add_metadata()?;
                    self.add_bounded(
                        ShapeField::Parameters,
                        u64::try_from(signature.parameters.len()).map_err(|_| budget_error())?,
                    )?;
                    self.add_bounded(
                        ShapeField::MetadataItems,
                        u64::try_from(signature.type_parameters.len())
                            .map_err(|_| budget_error())?,
                    )?;
                    for name in &signature.type_parameters {
                        self.add_string(name)?;
                    }
                    for bound in &signature.bounds {
                        self.add_string(&bound.parameter)?;
                        self.add_metadata()?;
                    }
                    for parameter in &signature.parameters {
                        pending.push(parameter);
                    }
                    pending.push(&signature.result);
                }
                SsaType::TypeParameter(name) => self.add_string(name)?,
                _ => {}
            }
        }
        Ok(())
    }

    pub(crate) fn add_frame(&mut self, frame: &crate::FrameState) -> Result<(), OptimizationError> {
        let facts = 1_u64
            .checked_add(u64::try_from(frame.locals.len()).map_err(|_| budget_error())?)
            .and_then(|value| value.checked_add(frame.operand_stack.len() as u64))
            .ok_or_else(budget_error)?;
        self.add_bounded(ShapeField::FrameFacts, facts)?;
        self.add_bounded(ShapeField::StringAndMetadataBytes, facts.saturating_mul(16))
    }

    pub(crate) fn add_instruction(
        &mut self,
        instruction: &Instruction,
    ) -> Result<(), OptimizationError> {
        self.add_bounded(ShapeField::Instructions, 1)?;
        self.add_metadata()?;
        self.add_type(&instruction.ty)?;
        if let Some(frame) = &instruction.metadata.frame_state {
            self.add_frame(frame)?;
        }
        let operands = instruction_operand_count(&instruction.kind)?;
        self.add_bounded(ShapeField::Operands, operands)?;
        match &instruction.kind {
            InstructionKind::Constant(Constant::Str(value) | Constant::Symbol(value)) => {
                self.add_string(value)?
            }
            InstructionKind::Runtime { signature, .. }
            | InstructionKind::Call { signature, .. } => {
                self.add_signature(signature)?;
                if let InstructionKind::Call {
                    instantiation: Some(instantiation),
                    ..
                } = &instruction.kind
                {
                    for substitution in &instantiation.substitutions {
                        self.add_string(&substitution.parameter)?;
                        self.add_type(&substitution.ty)?;
                    }
                    for witness in &instantiation.witnesses {
                        self.add_type(&witness.ty)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
