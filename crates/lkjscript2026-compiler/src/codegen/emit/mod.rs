//! Expression emission.

mod call;
mod special;

use crate::ast::Expr;
use lkjscript2026_core::{Chunk, Constant, Error, FunctionProto, Op, Result, Value};

pub struct Cx<'a> {
    pub chunk: &'a mut Chunk,
    pub locals: Vec<String>,
    pub proto: FunctionProto,
}

impl<'a> Cx<'a> {
    pub fn new(chunk: &'a mut Chunk, locals: Vec<String>, name: &str) -> Self {
        Self {
            chunk,
            locals,
            proto: FunctionProto {
                name: name.into(),
                arity: 0,
                locals: 0,
                code: Vec::new(),
            },
        }
    }
}

pub fn compile_expr(cx: &mut Cx<'_>, expr: &Expr) -> Result<()> {
    match expr {
        Expr::LitNil => cx.proto.emit(Op::Nil),
        Expr::LitBool(true) => cx.proto.emit(Op::True),
        Expr::LitBool(false) => cx.proto.emit(Op::False),
        Expr::LitNum {
            value: n,
            float_syntax: _,
        } => emit_number(cx, *n),
        Expr::LitStr(s) => {
            let cid = cx.chunk.add_const(Constant::Str(s.clone()));
            cx.proto.emit_op_u16(Op::LoadConst, cid.0);
        }
        Expr::Symbol(name) => compile_name(cx, name)?,
        Expr::Call { name, args } => compile_call(cx, name, args)?,
        Expr::List(_) => return Err(Error::msg("bare list not supported")),
    }
    Ok(())
}

fn emit_number(cx: &mut Cx<'_>, n: f64) {
    if n.fract() == 0.0 && n >= (i32::MIN as f64) && n <= (i32::MAX as f64) {
        let cid = cx
            .chunk
            .add_const(Constant::Value(Value::from_int(n as i64)));
        cx.proto.emit_op_u16(Op::LoadConst, cid.0);
    } else {
        let cid = cx.chunk.add_const(Constant::Float(n));
        cx.proto.emit_op_u16(Op::LoadConst, cid.0);
    }
}

pub(crate) fn compile_name(cx: &mut Cx<'_>, name: &str) -> Result<()> {
    if let Some(i) = cx.locals.iter().position(|n| n == name) {
        cx.proto.emit_op_u8(Op::LoadLocal, i as u8);
        return Ok(());
    }
    let gid = cx.chunk.intern_global(name);
    cx.proto.emit_op_u16(Op::LoadGlobal, gid);
    Ok(())
}

pub fn compile_call(cx: &mut Cx<'_>, name: &str, args: &[Expr]) -> Result<()> {
    let name = match name {
        "lte" => "<=",
        "gte" => ">=",
        "lt" => "<",
        "gt" => ">",
        "eq" => "=",
        "ne" => "!=",
        "div" => "/",
        other => other,
    };
    match name {
        "if" => special::compile_if(cx, args),
        "while" => special::compile_while(cx, args),
        "let" => special::compile_let(cx, args),
        "quote" => special::compile_quote(cx, args),
        "do" => special::compile_do_expr(cx, args),
        "fn" => Err(Error::msg("bare fn only allowed in def")),
        "and" | "or" => special::compile_logic(cx, name, args),
        "set" => special::compile_set(cx, args),
        _ => call::compile_plain_call(cx, name, args),
    }
}
