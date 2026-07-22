//! Special-form emitters.

use crate::ast::Expr;
use crate::codegen::emit::{compile_expr, Cx};
use lkjscript_core::{Constant, Error, Op, Result};

pub fn compile_do_expr(cx: &mut Cx<'_>, args: &[Expr]) -> Result<()> {
    if args.is_empty() {
        cx.proto.emit(Op::Nil);
        return Ok(());
    }
    for (i, e) in args.iter().enumerate() {
        compile_expr(cx, e)?;
        if i + 1 != args.len() {
            cx.proto.emit(Op::Pop);
        }
    }
    Ok(())
}

pub fn compile_if(cx: &mut Cx<'_>, args: &[Expr]) -> Result<()> {
    let (cond, then_e, else_e) = match args {
        [c, t, e] => (c, t, e),
        [c, t] => (c, t, &Expr::LitNil),
        _ => return Err(Error::msg("if expects 2 or 3 args")),
    };
    compile_expr(cx, cond)?;
    cx.proto.emit(Op::JumpIfFalse);
    let else_jump = cx.proto.len();
    cx.proto.emit_u16(0);
    compile_expr(cx, then_e)?;
    cx.proto.emit(Op::Jump);
    let end_jump = cx.proto.len();
    cx.proto.emit_u16(0);
    let else_at = cx.proto.len() as u16;
    cx.proto.patch_u16(else_jump, else_at);
    compile_expr(cx, else_e)?;
    let end_at = cx.proto.len() as u16;
    cx.proto.patch_u16(end_jump, end_at);
    Ok(())
}

/// `<while><cond/>body…</while>` — stack-safe loop; yields nil.
pub fn compile_while(cx: &mut Cx<'_>, args: &[Expr]) -> Result<()> {
    if args.is_empty() {
        return Err(Error::msg("while needs a condition"));
    }
    let cond = &args[0];
    let body = &args[1..];
    let loop_start = cx.proto.len() as u16;
    compile_expr(cx, cond)?;
    cx.proto.emit(Op::JumpIfFalse);
    let exit_jump = cx.proto.len();
    cx.proto.emit_u16(0);
    if body.is_empty() {
        cx.proto.emit(Op::Nil);
    } else {
        for (i, e) in body.iter().enumerate() {
            compile_expr(cx, e)?;
            if i + 1 != body.len() {
                cx.proto.emit(Op::Pop);
            }
        }
    }
    cx.proto.emit(Op::Pop);
    cx.proto.emit(Op::Jump);
    let back = cx.proto.len();
    cx.proto.emit_u16(0);
    cx.proto.patch_u16(back, loop_start);
    let end_at = cx.proto.len() as u16;
    cx.proto.patch_u16(exit_jump, end_at);
    cx.proto.emit(Op::Nil);
    Ok(())
}

pub fn compile_let(cx: &mut Cx<'_>, args: &[Expr]) -> Result<()> {
    if args.is_empty() {
        return Err(Error::msg("let needs body"));
    }
    let body = &args[args.len() - 1];
    let binds = &args[..args.len() - 1];
    let saved = cx.locals.len();
    for b in binds {
        let (name, val) = match b {
            Expr::Call { name, args } if name == "bind" && args.len() == 2 => {
                let nm = match &args[0] {
                    Expr::Symbol(s) => s.clone(),
                    Expr::Call { name: s, args } if args.is_empty() => s.clone(),
                    _ => return Err(Error::msg("bind name must be symbol")),
                };
                (nm, &args[1])
            }
            _ => return Err(Error::msg("let bindings must be <bind>…</bind>")),
        };
        compile_expr(cx, val)?;
        let slot = cx.locals.len() as u8;
        cx.locals.push(name);
        if cx.proto.locals < cx.locals.len() as u8 {
            cx.proto.locals = cx.locals.len() as u8;
        }
        cx.proto.emit_op_u8(Op::StoreLocal, slot);
        cx.proto.emit(Op::Pop);
    }
    compile_expr(cx, body)?;
    cx.locals.truncate(saved);
    Ok(())
}

pub fn compile_quote(cx: &mut Cx<'_>, args: &[Expr]) -> Result<()> {
    match args {
        [Expr::Symbol(s)] => {
            let cid = cx.chunk.add_const(Constant::Str(format!("sym:{s}")));
            cx.proto.emit_op_u16(Op::LoadConst, cid.0);
            Ok(())
        }
        [Expr::Call { name: s, args: a }] if a.is_empty() => {
            let cid = cx.chunk.add_const(Constant::Str(format!("sym:{s}")));
            cx.proto.emit_op_u16(Op::LoadConst, cid.0);
            Ok(())
        }
        [e] => compile_expr(cx, e),
        _ => Err(Error::msg("quote expects one form")),
    }
}

pub fn compile_logic(cx: &mut Cx<'_>, name: &str, args: &[Expr]) -> Result<()> {
    if args.is_empty() {
        cx.proto
            .emit(if name == "and" { Op::True } else { Op::False });
        return Ok(());
    }
    if args.len() == 1 {
        return compile_expr(cx, &args[0]);
    }
    let mut acc = args[args.len() - 1].clone();
    for e in args[..args.len() - 1].iter().rev() {
        if name == "or" {
            acc = Expr::Call {
                name: "if".into(),
                args: vec![e.clone(), Expr::LitBool(true), acc],
            };
        } else {
            acc = Expr::Call {
                name: "if".into(),
                args: vec![e.clone(), acc, Expr::LitBool(false)],
            };
        }
    }
    compile_expr(cx, &acc)
}

pub fn compile_set(cx: &mut Cx<'_>, args: &[Expr]) -> Result<()> {
    let (name, val) = match args {
        [Expr::Symbol(s), v] => (s.clone(), v),
        [Expr::Call { name: s, args: a }, v] if a.is_empty() => (s.clone(), v),
        _ => return Err(Error::msg("set expects <name/> and value")),
    };
    compile_expr(cx, val)?;
    let gid = cx.chunk.intern_global(&name);
    cx.proto.emit_op_u16(Op::StoreGlobal, gid);
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::codegen::emit::Cx;
    use lkjscript_core::Chunk;

    #[test]
    fn while_emits_back_edge() {
        let mut chunk = Chunk::default();
        let mut cx = Cx::new(&mut chunk, Vec::new(), "t");
        let cond = Expr::LitBool(false);
        compile_while(&mut cx, &[cond]).unwrap();
        let code = &cx.proto.code;
        assert!(code.contains(&(Op::Jump as u8)));
        assert!(code.contains(&(Op::JumpIfFalse as u8)));
    }
}
