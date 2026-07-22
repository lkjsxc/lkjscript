//! Lower AST forms into a bytecode chunk.

mod emit;

use crate::ast::Expr;
use crate::import::Program;
use crate::types::typecheck_program;
use emit::{compile_expr, Cx};
use lkjscript_core::{Chunk, Constant, Error, FunctionProto, Op, Result};

pub fn compile_program(program: &Program) -> Result<Chunk> {
    typecheck_program(program)?;
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
        "+",
        "-",
        "*",
        "/",
        "=",
        "!=",
        "<",
        "<=",
        ">",
        ">=",
        "not",
        "cons",
        "car",
        "cdr",
        "null?",
        "print",
        "flush",
        "read-byte",
        "write-byte",
        "exit",
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
    let name = def_name(args)?;
    match args {
        [_, Expr::Call { name: t, .. }, val] if t == "type" => {
            let (mut code, locals) = {
                let mut cx = Cx::new(chunk, Vec::new(), &name);
                compile_expr(&mut cx, val)?;
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
        [_, Expr::Call {
            name: fn_tag,
            args: fa,
        }] if fn_tag == "fn" => compile_fn_def(chunk, &name, fa),
        _ => Err(Error::msg("def expects fn/ or type/ value form")),
    }
}

fn def_name(args: &[Expr]) -> Result<String> {
    match args.first() {
        Some(Expr::Call {
            name: n,
            args: name_args,
        }) if n == "name" => match name_args.as_slice() {
            [Expr::LitStr(s)] | [Expr::Symbol(s)] => Ok(s.clone()),
            [Expr::Call { name: sym, args: a }] if a.is_empty() => Ok(sym.clone()),
            _ => Err(Error::msg("def name must be a symbol/string")),
        },
        _ => Err(Error::msg("def expects name/ … /name first")),
    }
}

fn compile_fn_def(chunk: &mut Chunk, name: &str, args: &[Expr]) -> Result<()> {
    let (params, body) = parse_fn_compile(args)?;
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

fn parse_fn_compile(args: &[Expr]) -> Result<(Vec<String>, &Expr)> {
    let mut params = None;
    let mut body = None;
    let mut saw_sig = false;
    for a in args {
        match a {
            Expr::Call { name, .. } if name == "sig" || name == "forall" => {
                if name == "sig" {
                    saw_sig = true;
                }
            }
            Expr::Call {
                name: p,
                args: plist,
            } if p == "params" => {
                let mut names = Vec::new();
                let mut i = 0;
                // typed params: name Type name Type …
                while i < plist.len() {
                    match &plist[i] {
                        Expr::Symbol(s) => names.push(s.clone()),
                        Expr::Call { name: s, args } if args.is_empty() => names.push(s.clone()),
                        _ => return Err(Error::msg("param must be a symbol")),
                    }
                    i += 1;
                    if i < plist.len() {
                        // skip type atom
                        i += 1;
                    }
                }
                params = Some(names);
            }
            other => {
                if body.is_some() {
                    return Err(Error::msg("fn expects one body"));
                }
                body = Some(other);
            }
        }
    }
    if !saw_sig {
        return Err(Error::msg("fn missing mandatory sig/ … /sig"));
    }
    let params = params.ok_or_else(|| Error::msg("fn expects params/ … /params"))?;
    let body = body.ok_or_else(|| Error::msg("fn expects body"))?;
    Ok((params, body))
}
