use crate::codegen::*;

impl Emitter<'_> {
    pub(in crate::codegen) fn emit_constant(&mut self, constant: &Constant) -> Result<()> {
        match constant {
            Constant::Unit => self.proto.emit(Op::Unit),
            Constant::Bool(false) => self.proto.emit(Op::False),
            Constant::Bool(true) => self.proto.emit(Op::True),
            Constant::I64(value) => {
                let constant = add_constant(self.chunk, BytecodeConstant::I64(*value))?;
                self.proto.emit_op_u16(Op::LoadConst, constant);
            }
            Constant::F64(value) => {
                let constant = add_constant(self.chunk, BytecodeConstant::F64(*value))?;
                self.proto.emit_op_u16(Op::LoadConst, constant);
            }
            Constant::Str(value) => {
                let constant = add_constant(self.chunk, BytecodeConstant::Str(value.clone()))?;
                self.proto.emit_op_u16(Op::LoadConst, constant);
            }
            Constant::Symbol(value) => {
                let constant = add_constant(self.chunk, BytecodeConstant::Symbol(value.clone()))?;
                self.proto.emit_op_u16(Op::LoadConst, constant);
            }
            Constant::EmptyList => self.proto.emit(Op::EmptyList),
            Constant::None => self.proto.emit(Op::OptionNone),
        }
        Ok(())
    }
}
