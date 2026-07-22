//! Bytecode interpreter.

mod calls;
mod dispatch;
mod ext_ops;
mod numeric;

use lkjscript_core::{Chunk, Constant, Error, HeapObj, JitHook, Result, Value};

use crate::arena::Arena;
use crate::host_ext::ResourceTable;

pub struct Frame {
    pub proto: u32,
    pub ip: usize,
    pub stack_base: usize,
    pub locals_base: usize,
}

pub struct Vm<'a, J: JitHook> {
    pub chunk: &'a Chunk,
    pub globals: Vec<Value>,
    pub stack: Vec<Value>,
    pub frames: Vec<Frame>,
    pub arena: Arena,
    pub jit: J,
    pub exit_code: Option<i32>,
    pub args: Vec<String>,
    pub resources: ResourceTable,
}

impl<'a, J: JitHook> Vm<'a, J> {
    pub fn new(chunk: &'a Chunk, jit: J, args: Vec<String>) -> Self {
        Self {
            chunk,
            globals: vec![Value::INVALID; chunk.global_names.len()],
            stack: Vec::new(),
            frames: Vec::new(),
            arena: Arena::default(),
            jit,
            exit_code: None,
            args,
            resources: ResourceTable::default(),
        }
    }

    pub fn run(&mut self) -> Result<Value> {
        self.frames.push(Frame {
            proto: u32::MAX,
            ip: 0,
            stack_base: 0,
            locals_base: 0,
        });
        for _ in 0..self.chunk.main.locals {
            self.stack.push(Value::INVALID);
        }
        loop {
            if let Some(code) = self.exit_code {
                crate::host_term::restore_tty();
                std::process::exit(code);
            }
            if self.frames.is_empty() {
                return self.pop();
            }
            if self.arena.needs_collect() {
                let mut roots = self.globals.clone();
                roots.extend_from_slice(&self.stack);
                self.arena.collect(&roots);
            }
            self.step()?;
        }
    }

    pub fn code_len(&self) -> Result<usize> {
        Ok(self.code()?.len())
    }

    pub fn code(&self) -> Result<&[u8]> {
        let fr = self.frames.last().ok_or_else(|| Error::msg("no frame"))?;
        if fr.proto == u32::MAX {
            Ok(&self.chunk.main.code)
        } else {
            self.chunk
                .protos
                .get(fr.proto as usize)
                .map(|proto| proto.code.as_slice())
                .ok_or_else(|| Error::msg("frame proto index out of range"))
        }
    }

    pub fn read_u8(&mut self) -> Result<u8> {
        let (proto, ip) = {
            let fr = self.frames.last().ok_or_else(|| Error::msg("no frame"))?;
            (fr.proto, fr.ip)
        };
        let code = if proto == u32::MAX {
            &self.chunk.main.code
        } else {
            &self
                .chunk
                .protos
                .get(proto as usize)
                .ok_or_else(|| Error::msg("frame proto index out of range"))?
                .code
        };
        let b = *code.get(ip).ok_or_else(|| Error::msg("ip out of range"))?;
        if let Some(fr) = self.frames.last_mut() {
            fr.ip += 1;
        }
        Ok(b)
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        let a = self.read_u8()? as u16;
        let b = self.read_u8()? as u16;
        Ok(a | (b << 8))
    }

    pub fn push(&mut self, v: Value) {
        self.stack.push(v);
    }

    pub fn make_i64(&mut self, number: i64) -> Value {
        Value::from_small_i64(number).unwrap_or_else(|| self.arena.alloc(HeapObj::Int(number)))
    }

    pub fn as_i64(&self, value: Value) -> Result<i64> {
        if let Some(number) = value.as_small_i64() {
            return Ok(number);
        }
        match value.as_heap().and_then(|_| self.arena.get(value).ok()) {
            Some(HeapObj::Int(number)) => Ok(*number),
            _ => Err(Error::msg("expected I64")),
        }
    }

    pub fn pop(&mut self) -> Result<Value> {
        let value = self
            .stack
            .pop()
            .ok_or_else(|| Error::msg("VM stack underflow"))?;
        if value.is_invalid() {
            return Err(Error::msg("uninitialized VM value"));
        }
        Ok(value)
    }

    pub fn peek(&self) -> Result<Value> {
        let value = self
            .stack
            .last()
            .copied()
            .ok_or_else(|| Error::msg("VM stack underflow"))?;
        if value.is_invalid() {
            return Err(Error::msg("uninitialized VM value"));
        }
        Ok(value)
    }

    pub fn load_const(&mut self, id: usize) -> Result<Value> {
        match self
            .chunk
            .constants
            .get(id)
            .ok_or_else(|| Error::msg("bad const"))?
        {
            Constant::I64(number) => Ok(self.make_i64(*number)),
            Constant::F64(number) => Ok(self.arena.alloc(HeapObj::Float(*number))),
            Constant::Str(text) => Ok(self.arena.alloc(HeapObj::Str(text.clone()))),
            Constant::Symbol(symbol) => Ok(self.arena.alloc(HeapObj::Symbol(symbol.clone()))),
            Constant::Proto(proto) => Ok(self.make_i64(i64::from(*proto))),
        }
    }

