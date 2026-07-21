//! Bytecode interpreter.

mod calls;
mod dispatch;
mod ext_ops;
mod numeric;

use lkjscript2026_core::{Chunk, Constant, Error, HeapObj, JitHook, Result, Value};

use crate::arena::Arena;
use crate::host_ext::FdTable;

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
    pub fds: FdTable,
}

impl<'a, J: JitHook> Vm<'a, J> {
    pub fn new(chunk: &'a Chunk, jit: J, args: Vec<String>) -> Self {
        Self {
            chunk,
            globals: vec![Value::NIL; chunk.global_names.len()],
            stack: Vec::new(),
            frames: Vec::new(),
            arena: Arena::default(),
            jit,
            exit_code: None,
            args,
            fds: FdTable::default(),
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
            self.stack.push(Value::NIL);
        }
        loop {
            if let Some(code) = self.exit_code {
                crate::host_term::restore_tty();
                std::process::exit(code);
            }
            if self.frames.is_empty() {
                return Ok(self.pop());
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
            Ok(&self.chunk.protos[fr.proto as usize].code)
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
            &self.chunk.protos[proto as usize].code
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

    pub fn pop(&mut self) -> Value {
        self.stack.pop().unwrap_or(Value::NIL)
    }

    pub fn peek(&self) -> Value {
        self.stack.last().copied().unwrap_or(Value::NIL)
    }

    pub fn load_const(&mut self, id: usize) -> Result<Value> {
        match self
            .chunk
            .constants
            .get(id)
            .ok_or_else(|| Error::msg("bad const"))?
        {
            Constant::Value(v) => Ok(*v),
            Constant::Float(f) => Ok(self.arena.alloc(HeapObj::Float(*f))),
            Constant::Str(s) => {
                if let Some(sym) = s.strip_prefix("sym:") {
                    Ok(self.arena.alloc(HeapObj::Symbol(sym.to_string())))
                } else {
                    Ok(self.arena.alloc(HeapObj::Str(s.clone())))
                }
            }
            Constant::Proto(p) => Ok(Value::from_int(*p as i64)),
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
            let ret = self.pop();
            if let Some(frame) = self.frames.pop() {
                self.stack.truncate(frame.stack_base);
            }
            self.push(ret);
            return Ok(());
        }
        let op = self.read_u8()?;
        dispatch::dispatch(self, op)
    }
}
