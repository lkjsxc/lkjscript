use crate::codegen::*;

impl Emitter<'_> {
    pub(in crate::codegen) fn emit_constant(&mut self, constant: &Constant) -> Result<()> {
        match constant {
            Constant::Unit => self.proto.try_emit(Op::Unit)?,
            Constant::Bool(false) => self.proto.try_emit(Op::False)?,
            Constant::Bool(true) => self.proto.try_emit(Op::True)?,
            Constant::I64(value) => {
                let constant = add_constant(self.chunk, BytecodeConstant::I64(*value))?;
                self.proto.try_emit_op_u64(Op::LoadConst, constant.0)?;
            }
            Constant::F64(value) => {
                let constant = add_constant(self.chunk, BytecodeConstant::F64(*value))?;
                self.proto.try_emit_op_u64(Op::LoadConst, constant.0)?;
            }
            Constant::Str(value) => {
                let constant = add_constant(self.chunk, BytecodeConstant::Str(value.clone()))?;
                self.proto.try_emit_op_u64(Op::LoadConst, constant.0)?;
            }
            Constant::StaticBytes(value) => {
                let constant = add_constant(
                    self.chunk,
                    BytecodeConstant::StaticBytes(value.clone().into_boxed_slice()),
                )?;
                self.proto.try_emit_op_u64(Op::LoadConst, constant.0)?;
            }
            Constant::Symbol(value) => {
                let constant = add_constant(self.chunk, BytecodeConstant::Symbol(value.clone()))?;
                self.proto.try_emit_op_u64(Op::LoadConst, constant.0)?;
            }
            Constant::EmptyList => self.proto.try_emit(Op::EmptyList)?,
        }
        Ok(())
    }
}
