//! Lower AST forms into a bytecode chunk.

mod emit;

use crate::ast::Expr;
use crate::import::Program;
use emit::{compile_expr, Cx};
use lkjscript2026_core::{Chunk, Constant, Error, FunctionProto, Op, Result};

pub fn compile_program(program: &Program) -> Result<Chunk> {
    let mut chunk = Chunk::new();
    install_builtins(&mut chunk);
    for file in &program.files {
        for form in &file.forms {
            match form {
                Expr::Call { name, .. } if name == "import" => {}
                Expr::Call { name, args } if name == "def" => compile_def(&mut chunk, args)?,
                Expr::Call { name, args } if name == "do" => compile_do(&mut chunk, args)?,
                other => {
                    return Err(Error::msg(format!("unsupported top-level: {other:?}")));
                }
            }
        }
    }
    chunk.main.emit(Op::Nil);
    chunk.main.emit(Op::Return);
    Ok(chunk)
}

fn install_builtins(chunk: &mut Chunk) {
    for name in [
        "+", "-", "*", "/", "=", "!=", "<", "<=", ">", ">=", "not", "cons", "car", "cdr",
        "null?", "print", "flush", "read-byte", "write-byte", "exit",
    ] {
        let gid = chunk.intern_global(name);
        let _ = gid;
    }
}

fn compile_do(chunk: &mut Chunk, args: &[Expr]) -> Result<()> {
    let (mut code, locals) = {
        let mut cx = Cx::new(chunk, Vec::new(), "<do>");
        for (i, e) in args.iter().enumerate() {
            compile_expr(&mut cx, e)?;
            if i + 1 != args.len() {
                cx.proto.emit(Op::Pop);
            }
        }
        (std::mem::take(&mut cx.proto.code), cx.proto.locals)
    };
    if locals > chunk.main.locals {
        chunk.main.locals = locals;
    }
    chunk.main.code.append(&mut code);
    Ok(())
}

fn compile_def(chunk: &mut Chunk, args: &[Expr]) -> Result<()> {
    let (name, body) = parse_def(args)?;
    match body {
        Expr::Call { name: fn_tag, args } if fn_tag == "fn" => {
            compile_fn_def(chunk, &name, args)
        }
        expr => {
            let (mut code, locals) = {
                let mut cx = Cx::new(chunk, Vec::new(), &name);
                compile_expr(&mut cx, expr)?;
                let gid = cx.chunk.intern_global(&name);
                cx.proto.emit_op_u16(Op::StoreGlobal, gid);
                (std::mem::take(&mut cx.proto.code), cx.proto.locals)
            };
            if locals > chunk.main.locals {
                chunk.main.locals = locals;
            }
            chunk.main.code.append(&mut code);
            chunk.main.emit(Op::Pop);
            Ok(())
        }
    }
}

fn parse_def(args: &[Expr]) -> Result<(String, &Expr)> {
    match args {
        [Expr::Call {
            name: n,
            args: name_args,
        }, body]
            if n == "name" =>
        {
            let nm = match name_args.as_slice() {
                [Expr::LitStr(s)] | [Expr::Symbol(s)] => s.clone(),
                [Expr::Call {
                    name: sym,
                    args: a,
                }] if a.is_empty() => sym.clone(),
                _ => return Err(Error::msg("def name must be a symbol/string")),
            };
            Ok((nm, body))
        }
        _ => Err(Error::msg("def expects <name>…</name> and a body")),
    }
}

fn compile_fn_def(chunk: &mut Chunk, name: &str, args: &[Expr]) -> Result<()> {
    let (params, body) = match args {
        [Expr::Call {
            name: p,
            args: plist,
        }, body]
            if p == "params" =>
        {
            let mut names = Vec::new();
            for p in plist {
                match p {
                    Expr::Symbol(s) => names.push(s.clone()),
                    Expr::Call { name: s, args } if args.is_empty() => names.push(s.clone()),
                    _ => return Err(Error::msg("param must be a symbol")),
                }
            }
            (names, body)
        }
        _ => return Err(Error::msg("fn expects <params>…</params> and body")),
    };
    let mut cx = Cx::new(chunk, params.clone(), name);
    cx.proto.arity = params.len() as u8;
    cx.proto.locals = params.len() as u8;
    compile_expr(&mut cx, body)?;
    cx.proto.emit(Op::Return);
    let proto_id = cx.chunk.protos.len() as u32;
    let proto = FunctionProto {
        name: cx.proto.name.clone(),
        arity: cx.proto.arity,
        locals: cx.proto.locals,
        code: std::mem::take(&mut cx.proto.code),
    };
    cx.chunk.protos.push(proto);
    drop(cx);
    let cid = chunk.add_const(Constant::Proto(proto_id));
    chunk.main.emit_op_u16(Op::LoadConst, cid.0);
    chunk.main.emit_op_u16(Op::MakeClosure, 0);
    let gid = chunk.intern_global(name);
    chunk.main.emit_op_u16(Op::StoreGlobal, gid);
    chunk.main.emit(Op::Pop);
    Ok(())
}