    fn step(&mut self) -> Result<()> {
        let code_len = self.code_len()?;
        let ip = self
            .frames
            .last()
            .map(|f| f.ip)
            .ok_or_else(|| Error::msg("no frame"))?;
        if ip >= code_len {
            return Err(Error::msg("function ended without Return"));
        }
        let op = self.read_u8()?;
        dispatch::dispatch(self, op)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use lkjscript_core::{Chunk, NullJit, Op, ProductFieldRef, ProductId, ProductMetadata};

    use super::Vm;

    fn vm_error(chunk: Chunk) -> String {
        let mut vm = Vm::new(&chunk, NullJit, Vec::new());
        vm.run().expect_err("malformed chunk must fail").to_string()
    }

    #[test]
    fn missing_and_uninitialized_vm_values_are_errors() {
        let mut chunk = Chunk::new();
        assert!(vm_error(chunk.clone()).contains("without Return"));

        chunk.main.code = vec![Op::Pop as u8, Op::Return as u8];
        assert!(vm_error(chunk).contains("stack underflow"));

        let mut local = Chunk::new();
        local.main.locals = 1;
        local.main.emit_op_u8(Op::LoadLocal, 0);
        local.main.emit(Op::Return);
        assert!(vm_error(local).contains("uninitialized slot"));

        let mut global = Chunk::new();
        global.global_names.push("missing".into());
        global.main.emit_op_u16(Op::LoadGlobal, 0);
        global.main.emit(Op::Return);
        assert!(vm_error(global).contains("uninitialized slot"));

        let mut store = Chunk::new();
        store.main.emit(Op::Unit);
        store.main.emit_op_u16(Op::StoreGlobal, 0);
        store.main.emit(Op::Return);
        assert!(vm_error(store).contains("StoreGlobal slot out of range"));
    }

    #[test]
    fn malformed_product_metadata_operands_and_categories_are_errors() {
        let mut missing_product = Chunk::new();
        missing_product.main.emit_op_u16(Op::MakeProduct, 0);
        missing_product.main.emit(Op::Return);
        assert!(vm_error(missing_product).contains("product metadata"));

        for opcode in [Op::MakeProduct, Op::LoadProductField, Op::WithProductField] {
            let mut truncated = Chunk::new();
            truncated.main.code = vec![opcode as u8, 0];
            assert!(vm_error(truncated).contains("ip out of range"));
        }

        let mut wrong_metadata_identity = Chunk::new();
        wrong_metadata_identity.products.push(ProductMetadata {
            id: ProductId::new(1),
            name: "Wrong".into(),
            fields: Vec::new(),
        });
        wrong_metadata_identity.main.emit_op_u16(Op::MakeProduct, 0);
        wrong_metadata_identity.main.emit(Op::Return);
        assert!(vm_error(wrong_metadata_identity).contains("identity is invalid"));

        let mut too_wide = Chunk::new();
        too_wide.products.push(ProductMetadata {
            id: ProductId::new(0),
            name: "Wide".into(),
            fields: (0..16).map(|index| format!("f{index}")).collect(),
        });
        too_wide.main.emit_op_u16(Op::MakeProduct, 0);
        too_wide.main.emit(Op::Return);
        assert!(vm_error(too_wide).contains("exceeds field limit"));

        let product = ProductMetadata {
            id: ProductId::new(0),
            name: "Point".into(),
            fields: vec!["x".into()],
        };

        let mut missing_value = Chunk::new();
        missing_value.products.push(product.clone());
        missing_value.main.emit_op_u16(Op::MakeProduct, 0);
        missing_value.main.emit(Op::Return);
        assert!(vm_error(missing_value).contains("stack underflow"));

        let mut missing_descriptor = Chunk::new();
        missing_descriptor.products.push(product.clone());
        missing_descriptor.main.emit(Op::Unit);
        missing_descriptor.main.emit_op_u16(Op::MakeProduct, 0);
        missing_descriptor.main.emit_op_u16(Op::LoadProductField, 0);
        missing_descriptor.main.emit(Op::Return);
        assert!(vm_error(missing_descriptor).contains("descriptor index out of range"));

        let mut bad_field = Chunk::new();
        bad_field.products.push(product.clone());
        bad_field.product_fields.push(ProductFieldRef {
            product: ProductId::new(0),
            field: 1,
        });
        bad_field.main.emit(Op::Unit);
        bad_field.main.emit_op_u16(Op::MakeProduct, 0);
        bad_field.main.emit_op_u16(Op::LoadProductField, 0);
        bad_field.main.emit(Op::Return);
        assert!(vm_error(bad_field).contains("product field index out of range"));

        let mut wrong_category = Chunk::new();
        wrong_category.products.push(product.clone());
        wrong_category.product_fields.push(ProductFieldRef {
            product: ProductId::new(0),
            field: 0,
        });
        wrong_category.main.emit(Op::Unit);
        wrong_category.main.emit_op_u16(Op::LoadProductField, 0);
        wrong_category.main.emit(Op::Return);
        assert!(vm_error(wrong_category).contains("expects Product"));

        let mut wrong_identity = Chunk::new();
        wrong_identity.products.push(product);
        wrong_identity.products.push(ProductMetadata {
            id: ProductId::new(1),
            name: "Other".into(),
            fields: vec!["x".into()],
        });
        wrong_identity.product_fields.push(ProductFieldRef {
            product: ProductId::new(1),
            field: 0,
        });
        wrong_identity.main.emit(Op::Unit);
        wrong_identity.main.emit_op_u16(Op::MakeProduct, 0);
        wrong_identity.main.emit_op_u16(Op::LoadProductField, 0);
        wrong_identity.main.emit(Op::Return);
        assert!(vm_error(wrong_identity).contains("identity mismatch"));

        let mut replacement_descriptor = Chunk::new();
        replacement_descriptor.products.push(ProductMetadata {
            id: ProductId::new(0),
            name: "Point".into(),
            fields: vec!["x".into()],
        });
        replacement_descriptor.main.emit(Op::Unit);
        replacement_descriptor.main.emit_op_u16(Op::MakeProduct, 0);
        replacement_descriptor.main.emit(Op::Unit);
        replacement_descriptor
            .main
            .emit_op_u16(Op::WithProductField, 0);
        replacement_descriptor.main.emit(Op::Return);
        assert!(vm_error(replacement_descriptor).contains("descriptor index out of range"));

        let mut replacement_field = Chunk::new();
        replacement_field.products.push(ProductMetadata {
            id: ProductId::new(0),
            name: "Point".into(),
            fields: vec!["x".into()],
        });
        replacement_field.product_fields.push(ProductFieldRef {
            product: ProductId::new(0),
            field: 1,
        });
        replacement_field.main.emit(Op::Unit);
        replacement_field.main.emit_op_u16(Op::MakeProduct, 0);
        replacement_field.main.emit(Op::Unit);
        replacement_field.main.emit_op_u16(Op::WithProductField, 0);
        replacement_field.main.emit(Op::Return);
        assert!(vm_error(replacement_field).contains("product field index out of range"));

        let mut replacement_identity = Chunk::new();
        replacement_identity.products.push(ProductMetadata {
            id: ProductId::new(0),
            name: "Point".into(),
            fields: vec!["x".into()],
        });
        replacement_identity.products.push(ProductMetadata {
            id: ProductId::new(1),
            name: "Other".into(),
            fields: vec!["x".into()],
        });
        replacement_identity.product_fields.push(ProductFieldRef {
            product: ProductId::new(1),
            field: 0,
        });
        replacement_identity.main.emit(Op::Unit);
        replacement_identity.main.emit_op_u16(Op::MakeProduct, 0);
        replacement_identity.main.emit(Op::Unit);
        replacement_identity
            .main
            .emit_op_u16(Op::WithProductField, 0);
        replacement_identity.main.emit(Op::Return);
        assert!(vm_error(replacement_identity).contains("identity mismatch"));

        let mut replacement_category = Chunk::new();
        replacement_category.products.push(ProductMetadata {
            id: ProductId::new(0),
            name: "Point".into(),
            fields: vec!["x".into()],
        });
        replacement_category.product_fields.push(ProductFieldRef {
            product: ProductId::new(0),
            field: 0,
        });
        replacement_category.main.emit(Op::Unit);
        replacement_category.main.emit(Op::Unit);
        replacement_category
            .main
            .emit_op_u16(Op::WithProductField, 0);
        replacement_category.main.emit(Op::Return);
        assert!(vm_error(replacement_category).contains("expects Product"));
    }

    #[test]
    fn malformed_conditions_and_removed_semantic_opcodes_are_errors() {
        let mut not = Chunk::new();
        not.main.emit(Op::Unit);
        not.main.emit(Op::Not);
        not.main.emit(Op::Return);
        assert!(vm_error(not).contains("not expects Bool"));

        let mut branch = Chunk::new();
        branch.main.emit(Op::Unit);
        branch.main.emit_op_u16(Op::JumpIfFalse, 0);
        branch.main.emit(Op::Unit);
        branch.main.emit(Op::Return);
        assert!(vm_error(branch).contains("JumpIfFalse expects Bool"));

        for removed in [21, 82, 145] {
            let mut removed_opcode = Chunk::new();
            removed_opcode.main.code = vec![removed];
            assert!(vm_error(removed_opcode).contains("unknown opcode"));
        }
    }
}
